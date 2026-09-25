//! Caller-session `PipeWire` presentation for USB sample channels. This is a
//! private, unprivileged pump; the worker and broker never join a user graph.
use crate::{AudioAccess, AudioError, AudioOptions, SampleDirection};
use gr_audio_contract::{AudioProfile, AudioStreamDescription};
use gr_audio_worker::{client::Control, client_pcm::SampleStreams};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub(crate) struct Bridge {
    stop: Arc<std::sync::atomic::AtomicBool>,
    error: Arc<Mutex<Option<AudioError>>>,
    metrics: Arc<Metrics>,
    thread: Option<JoinHandle<()>>,
}
#[derive(Default)]
struct Metrics {
    timings: Mutex<Vec<crate::AudioStreamTiming>>,
    source_underruns: AtomicU64,
    graph_input_dropped: AtomicU64,
    graph_input_fill: AtomicU64,
    graph_input_peak: AtomicU64,
    maximum_credit_request_us: AtomicU64,
    maximum_pump_gap_us: AtomicU64,
}
impl Metrics {
    fn capture(&self, session: &gr_audio_linux::Session) {
        self.source_underruns
            .store(session.underrun_frames(), Ordering::Release);
        self.graph_input_dropped
            .store(session.dropped_playback_frames(), Ordering::Release);
        if let Some(fill) = session.queued_playback_frames() {
            self.graph_input_fill.store(fill as u64, Ordering::Release);
            self.graph_input_peak
                .fetch_max(fill as u64, Ordering::Relaxed);
        }
        *self
            .timings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = session
            .timings()
            .into_iter()
            .map(|timing| crate::AudioStreamTiming {
                endpoint: timing.endpoint,
                graph_ticks: timing.graph_ticks,
                tick_rate: (timing.rate_num, timing.rate_denom),
                observed_at_ns: timing.monotonic_ns,
                estimated_graph_delay_ticks: timing.delay_ticks,
                discontinuities: timing.discontinuities,
                missed_graph_frames: timing.missed_graph_frames,
            })
            .collect();
    }
}
#[derive(Clone, Copy)]
struct PumpConfig {
    playback_native: bool,
    microphone_native: bool,
    playback_channels: usize,
    microphone_channels: usize,
}
// Keep native microphone production tied to actual USB host media time. A
// stopped capture must stop delivery before the worker queue fills; its spare
// capacity is not a steady-state target.
struct NativeMicCredit {
    host: u64,
    submitted: u64,
    started: bool,
}
impl NativeMicCredit {
    const FILL: u64 = 672; // 14 ms at the compiled 48 kHz profiles.
    fn update_host(&mut self, host: u64) {
        self.started |= host > 0;
        self.host = host;
        self.submitted = self.submitted.max(host);
    }
    fn available(&self) -> usize {
        if !self.started {
            return 0;
        }
        usize::try_from(
            self.host
                .saturating_add(Self::FILL)
                .saturating_sub(self.submitted)
                .min(128),
        )
        .unwrap_or(0)
    }
    fn accepted(&mut self, frames: usize) {
        self.submitted += frames as u64;
    }
}
fn spawn_host_clock_poller(
    control: Arc<Mutex<Control>>,
    microphone_silence: Arc<AtomicU64>,
    host_frames: Arc<AtomicU64>,
    poll_stop: Arc<std::sync::atomic::AtomicBool>,
    poll_error: Arc<Mutex<Option<AudioError>>>,
    metrics: Arc<Metrics>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        while !poll_stop.load(Ordering::Acquire) {
            let started = Instant::now();
            let result = control
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .microphone_host_frames();
            metrics.maximum_credit_request_us.fetch_max(
                u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
                Ordering::Relaxed,
            );
            match result {
                Ok((host, silence)) => {
                    host_frames.store(host, Ordering::Release);
                    microphone_silence.store(silence, Ordering::Release);
                }
                Err(error) => {
                    *poll_error
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) =
                        Some(AudioError::Backend {
                            reason: error.to_string(),
                        });
                    break;
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
    })
}
fn pump_session(
    session: &mut gr_audio_linux::Session,
    streams: &Arc<Mutex<SampleStreams>>,
    control: &Arc<Mutex<Control>>,
    microphone_silence: &Arc<AtomicU64>,
    stop: &std::sync::atomic::AtomicBool,
    metrics: &Arc<Metrics>,
    config: PumpConfig,
) -> Result<(), AudioError> {
    let host_frames = Arc::new(AtomicU64::new(0));
    let poll_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let poll_error = Arc::new(Mutex::new(None));
    let poller = config.microphone_native.then(|| {
        spawn_host_clock_poller(
            control.clone(),
            microphone_silence.clone(),
            host_frames.clone(),
            poll_stop.clone(),
            poll_error.clone(),
            metrics.clone(),
        )
    });
    let result = run(
        session,
        streams,
        &host_frames,
        &poll_error,
        stop,
        metrics,
        config,
    );
    poll_stop.store(true, Ordering::Release);
    if let Some(poller) = poller {
        let _ = poller.join();
    }
    result.and_then(|()| {
        poll_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .map_or(Ok(()), Err)
    })
}
impl Bridge {
    pub(crate) fn start(
        streams: Arc<Mutex<SampleStreams>>,
        control: Arc<Mutex<Control>>,
        microphone_silence: Arc<AtomicU64>,
        profile: &AudioProfile,
        options: AudioOptions,
        creation: u64,
    ) -> Result<(Option<Self>, [Option<String>; 2]), AudioError> {
        let (mirror, original) = mirror_profile(profile, options)?;
        let Some(mirror) = mirror else {
            return Ok((None, [None, None]));
        };
        let (startup, ready) = mpsc::sync_channel(1);
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let metrics = Arc::new(Metrics::default());
        let thread_stop = stop.clone();
        let thread_error = error.clone();
        let thread_metrics = metrics.clone();
        let config = PumpConfig {
            playback_channels: profile.streams()[0].format().channels().len(),
            microphone_channels: profile.streams()[1].format().channels().len(),
            playback_native: options.playback_access() == AudioAccess::NativeClient,
            microphone_native: options.microphone_access() == AudioAccess::NativeClient,
        };
        let thread = thread::Builder::new()
            .name("usb-audio-pipewire-bridge".into())
            .spawn(move || {
                let result = (|| {
                    let mut session = gr_audio_linux::Session::open_usb_bridge(
                        &mirror,
                        AudioOptions::new(crate::AudioExposure::Emulated),
                        creation,
                        128,
                    )?;
                    let mut names = [None, None];
                    for (endpoint, index) in session.endpoints().iter().zip(&original) {
                        names[*index] = Some(endpoint.host_node.clone());
                    }
                    let _ = startup.send(Ok(names));
                    let result = pump_session(
                        &mut session,
                        &streams,
                        &control,
                        &microphone_silence,
                        &thread_stop,
                        &thread_metrics,
                        config,
                    );
                    thread_metrics.capture(&session);
                    session.close();
                    result
                })();
                if let Err(problem) = result {
                    let _ = startup.send(Err(problem.clone()));
                    *thread_error
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(problem);
                }
            })
            .map_err(|error| AudioError::Backend {
                reason: error.to_string(),
            })?;
        match ready.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(names)) => Ok((
                Some(Self {
                    stop,
                    error,
                    metrics,
                    thread: Some(thread),
                }),
                names,
            )),
            Ok(Err(problem)) => {
                stop.store(true, std::sync::atomic::Ordering::Release);
                let _ = thread.join();
                Err(problem)
            }
            Err(problem) => {
                stop.store(true, std::sync::atomic::Ordering::Release);
                let _ = thread.join();
                Err(AudioError::Backend {
                    reason: format!("PipeWire bridge startup: {problem}"),
                })
            }
        }
    }
    pub(crate) fn error(&self) -> Option<AudioError> {
        self.error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    pub(crate) fn timings(&self) -> Vec<crate::AudioStreamTiming> {
        self.metrics
            .timings
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    pub(crate) fn source_underrun_frames(&self) -> u64 {
        self.metrics.source_underruns.load(Ordering::Acquire)
    }
    pub(crate) fn graph_input_dropped_frames(&self) -> u64 {
        self.metrics.graph_input_dropped.load(Ordering::Acquire)
    }
    pub(crate) fn graph_input_queue_frames(&self) -> (u64, u64) {
        (
            self.metrics.graph_input_fill.load(Ordering::Acquire),
            self.metrics.graph_input_peak.load(Ordering::Acquire),
        )
    }
    pub(crate) fn scheduling_us(&self) -> (u64, u64) {
        (
            self.metrics
                .maximum_credit_request_us
                .load(Ordering::Acquire),
            self.metrics.maximum_pump_gap_us.load(Ordering::Acquire),
        )
    }
    pub(crate) fn close(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            *self
                .error
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(AudioError::Backend {
                reason: "PipeWire bridge panicked".into(),
            });
        }
    }
}

