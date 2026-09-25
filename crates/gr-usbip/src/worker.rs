//! Unprivileged local device worker. No privileged setup or application-supplied
//! descriptors. PCM and USB data stay off the broker command socket.
use crate::{
    Direction, HEADER_BYTES, Header, MAX_FRAME_BYTES, MAX_ISO_PACKETS, MAX_TRANSFER_BYTES, Request,
    control::{Completion, ControlState},
    pending::{Pending, Ticket},
    profile::Profile,
};
use gr_audio_contract::queue::{PcmConsumer, PcmProducer};
use gr_hid::{Reply, Report, RequestKind};
use std::{
    io::{self, Read},
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

mod completion_queue;
use completion_queue::CompletionQueue;

const SLOT_COUNT: usize = 32;
/// Implemented by the trusted worker's controller personality, never privileged
/// broker code. Required report replies are synchronous and never optional events.
pub trait HidHandler {
    /// Service trusted worker control independently of host HID polling.
    fn service(&mut self, _now_us: u64) -> io::Result<()> {
        Ok(())
    }
    fn request(&mut self, request: &RequestKind, now_us: u64) -> Reply;
    /// Current compiled personalities must always supply an input report.
    /// `None` indicates terminal personality failure, not a USB endpoint stall.
    fn input(&mut self, now_us: u64) -> Option<Report>;
    fn output(&mut self, report: &[u8], now_us: u64) -> bool;
}
/// Diagnostics survive worker shutdown when retained by the session owner.
#[derive(Default)]
pub struct Counters {
    pub microphone_silence_frames: AtomicU64,
    pub completed_transfers: AtomicU64,
    pub stalled_transfers: AtomicU64,
    pub inactive_audio_transfers: AtomicU64,
    pub playback_frames: AtomicU64,
    pub capture_frames: AtomicU64,
    /// Actual microphone queue consumption, before USB completion batching.
    pub microphone_consumed_frames: AtomicU64,
    /// Serviced USB capture media time, including underrun silence. A request
    /// canceled after packet collection still advances this scheduling credit.
    pub microphone_host_frames: AtomicU64,
    /// IPC microphone frames discarded when host capture stops consuming.
    pub microphone_queue_dropped_frames: AtomicU64,
    /// Collected microphone frames abandoned before a USB completion.
    pub abandoned_capture_frames: AtomicU64,
    pub maximum_audio_lateness_us: AtomicU64,
}
impl Counters {
    fn record_transfer(
        &self,
        header: Header,
        profile: &Profile,
        status: i32,
        actual: usize,
    ) -> io::Result<()> {
        if status != 0 {
            self.stalled_transfers.fetch_add(1, Ordering::Relaxed);
            if header.packets() != 0 && matches!(header.endpoint(), 1 | 2) {
                self.inactive_audio_transfers
                    .fetch_add(1, Ordering::Relaxed);
            }
        } else if header.packets() != 0 {
            let (counter, channels) = if header.direction() == Direction::Out {
                (&self.playback_frames, profile.playback_channels())
            } else {
                (&self.capture_frames, profile.microphone_channels())
            };
            counter.fetch_add(
                u64::try_from(actual / (usize::from(channels) * 2)).map_err(error)?,
                Ordering::Relaxed,
            );
        }
        Ok(())
    }
}

struct Slot {
    bytes: Vec<u8>,
    length: usize,
    ticket: Option<Ticket>,
    ready: u64,
    order: u64,
    stream: usize,
    capture_packets: usize,
    captured: usize,
    capture_data: Vec<u8>,
}
/// One authenticated connection-owned device. Its socket is provided only by the
/// broker to trusted worker code, not directly to an application process.
pub struct Worker<H> {
    socket: UnixStream,
    device: u32,
    profile: Profile,
    control: ControlState,
    hid: H,
    playback: PcmProducer,
    microphone: PcmConsumer,
    counters: Arc<Counters>,
    pending: Pending,
    slots: Vec<Slot>,
    receive: Vec<u8>,
    received: usize,
    expected: usize,
    frame_started: Option<u64>,
    next_order: u64,
    outgoing: CompletionQueue,
}
impl<H: HidHandler> Worker<H> {
    /// Allocate all USB frame storage before streaming starts.
    pub fn new(
        socket: UnixStream,
        device: u32,
        profile: Profile,
        hid: H,
        playback: PcmProducer,
        microphone: PcmConsumer,
        counters: Arc<Counters>,
    ) -> io::Result<Self> {
        if !profile.accepts_playback(playback.format())
            || !profile.accepts_microphone(microphone.format())
        {
            return Err(error(
                "PCM format or channel roles do not match the compiled USB profile",
            ));
        }
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            device,
            profile,
            control: ControlState::default(),
            hid,
            playback,
            microphone,
            counters,
            pending: Pending::new(u64::from(device)),
            slots: (0..SLOT_COUNT)
                .map(|_| Slot {
                    bytes: vec![0; MAX_FRAME_BYTES],
                    length: 0,
                    ticket: None,
                    ready: 0,
                    order: 0,
                    stream: 0,
                    capture_packets: 0,
                    captured: 0,
                    capture_data: vec![0; MAX_TRANSFER_BYTES],
                })
                .collect(),
            receive: vec![0; MAX_FRAME_BYTES],
            received: 0,
            expected: HEADER_BYTES,
            frame_started: None,
            next_order: 0,
            outgoing: CompletionQueue::new(SLOT_COUNT, MAX_FRAME_BYTES),
        })
    }
    /// Runs independently from GUI/HID caller polling. Stop or any I/O/protocol
    /// failure terminally closes this connection and its stream endpoints.
    pub fn run(mut self, stop: &AtomicBool) -> io::Result<()> {
        let result = self.run_inner(stop);
        let _ = self.socket.shutdown(std::net::Shutdown::Both);
        self.pending.cancel_all();
        for slot in &self.slots {
            if slot.ticket.is_some() {
                self.counters
                    .abandoned_capture_frames
                    .fetch_add(slot.captured as u64 * 48, Ordering::Relaxed);
            }
        }
        self.playback.close();
        self.microphone.close();
        result
    }
    fn run_inner(&mut self, stop: &AtomicBool) -> io::Result<()> {
        let started = Instant::now();
        let mut reply = vec![0; MAX_FRAME_BYTES];
        let mut data = vec![0; MAX_TRANSFER_BYTES];
        while !stop.load(Ordering::Acquire) {
            let now = u64::try_from(started.elapsed().as_micros()).map_err(error)?;
            self.hid.service(now)?;
            for _ in 0..SLOT_COUNT {
                if !self.receive_one(now, &mut reply)? {
                    break;
                }
            }
            while let Some(index) = self.next_ready(now) {
                if self.capture_due(index).is_some_and(|due| due <= now) {
                    self.counters.maximum_audio_lateness_us.fetch_max(
                        now.saturating_sub(self.capture_due(index).unwrap_or(now)),
                        Ordering::Relaxed,
                    );
                    self.capture_one(index)?;
                    continue;
                }
                let size = self.complete(index, now, &mut reply, &mut data)?;
                self.outgoing.push(&reply[..size], now)?;
                self.outgoing.flush(&mut self.socket, now)?;
                if crate::word(&reply, 20) != 0 {
                    self.counters
                        .abandoned_capture_frames
                        .fetch_add(self.slots[index].captured as u64 * 48, Ordering::Relaxed);
                }
                self.slots[index].ticket = None;
                self.counters
                    .completed_transfers
                    .fetch_add(1, Ordering::Relaxed);
            }
            if self.pending.expire(now).map_err(error)?.is_some() {
                return Err(error("USB request deadline exceeded"));
            }
            if self
                .frame_started
                .is_some_and(|start| now.saturating_sub(start) >= 1_000_000)
            {
                return Err(error("partial USB/IP frame deadline exceeded"));
            }
            self.outgoing.flush(&mut self.socket, now)?;
            self.counters
                .microphone_queue_dropped_frames
                .store(self.microphone.discarded_frames(), Ordering::Relaxed);
            self.wait_ready(started)?;
        }
        Ok(())
    }
    fn wait_ready(&self, started: Instant) -> io::Result<()> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};
        let now = u64::try_from(started.elapsed().as_micros()).map_err(error)?;
        // The stop flag is checked at least every 10 ms even when idle. During
        // streaming, packet/completion deadlines bound the wait more tightly.
        let due = self
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.ticket.is_some())
            .map(|(index, slot)| self.capture_due(index).unwrap_or(slot.ready))
            .chain(self.pending.next_deadline())
            .chain(self.frame_started.map(|t| t.saturating_add(1_000_000)))
            .chain(self.outgoing.deadline())
            .min()
            .unwrap_or(now.saturating_add(10_000));
        let timeout =
            Timespec::try_from(Duration::from_micros(due.saturating_sub(now).min(10_000)))
                .map_err(error)?;
        let mut flags = PollFlags::IN;
        if !self.outgoing.is_empty() {
            flags |= PollFlags::OUT;
        }
        let mut fds = [PollFd::new(&self.socket, flags)];
        match poll(&mut fds, Some(&timeout)) {
            Ok(_) | Err(rustix::io::Errno::INTR) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    fn next_ready(&self, now: u64) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(index, slot)| {
                slot.ticket.is_some() && self.capture_due(*index).unwrap_or(slot.ready) <= now
            })
            .min_by_key(|(index, slot)| {
                (self.capture_due(*index).unwrap_or(slot.ready), slot.order)
            })
            .map(|(index, _)| index)
    }
    fn capture_due(&self, index: usize) -> Option<u64> {
        let slot = &self.slots[index];
        (self.control.microphone_active() && slot.captured < slot.capture_packets)
            .then(|| slot.ready - (slot.capture_packets - slot.captured - 1) as u64 * 1000)
    }
    fn capture_one(&mut self, index: usize) -> io::Result<()> {
        let slot = &mut self.slots[index];
        let (Request::Submit(request), _) =
            crate::decode(&slot.bytes[..slot.length], self.device).map_err(error)?
        else {
            return Err(error("non-submit capture"));
        };
        let packet = request
            .packets()
            .nth(slot.captured)
            .ok_or_else(|| error("missing capture packet"))?;
        let channels = usize::from(self.profile.microphone_channels());
        let offset = slot.captured * 48 * channels * 2;
        capture_packet(
            packet.length,
            &mut self.microphone,
            channels,
            &self.counters,
            &mut slot.capture_data[offset..],
        )?;
        slot.captured += 1;
        Ok(())
    }
    fn receive_one(&mut self, now: u64, reply: &mut [u8]) -> io::Result<bool> {
        if self.received < self.expected {
            match self
                .socket
                .read(&mut self.receive[self.received..self.expected])
            {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "USB/IP peer closed",
                    ));
                }
                Ok(n) => {
                    self.frame_started.get_or_insert(now);
                    self.received += n;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(false),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(true),
                Err(e) => return Err(e),
            }
        }
        if self.received < self.expected {
            return Ok(true);
        }
        let header = Header::decode(&self.receive[..self.received], self.device).map_err(error)?;
        self.expected = header.wire_bytes();
        if self.received < self.expected {
            return Ok(true);
        }
        let (request, _) =
            crate::decode(&self.receive[..self.received], self.device).map_err(error)?;
        match request {
            Request::Unlink { sequence, target } => {
                let cancelled = self.pending.unlink(target);
                for slot in &mut self.slots {
                    if slot.ticket.is_some_and(|t| t.sequence() == target) {
                        self.counters
                            .abandoned_capture_frames
                            .fetch_add(slot.captured as u64 * 48, Ordering::Relaxed);
                        slot.ticket = None;
                    }
                }
                let size = crate::unlink_reply(reply, sequence, cancelled).map_err(error)?;
                self.outgoing.push(&reply[..size], now)?;
                self.outgoing.flush(&mut self.socket, now)?;
            }
            Request::Submit(_) => {
                let stream = usize::from(header.endpoint())
                    + if header.direction() == Direction::In {
                        16
                    } else {
                        0
                    };
                let delay = if header.packets() != 0 {
                    u64::try_from(header.packets()).map_err(error)? * 1000
                } else if header.endpoint() == 3 && header.direction() == Direction::In {
                    4000
                } else {
                    0
                };
                let ready = self
                    .slots
                    .iter()
                    .filter(|slot| slot.ticket.is_some() && slot.stream == stream)
                    .map(|slot| slot.ready)
                    .max()
                    .unwrap_or(now)
                    .max(now)
                    .checked_add(delay)
                    .ok_or_else(|| error("USB clock overflow"))?;
                let slot = self
                    .slots
                    .iter_mut()
                    .find(|slot| slot.ticket.is_none())
                    .ok_or_else(|| error("USB pending quota reached"))?;
                let ticket = self.pending.insert(header, now, 1_000_000).map_err(error)?;
                std::mem::swap(&mut slot.bytes, &mut self.receive);
                slot.length = self.received;
                slot.ticket = Some(ticket);
                slot.ready = ready;
                slot.stream = stream;
                slot.capture_packets =
                    if header.endpoint() == 2 && header.direction() == Direction::In {
                        header.packets()
                    } else {
                        0
                    };
                slot.captured = 0;
                slot.order = self.next_order;
                self.next_order = self
                    .next_order
                    .checked_add(1)
                    .ok_or_else(|| error("USB ordering clock overflow"))?;
            }
        }
        self.received = 0;
        self.expected = HEADER_BYTES;
        self.frame_started = None;
        Ok(true)
    }
    fn complete(
        &mut self,
        index: usize,
        now: u64,
        reply: &mut [u8],
        data: &mut [u8],
    ) -> io::Result<usize> {
        let slot = &self.slots[index];
        let ticket = slot.ticket.ok_or_else(|| error("missing USB ticket"))?;
        self.pending.complete(ticket, now).map_err(error)?;
        let (Request::Submit(request), _) =
            crate::decode(&slot.bytes[..slot.length], self.device).map_err(error)?
        else {
            return Err(error("non-submit queued"));
        };
        let header = request.header();
        if header.packets() != 0 {
            self.counters
                .maximum_audio_lateness_us
                .fetch_max(now.saturating_sub(slot.ready), Ordering::Relaxed);
        }
        let mut iso_actual = [0_u32; MAX_ISO_PACKETS];
        let mut actual = 0;
        let mut status = -32; // Explicit STALL for unsupported endpoints/requests.
        match (header.endpoint(), header.direction(), header.packets()) {
            (0, direction, 0)
                if (header.setup()[0] & 0x80 != 0) == (direction == Direction::In)
                    && usize::from(u16::from_le_bytes([header.setup()[6], header.setup()[7]]))
                        == header.transfer_bytes() =>
            {
                let result = self
                    .control
                    .handle(&self.profile, header.setup(), request.payload());
                let response = control_response(result, &mut self.hid, now);
                if let Some(bytes) = response {
                    actual = if direction == Direction::In {
                        bytes.len().min(header.transfer_bytes()).min(data.len())
                    } else {
                        request.payload().len()
                    };
                    if direction == Direction::In {
                        data[..actual].copy_from_slice(&bytes[..actual]);
                    }
                    status = 0;
                }
            }
            (3, Direction::In, 0) if self.control.configured() => {
                let report = self
                    .hid
                    .input(now)
                    .ok_or_else(|| error("required HID input unavailable"))?;
                if report.kind != gr_hid::ReportType::Input {
                    return Err(error("personality returned a non-input report"));
                }
                let bytes = report.wire();
                actual = bytes.len().min(header.transfer_bytes()).min(data.len());
                data[..actual].copy_from_slice(&bytes[..actual]);
                status = 0;
            }
            (4, Direction::Out, 0)
                if self.control.configured() && self.hid.output(request.payload(), now) =>
            {
                actual = request.payload().len();
                status = 0;
            }
            (1, Direction::Out, packets) if packets > 0 && self.control.playback_active() => {
                actual = playback_packets(
                    &request,
                    &mut self.playback,
                    usize::from(self.profile.playback_channels()),
                    &mut iso_actual,
                )?;
                status = 0;
            }
            (2, Direction::In, packets) if packets > 0 && self.control.microphone_active() => {
                actual = capture_packets(
                    &request,
                    slot,
                    &mut self.microphone,
                    &self.counters,
                    data,
                    &mut iso_actual,
                )?;
                status = 0;
            }
            _ => {}
        }
        self.counters
            .record_transfer(header, &self.profile, status, actual)?;
        let payload = if header.direction() == Direction::In {
            &data[..actual]
        } else {
            &[]
        };
        request
            .complete(
                reply,
                status,
                payload,
                actual,
                &iso_actual[..header.packets()],
            )
            .map_err(error)
    }
}
fn error(value: impl std::fmt::Debug) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("USB worker: {value:?}"))
}

