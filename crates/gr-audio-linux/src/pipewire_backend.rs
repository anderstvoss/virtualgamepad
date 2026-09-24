use crate::timing;
use gr_audio_contract::{
    AudioAccess, AudioError, AudioOptions, AudioProfile, PcmFormat, SampleDirection,
    queue::{PcmConsumer, PcmObserver, PcmProducer, PcmRead, pcm_queue},
};
use pipewire as pw;
use pw::{properties::properties, spa};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

const QUEUE_FRAMES: usize = 2_048;
const SCRATCH_SAMPLES: usize = 32 * 4_096;
// libspa 0.10 exposes CORRUPTED but omits SPA_CHUNK_FLAG_EMPTY (1 << 1).
const SPA_CHUNK_FLAG_EMPTY: i32 = 1 << 1;

/// Names are exact per-creation `PipeWire` selectors, not persistent identity.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub host_node: String,
    pub caller_node: Option<String>,
    pub format: PcmFormat,
    pub direction: SampleDirection,
    pub access: AudioAccess,
}
/// Owns the worker and its endpoint lifetime. Never detaches a worker on drop.
pub struct Session {
    endpoints: Vec<Endpoint>,
    clocks: Vec<(String, Arc<timing::Shared>)>,
    playback: Option<PcmConsumer>,
    playback_observer: Option<PcmObserver>,
    microphone: Option<PcmProducer>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    underruns: Arc<AtomicU64>,
    activate_input: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Result<(), AudioError>>>,
    error: Option<AudioError>,
}
impl Session {
    /// # Errors
    /// Returns an error if every endpoint cannot register within the startup deadline.
    pub fn open(
        profile: &AudioProfile,
        options: AudioOptions,
        creation: u64,
    ) -> Result<Self, AudioError> {
        Self::open_inner(profile, options, creation, 0, true)
    }
    /// Use a bounded operating lead for a USB-to-PipeWire bridge. The source
    /// waits for one requested graph block plus this many frames before first
    /// delivery, and rearms after an underrun. The ordinary audio path uses 0.
    /// # Errors
    /// Returns an error if endpoint registration fails.
    pub fn open_with_source_preroll(
        profile: &AudioProfile,
        options: AudioOptions,
        creation: u64,
        source_preroll: usize,
    ) -> Result<Self, AudioError> {
        Self::open_inner(profile, options, creation, source_preroll, true)
    }
    /// A USB bridge registers its native microphone input before the host
    /// begins capture, but activates graph processing only after host media
    /// time starts. Endpoint identity remains fixed for the session.
    /// # Errors
    /// Returns an error if endpoint registration fails.
    pub fn open_usb_bridge(
        profile: &AudioProfile,
        options: AudioOptions,
        creation: u64,
        source_preroll: usize,
    ) -> Result<Self, AudioError> {
        Self::open_inner(profile, options, creation, source_preroll, false)
    }
    fn open_inner(
        profile: &AudioProfile,
        options: AudioOptions,
        creation: u64,
        source_preroll: usize,
        input_initially_active: bool,
    ) -> Result<Self, AudioError> {
        if source_preroll > 512 {
            return Err(AudioError::InvalidRequirement);
        }
        if !valid_profile(profile) {
            return Err(AudioError::IncompatibleTopology);
        }
        let stop = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let underruns = Arc::new(AtomicU64::new(0));
        let activate_input = Arc::new(AtomicBool::new(input_initially_active));
        let Prepared {
            specs,
            endpoints,
            playback,
            playback_observer,
            microphone,
        } = prepare(profile, options, creation, source_preroll)?;
        let clocks = specs
            .iter()
            .map(|s| (s.name.clone(), s.clock.clone()))
            .collect();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker_stop = stop.clone();
        let worker_failed = failed.clone();
        let worker_underruns = underruns.clone();
        let worker_activate_input = activate_input.clone();
        let worker = thread::Builder::new()
            .name("controller-audio".into())
            .spawn(move || {
                let result = run(
                    specs,
                    &worker_stop,
                    &worker_failed,
                    &worker_underruns,
                    &worker_activate_input,
                    &ready_tx,
                );
                if result.is_err() {
                    worker_failed.store(true, Ordering::Release);
                }
                result
            })
            .map_err(backend)?;
        let mut session = Self {
            endpoints,
            clocks,
            playback,
            playback_observer,
            microphone,
            stop,
            failed,
            underruns,
            activate_input,
            worker: Some(worker),
            error: None,
        };
        if ready_rx.recv_timeout(Duration::from_secs(4)).is_ok() {
            Ok(session)
        } else {
            session.close();
            Err(session.error.take().unwrap_or(AudioError::Backend {
                reason: "PipeWire endpoint startup timed out".into(),
            }))
        }
    }
    #[must_use]
    pub fn timings(&self) -> Vec<timing::StreamTiming> {
        self.clocks
            .iter()
            .filter_map(|(name, clock)| clock.snapshot(name))
            .collect()
    }
    #[must_use]
    pub fn endpoints(&self) -> &[Endpoint] {
        &self.endpoints
    }
    #[must_use]
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
            || self
                .worker
                .as_ref()
                .is_some_and(std::thread::JoinHandle::is_finished)
    }
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.stop.load(Ordering::Acquire) || self.failed()
    }
    fn check(&self) -> Result<(), AudioError> {
        if self.is_closed() {
            Err(AudioError::Closed)
        } else {
            Ok(())
        }
    }
    /// # Errors
    /// Returns Closed, incompatible access mode, or incomplete frame alignment.
    pub fn read_playback(&mut self, dest: &mut [i16]) -> Result<PcmRead, AudioError> {
        self.check()?;
        self.playback
            .as_mut()
            .ok_or(AudioError::IncompatibleTopology)?
            .read(dest)
    }
    /// # Errors
    /// Returns Closed, incompatible access mode, or incomplete frame alignment.
    pub fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError> {
        self.check()?;
        self.microphone
            .as_mut()
            .ok_or(AudioError::IncompatibleTopology)?
            .push(samples)
    }
    #[must_use]
    pub fn queued_microphone_frames(&self) -> Option<usize> {
        self.microphone.as_ref().map(PcmProducer::queued_frames)
    }
    /// Discard queued microphone audio at the next backend read.
    /// # Errors
    /// Returns Closed or incompatible access mode.
    pub fn flush_microphone(&mut self) -> Result<(), AudioError> {
        self.check()?;
        self.microphone
            .as_mut()
            .ok_or(AudioError::IncompatibleTopology)?
            .flush()
    }
    /// # Errors
    /// Returns Closed or incompatible access mode.
    pub fn flush_playback(&mut self) -> Result<(), AudioError> {
        self.check()?;
        self.playback
            .as_mut()
            .ok_or(AudioError::IncompatibleTopology)?
            .flush()
    }
    #[must_use]
    pub fn underrun_frames(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }
    #[must_use]
    pub fn dropped_playback_frames(&self) -> u64 {
        self.playback_observer
            .as_ref()
            .map_or(0, PcmObserver::discarded_frames)
    }
    #[must_use]
    pub fn queued_playback_frames(&self) -> Option<usize> {
        self.playback.as_ref().map(PcmConsumer::queued_frames)
    }
    pub fn activate_inputs(&self) {
        self.activate_input.store(true, Ordering::Release);
    }
    #[must_use]
    pub fn error(&self) -> Option<&AudioError> {
        self.error.as_ref()
    }
    pub fn close(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            self.error = match worker.join() {
                Ok(Ok(())) => None,
                Ok(Err(e)) => Some(e),
                Err(_) => Some(AudioError::Backend {
                    reason: "audio worker panicked".into(),
                }),
            };
        }
        if self.error.is_some() {
            self.failed.store(true, Ordering::Release);
        }
        if let Some(tx) = &mut self.microphone {
            tx.close();
        }
        if let Some(rx) = &mut self.playback {
            rx.close();
        }
    }
}
fn valid_profile(profile: &AudioProfile) -> bool {
    !profile.streams().is_empty()
        && profile.streams().len() <= 2
        && profile
            .streams()
            .iter()
            .filter(|s| s.direction() == SampleDirection::HostToController)
            .count()
            <= 1
        && profile
            .streams()
            .iter()
            .filter(|s| s.direction() == SampleDirection::ControllerToHost)
            .count()
            <= 1
}
impl Drop for Session {
    fn drop(&mut self) {
        self.close();
    }
}
fn backend(e: impl std::fmt::Display) -> AudioError {
    AudioError::Backend {
        reason: e.to_string(),
    }
}
enum Io {
    Sink(PcmProducer),
    Source(PcmConsumer),
}
struct Spec {
    clock: Arc<timing::Shared>,
    name: String,
    format: PcmFormat,
    io: Io,
    source_preroll: usize,
}
struct ControlCallback {
    generation: Arc<AtomicU64>,
    format: PcmFormat,
    failed: Arc<AtomicBool>,
}
struct Callback {
    generation: Arc<AtomicU64>,
    seen_generation: u64,
    clock: Arc<timing::Shared>,
    tracker: timing::Tracker,
    sample_rate: u32,
    io: Io,
    channels: usize,
    scratch: Box<[i16]>,
    failed: Arc<AtomicBool>,
    underruns: Arc<AtomicU64>,
    source_preroll: usize,
    source_started: bool,
}
// Each listener owns separate userdata: the binding creates &mut callback
// state even before dispatching an event. Only process state crosses onto the
// data loop. Disconnect synchronously removes the node from that loop before
// either listener is freed (including rollback and unwinding).
struct ManagedStream {
    stream: pw::stream::StreamRc,
    control: Option<pw::stream::StreamListener<ControlCallback>>,
    process: Option<pw::stream::StreamListener<Callback>>,
    input: bool,
}
impl ManagedStream {
    fn open(
        core: &pw::core::CoreRc,
        spec: Spec,
        failed: &Arc<AtomicBool>,
        underruns: &Arc<AtomicU64>,
        input_initially_active: bool,
    ) -> Result<Self, AudioError> {
        let input = matches!(spec.io, Io::Sink(_));
        let bytes = format_pod(&spec.format)?;
        let pod = spa::pod::Pod::from_bytes(&bytes).ok_or(AudioError::InvalidRequirement)?;
        let stream = pw::stream::StreamRc::new(
            core.clone(),
            &spec.name,
            endpoint_properties(&spec.name, input, spec.format.sample_rate_hz()),
        )
        .map_err(backend)?;
        let generation = Arc::new(AtomicU64::new(0));
        let cb = Callback {
            generation: generation.clone(),
            seen_generation: 0,
            clock: spec.clock.clone(),
            tracker: timing::Tracker::default(),
            sample_rate: spec.format.sample_rate_hz(),
            io: spec.io,
            channels: spec.format.channels().len(),
            scratch: vec![0; SCRATCH_SAMPLES].into_boxed_slice(),
            failed: failed.clone(),
            underruns: underruns.clone(),
            source_preroll: spec.source_preroll,
            source_started: false,
        };
        let control = stream
            .add_local_listener_with_user_data(ControlCallback {
                generation,
                format: spec.format.clone(),
                failed: failed.clone(),
            })
            .state_changed(|_, data, old, new| {
                if matches!(
                    new,
                    pw::stream::StreamState::Paused | pw::stream::StreamState::Unconnected
                ) {
                    data.generation.fetch_add(1, Ordering::Release);
                }
                if matches!(new, pw::stream::StreamState::Error(_))
                    || (matches!(new, pw::stream::StreamState::Unconnected)
                        && !matches!(old, pw::stream::StreamState::Unconnected))
                {
                    data.failed.store(true, Ordering::Release);
                }
            })
            .param_changed(|_, data, id, pod| check_format(data, id, pod))
            .register()
            .map_err(backend)?;
        require_send(&cb);
        let process = stream
            .add_local_listener_with_user_data(cb)
            .process(process)
            .register()
            .map_err(backend)?;
        let mut managed = ManagedStream {
            stream,
            control: Some(control),
            process: Some(process),
            input,
        };
        let connected = managed
            .stream
            .connect(
                if input {
                    spa::utils::Direction::Input
                } else {
                    spa::utils::Direction::Output
                },
                None,
                stream_flags(),
                &mut [pod],
            )
            .map_err(backend);
        if connected.is_err() {
            finish_cleanup(connected, std::iter::once(managed.disconnect()))?;
        }
        if input && !input_initially_active {
            managed.stream.set_active(false).map_err(backend)?;
        }
        Ok(managed)
    }

