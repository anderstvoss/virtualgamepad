//! Test-only graph-driven source: no pipe I/O, sleeps or allocation in processing.
use pipewire::{self as pw, properties::properties, spa};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub struct Source {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    generated: Option<Arc<AtomicU64>>,
}
impl Source {
    pub fn start(
        target: String,
        stamps: Arc<Vec<AtomicU64>>,
        started: Instant,
        channels: usize,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let generated = Arc::new(AtomicU64::new(0));
        let worker_generated = Arc::clone(&generated);
        let worker = thread::spawn(move || {
            pw::init();
            let main = pw::main_loop::MainLoopRc::new(None).unwrap();
            let context = pw::context::ContextRc::new(&main, None).unwrap();
            let core = context.connect_rc(None).unwrap();
            let stream = pw::stream::StreamRc::new(
                core.clone(),
                "synthetic-marker-source",
                properties! {
                    "media.type" => "Audio",
                    "media.category" => "Playback",
                    "target.object" => target,
                    "node.latency" => "128/48000",
                    "stream.dont-remix" => "true",
                },
            )
            .unwrap();
            let mut position = 0_usize;
            let listener = stream
                .add_local_listener::<()>()
                .process(move |stream, ()| {
                    let Some(mut buffer) = stream.dequeue_buffer() else {
                        return;
                    };
                    let requested = usize::try_from(buffer.requested()).unwrap();
                    let Some(data) = buffer.datas_mut().first_mut() else {
                        return;
                    };
                    let Some(bytes) = data.data() else {
                        return;
                    };
                    let stride = channels * 2;
                    let frames = requested.min(bytes.len() / stride);
                    let now = u64::try_from(started.elapsed().as_nanos()).unwrap() + 1;
                    let markers = fill_markers(
                        &mut bytes[..frames * stride],
                        channels,
                        &mut position,
                        &stamps,
                        now,
                    );
                    worker_generated.fetch_add(markers as u64, Ordering::Release);
                    let chunk = data.chunk_mut();
                    *chunk.offset_mut() = 0;
                    *chunk.stride_mut() = i32::try_from(stride).unwrap();
                    *chunk.size_mut() = u32::try_from(frames * stride).unwrap();
                })
                .register()
                .unwrap();
            let bytes = format_bytes(channels);
            stream
                .connect(
                    spa::utils::Direction::Output,
                    None,
                    pw::stream::StreamFlags::RT_PROCESS
                        | pw::stream::StreamFlags::MAP_BUFFERS
                        | pw::stream::StreamFlags::AUTOCONNECT
                        | pw::stream::StreamFlags::DONT_RECONNECT,
                    &mut [spa::pod::Pod::from_bytes(&bytes).unwrap()],
                )
                .unwrap();
            while !worker_stop.load(Ordering::Acquire) {
                main.loop_()
                    .iterate(pw::loop_::Timeout::Finite(Duration::from_millis(5)));
            }
            stream.disconnect().unwrap();
            drop(listener);
        });
        Self {
            stop,
            worker: Some(worker),
            generated: Some(generated),
        }
    }
    pub fn generated_frames(&self) -> u64 {
        self.generated
            .as_ref()
            .expect("producer accounting")
            .load(Ordering::Acquire)
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let result = worker.join();
            if !thread::panicking() {
                result.unwrap();
            }
        }
    }
}

// Counts only non-silent marker frames actually submitted by the producer.
fn fill_markers(
    bytes: &mut [u8],
    channels: usize,
    position: &mut usize,
    stamps: &[AtomicU64],
    now: u64,
) -> usize {
    let mut generated = 0;
    for frame in bytes.chunks_exact_mut(channels * 2) {
        let block = *position / 128;
        let marker = if block < stamps.len() {
            if *position % 128 == 0 {
                stamps[block].store(now, Ordering::Release);
            }
            generated += 1;
            i16::try_from(block + 1).unwrap()
        } else {
            0
        };
        for sample in frame.chunks_exact_mut(2) {
            sample.copy_from_slice(&marker.to_le_bytes());
        }
        *position += 1;
    }
    generated
}

#[test]
fn producer_accounting_distinguishes_generated_markers_from_silent_drain() {
    let stamps = [AtomicU64::new(0), AtomicU64::new(0)];
    let mut position = 0;
    let mut bytes = [0; 260 * 4];
    assert_eq!(fill_markers(&mut bytes, 2, &mut position, &stamps, 37), 256);
    assert_eq!(position, 260);
    assert_eq!(stamps.map(|stamp| stamp.load(Ordering::Acquire)), [37, 37]);
    assert!(bytes[256 * 4..].iter().all(|byte| *byte == 0));
    assert_eq!(i16::from_le_bytes([bytes[128 * 4], bytes[128 * 4 + 1]]), 2);
}

