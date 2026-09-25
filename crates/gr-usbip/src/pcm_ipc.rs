//! Implementation-only, versioned PCM channels between the client and worker.
//! One anonymous stream socket per direction; no broker messages or descriptors.
//! Buffers are fixed at construction and sample operations never allocate.
use crate::profile::ProfileId;
use std::{
    io::{self, Read, Write},
    net::Shutdown,
    os::unix::net::UnixStream,
};

const HEADER: usize = 40;
const BLOCK_FRAMES: usize = 128;
const MAX_BYTES: usize = HEADER + BLOCK_FRAMES * 4 * 2;
const DEADLINE_US: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Playback,
    Microphone,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Format {
    profile: ProfileId,
    direction: Direction,
    generation: u64,
}
impl Format {
    pub fn new(profile: ProfileId, direction: Direction, generation: u64) -> io::Result<Self> {
        if generation == 0 {
            return Err(invalid("zero PCM generation"));
        }
        Ok(Self {
            profile,
            direction,
            generation,
        })
    }
    #[must_use]
    pub const fn channels(self) -> usize {
        match (self.profile, self.direction) {
            (ProfileId::DualSenseEmulated, Direction::Playback) => 4,
            (ProfileId::DualSenseEmulated, Direction::Microphone) | (_, Direction::Playback) => 2,
            (_, Direction::Microphone) => 1,
        }
    }
    fn tag(self) -> u8 {
        match self.profile {
            ProfileId::DualSenseEmulated => 1,
            ProfileId::DualShock4Emulated => 2,
            ProfileId::Xbox360HidEmulated => 3,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Block {
    pub frames: usize,
    pub first_frame: u64,
    pub discontinuity: bool,
    /// Producer's monotonic timestamp, not an end-to-end latency measurement.
    pub produced_at_us: u64,
}
fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}
fn encode(
    format: Format,
    block: Block,
    samples: &[i16],
    out: &mut [u8; MAX_BYTES],
) -> io::Result<usize> {
    if block.frames == 0
        || block.frames > BLOCK_FRAMES
        || samples.len() != block.frames * format.channels()
        || block.first_frame.checked_add(block.frames as u64).is_none()
    {
        return Err(invalid("invalid PCM block"));
    }
    out[..HEADER].fill(0);
    out[..4].copy_from_slice(b"VGPA");
    out[4] = 1; // Version; byte 5 reserved.
    out[6] = format.tag();
    out[7] = u8::from(format.direction == Direction::Microphone);
    out[8..16].copy_from_slice(&format.generation.to_le_bytes());
    out[16..24].copy_from_slice(&block.first_frame.to_le_bytes());
    out[24..28].copy_from_slice(
        &u32::try_from(block.frames)
            .map_err(|_| invalid("PCM frame count"))?
            .to_le_bytes(),
    );
    out[28] = u8::from(block.discontinuity); // 29..32 reserved.
    out[32..40].copy_from_slice(&block.produced_at_us.to_le_bytes());
    for (pair, sample) in out[HEADER..].chunks_exact_mut(2).zip(samples) {
        pair.copy_from_slice(&sample.to_le_bytes());
    }
    Ok(HEADER + samples.len() * 2)
}
fn decode(format: Format, bytes: &[u8]) -> io::Result<Block> {
    if bytes.len() < HEADER
        || &bytes[..4] != b"VGPA"
        || bytes[4..6] != [1, 0]
        || bytes[6] != format.tag()
        || bytes[7] != u8::from(format.direction == Direction::Microphone)
        || bytes[8..16] != format.generation.to_le_bytes()
        || bytes[28] > 1
        || bytes[29..32] != [0; 3]
    {
        return Err(invalid("PCM version, format or generation mismatch"));
    }
    let frames = u32::from_le_bytes(
        bytes[24..28]
            .try_into()
            .map_err(|_| invalid("PCM frames"))?,
    ) as usize;
    let first_frame = u64::from_le_bytes(
        bytes[16..24]
            .try_into()
            .map_err(|_| invalid("PCM position"))?,
    );
    if !(1..=BLOCK_FRAMES).contains(&frames) || first_frame.checked_add(frames as u64).is_none() {
        return Err(invalid("PCM frame bounds"));
    }
    Ok(Block {
        frames,
        first_frame,
        discontinuity: bytes[28] == 1,
        produced_at_us: u64::from_le_bytes(
            bytes[32..40].try_into().map_err(|_| invalid("PCM clock"))?,
        ),
    })
}
struct Channel {
    socket: UnixStream,
    format: Format,
    bytes: [u8; MAX_BYTES],
    offset: usize,
    length: usize,
    deadline: Option<u64>,
    last_now: u64,
    closed: bool,
}
impl Channel {
    fn new(socket: UnixStream, format: Format) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            format,
            bytes: [0; MAX_BYTES],
            offset: 0,
            length: 0,
            deadline: None,
            last_now: 0,
            closed: false,
        })
    }
    fn check(&mut self, now: u64) -> io::Result<()> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if now < self.last_now || self.deadline.is_some_and(|due| now >= due) {
            self.close();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "PCM clock or partial-frame deadline",
            ));
        }
        self.last_now = now;
        Ok(())
    }
    fn begin(&mut self, now: u64) -> io::Result<()> {
        self.deadline = Some(
            now.checked_add(DEADLINE_US)
                .ok_or_else(|| invalid("PCM deadline overflow"))?,
        );
        Ok(())
    }
    fn close(&mut self) {
        self.closed = true;
        let _ = self.socket.shutdown(Shutdown::Both);
    }
}
impl Drop for Channel {
    fn drop(&mut self) {
        self.close();
    }
}