fn mirror_profile(
    profile: &AudioProfile,
    options: AudioOptions,
) -> Result<(Option<AudioProfile>, Vec<usize>), AudioError> {
    let mut descriptions = Vec::new();
    let mut original = Vec::new();
    for (index, stream) in profile.streams().iter().enumerate() {
        let native = match stream.direction() {
            SampleDirection::HostToController => {
                options.playback_access() == AudioAccess::NativeClient
            }
            SampleDirection::ControllerToHost => {
                options.microphone_access() == AudioAccess::NativeClient
            }
            _ => return Err(AudioError::IncompatibleTopology),
        };
        if native {
            let reverse = match stream.direction() {
                SampleDirection::HostToController => SampleDirection::ControllerToHost,
                SampleDirection::ControllerToHost => SampleDirection::HostToController,
                _ => return Err(AudioError::IncompatibleTopology),
            };
            descriptions.push(AudioStreamDescription::new(
                stream.name(),
                reverse,
                stream.format().clone(),
            ));
            original.push(index);
        }
    }
    Ok((
        (!descriptions.is_empty()).then(|| {
            AudioProfile::new(
                "usbip-caller",
                &descriptions,
                "Caller-session PipeWire presentation of the associated USB PCM stream",
            )
        }),
        original,
    ))
}