fn format_bytes(channels: usize) -> Vec<u8> {
    let mut format = spa::param::audio::AudioInfoRaw::new();
    format.set_format(spa::param::audio::AudioFormat::S16LE);
    format.set_rate(48_000);
    format.set_channels(u32::try_from(channels).unwrap());
    let mut positions = [0; 64];
    let roles = match channels {
        1 => &[spa::sys::SPA_AUDIO_CHANNEL_MONO][..],
        2 => &[
            spa::sys::SPA_AUDIO_CHANNEL_FL,
            spa::sys::SPA_AUDIO_CHANNEL_FR,
        ][..],
        4 => &[
            spa::sys::SPA_AUDIO_CHANNEL_FL,
            spa::sys::SPA_AUDIO_CHANNEL_FR,
            spa::sys::SPA_AUDIO_CHANNEL_RL,
            spa::sys::SPA_AUDIO_CHANNEL_RR,
        ][..],
        _ => panic!("unsupported test channel count"),
    };
    positions[..channels].copy_from_slice(roles);
    format.set_position(positions);
    let object = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: format.into(),
    };
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(object),
    )
    .unwrap()
    .0
    .into_inner()
}

pub struct Observations {
    pub counts: Vec<AtomicU64>,
    latencies: Vec<AtomicU64>,
    invalid: AtomicU64,
}
impl Observations {
    pub fn new(blocks: usize) -> Arc<Self> {
        Arc::new(Self {
            counts: (0..blocks).map(|_| AtomicU64::new(0)).collect(),
            latencies: (0..blocks * 128).map(|_| AtomicU64::new(0)).collect(),
            invalid: AtomicU64::new(0),
        })
    }
    pub fn snapshot(&self, stamps: &[AtomicU64]) -> (Vec<u64>, Vec<usize>, usize) {
        let mut times = Vec::new();
        let counts: Vec<_> = self
            .counts
            .iter()
            .map(|n| usize::try_from(n.load(Ordering::Acquire)).unwrap())
            .collect();
        for (block, stamp) in stamps.iter().enumerate() {
            if stamp.load(Ordering::Acquire) > 2_000_000_000 {
                times.extend(
                    self.latencies[block * 128..block * 128 + counts[block].min(128)]
                        .iter()
                        .map(|v| v.load(Ordering::Acquire)),
                );
            }
        }
        (
            times,
            counts,
            usize::try_from(self.invalid.load(Ordering::Acquire)).unwrap(),
        )
    }
    pub fn record(&self, bytes: &[u8], channels: usize, stamps: &[AtomicU64], now: u64) {
        if channels == 0 || bytes.len() % (channels * 2) != 0 {
            self.invalid.fetch_add(1, Ordering::Relaxed);
            return;
        }
        for frame in bytes.chunks_exact(channels * 2) {
            if frame.iter().all(|v| *v == 0) {
                continue;
            }
            let value = i16::from_le_bytes([frame[0], frame[1]]);
            let marker = usize::try_from(value).unwrap_or(0);
            if !(1..=stamps.len()).contains(&marker)
                || frame.chunks_exact(2).any(|v| v != &frame[..2])
            {
                self.invalid.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let stamp = stamps[marker - 1].load(Ordering::Acquire);
            if stamp == 0 || stamp - 1 > now {
                self.invalid.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let count = usize::try_from(self.counts[marker - 1].load(Ordering::Relaxed)).unwrap();
            if count < 128 {
                self.latencies[(marker - 1) * 128 + count]
                    .store(now - (stamp - 1), Ordering::Relaxed);
            }
            self.counts[marker - 1].store(u64::try_from(count + 1).unwrap(), Ordering::Release);
        }
    }
}

// Same RAII thread owner as Source; capture writes only to preallocated storage.
pub fn capture(
    target: String,
    stamps: Arc<Vec<AtomicU64>>,
    started: Instant,
    channels: usize,
    observations: Arc<Observations>,
) -> Source {
    capture_inner(target, stamps, started, channels, observations, false)
}

/// A graph sink which observes incoming markers directly, with no library queue.
pub fn direct_sink(
    name: String,
    stamps: Arc<Vec<AtomicU64>>,
    started: Instant,
    channels: usize,
    observations: Arc<Observations>,
) -> Source {
    capture_inner(name, stamps, started, channels, observations, true)
}

fn capture_inner(
    target: String,
    stamps: Arc<Vec<AtomicU64>>,
    started: Instant,
    channels: usize,
    observations: Arc<Observations>,
    direct: bool,
) -> Source {
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = thread::spawn(move || {
        pw::init();
        let main = pw::main_loop::MainLoopRc::new(None).unwrap();
        let context = pw::context::ContextRc::new(&main, None).unwrap();
        let core = context.connect_rc(None).unwrap();
        let props = capture_properties(&target, direct);
        let stream =
            pw::stream::StreamRc::new(core.clone(), "synthetic-marker-capture", props).unwrap();
        let listener = stream
            .add_local_listener::<()>()
            .process(move |stream, ()| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let Some(data) = buffer.datas_mut().first_mut() else {
                    return;
                };
                let offset = data.chunk().offset() as usize;
                let size = data.chunk().size() as usize;
                if data
                    .chunk()
                    .flags()
                    .contains(spa::buffer::ChunkFlags::CORRUPTED)
                {
                    observations.invalid.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                if data.chunk().flags().bits() & 2 != 0 {
                    return;
                }
                let Some(bytes) = data
                    .data()
                    .and_then(|bytes| bytes.get(offset..offset.saturating_add(size)))
                else {
                    observations.invalid.fetch_add(1, Ordering::Relaxed);
                    return;
                };
                observations.record(
                    bytes,
                    channels,
                    &stamps,
                    u64::try_from(started.elapsed().as_nanos()).unwrap(),
                );
            })
            .register()
            .unwrap();
        let bytes = format_bytes(channels);
        let mut flags = pw::stream::StreamFlags::RT_PROCESS
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::DONT_RECONNECT;
        if !direct {
            flags |= pw::stream::StreamFlags::AUTOCONNECT;
        }
        stream
            .connect(
                spa::utils::Direction::Input,
                None,
                flags,
                &mut [spa::pod::Pod::from_bytes(&bytes).unwrap()],
            )
            .unwrap();
        while !worker_stop.load(Ordering::Acquire) {
            main.loop_()
                .iterate(pw::loop_::Timeout::Finite(Duration::from_millis(5)));
        }
        stream.disconnect().unwrap();
        drop(listener);
    });
    Source {
        stop,
        worker: Some(worker),
        generated: None,
    }
}

fn capture_properties(target: &str, direct: bool) -> pw::properties::PropertiesBox {
    if direct {
        properties! {
            "node.name" => target, "node.virtual" => "true",
            "media.type" => "Audio", "media.class" => "Audio/Sink",
            "node.autoconnect" => "false", "priority.session" => "0",
            "node.latency" => "128/48000", "stream.dont-remix" => "true",
        }
    } else {
        properties! {
            "media.type" => "Audio", "media.category" => "Capture",
            "target.object" => target, "node.latency" => "128/48000",
            "stream.dont-remix" => "true",
        }
    }
}

#[test]
fn direct_control_has_an_owned_sink_and_normal_capture_keeps_its_exact_target() {
    let sink = capture_properties("synthetic.control.7", true);
    assert_eq!(sink.get("node.name"), Some("synthetic.control.7"));
    assert_eq!(sink.get("media.class"), Some("Audio/Sink"));
    assert_eq!(sink.get("node.autoconnect"), Some("false"));
    assert_eq!(sink.get("target.object"), None);
    let capture = capture_properties("synthetic.microphone.8", false);
    assert_eq!(capture.get("target.object"), Some("synthetic.microphone.8"));
    assert_eq!(capture.get("media.category"), Some("Capture"));
    assert_eq!(capture.get("node.name"), None);
}

#[test]
fn capture_observations_preserve_duplicates_and_reject_channel_corruption() {
    let stamps = [AtomicU64::new(2_000_000_001)];
    let observed = Observations::new(1);
    observed.record(&[1, 0, 1, 0].repeat(129), 2, &stamps, 2_000_000_100);
    observed.record(&[1, 0, 0, 0], 2, &stamps, 2_000_000_100);
    let (times, counts, invalid) = observed.snapshot(&stamps);
    assert_eq!(times, vec![100; 128]);
    assert_eq!(counts, [129]);
    assert_eq!(invalid, 1);
}

#[test]
fn capture_rejects_partial_frames_without_counting_the_valid_prefix() {
    let stamps = [AtomicU64::new(2_000_000_001)];
    let observed = Observations::new(1);
    observed.record(&[1, 0, 1, 0, 1], 2, &stamps, 2_000_000_100);
    observed.record(&[], 0, &stamps, 2_000_000_100);
    let (times, counts, invalid) = observed.snapshot(&stamps);
    assert!(times.is_empty());
    assert_eq!(counts, [0]);
    assert_eq!(invalid, 2);
}