fn capture_packets(
    request: &crate::Submit<'_>,
    slot: &Slot,
    microphone: &mut PcmConsumer,
    counters: &Counters,
    data: &mut [u8],
    iso_actual: &mut [u32],
) -> io::Result<usize> {
    let channels = microphone.format().channels().len();
    let bytes = 48 * channels * 2;
    let mut actual = 0;
    for (i, packet) in request.packets().enumerate() {
        if i < slot.captured {
            data[actual..actual + bytes]
                .copy_from_slice(&slot.capture_data[actual..actual + bytes]);
        } else {
            capture_packet(
                packet.length,
                microphone,
                channels,
                counters,
                &mut data[actual..],
            )?;
        }
        iso_actual[i] = u32::try_from(bytes).map_err(error)?;
        actual += bytes;
    }
    Ok(actual)
}

fn capture_packet(
    length: u32,
    microphone: &mut PcmConsumer,
    channels: usize,
    counters: &Counters,
    data: &mut [u8],
) -> io::Result<usize> {
    let mut samples = [0_i16; 96];
    let count = 48 * channels;
    let bytes = count * 2;
    if length < u32::try_from(bytes).map_err(error)? || bytes > data.len() {
        return Err(error("invalid capture ISO packet"));
    }
    let read = microphone.read(&mut samples[..count]).map_err(error)?;
    counters.microphone_consumed_frames.fetch_add(
        u64::try_from(read.frames).map_err(error)?,
        Ordering::Relaxed,
    );
    counters.microphone_silence_frames.fetch_add(
        u64::try_from(48 - read.frames).map_err(error)?,
        Ordering::Relaxed,
    );
    counters
        .microphone_host_frames
        .fetch_add(48, Ordering::Relaxed);
    for (pair, sample) in data[..bytes].chunks_exact_mut(2).zip(&samples[..count]) {
        pair.copy_from_slice(&sample.to_le_bytes());
    }
    Ok(bytes)
}