fn run(
    session: &mut gr_audio_linux::Session,
    streams: &Arc<Mutex<SampleStreams>>,
    host_frames: &AtomicU64,
    poll_error: &Mutex<Option<AudioError>>,
    stop: &std::sync::atomic::AtomicBool,
    metrics: &Metrics,
    config: PumpConfig,
) -> Result<(), AudioError> {
    const FRAMES: usize = 128;
    let mut playback = [0_i16; FRAMES * 4];
    let mut microphone = [0_i16; FRAMES * 4];
    let mut playback_pending = (0, 0);
    let mut microphone_pending = (0, 0);
    let mut last_timing = Instant::now();
    let mut previous_pump = Instant::now();
    let mut credit = NativeMicCredit {
        host: 0,
        submitted: 0,
        started: false,
    };
    while !stop.load(std::sync::atomic::Ordering::Acquire) {
        if let Some(error) = poll_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
        {
            return Err(error);
        }
        let observed = Instant::now();
        metrics.maximum_pump_gap_us.fetch_max(
            u64::try_from(observed.duration_since(previous_pump).as_micros()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        previous_pump = observed;
        if config.playback_native {
            if playback_pending.0 == playback_pending.1 {
                let block = streams
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .read_playback(&mut playback[..FRAMES * config.playback_channels])?;
                playback_pending = (0, block.frames);
            }
            if playback_pending.0 < playback_pending.1 {
                let offset = playback_pending.0 * config.playback_channels;
                let end = playback_pending.1 * config.playback_channels;
                playback_pending.0 += session.write_microphone(&playback[offset..end])?;
            }
        }
        if config.microphone_native {
            let host = host_frames.load(Ordering::Acquire);
            if credit.host == 0 && host > 0 {
                session.activate_inputs();
            }
            credit.update_host(host);
            let available = credit.available();
            if available > 0 && microphone_pending.0 == microphone_pending.1 {
                let block = session
                    .read_playback(&mut microphone[..FRAMES * config.microphone_channels])?;
                microphone_pending = (0, block.frames);
            }
            if available > 0 && microphone_pending.0 < microphone_pending.1 {
                let offset = microphone_pending.0 * config.microphone_channels;
                let end = (microphone_pending.0 + available).min(microphone_pending.1)
                    * config.microphone_channels;
                let accepted = streams
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .write_microphone(&microphone[offset..end])?;
                microphone_pending.0 += accepted;
                credit.accepted(accepted);
            }
        }
        if last_timing.elapsed() >= Duration::from_millis(100) {
            metrics.capture(session);
            last_timing = Instant::now();
        }
        thread::sleep(Duration::from_micros(500));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AudioExposure, AudioOptions};
    use gr_usbip::profile::ProfileId;
    use std::os::unix::net::UnixStream;
    fn test_control(progress: bool) -> (Arc<Mutex<Control>>, thread::JoinHandle<()>) {
        let (mut server, client) = UnixStream::pair().unwrap();
        let task = thread::spawn(move || {
            let started = Instant::now();
            while let Ok((tag, generation)) = gr_privileged_broker::read_message(&mut server) {
                assert_eq!((tag, generation.as_slice()), (7, &7_u64.to_le_bytes()[..]));
                let host = if progress {
                    u64::try_from(started.elapsed().as_micros()).unwrap() * 48 / 1_000
                } else {
                    0
                };
                let reply = [
                    generation,
                    host.to_le_bytes().to_vec(),
                    0_u64.to_le_bytes().to_vec(),
                ]
                .concat();
                gr_privileged_broker::write_message(&mut server, 7, &reply).unwrap();
            }
        });
        (
            Arc::new(Mutex::new(Control::new(client, 7, 1).unwrap())),
            task,
        )
    }
    #[test]
    fn native_microphone_credit_stops_with_host_and_resumes_with_bounded_fill() {
        let mut credit = NativeMicCredit {
            host: 0,
            submitted: 0,
            started: false,
        };
        assert_eq!(credit.available(), 0);
        credit.update_host(0);
        assert_eq!(credit.available(), 0);
        credit.update_host(48);
        assert_eq!(credit.available(), 128);
        for _ in 0..5 {
            credit.accepted(128);
        }
        assert_eq!(credit.available(), 32);
        credit.accepted(32);
        assert_eq!(credit.available(), 0);
        credit.update_host(96);
        assert_eq!(credit.available(), 48);
        credit.accepted(48);
        credit.update_host(960);
        assert_eq!(credit.submitted, 960);
        assert_eq!(credit.available(), 128);
    }
    #[test]
    fn graph_diagnostics_remain_readable_after_bridge_closure() {
        let timing = crate::AudioStreamTiming {
            endpoint: "synthetic.native".into(),
            graph_ticks: 42,
            tick_rate: (1, 48_000),
            observed_at_ns: 123,
            estimated_graph_delay_ticks: 7,
            discontinuities: 2,
            missed_graph_frames: 256,
        };
        let mut bridge = Bridge {
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            error: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Metrics {
                timings: Mutex::new(vec![timing.clone()]),
                source_underruns: AtomicU64::new(512),
                graph_input_dropped: AtomicU64::new(24),
                graph_input_fill: AtomicU64::new(128),
                graph_input_peak: AtomicU64::new(512),
                maximum_credit_request_us: AtomicU64::new(1_500),
                maximum_pump_gap_us: AtomicU64::new(3_000),
            }),
            thread: None,
        };
        bridge.close();
        bridge.close();
        assert_eq!(bridge.timings(), vec![timing]);
        assert_eq!(bridge.source_underrun_frames(), 512);
        assert_eq!(bridge.graph_input_dropped_frames(), 24);
        assert_eq!(bridge.graph_input_queue_frames(), (128, 512));
        assert_eq!(bridge.scheduling_us(), (1_500, 3_000));
    }
    #[test]
    fn mirror_preserves_group_formats_and_reverses_only_native_directions() {
        for profile in [
            gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap(),
            gr_curated_controllers::audio::dualshock4(AudioExposure::Emulated).unwrap(),
            gr_curated_controllers::audio::xbox360(AudioExposure::Emulated).unwrap(),
        ] {
            for native in [
                AudioOptions::new(AudioExposure::Emulated)
                    .with_playback_access(AudioAccess::NativeClient),
                AudioOptions::new(AudioExposure::Emulated)
                    .with_microphone_access(AudioAccess::NativeClient),
                AudioOptions::new(AudioExposure::Emulated)
                    .with_playback_access(AudioAccess::NativeClient)
                    .with_microphone_access(AudioAccess::NativeClient),
            ] {
                let (mirror, indices) = mirror_profile(&profile, native).unwrap();
                let mirror = mirror.unwrap();
                assert_eq!(mirror.streams().len(), indices.len());
                for (stream, index) in mirror.streams().iter().zip(indices) {
                    let original = &profile.streams()[index];
                    assert_eq!(stream.name(), original.name());
                    assert_eq!(stream.format(), original.format());
                    assert_ne!(stream.direction(), original.direction());
                }
            }
        }
    }

    #[test]
    #[ignore = "requires an isolated caller-session PipeWire graph"]
    fn native_bridge_registers_both_directions_and_closes() {
        let profile = gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap();
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let streams = SampleStreams::new(
            ProfileId::DualSenseEmulated,
            7,
            client_playback,
            client_microphone,
        )
        .unwrap();
        let options = AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(AudioAccess::NativeClient)
            .with_microphone_access(AudioAccess::NativeClient);
        let (control, task) = test_control(false);
        let (mut bridge, nodes) = Bridge::start(
            Arc::new(Mutex::new(streams)),
            control.clone(),
            Arc::new(AtomicU64::new(0)),
            &profile,
            options,
            7,
        )
        .unwrap();
        assert!(nodes.into_iter().all(|node| node.is_some()));
        assert!(bridge.as_ref().unwrap().error().is_none());
        bridge.as_mut().unwrap().close();
        drop(control);
        task.join().unwrap();
        drop((worker_playback, worker_microphone));
    }

    #[test]
    #[ignore = "requires an isolated PipeWire graph and pw-cat"]
    fn virtual_usb_playback_reaches_native_pipewire_source() {
        use gr_usbip::pcm_ipc::{Direction, Format, Sender};
        use std::{
            io::Read,
            process::{Command, Stdio},
            time::Instant,
        };
        let profile = gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap();
        let (worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (_worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let streams = SampleStreams::new(
            ProfileId::DualSenseEmulated,
            7,
            client_playback,
            client_microphone,
        )
        .unwrap();
        let options = AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(AudioAccess::NativeClient);
        let (control, task) = test_control(false);
        let (mut bridge, nodes) = Bridge::start(
            Arc::new(Mutex::new(streams)),
            control.clone(),
            Arc::new(AtomicU64::new(0)),
            &profile,
            options,
            71,
        )
        .unwrap();
        let node = nodes[0].as_ref().unwrap();
        let mut capture = Command::new("stdbuf")
            .args([
                "--output=0",
                "pw-cat",
                "--record",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                "4",
                "--latency",
                "128",
                "--target",
                node,
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let output = capture.stdout.take().unwrap();
        let reader = thread::spawn(move || {
            let mut output = output;
            let mut bytes = Vec::new();
            output.read_to_end(&mut bytes).unwrap();
            bytes
        });
        let mut sender = Sender::new(
            worker_playback,
            Format::new(ProfileId::DualSenseEmulated, Direction::Playback, 7).unwrap(),
        )
        .unwrap();
        let block = [101_i16, -202, 303, -404].repeat(128);
        let started = Instant::now();
        let mut position = 0_u64;
        while started.elapsed() < Duration::from_secs(2) {
            let now = u64::try_from(started.elapsed().as_micros()).unwrap();
            sender.pump(now).unwrap();
            if sender.send(&block, position, false, now).unwrap() == 128 {
                position += 128;
            }
            thread::sleep(Duration::from_millis(2));
        }
        capture.kill().unwrap();
        capture.wait().unwrap();
        let bytes = reader.join().unwrap();
        let marker = [101_i16, -202, 303, -404]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        assert!(
            bytes.windows(marker.len()).any(|frame| frame == marker),
            "no synthetic USB audio reached the caller PipeWire source"
        );
        bridge.as_mut().unwrap().close();
        drop(control);
        task.join().unwrap();
    }
    #[test]
    #[ignore = "requires an isolated PipeWire graph and pw-cat"]
    fn native_pipewire_microphone_reaches_virtual_usb_capture() {
        use gr_usbip::pcm_ipc::{Direction, Format, Receiver};
        use std::{
            io::Write,
            process::{Command, Stdio},
            time::Instant,
        };
        let profile = gr_curated_controllers::audio::dualsense(AudioExposure::Emulated).unwrap();
        let (_worker_playback, client_playback) = UnixStream::pair().unwrap();
        let (worker_microphone, client_microphone) = UnixStream::pair().unwrap();
        let streams = SampleStreams::new(
            ProfileId::DualSenseEmulated,
            7,
            client_playback,
            client_microphone,
        )
        .unwrap();
        let options = AudioOptions::new(AudioExposure::Emulated)
            .with_microphone_access(AudioAccess::NativeClient);
        let (control, task) = test_control(true);
        let (mut bridge, nodes) = Bridge::start(
            Arc::new(Mutex::new(streams)),
            control.clone(),
            Arc::new(AtomicU64::new(0)),
            &profile,
            options,
            72,
        )
        .unwrap();
        let node = nodes[1].as_ref().unwrap();
        let mut client = Command::new("pw-cat")
            .args([
                "--playback",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                "2",
                "--latency",
                "128",
                "--target",
                node,
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut input = client.stdin.take().unwrap();
        let writer = thread::spawn(move || {
            let data = [101_i16, -202]
                .repeat(48_000)
                .into_iter()
                .flat_map(i16::to_le_bytes)
                .collect::<Vec<_>>();
            let _ = input.write_all(&data);
        });
        let mut receiver = Receiver::new(
            worker_microphone,
            Format::new(ProfileId::DualSenseEmulated, Direction::Microphone, 7).unwrap(),
        )
        .unwrap();
        let mut samples = [0_i16; 128 * 2];
        let started = Instant::now();
        let mut found = false;
        while started.elapsed() < Duration::from_secs(4) {
            let now = u64::try_from(started.elapsed().as_micros()).unwrap();
            if let Some(block) = receiver.receive(&mut samples, now).unwrap() {
                found |= samples[..block.frames * 2]
                    .chunks_exact(2)
                    .any(|frame| frame == [101, -202]);
                if found {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        client.kill().ok();
        client.wait().unwrap();
        writer.join().unwrap();
        assert!(
            found,
            "no caller PipeWire microphone samples reached virtual USB capture"
        );
        bridge.as_mut().unwrap().close();
        drop(control);
        task.join().unwrap();
    }
}