pub struct Sender(Channel);
impl Sender {
    pub fn new(socket: UnixStream, format: Format) -> io::Result<Self> {
        Ok(Self(Channel::new(socket, format)?))
    }
    /// Accept at most one small block. Zero means backpressure; retry the suffix.
    pub fn send(
        &mut self,
        samples: &[i16],
        first_frame: u64,
        discontinuity: bool,
        now: u64,
    ) -> io::Result<usize> {
        self.pump(now)?;
        let c = &mut self.0;
        if samples.len() % c.format.channels() != 0 {
            return Err(invalid("PCM sample alignment"));
        }
        if c.length != 0 || samples.is_empty() {
            return Ok(0);
        }
        let frames = (samples.len() / c.format.channels()).min(BLOCK_FRAMES);
        let block = Block {
            frames,
            first_frame,
            discontinuity,
            produced_at_us: now,
        };
        c.length = encode(
            c.format,
            block,
            &samples[..frames * c.format.channels()],
            &mut c.bytes,
        )?;
        c.begin(now)?;
        self.pump(now)?;
        Ok(frames)
    }
    pub fn pump(&mut self, now: u64) -> io::Result<()> {
        let c = &mut self.0;
        c.check(now)?;
        if c.length == 0 {
            return Ok(());
        }
        match c.socket.write(&c.bytes[c.offset..c.length]) {
            Ok(0) => {
                c.close();
                return Err(io::ErrorKind::WriteZero.into());
            }
            Ok(n) => c.offset += n,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                return Ok(());
            }
            Err(e) => {
                c.close();
                return Err(e);
            }
        }
        if c.offset == c.length {
            c.offset = 0;
            c.length = 0;
            c.deadline = None;
        }
        Ok(())
    }
    pub fn close(&mut self) {
        self.0.close();
    }
}
pub struct Receiver {
    channel: Channel,
    next_frame: u64,
}
impl Receiver {
    pub fn new(socket: UnixStream, format: Format) -> io::Result<Self> {
        Ok(Self {
            channel: Channel::new(socket, format)?,
            next_frame: 0,
        })
    }
    /// Caller supplies space for 128 frames; a partial packet is retained.
    pub fn receive(&mut self, dest: &mut [i16], now: u64) -> io::Result<Option<Block>> {
        let result = self.receive_inner(dest, now);
        if result.is_err() {
            self.channel.close();
        }
        result
    }
    fn receive_inner(&mut self, dest: &mut [i16], now: u64) -> io::Result<Option<Block>> {
        let c = &mut self.channel;
        c.check(now)?;
        if dest.len() < BLOCK_FRAMES * c.format.channels() {
            return Err(invalid("PCM destination too short"));
        }
        if c.length == 0 {
            c.length = HEADER;
        }
        // At most header and body per pump; no unbounded read loop.
        for _ in 0..2 {
            if c.offset < c.length {
                match c.socket.read(&mut c.bytes[c.offset..c.length]) {
                    Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                    Ok(n) => {
                        if c.deadline.is_none() {
                            c.begin(now)?;
                        }
                        c.offset += n;
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                        ) =>
                    {
                        return Ok(None);
                    }
                    Err(e) => return Err(e),
                }
            }
            if c.offset < c.length {
                return Ok(None);
            }
            let block = decode(c.format, &c.bytes[..HEADER])?;
            c.length = HEADER + block.frames * c.format.channels() * 2;
            if c.offset < c.length {
                continue;
            }
            if block.first_frame < self.next_frame
                || (block.first_frame != self.next_frame && !block.discontinuity)
            {
                return Err(invalid("PCM duplicate, reorder or unannounced gap"));
            }
            for (sample, pair) in dest
                .iter_mut()
                .zip(c.bytes[HEADER..c.length].chunks_exact(2))
            {
                *sample = i16::from_le_bytes([pair[0], pair[1]]);
            }
            self.next_frame = block.first_frame + block.frames as u64;
            c.offset = 0;
            c.length = 0;
            c.deadline = None;
            return Ok(Some(block));
        }
        Ok(None)
    }
    pub fn close(&mut self) {
        self.channel.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn format() -> Format {
        Format::new(ProfileId::DualSenseEmulated, Direction::Playback, 7).unwrap()
    }
    #[test]
    fn all_profiles_and_directions_preserve_samples_and_positions() {
        for profile in [
            ProfileId::DualSenseEmulated,
            ProfileId::DualShock4Emulated,
            ProfileId::Xbox360HidEmulated,
        ] {
            for direction in [Direction::Playback, Direction::Microphone] {
                let format = Format::new(profile, direction, 7).unwrap();
                let (a, b) = UnixStream::pair().unwrap();
                let mut tx = Sender::new(a, format).unwrap();
                let mut rx = Receiver::new(b, format).unwrap();
                let mut out = [0; 512];
                let input = (0..256 * format.channels())
                    .map(|n| i16::try_from(n).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(tx.send(&input, 0, false, 0).unwrap(), 128);
                let block = rx.receive(&mut out, 1).unwrap().unwrap();
                assert_eq!((block.frames, block.first_frame), (128, 0));
                assert_eq!(
                    &out[..128 * format.channels()],
                    &input[..128 * format.channels()]
                );
                assert_eq!(
                    tx.send(&input[..format.channels()], 140, true, 2).unwrap(),
                    1
                );
                assert!(rx.receive(&mut out, 3).unwrap().unwrap().discontinuity);
                tx.close();
                tx.close();
                assert!(rx.receive(&mut out, 4).is_err());
            }
        }
    }
    #[test]
    fn malformed_or_foreign_headers_and_replayed_positions_fail_closed() {
        let mut bytes = [0; MAX_BYTES];
        let block = Block {
            frames: 1,
            first_frame: 0,
            discontinuity: false,
            produced_at_us: 1,
        };
        let n = encode(format(), block, &[1, 2, 3, 4], &mut bytes).unwrap();
        for offset in [0, 4, 5, 6, 7, 8, 24, 28, 29] {
            let (mut a, b) = UnixStream::pair().unwrap();
            let mut bad = bytes;
            bad[offset] ^= 0xff;
            a.write_all(&bad[..n]).unwrap();
            let mut rx = Receiver::new(b, format()).unwrap();
            assert!(rx.receive(&mut [0; 512], 0).is_err(), "offset {offset}");
            assert!(rx.receive(&mut [0; 512], 1).is_err());
        }
        let (mut a, b) = UnixStream::pair().unwrap();
        let mut rx = Receiver::new(b, format()).unwrap();
        a.write_all(&bytes[..n]).unwrap();
        a.write_all(&bytes[..n]).unwrap();
        assert!(rx.receive(&mut [0; 512], 0).unwrap().is_some());
        assert!(rx.receive(&mut [0; 512], 1).is_err());
    }
    #[test]
    fn fragmented_packets_have_an_absolute_deadline() {
        let (mut a, b) = UnixStream::pair().unwrap();
        let mut rx = Receiver::new(b, format()).unwrap();
        a.write_all(b"V").unwrap();
        assert_eq!(rx.receive(&mut [0; 512], 5).unwrap(), None);
        a.write_all(b"G").unwrap();
        assert_eq!(rx.receive(&mut [0; 512], 999_999).unwrap(), None);
        assert_eq!(
            rx.receive(&mut [0; 512], 1_000_005).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }
}

/// Bounded queue-to-socket pump. A pending prefix is retained across backpressure.
pub struct Outbound {
    source: gr_audio_contract::queue::PcmConsumer,
    sender: Sender,
    samples: [i16; 512],
    pending: Option<gr_audio_contract::queue::PcmRead>,
}
/// Bounded socket-to-queue pump. Host inactivity is recoverable: accepted IPC
/// frames that cannot fit are counted as loss rather than holding the socket
/// until its partial-message deadline terminates the controller.
pub struct Inbound {
    destination: gr_audio_contract::queue::PcmProducer,
    receiver: Receiver,
    samples: [i16; 512],
    next_frame: u64,
}
fn validate_queue(format: Format, pcm: &gr_audio_contract::PcmFormat) -> io::Result<()> {
    let profile = crate::profile::Profile::new(format.profile);
    let valid = match format.direction {
        Direction::Playback => profile.accepts_playback(pcm),
        Direction::Microphone => profile.accepts_microphone(pcm),
    };
    if valid {
        Ok(())
    } else {
        Err(invalid("PCM queue does not match compiled profile"))
    }
}
impl Outbound {
    pub fn new(
        source: gr_audio_contract::queue::PcmConsumer,
        socket: UnixStream,
        format: Format,
    ) -> io::Result<Self> {
        validate_queue(format, source.format())?;
        Ok(Self {
            source,
            sender: Sender::new(socket, format)?,
            samples: [0; 512],
            pending: None,
        })
    }
    pub fn pump(&mut self, now: u64) -> io::Result<()> {
        let result = self.pump_inner(now);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn pump_inner(&mut self, now: u64) -> io::Result<()> {
        self.sender.pump(now)?;
        if self.sender.0.length != 0 {
            return Ok(());
        }
        let channels = self.sender.0.format.channels();
        if self.pending.is_none() {
            let read = self
                .source
                .read(&mut self.samples[..BLOCK_FRAMES * channels])
                .map_err(io::Error::other)?;
            if read.frames == 0 {
                return Ok(());
            }
            self.pending = Some(read);
        }
        if let Some(read) = self.pending {
            let sent = self.sender.send(
                &self.samples[..read.frames * channels],
                read.first_frame,
                read.discontinuity,
                now,
            )?;
            if sent == read.frames {
                self.pending = None;
            }
        }
        Ok(())
    }
    pub fn close(&mut self) {
        self.source.close();
        self.sender.close();
    }
}
impl Inbound {
    /// Wait for incoming PCM without delaying the pump's other deadlines.
    /// The half-millisecond timeout also services the outbound queue when
    /// the microphone is idle. No descriptor or unsafe operation escapes.
    pub fn wait_readable(&self) -> io::Result<bool> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};
        let mut fds = [PollFd::new(&self.receiver.channel.socket, PollFlags::IN)];
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: 500_000,
        };
        let ready = match poll(&mut fds, Some(&timeout)) {
            Ok(ready) => ready,
            Err(rustix::io::Errno::INTR) => return Ok(false),
            Err(error) => return Err(io::Error::from(error)),
        };
        let flags = fds[0].revents();
        if flags.intersects(PollFlags::ERR | PollFlags::NVAL) {
            return Err(io::Error::other("PCM input readiness failed"));
        }
        Ok(ready > 0 && flags.intersects(PollFlags::IN | PollFlags::HUP))
    }
    pub fn new(
        destination: gr_audio_contract::queue::PcmProducer,
        socket: UnixStream,
        format: Format,
    ) -> io::Result<Self> {
        validate_queue(format, destination.format())?;
        Ok(Self {
            destination,
            receiver: Receiver::new(socket, format)?,
            samples: [0; 512],
            next_frame: 0,
        })
    }
    pub fn pump(&mut self, now: u64) -> io::Result<()> {
        let result = self.pump_inner(now);
        if result.is_err() {
            self.close();
        }
        result
    }
    fn pump_inner(&mut self, now: u64) -> io::Result<()> {
        let Some(block) = self.receiver.receive(&mut self.samples, now)? else {
            return Ok(());
        };
        if block.first_frame > self.next_frame {
            self.destination
                .discard(block.first_frame - self.next_frame)
                .map_err(io::Error::other)?;
        }
        let channels = self.receiver.channel.format.channels();
        let accepted = self
            .destination
            .push(&self.samples[..block.frames * channels])
            .map_err(io::Error::other)?;
        self.destination
            .discard(u64::try_from(block.frames - accepted).map_err(io::Error::other)?)
            .map_err(io::Error::other)?;
        self.next_frame = block.first_frame + block.frames as u64;
        Ok(())
    }
    pub fn close(&mut self) {
        self.destination.close();
        self.receiver.close();
    }
}

#[cfg(test)]
mod pump_tests {
    use super::*;
    use gr_audio_contract::{AudioChannel, PcmFormat, queue::pcm_queue};
    #[test]
    fn inbound_readiness_wakes_for_pcm_and_peer_closure() {
        let pcm = PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap();
        let format = Format::new(ProfileId::Xbox360HidEmulated, Direction::Microphone, 1).unwrap();
        let (a, b) = UnixStream::pair().unwrap();
        let mut sender = Sender::new(a, format).unwrap();
        let (destination, mut consumer) = pcm_queue(&pcm, 128).unwrap();
        let mut inbound = Inbound::new(destination, b, format).unwrap();
        assert!(!inbound.wait_readable().unwrap());
        sender.send(&[7], 0, false, 0).unwrap();
        assert!(inbound.wait_readable().unwrap());
        inbound.pump(1).unwrap();
        let mut sample = [0];
        assert_eq!(consumer.read(&mut sample).unwrap().frames, 1);
        assert_eq!(sample, [7]);
        assert!(!inbound.wait_readable().unwrap());
        sender.close();
        assert!(inbound.wait_readable().unwrap());
    }
    #[test]
    fn stalled_consumer_counts_loss_and_recovers_after_host_resumes() {
        let pcm = PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap();
        let format = Format::new(ProfileId::Xbox360HidEmulated, Direction::Microphone, 1).unwrap();
        let (a, b) = UnixStream::pair().unwrap();
        let mut sender = Sender::new(a, format).unwrap();
        let (destination, mut consumer) = pcm_queue(&pcm, 1).unwrap();
        let mut inbound = Inbound::new(destination, b, format).unwrap();
        sender.send(&[1, 2, 3], 0, false, 0).unwrap();
        inbound.pump(1).unwrap();
        inbound.pump(1_000_001).unwrap();
        assert_eq!(consumer.discarded_frames(), 2);
        assert_eq!(sender.send(&[4], 3, false, 1_000_002).unwrap(), 1);
        inbound.pump(1_000_003).unwrap();
        assert_eq!(consumer.discarded_frames(), 3);
        let mut sample = [0];
        assert_eq!(consumer.read(&mut sample).unwrap().frames, 1);
        assert_eq!(sample, [1]);
        assert_eq!(sender.send(&[5], 4, false, 1_000_004).unwrap(), 1);
        inbound.pump(1_000_005).unwrap();
        let read = consumer.read(&mut sample).unwrap();
        assert_eq!(
            (read.frames, read.first_frame, read.discontinuity),
            (1, 4, true)
        );
        assert_eq!(sample, [5]);
    }
    #[test]
    fn queue_backpressure_preserves_every_suffix_and_gap() {
        let pcm = PcmFormat::new(48_000, &[AudioChannel::Microphone]).unwrap();
        let format = Format::new(ProfileId::Xbox360HidEmulated, Direction::Microphone, 1).unwrap();
        let (mut producer, source) = pcm_queue(&pcm, 256).unwrap();
        let (destination, mut consumer) = pcm_queue(&pcm, 128).unwrap();
        let (a, b) = UnixStream::pair().unwrap();
        let mut outbound = Outbound::new(source, a, format).unwrap();
        let mut inbound = Inbound::new(destination, b, format).unwrap();
        producer.push(&[1, 2, 3, 4, 5, 6, 7]).unwrap();
        producer.discard(2).unwrap();
        producer.push(&[8, 9]).unwrap();
        let mut received = Vec::new();
        let mut gaps = Vec::new();
        for now in 0..20 {
            outbound.pump(now).unwrap();
            inbound.pump(now).unwrap();
            let mut samples = [0; 2];
            let read = consumer.read(&mut samples).unwrap();
            if read.discontinuity {
                gaps.push(read.first_frame);
            }
            received.extend_from_slice(&samples[..read.frames]);
        }
        assert_eq!(received, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(gaps, [9]);
        assert_eq!(consumer.discarded_frames(), 2);
        outbound.close();
        outbound.close();
        assert!(producer.push(&[1]).is_err());
        assert!(inbound.pump(21).is_err());
        assert!(consumer.read(&mut [0]).is_err());
    }
}
