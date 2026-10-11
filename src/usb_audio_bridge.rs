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
#[derive(Clone, Copy, Default)]
struct BridgeSnapshot {
    sampled_at: Option<Instant>,
    credit_host_frames: u64,
    credit_submitted_frames: u64,
    accepted_source_frames: u64,
    graph_queue_frames: Option<u64>,
    client_queue_frames: u64,
    client_pump_maximum_interval_us: u64,
    client_pump_policy: Option<u32>,
    client_pump_kernel_priority: Option<i32>,
    pending_frames: u64,
}
struct SilenceLedger {
    events: Vec<crate::audio::AudioNativeMicrophoneSilenceEvent>,
    omitted_events: u64,
}
impl Default for SilenceLedger {
    fn default() -> Self {
        Self {
            events: Vec::with_capacity(64),
            omitted_events: 0,
        }
    }
}
impl SilenceLedger {
    fn record(&mut self, event: crate::audio::AudioNativeMicrophoneSilenceEvent) {
        if self.events.len() < 64 {
            self.events.push(event);
        } else {
            self.omitted_events = self.omitted_events.saturating_add(1);
        }
    }
}
#[derive(Default)]
struct Metrics {
    timings: Mutex<Vec<crate::audio::AudioStreamTiming>>,
    loss_trace: Mutex<Vec<crate::audio::AudioInputLossTrace>>,
    bridge_snapshot: Mutex<BridgeSnapshot>,
    silence_ledger: Mutex<SilenceLedger>,
    source_underruns: AtomicU64,
    graph_input_dropped: AtomicU64,
    graph_input_empty: AtomicU64,
    graph_input_corrupted: AtomicU64,
    graph_input_overflow: AtomicU64,
    graph_input_fill: AtomicU64,
    graph_input_peak: AtomicU64,
    maximum_credit_request_us: AtomicU64,
    maximum_pump_gap_us: AtomicU64,
    maximum_host_poll_gap_us: AtomicU64,
    maximum_host_advance_frames: AtomicU64,
}
impl Metrics {
    fn capture_bridge_state(
        &self,
        session: &gr_audio_linux::Session,
        streams: &Arc<Mutex<SampleStreams>>,
        credit: &NativeMicCredit,
        accepted_source_frames: u64,
        pending_frames: usize,
    ) {
        let (client_queue_frames, client_pump_maximum_interval_us, client_pump_scheduling) = {
            let streams = streams
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (
                streams.queued_microphone_frames() as u64,
                streams.maximum_pump_interval_us(),
                streams.pump_scheduling(),
            )
        };
        let graph_queue_frames = session.queued_playback_frames().map(|frames| frames as u64);
        *self
            .bridge_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = BridgeSnapshot {
            sampled_at: Some(Instant::now()),
            credit_host_frames: credit.host,
            credit_submitted_frames: credit.submitted,
            accepted_source_frames,
            graph_queue_frames,
            client_queue_frames,
            client_pump_maximum_interval_us,
            client_pump_policy: client_pump_scheduling.map(|scheduling| scheduling.policy),
            client_pump_kernel_priority: client_pump_scheduling
                .map(|scheduling| scheduling.kernel_priority),
            pending_frames: pending_frames as u64,
        };
    }
    fn observe_silence(
        &self,
        previous_host: u64,
        host: u64,
        previous_silence: u64,
        silence: u64,
        gap: Duration,
    ) {
        if silence <= previous_silence {
            return;
        }
        let state = *self
            .bridge_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let observed = Instant::now();
        let event = crate::audio::AudioNativeMicrophoneSilenceEvent {
            previous_host_frames: previous_host,
            host_frames: host,
            previous_silence_frames: previous_silence,
            silence_frames: silence,
            reply_interval_us: u64::try_from(gap.as_micros()).unwrap_or(u64::MAX),
            bridge: state
                .sampled_at
                .and_then(|sampled| observed.checked_duration_since(sampled))
                .map(|age| crate::audio::AudioNativeMicrophoneBridgeSample {
                    age_us: u64::try_from(age.as_micros()).unwrap_or(u64::MAX),
                    credit_host_frames: state.credit_host_frames,
                    credit_submitted_frames: state.credit_submitted_frames,
                    accepted_source_frames: state.accepted_source_frames,
                    graph_queue_frames: state.graph_queue_frames,
                    client_queue_frames: state.client_queue_frames,
                    client_pump_maximum_interval_us: state.client_pump_maximum_interval_us,
                    client_pump_policy: state.client_pump_policy,
                    client_pump_kernel_priority: state.client_pump_kernel_priority,
                    pending_frames: state.pending_frames,
                }),
        };
        self.silence_ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .record(event);
    }
    fn observe_host_clock(&self, gap: Duration, previous: u64, host: u64) {
        self.maximum_host_poll_gap_us.fetch_max(
            u64::try_from(gap.as_micros()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        self.maximum_host_advance_frames
            .fetch_max(host.saturating_sub(previous), Ordering::Relaxed);
    }
    fn capture(&self, session: &gr_audio_linux::Session) {
        self.source_underruns
            .store(session.underrun_frames(), Ordering::Release);
        self.graph_input_dropped
            .store(session.dropped_playback_frames(), Ordering::Release);
        *self
            .loss_trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = session
            .input_loss_trace()
            .into_iter()
            .map(|trace| crate::audio::AudioInputLossTrace {
                endpoint: trace.endpoint,
                received_frames: trace.received_frames,
                omitted_events: trace.omitted_events,
                events: trace
                    .events
                    .into_iter()
                    .map(|event| crate::audio::AudioInputLossEvent {
                        first_frame: event.first_frame,
                        frames: event.frames,
                        graph_ticks: event.graph_ticks,
                        reason: match event.reason {
                            gr_audio_linux::InputLossReason::Empty => {
                                crate::audio::AudioInputLossReason::Empty
                            }
                            gr_audio_linux::InputLossReason::Corrupted => {
                                crate::audio::AudioInputLossReason::Corrupted
                            }
                            gr_audio_linux::InputLossReason::QueueOverflow => {
                                crate::audio::AudioInputLossReason::QueueOverflow
                            }
                        },
                    })
                    .collect(),
            })
            .collect();
        let (empty, corrupted, overflow) = session.input_loss_frames();
        self.graph_input_empty.store(empty, Ordering::Release);
        self.graph_input_corrupted
            .store(corrupted, Ordering::Release);
        self.graph_input_overflow.store(overflow, Ordering::Release);
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
            .map(|timing| crate::audio::AudioStreamTiming {
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
fn refill_native_microphone(
    credit: &mut NativeMicCredit,
    mut transfer: impl FnMut(usize) -> Result<usize, AudioError>,
) -> Result<(), AudioError> {
    // One delayed wake-up may represent several wire packets. Refill only the
    // existing operating fill, with a finite per-cycle budget and no busy wait.
    for _ in 0..NativeMicCredit::FILL.div_ceil(128) {
        let available = credit.available();
        if available == 0 {
            break;
        }
        let accepted = transfer(available)?;
        if accepted > available {
            return Err(AudioError::InvalidRequirement);
        }
        if accepted == 0 {
            break;
        }
        credit.accepted(accepted);
    }
    Ok(())
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
        let mut previous_completion = Instant::now();
        let mut previous_host = 0;
        let mut previous_silence = 0;
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
                    let completed = Instant::now();
                    metrics.observe_host_clock(
                        completed.duration_since(previous_completion),
                        previous_host,
                        host,
                    );
                    metrics.observe_silence(
                        previous_host,
                        host,
                        previous_silence,
                        silence,
                        completed.duration_since(previous_completion),
                    );
                    previous_completion = completed;
                    previous_host = host;
                    previous_silence = silence;
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
                    match crate::audio::merge_errors(result.err(), session.error().cloned()) {
                        Some(error) => Err(error),
                        None => Ok(()),
                    }
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
    pub(crate) fn timings(&self) -> Vec<crate::audio::AudioStreamTiming> {
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
    pub(crate) fn graph_input_loss_frames(&self) -> (u64, u64, u64) {
        (
            self.metrics.graph_input_empty.load(Ordering::Acquire),
            self.metrics.graph_input_corrupted.load(Ordering::Acquire),
            self.metrics.graph_input_overflow.load(Ordering::Acquire),
        )
    }
    pub(crate) fn graph_input_loss_trace(&self) -> Vec<crate::audio::AudioInputLossTrace> {
        self.metrics
            .loss_trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
    pub(crate) fn silence_trace(&self) -> crate::audio::AudioNativeMicrophoneSilenceTrace {
        let ledger = self
            .metrics
            .silence_ledger
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::audio::AudioNativeMicrophoneSilenceTrace {
            events: ledger.events.clone(),
            omitted_events: ledger.omitted_events,
        }
    }
    pub(crate) fn host_clock_progress(&self) -> (u64, u64) {
        (
            self.metrics
                .maximum_host_poll_gap_us
                .load(Ordering::Acquire),
            self.metrics
                .maximum_host_advance_frames
                .load(Ordering::Acquire),
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
    let mut accepted_source_frames = 0_u64;
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
            refill_native_microphone(&mut credit, |available| {
                if microphone_pending.0 == microphone_pending.1 {
                    let block = session
                        .read_playback(&mut microphone[..FRAMES * config.microphone_channels])?;
                    microphone_pending = (0, block.frames);
                }
                if microphone_pending.0 == microphone_pending.1 {
                    return Ok(0);
                }
                let offset = microphone_pending.0 * config.microphone_channels;
                let end = (microphone_pending.0 + available).min(microphone_pending.1)
                    * config.microphone_channels;
                let accepted = streams
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .write_microphone(&microphone[offset..end])?;
                microphone_pending.0 += accepted;
                accepted_source_frames = accepted_source_frames.saturating_add(accepted as u64);
                Ok(accepted)
            })?;
            metrics.capture_bridge_state(
                session,
                streams,
                &credit,
                accepted_source_frames,
                microphone_pending.1 - microphone_pending.0,
            );
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
    fn native_microphone_refill_restores_existing_fill_after_delayed_host_progress() {
        let mut credit = NativeMicCredit {
            host: 0,
            submitted: 0,
            started: false,
        };
        refill_native_microphone(&mut credit, |_| panic!("capture has not started")).unwrap();
        credit.update_host(48);
        let mut packets = Vec::new();
        refill_native_microphone(&mut credit, |available| {
            packets.push(available);
            Ok(available)
        })
        .unwrap();
        assert_eq!(packets, [128, 128, 128, 128, 128, 32]);
        assert_eq!(credit.submitted, 48 + NativeMicCredit::FILL);
        // Six milliseconds pass while the bridge is asleep. One packet cannot
        // replace the 288 frames consumed; catch up without expanding the fill.
        credit.update_host(48 + 288);
        packets.clear();
        refill_native_microphone(&mut credit, |available| {
            packets.push(available);
            Ok(available)
        })
        .unwrap();
        assert_eq!(packets, [128, 128, 32]);
        assert_eq!(credit.available(), 0);
        assert_eq!(credit.submitted, credit.host + NativeMicCredit::FILL);
    }

    #[test]
    fn native_microphone_refill_bounds_partial_progress_and_preserves_backpressure_errors() {
        let mut credit = NativeMicCredit {
            host: 48,
            submitted: 48,
            started: true,
        };
        let mut attempts = 0;
        refill_native_microphone(&mut credit, |_| {
            attempts += 1;
            Ok(0)
        })
        .unwrap();
        assert_eq!(attempts, 1);
        assert_eq!(credit.submitted, 48);
        attempts = 0;
        refill_native_microphone(&mut credit, |_| {
            attempts += 1;
            Ok(1)
        })
        .unwrap();
        assert_eq!(attempts, 6);
        assert_eq!(credit.submitted, 54);
        let error = AudioError::Backend {
            reason: "synthetic transfer failure".into(),
        };
        assert_eq!(
            refill_native_microphone(&mut credit, |_| Err(error.clone())),
            Err(error)
        );
        assert_eq!(credit.submitted, 54);
        assert_eq!(
            refill_native_microphone(&mut credit, |available| Ok(available + 1)),
            Err(AudioError::InvalidRequirement)
        );
        assert_eq!(credit.submitted, 54);
    }

    #[test]
    fn graph_diagnostics_remain_readable_after_bridge_closure() {
        let timing = crate::audio::AudioStreamTiming {
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
                loss_trace: Mutex::new(vec![crate::audio::AudioInputLossTrace {
                    endpoint: "synthetic.native".into(),
                    received_frames: 256,
                    events: vec![crate::audio::AudioInputLossEvent {
                        reason: crate::audio::AudioInputLossReason::Empty,
                        first_frame: 0,
                        frames: 8,
                        graph_ticks: Some(42),
                    }],
                    omitted_events: 3,
                }]),
                source_underruns: AtomicU64::new(512),
                graph_input_dropped: AtomicU64::new(24),
                graph_input_empty: AtomicU64::new(8),
                graph_input_corrupted: AtomicU64::new(4),
                graph_input_overflow: AtomicU64::new(12),
                graph_input_fill: AtomicU64::new(128),
                graph_input_peak: AtomicU64::new(512),
                maximum_credit_request_us: AtomicU64::new(1_500),
                maximum_pump_gap_us: AtomicU64::new(3_000),
                maximum_host_poll_gap_us: AtomicU64::new(9_000),
                maximum_host_advance_frames: AtomicU64::new(432),
                bridge_snapshot: Mutex::new(BridgeSnapshot::default()),
                silence_ledger: Mutex::new(SilenceLedger::default()),
            }),
            thread: None,
        };
        bridge
            .metrics
            .observe_silence(0, 48, 0, 8, Duration::from_millis(1));
        let retained_silence = bridge.silence_trace();
        bridge.close();
        bridge.close();
        assert_eq!(bridge.silence_trace(), retained_silence);
        assert_eq!(bridge.timings(), vec![timing]);
        assert_eq!(bridge.source_underrun_frames(), 512);
        assert_eq!(bridge.graph_input_dropped_frames(), 24);
        assert_eq!(bridge.graph_input_loss_frames(), (8, 4, 12));
        let trace = bridge.graph_input_loss_trace();
        assert_eq!(trace[0].received_frames, 256);
        assert_eq!(trace[0].events[0].frames, 8);
        assert_eq!(trace[0].omitted_events, 3);
        assert_eq!(bridge.graph_input_queue_frames(), (128, 512));
        assert_eq!(bridge.scheduling_us(), (1_500, 3_000));
        assert_eq!(bridge.host_clock_progress(), (9_000, 432));
        assert_eq!(bridge.silence_trace().events.len(), 1);
        assert_eq!(bridge.silence_trace().omitted_events, 0);
    }
    #[test]
    fn silence_ledger_preserves_host_bounds_and_unavailable_bridge_samples() {
        let metrics = Metrics::default();
        metrics.observe_silence(0, 48, 0, 0, Duration::from_millis(1));
        assert!(metrics.silence_ledger.lock().unwrap().events.is_empty());
        metrics.observe_silence(48, 96, 0, 16, Duration::from_millis(2));
        let event = metrics.silence_ledger.lock().unwrap().events[0].clone();
        assert_eq!((event.previous_host_frames, event.host_frames), (48, 96));
        assert_eq!(
            (event.previous_silence_frames, event.silence_frames),
            (0, 16)
        );
        assert_eq!(event.reply_interval_us, 2_000);
        assert_eq!(event.bridge, None);
        *metrics.bridge_snapshot.lock().unwrap() = BridgeSnapshot {
            sampled_at: Some(
                Instant::now()
                    .checked_sub(Duration::from_millis(1))
                    .unwrap(),
            ),
            credit_host_frames: 96,
            credit_submitted_frames: 768,
            accepted_source_frames: 672,
            graph_queue_frames: Some(256),
            client_queue_frames: 128,
            pending_frames: 32,
            client_pump_maximum_interval_us: 2_000,
            client_pump_policy: Some(2),
            client_pump_kernel_priority: Some(79),
        };
        metrics.observe_silence(96, 144, 16, 24, Duration::from_millis(3));
        let sample = metrics.silence_ledger.lock().unwrap().events[1]
            .bridge
            .clone()
            .unwrap();
        assert!(sample.age_us >= 1_000);
        assert_eq!(
            (sample.credit_host_frames, sample.credit_submitted_frames),
            (96, 768)
        );
        assert_eq!(sample.accepted_source_frames, 672);
        assert_eq!(sample.graph_queue_frames, Some(256));
        assert_eq!(
            (sample.client_queue_frames, sample.pending_frames),
            (128, 32)
        );
        metrics.bridge_snapshot.lock().unwrap().sampled_at =
            Some(Instant::now() + Duration::from_secs(3_600));
        metrics.observe_silence(144, 192, 24, 32, Duration::from_millis(1));
        assert_eq!(
            metrics.silence_ledger.lock().unwrap().events[2].bridge,
            None
        );
    }

    #[test]
    fn silence_ledger_bounds_events_without_allocating_more_slots() {
        let metrics = Metrics::default();
        let capacity = metrics.silence_ledger.lock().unwrap().events.capacity();
        for i in 0..70 {
            metrics.observe_silence(i * 48, (i + 1) * 48, i, i + 1, Duration::from_millis(1));
        }
        // Equal or regressing counters are not fabricated as loss increases.
        metrics.observe_silence(3_360, 3_408, 70, 70, Duration::from_millis(1));
        metrics.observe_silence(3_408, 3_456, 70, 0, Duration::from_millis(1));
        let ledger = metrics.silence_ledger.lock().unwrap();
        assert_eq!(ledger.events.len(), 64);
        assert_eq!(ledger.events.capacity(), capacity);
        assert_eq!(ledger.omitted_events, 6);
        assert_eq!(ledger.events[0].previous_host_frames, 0);
        assert_eq!(ledger.events[63].host_frames, 3_072);
    }

    #[test]
    fn silence_ledger_reads_one_coherent_bridge_sample_during_publication() {
        let metrics = Arc::new(Metrics::default());
        *metrics.bridge_snapshot.lock().unwrap() = BridgeSnapshot {
            sampled_at: Some(Instant::now()),
            graph_queue_frames: Some(0),
            ..BridgeSnapshot::default()
        };
        let writer = metrics.clone();
        let thread = std::thread::spawn(move || {
            for value in 0..500 {
                *writer.bridge_snapshot.lock().unwrap() = BridgeSnapshot {
                    sampled_at: Some(Instant::now()),
                    credit_host_frames: value,
                    credit_submitted_frames: value,
                    accepted_source_frames: value,
                    graph_queue_frames: Some(value),
                    client_queue_frames: value,
                    pending_frames: value,
                    ..BridgeSnapshot::default()
                };
            }
        });
        for i in 0..64 {
            metrics.observe_silence(i * 48, (i + 1) * 48, i, i + 1, Duration::from_millis(1));
        }
        thread.join().unwrap();
        for event in &metrics.silence_ledger.lock().unwrap().events {
            assert!(event.bridge.is_some());
            if let Some(sample) = &event.bridge {
                assert_eq!(sample.credit_host_frames, sample.credit_submitted_frames);
                assert_eq!(sample.credit_host_frames, sample.accepted_source_frames);
                assert_eq!(Some(sample.credit_host_frames), sample.graph_queue_frames);
                assert_eq!(sample.credit_host_frames, sample.client_queue_frames);
                assert_eq!(sample.credit_host_frames, sample.pending_frames);
            }
        }
    }

    #[test]
    fn host_clock_observations_include_poll_sleep_and_preserve_maxima() {
        let metrics = Metrics::default();
        metrics.observe_host_clock(Duration::from_millis(15), 100, 820);
        metrics.observe_host_clock(Duration::from_millis(2), 820, 916);
        metrics.observe_host_clock(Duration::from_millis(3), 916, 900);
        assert_eq!(
            metrics.maximum_host_poll_gap_us.load(Ordering::Acquire),
            15_000
        );
        assert_eq!(
            metrics.maximum_host_advance_frames.load(Ordering::Acquire),
            720
        );
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
