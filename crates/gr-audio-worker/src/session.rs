use gr_audio_contract::{AudioExposure, queue::pcm_queue};
use gr_curated_controllers::{
    audio,
    usb_personality::{
        self,
        state::{NativeState, Transactions},
    },
};
use gr_hid::{Protocol, Reply, Report, ReportType, RequestKind};
use gr_realization_api::RawReverseEvent;
use gr_usbip::{
    pcm_ipc::{Direction, Format, Inbound, Outbound},
    profile::{Profile, ProfileId},
    worker::{Counters, HidHandler, Worker},
};
use std::{
    io,
    net::Shutdown,
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub struct Setup {
    pub profile: ProfileId,
    pub generation: u64,
    pub device: u32,
    pub identity: [u8; 6],
}
pub struct Channels {
    pub usb: UnixStream,
    pub control: UnixStream,
    pub playback: UnixStream,
    pub microphone: UnixStream,
}
struct Update {
    generation: u64,
    sequence: u64,
    bytes: Vec<u8>,
}
struct Handler<P: Protocol, F> {
    protocol: P,
    state: Transactions,
    extract: F,
    numbered: bool,
    updates: mpsc::Receiver<Update>,
    acknowledgements: mpsc::SyncSender<(u64, bool)>,
    events: mpsc::SyncSender<RawReverseEvent>,
    lost: Arc<AtomicU64>,
}
impl<P, F> Handler<P, F>
where
    P: Protocol<Output = RawReverseEvent>,
    F: Fn(&NativeState) -> Option<P::State>,
{
    fn event(&self, event: Option<RawReverseEvent>) {
        if let Some(event) = event {
            if self.events.try_send(event).is_err() {
                self.lost.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
impl<P, F> HidHandler for Handler<P, F>
where
    P: Protocol<Output = RawReverseEvent>,
    F: Fn(&NativeState) -> Option<P::State>,
{
    fn service(&mut self, _: u64) -> io::Result<()> {
        // Bounded control work cannot starve USB packet servicing.
        for _ in 0..8 {
            match self.updates.try_recv() {
                Ok(update) => {
                    let accepted = self
                        .state
                        .apply(update.generation, update.sequence, &update.bytes)
                        .is_ok();
                    self.acknowledgements
                        .try_send((update.sequence, accepted))
                        .map_err(io::Error::other)?;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(io::Error::other("worker control closed"));
                }
            }
        }
        Ok(())
    }
    fn input(&mut self, now: u64) -> Option<Report> {
        self.protocol
            .input(&(self.extract)(self.state.state())?, now)
            .ok()?
            .into_iter()
            .next()
    }
    fn request(&mut self, request: &RequestKind, now: u64) -> Reply {
        let (reply, event) = self.protocol.request(request, now);
        self.event(event);
        reply
    }
    fn output(&mut self, bytes: &[u8], now: u64) -> bool {
        let Ok(report) = Report::from_wire(ReportType::Output, self.numbered, bytes) else {
            return false;
        };
        match self.protocol.output(report, now) {
            Ok(event) => {
                self.event(event);
                true
            }
            Err(_) => false,
        }
    }
}
struct CloseOnExit {
    socket: UnixStream,
    stopping: Arc<AtomicBool>,
}
impl Drop for CloseOnExit {
    fn drop(&mut self) {
        if !self.stopping.load(Ordering::Acquire) {
            // Wake the control reader without closing the worker's write half:
            // `finish` still needs to send the terminal failure reason.
            let _ = self.socket.shutdown(Shutdown::Read);
        }
    }
}
struct Threads {
    stop: Arc<AtomicBool>,
    joins: Vec<thread::JoinHandle<io::Result<()>>>,
}
impl Threads {
    fn close(&mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        let mut error = None;
        for join in self.joins.drain(..) {
            match join.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    error.get_or_insert(e);
                }
                Err(_) => {
                    error.get_or_insert_with(|| io::Error::other("worker thread panicked"));
                }
            }
        }
        error.map_or(Ok(()), Err)
    }
}
impl Drop for Threads {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

pub fn run(setup: Setup, channels: Channels) -> io::Result<()> {
    if setup.device == 0 || setup.generation == 0 {
        return Err(io::Error::other("invalid worker setup"));
    }
    for socket in [
        &channels.usb,
        &channels.control,
        &channels.playback,
        &channels.microphone,
    ] {
        socket.peer_addr()?;
    }
    match setup.profile {
        ProfileId::DualSenseEmulated => run_typed(
            usb_personality::dualsense(setup.identity),
            NativeState::DualSense(gr_curated_controllers::DualSenseState::default()),
            |s| {
                if let NativeState::DualSense(s) = s {
                    Some(s.clone())
                } else {
                    None
                }
            },
            setup,
            channels,
        ),
        ProfileId::DualShock4Emulated => run_typed(
            usb_personality::dualshock4(setup.identity),
            NativeState::DualShock4(gr_curated_controllers::DualShock4State::default()),
            |s| {
                if let NativeState::DualShock4(s) = s {
                    Some(s.clone())
                } else {
                    None
                }
            },
            setup,
            channels,
        ),
        ProfileId::Xbox360HidEmulated => run_typed(
            usb_personality::xbox360(),
            NativeState::Xbox360(gr_curated_controllers::Xbox360State::default()),
            |s| {
                if let NativeState::Xbox360(s) = s {
                    Some(s.clone())
                } else {
                    None
                }
            },
            setup,
            channels,
        ),
    }
}
fn run_typed<P, F>(
    protocol: P,
    neutral: NativeState,
    extract: F,
    setup: Setup,
    channels: Channels,
) -> io::Result<()>
where
    P: Protocol<Output = RawReverseEvent> + Send + 'static,
    P::State: Send,
    F: Fn(&NativeState) -> Option<P::State> + Send + 'static,
{
    let profile = match setup.profile {
        ProfileId::DualSenseEmulated => audio::dualsense(AudioExposure::Emulated),
        ProfileId::DualShock4Emulated => audio::dualshock4(AudioExposure::Emulated),
        ProfileId::Xbox360HidEmulated => audio::xbox360(AudioExposure::Emulated),
    }
    .map_err(io::Error::other)?;
    let (playback, reader) =
        pcm_queue(profile.streams()[0].format(), 2048).map_err(io::Error::other)?;
    let (writer, microphone) =
        pcm_queue(profile.streams()[1].format(), 2048).map_err(io::Error::other)?;
    let mut outbound = Outbound::new(
        reader,
        channels.playback,
        Format::new(setup.profile, Direction::Playback, setup.generation)?,
    )?;
    let mut inbound = Inbound::new(
        writer,
        channels.microphone,
        Format::new(setup.profile, Direction::Microphone, setup.generation)?,
    )?;
    let (updates, receive_updates) = mpsc::sync_channel(8);
    let (acknowledgements, receive_acknowledgements) = mpsc::sync_channel(8);
    let (events, receive_events) = mpsc::sync_channel(128);
    let lost = Arc::new(AtomicU64::new(0));
    let counters = Arc::new(Counters::default());
    let pcm_lateness = Arc::new(AtomicU64::new(0));
    let handler = Handler {
        protocol,
        state: Transactions::new(setup.generation, neutral)
            .map_err(|e| io::Error::other(format!("{e:?}")))?,
        extract,
        numbered: setup.profile != ProfileId::Xbox360HidEmulated,
        updates: receive_updates,
        acknowledgements,
        events,
        lost: lost.clone(),
    };
    let worker = Worker::new(
        channels.usb,
        setup.device,
        Profile::new(setup.profile),
        handler,
        playback,
        microphone,
        counters.clone(),
    )?;
    let mut threads = Threads {
        stop: Arc::new(AtomicBool::new(false)),
        joins: Vec::with_capacity(2),
    };
    let control = channels.control;
    control.set_write_timeout(Some(Duration::from_secs(1)))?;
    let notify = CloseOnExit {
        socket: control.try_clone()?,
        stopping: threads.stop.clone(),
    };
    let stop = threads.stop.clone();
    let pump_lateness = pcm_lateness.clone();
    threads.joins.push(
        thread::Builder::new()
            .name("controller-usb".into())
            .spawn(move || {
                let _notify = notify;
                worker.run(&stop)
            })?,
    );
    let notify = CloseOnExit {
        socket: control.try_clone()?,
        stopping: threads.stop.clone(),
    };
    let stop = threads.stop.clone();
    threads.joins.push(
        thread::Builder::new()
            .name("controller-pcm".into())
            .spawn(move || {
                let _notify = notify;
                run_pcm(&mut outbound, &mut inbound, &stop, &pump_lateness)
            })?,
    );
    let result = control_loop(
        &control,
        setup.generation,
        &updates,
        &receive_acknowledgements,
        &receive_events,
        &ControlStats {
            lost: &lost,
            usb: &counters,
            pcm_lateness: &pcm_lateness,
        },
    );
    finish(&control, setup.generation, result, threads.close())
}
fn run_pcm(
    outbound: &mut Outbound,
    inbound: &mut Inbound,
    stop: &AtomicBool,
    lateness: &AtomicU64,
) -> io::Result<()> {
    let started = Instant::now();
    let mut previous = started;
    while !stop.load(Ordering::Acquire) {
        let observed = Instant::now();
        lateness.fetch_max(
            pcm_lateness_us(observed.duration_since(previous)),
            Ordering::Relaxed,
        );
        previous = observed;
        let now = u64::try_from(started.elapsed().as_micros()).map_err(io::Error::other)?;
        outbound.pump(now)?;
        inbound.pump(now)?;
        thread::sleep(Duration::from_micros(500));
    }
    outbound.close();
    inbound.close();
    Ok(())
}
fn pcm_lateness_us(interval: Duration) -> u64 {
    u64::try_from(interval.as_micros())
        .unwrap_or(u64::MAX)
        .saturating_sub(500)
}
fn finish(
    control: &UnixStream,
    generation: u64,
    result: io::Result<()>,
    cleanup: io::Result<()>,
) -> io::Result<()> {
    // A close acknowledgement certifies both processing threads have stopped.
    // Worker failures take precedence over the EOF used to wake this reader.
    let outcome = match (result, cleanup) {
        (Err(control), Err(worker)) => Err(io::Error::other(format!(
            "worker: {worker}; control: {control}"
        ))),
        (Err(error), _) | (_, Err(error)) => Err(error),
        _ => Ok(()),
    };
    let acknowledgement = if let Err(problem) = &outcome {
        let reason = problem.to_string();
        let mut end = reason.len().min(192);
        while !reason.is_char_boundary(end) {
            end -= 1;
        }
        let mut payload = generation.to_le_bytes().to_vec();
        payload.extend_from_slice(&reason.as_bytes()[..end]);
        gr_privileged_broker::write_message(&mut &*control, 0x81, &payload)
    } else {
        gr_privileged_broker::write_message(&mut &*control, 4, &generation.to_le_bytes())
    };
    let _ = control.shutdown(Shutdown::Both);
    outcome.and(acknowledgement)
}
struct ControlStats<'a> {
    lost: &'a AtomicU64,
    usb: &'a Counters,
    pcm_lateness: &'a AtomicU64,
}
fn control_loop(
    socket: &UnixStream,
    generation: u64,
    updates: &mpsc::SyncSender<Update>,
    acknowledgements: &mpsc::Receiver<(u64, bool)>,
    events: &mpsc::Receiver<RawReverseEvent>,
    stats: &ControlStats<'_>,
) -> io::Result<()> {
    use gr_privileged_broker::{socket_wire::read_frame, write_message};
    let mut socket = socket;
    let mut ready = vec![1];
    ready.extend(generation.to_le_bytes());
    write_message(&mut socket, 0, &ready)?;
    loop {
        let (tag, body) = read_frame(socket, Duration::from_secs(1))?;
        if body.len() < 8 || body[..8] != generation.to_le_bytes() {
            return Err(io::Error::other("worker generation mismatch"));
        }
        match tag {
            1 if (17..=144).contains(&body.len()) => {
                let sequence =
                    u64::from_le_bytes(body[8..16].try_into().map_err(io::Error::other)?);
                updates
                    .try_send(Update {
                        generation,
                        sequence,
                        bytes: body[16..].to_vec(),
                    })
                    .map_err(io::Error::other)?;
                let (ack, accepted) = acknowledgements
                    .recv_timeout(Duration::from_secs(1))
                    .map_err(io::Error::other)?;
                if ack != sequence {
                    return Err(io::Error::other("worker acknowledgement mismatch"));
                }
                let mut response = body[..16].to_vec();
                response.push(u8::from(accepted));
                write_message(&mut socket, 1, &response)?;
            }
            2 if body.len() == 8 => {
                let mut response = generation.to_le_bytes().to_vec();
                match events.try_recv() {
                    Ok(RawReverseEvent::HidOutput { report_id, bytes }) if bytes.len() <= 240 => {
                        response.extend([1, report_id.unwrap_or(0)]);
                        response.extend(bytes);
                    }
                    Err(mpsc::TryRecvError::Empty) => response.push(0),
                    _ => return Err(io::Error::other("worker output contract failure")),
                }
                write_message(&mut socket, 2, &response)?;
            }
            3 if body.len() == 8 => {
                let mut response = generation.to_le_bytes().to_vec();
                for counter in [
                    stats.lost,
                    &stats.usb.completed_transfers,
                    &stats.usb.microphone_silence_frames,
                    &stats.usb.stalled_transfers,
                    &stats.usb.playback_frames,
                    &stats.usb.capture_frames,
                    &stats.usb.abandoned_capture_frames,
                    &stats.usb.maximum_audio_lateness_us,
                ] {
                    response.extend(counter.load(Ordering::Relaxed).to_le_bytes());
                }
                write_message(&mut socket, 3, &response)?;
            }
            5 if body.len() == 8 => {
                let mut response = generation.to_le_bytes().to_vec();
                response.extend(
                    stats
                        .usb
                        .microphone_consumed_frames
                        .load(Ordering::Relaxed)
                        .to_le_bytes(),
                );
                write_message(&mut socket, 5, &response)?;
            }
            6 if body.len() == 8 => {
                let mut response = generation.to_le_bytes().to_vec();
                response.extend(stats.pcm_lateness.load(Ordering::Relaxed).to_le_bytes());
                write_message(&mut socket, 6, &response)?;
            }
            7 if body.len() == 8 => {
                let mut response = generation.to_le_bytes().to_vec();
                response.extend(
                    stats
                        .usb
                        .microphone_host_frames
                        .load(Ordering::Relaxed)
                        .to_le_bytes(),
                );
                response.extend(
                    stats
                        .usb
                        .microphone_silence_frames
                        .load(Ordering::Relaxed)
                        .to_le_bytes(),
                );
                write_message(&mut socket, 7, &response)?;
            }
            4 if body.len() == 8 => {
                return Ok(());
            }
            _ => return Err(io::Error::other("unknown worker control operation")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_privileged_broker::{read_message, write_message};
    use std::io::Read;
    fn pair() -> (UnixStream, UnixStream) {
        let (a, b) = UnixStream::pair().unwrap();
        a.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        b.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        (a, b)
    }
    #[test]
    fn pcm_pump_delay_reports_only_excess_over_nominal_interval() {
        assert_eq!(pcm_lateness_us(Duration::ZERO), 0);
        assert_eq!(pcm_lateness_us(Duration::from_micros(499)), 0);
        assert_eq!(pcm_lateness_us(Duration::from_micros(500)), 0);
        assert_eq!(pcm_lateness_us(Duration::from_millis(2)), 1_500);
    }
    #[test]
    fn cleanup_failure_never_acknowledges_success() {
        let (control, mut client) = pair();
        assert!(finish(&control, 9, Ok(()), Err(io::Error::other("cleanup failed"))).is_err());
        let (tag, body) = read_message(&mut client).unwrap();
        assert_eq!(tag, 0x81);
        assert_eq!(&body[..8], &9_u64.to_le_bytes());
        assert_eq!(&body[8..], b"cleanup failed");
        assert_eq!(client.read(&mut [0]).unwrap(), 0);
    }
    #[test]
    fn worker_failure_wakes_control_reader_but_keeps_failure_reply_writable() {
        let (mut worker, mut client) = pair();
        let notifier = CloseOnExit {
            socket: worker.try_clone().unwrap(),
            stopping: Arc::new(AtomicBool::new(false)),
        };
        drop(notifier);
        let mut byte = [0];
        assert_eq!(worker.read(&mut byte).unwrap(), 0);
        write_message(&mut &worker, 0x81, b"bounded cause").unwrap();
        assert_eq!(
            read_message(&mut client).unwrap(),
            (0x81, b"bounded cause".to_vec())
        );
    }
    #[test]
    fn every_family_updates_without_host_polling_and_closes_owned_channels() {
        for (profile, state) in [
            (
                ProfileId::DualSenseEmulated,
                NativeState::DualSense(gr_curated_controllers::DualSenseState::default()),
            ),
            (
                ProfileId::DualShock4Emulated,
                NativeState::DualShock4(gr_curated_controllers::DualShock4State::default()),
            ),
            (
                ProfileId::Xbox360HidEmulated,
                NativeState::Xbox360(gr_curated_controllers::Xbox360State::default()),
            ),
        ] {
            let (usb, _host) = pair();
            let (control, mut client) = pair();
            let (playback, mut samples) = pair();
            let (microphone, _mic) = pair();
            let worker = thread::spawn(move || {
                run(
                    Setup {
                        profile,
                        generation: 9,
                        device: 1,
                        identity: [2, 1, 2, 3, 4, 5],
                    },
                    Channels {
                        usb,
                        control,
                        playback,
                        microphone,
                    },
                )
            });
            assert_eq!(
                read_message(&mut client).unwrap(),
                (0, [vec![1], 9_u64.to_le_bytes().to_vec()].concat())
            );
            let mut update = 9_u64.to_le_bytes().to_vec();
            update.extend(1_u64.to_le_bytes());
            update.extend(state.encode());
            for _ in 0..2 {
                write_message(&mut client, 1, &update).unwrap();
                let (tag, ack) = read_message(&mut client).unwrap();
                assert_eq!(tag, 1);
                assert_eq!(&ack[..16], &update[..16]);
                assert_eq!(ack[16], 1);
            }
            update[18] ^= 1; // Conflicting retry must fail rather than replace state.
            write_message(&mut client, 1, &update).unwrap();
            assert_eq!(read_message(&mut client).unwrap().1[16], 0);
            write_message(&mut client, 3, &9_u64.to_le_bytes()).unwrap();
            let (tag, stats) = read_message(&mut client).unwrap();
            assert_eq!(tag, 3);
            assert_eq!(stats.len(), 72);
            write_message(&mut client, 5, &9_u64.to_le_bytes()).unwrap();
            assert_eq!(
                read_message(&mut client).unwrap(),
                (5, [9_u64.to_le_bytes(), 0_u64.to_le_bytes()].concat())
            );
            write_message(&mut client, 6, &9_u64.to_le_bytes()).unwrap();
            let (tag, timing) = read_message(&mut client).unwrap();
            assert_eq!(tag, 6);
            assert_eq!(&timing[..8], &9_u64.to_le_bytes());
            assert_eq!(timing.len(), 16);
            write_message(&mut client, 4, &9_u64.to_le_bytes()).unwrap();
            assert_eq!(
                read_message(&mut client).unwrap(),
                (4, 9_u64.to_le_bytes().to_vec())
            );
            samples.set_nonblocking(true).unwrap();
            assert_eq!(samples.read(&mut [0]).unwrap(), 0);
            assert!(worker.join().unwrap().is_ok());
        }
    }
    #[test]
    fn wrong_generation_terminates_the_worker_and_pcm_endpoints() {
        let (usb, _host) = pair();
        let (control, mut client) = pair();
        let (playback, mut samples) = pair();
        let (microphone, _mic) = pair();
        let worker = thread::spawn(move || {
            run(
                Setup {
                    profile: ProfileId::Xbox360HidEmulated,
                    generation: 9,
                    device: 1,
                    identity: [0; 6],
                },
                Channels {
                    usb,
                    control,
                    playback,
                    microphone,
                },
            )
        });
        read_message(&mut client).unwrap();
        write_message(&mut client, 3, &8_u64.to_le_bytes()).unwrap();
        assert!(worker.join().unwrap().is_err());
        let (tag, reason) = read_message(&mut client).unwrap();
        assert_eq!(tag, 0x81);
        assert_eq!(&reason[..8], &9_u64.to_le_bytes());
        assert!(String::from_utf8_lossy(&reason[8..]).contains("generation mismatch"));
        assert_eq!(samples.read(&mut [0]).unwrap(), 0);
    }
    #[test]
    fn host_socket_death_reports_worker_cause_before_terminal_cleanup() {
        let (usb, host) = pair();
        let (control, mut client) = pair();
        let (playback, mut samples) = pair();
        let (microphone, _mic) = pair();
        let worker = thread::spawn(move || {
            run(
                Setup {
                    profile: ProfileId::DualSenseEmulated,
                    generation: 17,
                    device: 1,
                    identity: [0; 6],
                },
                Channels {
                    usb,
                    control,
                    playback,
                    microphone,
                },
            )
        });
        assert_eq!(read_message(&mut client).unwrap().0, 0);
        drop(host);
        let (tag, reason) = read_message(&mut client).unwrap();
        assert_eq!(tag, 0x81);
        assert_eq!(&reason[..8], &17_u64.to_le_bytes());
        assert!(!reason[8..].is_empty());
        assert!(worker.join().unwrap().is_err());
        assert_eq!(samples.read(&mut [0]).unwrap(), 0);
    }
}