    fn disconnect(&mut self) -> Result<(), AudioError> {
        if self.process.is_none() {
            return Ok(());
        }
        // Intentional teardown must not be reported as an unexpected host
        // disconnect. The control listener has no process callback.
        drop(self.control.take());
        let result = self.stream.disconnect().map_err(backend);
        drop(self.process.take());
        result
    }
}
impl Drop for ManagedStream {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

fn require_send<T: Send>(_: &T) {}

fn run(
    specs: Vec<Spec>,
    stop: &Arc<AtomicBool>,
    failed: &Arc<AtomicBool>,
    underruns: &Arc<AtomicU64>,
    activate_input: &Arc<AtomicBool>,
    ready: &mpsc::SyncSender<()>,
) -> Result<(), AudioError> {
    pw::init();
    let main = pw::main_loop::MainLoopRc::new(None).map_err(backend)?;
    let context = pw::context::ContextRc::new(&main, None).map_err(backend)?;
    let core = context.connect_rc(None).map_err(backend)?;
    let core_failed = failed.clone();
    let _core_listener = core
        .add_listener_local()
        .error(move |_, _, _, _| {
            core_failed.store(true, Ordering::Release);
        })
        .register();
    let mut streams = Vec::new();
    let result = (|| {
        for spec in specs {
            streams.push(ManagedStream::open(
                &core,
                spec,
                failed,
                underruns,
                activate_input.load(Ordering::Acquire),
            )?);
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut notified = false;
        let mut input_activated = activate_input.load(Ordering::Acquire);
        while !stop.load(Ordering::Acquire) {
            if failed.load(Ordering::Acquire) {
                return Err(backend("PipeWire endpoint or server failed"));
            }
            main.loop_()
                .iterate(pw::loop_::Timeout::Finite(Duration::from_millis(5)));
            if !input_activated && activate_input.load(Ordering::Acquire) {
                for stream in streams.iter().filter(|stream| stream.input) {
                    stream.stream.set_active(true).map_err(backend)?;
                }
                input_activated = true;
            }
            if !notified {
                if streams.iter().all(|s| {
                    matches!(
                        s.stream.state(),
                        pw::stream::StreamState::Paused | pw::stream::StreamState::Streaming
                    )
                }) {
                    ready.send(()).map_err(backend)?;
                    notified = true;
                } else if Instant::now() >= deadline {
                    return Err(backend("PipeWire registration deadline exceeded"));
                }
            }
        }
        Ok(())
    })();
    // Drain every teardown, including partial registration and backend failure.
    // Drop still protects unwinding, but ordinary errors must retain cleanup causes.
    finish_cleanup(result, streams.iter_mut().map(ManagedStream::disconnect))
}

fn finish_cleanup(
    mut result: Result<(), AudioError>,
    cleanup: impl Iterator<Item = Result<(), AudioError>>,
) -> Result<(), AudioError> {
    for next in cleanup {
        if let Err(cleanup_error) = next {
            result = Err(match result {
                Ok(()) => cleanup_error,
                Err(primary) => backend(format!("{primary}; cleanup: {cleanup_error}")),
            });
        }
    }
    result
}

// Request a ~2.67 ms quantum, without forcing global graph settings. Host
// policy can clamp this request; acceptance must measure the resulting path.
fn endpoint_properties(name: &str, input: bool, rate: u32) -> pw::properties::PropertiesBox {
    properties! {
        "node.name" => name,
        "node.description" => name,
        "media.type" => "Audio",
        "media.class" => if input { "Audio/Sink" } else { "Audio/Source" },
        "node.virtual" => "true",
        "node.autoconnect" => "false",
        "node.dont-reconnect" => "true",
        "stream.dont-remix" => "true",
        "priority.session" => "0",
        "node.latency" => format!("{}/{}", rate.div_ceil(375), rate),
    }
}

fn process(stream: &pw::stream::Stream, data: &mut Callback) {
    if data.failed.load(Ordering::Acquire) {
        return;
    }
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return;
    };
    let generation = data.generation.load(Ordering::Acquire);
    if generation != data.seen_generation {
        data.tracker.restart();
        data.seen_generation = generation;
    }
    let clock_time = stream.time().ok();
    let requested = usize::try_from(buffer.requested()).unwrap_or(0);
    let Some(block) = buffer.datas_mut().first_mut() else {
        data.failed.store(true, Ordering::Release);
        return;
    };
    let stride = block.chunk().stride();
    let offset = block.chunk().offset() as usize;
    let size = block.chunk().size() as usize;
    let flags = block.chunk().flags().bits();
    let Some(bytes) = block.data() else {
        data.failed.store(true, Ordering::Release);
        return;
    };
    let channels = data.channels;
    let transferred;
    match &mut data.io {
        Io::Sink(tx) => {
            let Some(bytes) = playback_payload(bytes, offset, size, stride, channels) else {
                data.failed.store(true, Ordering::Release);
                return;
            };
            transferred = bytes.len() / (channels * 2);
            if ingest(tx, &mut data.scratch, channels, bytes, flags).is_err() {
                data.failed.store(true, Ordering::Release);
            }
        }
        Io::Source(rx) => {
            let maximum = (bytes.len() / 2).min(data.scratch.len()) / channels;
            let frames = if requested == 0 {
                maximum
            } else {
                requested.min(maximum)
            };
            transferred = frames;
            let count = frames * channels;
            data.scratch[..count].fill(0);
            if source_ready(
                data.source_started,
                rx.queued_frames(),
                frames,
                data.source_preroll,
            ) {
                data.source_started = true;
                match rx.read(&mut data.scratch[..count]) {
                    Ok(read) => {
                        if read.frames < frames && data.source_preroll != 0 {
                            data.source_started = false;
                        }
                        data.underruns
                            .fetch_add((frames - read.frames) as u64, Ordering::Relaxed);
                    }
                    Err(_) => {
                        data.failed.store(true, Ordering::Release);
                    }
                }
            } else {
                data.underruns.fetch_add(frames as u64, Ordering::Relaxed);
            }
            for (pair, sample) in bytes[..count * 2]
                .chunks_exact_mut(2)
                .zip(&data.scratch[..count])
            {
                pair.copy_from_slice(&sample.to_le_bytes());
            }
            let chunk = block.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.size_mut() = u32::try_from(count * 2).unwrap_or(0);
            *chunk.stride_mut() = i32::try_from(channels * 2).unwrap_or(0);
        }
    }
    if let Some(time) = clock_time {
        let rate = time.rate();
        data.tracker.record(
            &data.clock,
            time.ticks(),
            (rate.num, rate.denom),
            transferred as u64,
            data.sample_rate,
            (time.now(), time.delay()),
        );
    }
}

fn source_ready(started: bool, queued: usize, requested: usize, preroll: usize) -> bool {
    started || preroll == 0 || queued >= requested.saturating_add(preroll)
}

// Only interleaved S16LE is negotiated. Reject a mismatched layout rather than
// silently interpreting padding or a partial frame as controller samples.
fn playback_payload(
    bytes: &[u8],
    offset: usize,
    size: usize,
    stride: i32,
    channels: usize,
) -> Option<&[u8]> {
    let frame_bytes = channels.checked_mul(2).filter(|n| *n != 0)?;
    if size % frame_bytes != 0 || (stride != 0 && usize::try_from(stride).ok()? != frame_bytes) {
        return None;
    }
    bytes.get(offset..offset.checked_add(size)?)
}

fn ingest(
    tx: &mut PcmProducer,
    scratch: &mut [i16],
    channels: usize,
    bytes: &[u8],
    flags: i32,
) -> Result<(), AudioError> {
    if bytes.len() % (channels * 2) != 0 {
        return Err(AudioError::InvalidRequirement);
    }
    if flags & spa::buffer::ChunkFlags::CORRUPTED.bits() != 0 {
        return tx.discard(u64::try_from(bytes.len() / (channels * 2)).map_err(backend)?);
    }
    // EMPTY means the host supplied no sample content. Account for its media
    // time as loss, so callers observe a gap instead of fabricated silence that
    // looks like accepted controller audio.
    let empty = flags & SPA_CHUNK_FLAG_EMPTY != 0;
    if empty {
        return tx.discard(u64::try_from(bytes.len() / (channels * 2)).map_err(backend)?);
    }
    for packet in bytes.chunks(scratch.len() / channels * channels * 2) {
        let count = packet.len() / 2;
        for (sample, pair) in scratch[..count].iter_mut().zip(packet.chunks_exact(2)) {
            *sample = i16::from_le_bytes([pair[0], pair[1]]);
        }
        let n = tx.push(&scratch[..count])?;
        tx.discard(u64::try_from(count / channels - n).map_err(backend)?)?;
    }
    Ok(())
}

struct Prepared {
    specs: Vec<Spec>,
    endpoints: Vec<Endpoint>,
    playback: Option<PcmConsumer>,
    playback_observer: Option<PcmObserver>,
    microphone: Option<PcmProducer>,
}
fn prepare(
    profile: &AudioProfile,
    options: AudioOptions,
    creation: u64,
    source_preroll: usize,
) -> Result<Prepared, AudioError> {
    let mut specs = Vec::new();
    let mut endpoints = Vec::new();
    let mut playback = None;
    let mut playback_observer = None;
    let mut microphone = None;
    for description in profile.streams() {
        let format = description.format().clone();
        let direction = description.direction();
        let access = match direction {
            SampleDirection::HostToController => options.playback_access(),
            SampleDirection::ControllerToHost => options.microphone_access(),
            _ => return Err(AudioError::IncompatibleTopology),
        };
        let host_node = format!(
            "virtualgamepad.{}.{}.{}",
            std::process::id(),
            creation,
            description.name()
        );
        let caller_node =
            (access == AudioAccess::NativeClient).then(|| format!("{host_node}.caller"));
        let (tx, rx) = pcm_queue(&format, QUEUE_FRAMES)?;
        match direction {
            SampleDirection::HostToController => {
                playback_observer = Some(tx.observer());
                specs.push(Spec {
                    clock: Arc::default(),
                    name: host_node.clone(),
                    format: format.clone(),
                    io: Io::Sink(tx),
                    source_preroll: 0,
                });
                if let Some(name) = &caller_node {
                    specs.push(Spec {
                        clock: Arc::default(),
                        name: name.clone(),
                        format: format.clone(),
                        io: Io::Source(rx),
                        source_preroll,
                    });
                } else {
                    playback = Some(rx);
                }
            }
            SampleDirection::ControllerToHost => {
                specs.push(Spec {
                    clock: Arc::default(),
                    name: host_node.clone(),
                    format: format.clone(),
                    io: Io::Source(rx),
                    source_preroll,
                });
                if let Some(name) = &caller_node {
                    specs.push(Spec {
                        clock: Arc::default(),
                        name: name.clone(),
                        format: format.clone(),
                        io: Io::Sink(tx),
                        source_preroll: 0,
                    });
                } else {
                    microphone = Some(tx);
                }
            }
            _ => return Err(AudioError::IncompatibleTopology),
        }
        endpoints.push(Endpoint {
            host_node,
            caller_node,
            format,
            direction,
            access,
        });
    }
    Ok(Prepared {
        specs,
        endpoints,
        playback,
        playback_observer,
        microphone,
    })
}

fn check_format(data: &ControlCallback, id: u32, pod: Option<&spa::pod::Pod>) {
    if id == spa::param::ParamType::Format.as_raw() {
        if let Some(pod) = pod {
            if !matches_format(pod, &data.format) {
                data.failed.store(true, Ordering::Release);
            }
        }
    }
}
fn stream_flags() -> pw::stream::StreamFlags {
    pw::stream::StreamFlags::MAP_BUFFERS
        | pw::stream::StreamFlags::DONT_RECONNECT
        | pw::stream::StreamFlags::RT_PROCESS
}
fn matches_format(pod: &spa::pod::Pod, expected: &PcmFormat) -> bool {
    let mut actual = spa::param::audio::AudioInfoRaw::new();
    if actual.parse(pod).is_err() {
        return false;
    }
    actual.format() == spa::param::audio::AudioFormat::S16LE
        && actual.rate() == expected.sample_rate_hz()
        && usize::try_from(actual.channels()).ok() == Some(expected.channels().len())
        && actual
            .position()
            .iter()
            .zip(expected.channels())
            .all(|(position, role)| Some(*position) == channel_position(*role))
}
fn channel_position(role: gr_audio_contract::AudioChannel) -> Option<u32> {
    use gr_audio_contract::AudioChannel as C;
    Some(match role {
        C::AudibleLeft | C::MicrophoneLeft => spa::sys::SPA_AUDIO_CHANNEL_FL,
        C::AudibleRight | C::MicrophoneRight => spa::sys::SPA_AUDIO_CHANNEL_FR,
        C::HapticLeft => spa::sys::SPA_AUDIO_CHANNEL_RL,
        C::HapticRight => spa::sys::SPA_AUDIO_CHANNEL_RR,
        C::Speaker | C::Microphone => spa::sys::SPA_AUDIO_CHANNEL_MONO,
        _ => return None,
    })
}
fn format_pod(format: &PcmFormat) -> Result<Vec<u8>, AudioError> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::S16LE);
    info.set_rate(format.sample_rate_hz());
    info.set_channels(u32::try_from(format.channels().len()).map_err(backend)?);
    let mut positions = [0; 64];
    for (position, role) in positions.iter_mut().zip(format.channels()) {
        *position = channel_position(*role).ok_or(AudioError::IncompatibleTopology)?;
    }
    info.set_position(positions);
    let object = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    };
    let bytes = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(object),
    )
    .map_err(backend)?
    .0
    .into_inner();
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_audio_contract::{AudioChannel as C, AudioStreamDescription};
    #[test]
    fn usb_source_preroll_uses_requested_graph_block_and_rearms_after_gap() {
        assert!(!source_ready(false, 383, 256, 128));
        assert!(source_ready(false, 384, 256, 128));
        assert!(source_ready(true, 1, 256, 128));
        assert!(!source_ready(false, 1, 256, 128));
        assert!(source_ready(false, 0, 256, 0));
    }
    #[test]
    fn one_direction_profile_prepares_only_its_owned_queue() {
        let format = PcmFormat::new(48_000, &[C::AudibleLeft, C::AudibleRight]).unwrap();
        for direction in [
            SampleDirection::HostToController,
            SampleDirection::ControllerToHost,
        ] {
            let profile = AudioProfile::new(
                "one-direction",
                &[AudioStreamDescription::new(
                    "only",
                    direction,
                    format.clone(),
                )],
                "test",
            );
            assert!(valid_profile(&profile));
            let prepared = prepare(
                &profile,
                AudioOptions::new(gr_audio_contract::AudioExposure::Emulated),
                7,
                0,
            )
            .unwrap();
            assert_eq!(prepared.endpoints.len(), 1);
            assert_eq!(prepared.specs.len(), 1);
            assert_eq!(
                prepared.playback.is_some(),
                direction == SampleDirection::HostToController
            );
            assert_eq!(
                prepared.microphone.is_some(),
                direction == SampleDirection::ControllerToHost
            );
        }
    }
    #[test]
    fn malformed_playback_layout_never_reaches_the_pcm_queue() {
        let bytes = [99, 1, 0, 2, 0, 99];
        for stride in [0, 4] {
            assert_eq!(
                playback_payload(&bytes, 1, 4, stride, 2),
                Some(&bytes[1..5])
            );
        }
        for (offset, size, stride, channels) in [
            (1, 3, 4, 2),
            (1, 4, 2, 2),
            (1, 4, -4, 2),
            (3, 4, 4, 2),
            (usize::MAX, 4, 4, 2),
            (0, 0, 0, 0),
        ] {
            assert!(playback_payload(&bytes, offset, size, stride, channels).is_none());
        }
    }
    #[test]
    fn failure_and_partial_registration_cleanup_retains_every_cause() {
        for primary in [Ok(()), Err(backend("registration failed"))] {
            let had_primary = primary.is_err();
            let mut attempts = 0;
            let cleanup = [
                Err(backend("first endpoint")),
                Ok(()),
                Err(backend("last endpoint")),
            ];
            let error = finish_cleanup(primary, cleanup.into_iter().inspect(|_| attempts += 1))
                .unwrap_err()
                .to_string();
            assert_eq!(attempts, 3);
            assert!(error.contains("first endpoint"));
            assert!(error.contains("last endpoint"));
            assert_eq!(error.contains("registration failed"), had_primary);
        }
        assert_eq!(
            finish_cleanup(Err(AudioError::Closed), std::iter::empty()),
            Err(AudioError::Closed)
        );
        assert_eq!(finish_cleanup(Ok(()), [Ok(()), Ok(())].into_iter()), Ok(()));
    }
    #[test]
    fn endpoints_request_small_quanta_without_forcing_graph_or_routing() {
        for input in [true, false] {
            let props = endpoint_properties("synthetic.endpoint", input, 48_000);
            assert_eq!(props.get("node.latency"), Some("128/48000"));
            assert_eq!(props.get("node.force-quantum"), None);
            assert_eq!(props.get("node.force-rate"), None);
            assert_eq!(props.get("node.autoconnect"), Some("false"));
            assert_eq!(props.get("stream.dont-remix"), Some("true"));
        }
        assert_eq!(
            endpoint_properties("synthetic", true, 3000).get("node.latency"),
            Some("8/3000")
        );
    }
    #[test]
    fn negotiated_format_keeps_audible_and_haptic_positions_explicit() {
        let format = PcmFormat::new(
            48_000,
            &[
                C::AudibleLeft,
                C::AudibleRight,
                C::HapticLeft,
                C::HapticRight,
            ],
        )
        .unwrap();
        let bytes = format_pod(&format).unwrap();
        let pod = spa::pod::Pod::from_bytes(&bytes).unwrap();
        let mut raw = spa::param::audio::AudioInfoRaw::new();
        raw.parse(pod).unwrap();
        assert_eq!(raw.format(), spa::param::audio::AudioFormat::S16LE);
        assert_eq!(raw.rate(), 48_000);
        assert_eq!(raw.channels(), 4);
        assert_eq!(
            &raw.position()[..4],
            &[
                spa::sys::SPA_AUDIO_CHANNEL_FL,
                spa::sys::SPA_AUDIO_CHANNEL_FR,
                spa::sys::SPA_AUDIO_CHANNEL_RL,
                spa::sys::SPA_AUDIO_CHANNEL_RR
            ]
        );
    }
    #[test]
    fn dedicated_process_dispatch_and_format_changes_fail_closed() {
        assert!(stream_flags().contains(pw::stream::StreamFlags::RT_PROCESS));
        let expected = PcmFormat::new(48_000, &[C::AudibleLeft, C::AudibleRight]).unwrap();
        let correct = format_pod(&expected).unwrap();
        assert!(matches_format(
            spa::pod::Pod::from_bytes(&correct).unwrap(),
            &expected
        ));
        let wrong = format_pod(&PcmFormat::new(44_100, &[C::Microphone]).unwrap()).unwrap();
        assert!(!matches_format(
            spa::pod::Pod::from_bytes(&wrong).unwrap(),
            &expected
        ));
    }
    #[test]
    fn corrupted_and_empty_chunks_report_loss_without_fabricating_samples() {
        let format = PcmFormat::new(48_000, &[C::AudibleLeft, C::AudibleRight]).unwrap();
        let (mut tx, mut rx) = pcm_queue(&format, 4).unwrap();
        let mut scratch = [0; 16];
        ingest(
            &mut tx,
            &mut scratch,
            2,
            &[1, 0, 2, 0],
            spa::buffer::ChunkFlags::CORRUPTED.bits(),
        )
        .unwrap();
        ingest(
            &mut tx,
            &mut scratch,
            2,
            &[99, 99, 99, 99],
            SPA_CHUNK_FLAG_EMPTY,
        )
        .unwrap();
        ingest(&mut tx, &mut scratch, 2, &[3, 0, 4, 0], 0).unwrap();
        let mut samples = [42; 2];
        let read = rx.read(&mut samples).unwrap();
        assert_eq!(samples, [3, 4]);
        assert_eq!(read.first_frame, 2);
        assert!(read.discontinuity);
        assert_eq!(rx.discarded_frames(), 2);
        assert!(ingest(&mut tx, &mut scratch, 2, &[0; 3], 0).is_err());
    }
    #[test]
    fn duplicate_directions_fail_before_pipewire_access() {
        let stream = AudioStreamDescription::new(
            "duplicate",
            SampleDirection::HostToController,
            PcmFormat::new(48_000, &[C::Speaker]).unwrap(),
        );
        let profile = AudioProfile::new("invalid", &[stream.clone(), stream], "test");
        assert!(matches!(
            Session::open(&profile, AudioOptions::default(), 0),
            Err(AudioError::IncompatibleTopology)
        ));
    }
}