fn playback_packets(
    request: &crate::Submit<'_>,
    playback: &mut PcmProducer,
    channels: usize,
    iso_actual: &mut [u32],
) -> io::Result<usize> {
    let mut actual = 0;
    let mut samples = [0_i16; 196];
    for (i, packet) in request.packets().enumerate() {
        let start = usize::try_from(packet.offset).map_err(error)?;
        let size = usize::try_from(packet.length).map_err(error)?;
        if size > samples.len() * 2 || size % (channels * 2) != 0 {
            return Err(error("invalid playback ISO packet"));
        }
        for (sample, pair) in samples[..size / 2]
            .iter_mut()
            .zip(request.payload()[start..start + size].chunks_exact(2))
        {
            *sample = i16::from_le_bytes([pair[0], pair[1]]);
        }
        let frames = playback.push(&samples[..size / 2]).map_err(error)?;
        playback
            .discard(u64::try_from(size / (channels * 2) - frames).map_err(error)?)
            .map_err(error)?;
        iso_actual[i] = packet.length;
        actual += size;
    }
    Ok(actual)
}

fn control_response(result: Completion, hid: &mut impl HidHandler, now: u64) -> Option<Vec<u8>> {
    match result {
        Completion::Data(bytes) => Some(bytes),
        Completion::Stall => None,
        Completion::Hid(kind) => match (hid.request(&kind, now), kind) {
            (Reply::Get(Ok(report)), RequestKind::Get { kind, id })
                if report.kind == kind && report.id() == id =>
            {
                Some(report.wire())
            }
            (Reply::Set(Ok(())), RequestKind::Set(_)) => Some(vec![]),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests;
