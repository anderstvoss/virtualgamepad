#![cfg(all(target_os = "linux", feature = "pipewire"))]
use gr_audio_contract::{
    AudioAccess, AudioChannel as C, AudioExposure, AudioOptions, AudioProfile,
    AudioStreamDescription, PcmFormat, SampleDirection,
};
use gr_audio_linux::Session;
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn profile() -> AudioProfile {
    AudioProfile::new(
        "synthetic.audio",
        &[
            AudioStreamDescription::new(
                "playback",
                SampleDirection::HostToController,
                PcmFormat::new(
                    48_000,
                    &[
                        C::AudibleLeft,
                        C::AudibleRight,
                        C::HapticLeft,
                        C::HapticRight,
                    ],
                )
                .unwrap(),
            ),
            AudioStreamDescription::new(
                "microphone",
                SampleDirection::ControllerToHost,
                PcmFormat::new(48_000, &[C::MicrophoneLeft, C::MicrophoneRight]).unwrap(),
            ),
        ],
        "Synthetic live test; no physical fidelity claim",
    )
}
#[test]
#[ignore = "requires a running PipeWire session and pw-cat"]
fn pipewire_sample_flow_and_independent_cleanup() {
    let mut session =
        Session::open(&profile(), AudioOptions::new(AudioExposure::Emulated), 991).unwrap();
    let mut sibling =
        Session::open(&profile(), AudioOptions::new(AudioExposure::Emulated), 992).unwrap();
    let mut client = ChildGuard(
        Command::new("pw-cat")
            .args([
                "--playback",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                "4",
                "--latency",
                "256",
                "--target",
                &session.endpoints()[0].host_node,
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut bytes = Vec::new();
    for _ in 0..1024 {
        for sample in [101_i16, -202, 303, -404] {
            bytes.extend(sample.to_le_bytes());
        }
    }
    client.0.stdin.take().unwrap().write_all(&bytes).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    let mut buffer = [0; 4096];
    let mut found = false;
    let mut total = 0;
    while Instant::now() < until {
        let read = session.read_playback(&mut buffer).unwrap();
        total += read.frames;
        if buffer[..read.frames * 4]
            .chunks_exact(4)
            .any(|f| f == [101, -202, 303, -404])
        {
            found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        found,
        "distinct audible/haptic samples were not received; total frames={total}"
    );
    session.close();
    session.close();
    assert!(session.is_closed());
    assert!(!sibling.is_closed());
    assert!(session.read_playback(&mut buffer).is_err());
    sibling.close();
    assert!(sibling.error().is_none());
    assert!(session.error().is_none());
    assert!(
        !session.failed(),
        "intentional close is not backend failure"
    );
}
#[test]
#[ignore = "requires a running PipeWire session"]
fn pipewire_native_mode_has_explicit_caller_endpoints() {
    let mut s = Session::open(
        &profile(),
        AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(AudioAccess::NativeClient)
            .with_microphone_access(AudioAccess::NativeClient),
        993,
    )
    .unwrap();
    assert!(s.endpoints().iter().all(|e| e.caller_node.is_some()));
    assert!(s.read_playback(&mut [0; 4]).is_err());
    assert!(s.write_microphone(&[0; 2]).is_err());
    assert!(s.flush_microphone().is_err());
    s.close();
    assert!(s.error().is_none());
    assert_eq!(
        s.flush_microphone(),
        Err(gr_audio_contract::AudioError::Closed)
    );
}

fn playback_client(target: &str, channels: usize) -> ChildGuard {
    ChildGuard(
        Command::new("pw-cat")
            .args([
                "--playback",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                &channels.to_string(),
                "--latency",
                "256",
                "--target",
                target,
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    )
}
fn capture_client(
    target: &str,
    pattern: Vec<i16>,
) -> (
    ChildGuard,
    std::sync::mpsc::Receiver<bool>,
    std::thread::JoinHandle<()>,
) {
    use std::io::Read;
    let mut client = ChildGuard(
        Command::new("pw-cat")
            .args([
                "--record",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                &pattern.len().to_string(),
                "--latency",
                "256",
                "--target",
                target,
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut stdout = client.0.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let mut frame = vec![0; pattern.len() * 2];
        while stdout.read_exact(&mut frame).is_ok() {
            if frame
                .chunks_exact(2)
                .zip(&pattern)
                .all(|(b, p)| i16::from_le_bytes([b[0], b[1]]) == *p)
            {
                let _ = tx.send(true);
                return;
            }
        }
    });
    (client, rx, worker)
}
#[test]
#[ignore = "requires a running PipeWire session and pw-cat"]
fn microphone_samples_reach_host_capture() {
    let mut session =
        Session::open(&profile(), AudioOptions::new(AudioExposure::Emulated), 994).unwrap();
    let (capture, received, worker) =
        capture_client(&session.endpoints()[1].host_node, vec![1234, -2345]);
    let deadline = Instant::now() + Duration::from_secs(5);
    let samples = [1234, -2345].repeat(256);
    let mut found = false;
    while Instant::now() < deadline {
        session.write_microphone(&samples).unwrap();
        if received.try_recv().is_ok() {
            found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(capture);
    worker.join().unwrap();
    session.close();
    assert!(found, "microphone samples never reached the host");
}
#[test]
#[ignore = "requires a running PipeWire session and pw-cat"]
fn native_clients_exchange_samples_in_both_directions() {
    let options = AudioOptions::new(AudioExposure::Emulated)
        .with_playback_access(AudioAccess::NativeClient)
        .with_microphone_access(AudioAccess::NativeClient);
    let mut s = Session::open(&profile(), options, 995).unwrap();
    for (index, pattern) in [(0, vec![101_i16, -202, 303, -404]), (1, vec![1234, -2345])] {
        let endpoint = &s.endpoints()[index];
        let (producer, consumer) = if index == 0 {
            (
                endpoint.host_node.as_str(),
                endpoint.caller_node.as_deref().unwrap(),
            )
        } else {
            (
                endpoint.caller_node.as_deref().unwrap(),
                endpoint.host_node.as_str(),
            )
        };
        let (capture, received, worker) = capture_client(consumer, pattern.clone());
        let mut playback = playback_client(producer, pattern.len());
        let mut input = playback.0.stdin.take().unwrap();
        let pattern_bytes: Vec<_> = pattern.iter().flat_map(|p| p.to_le_bytes()).collect();
        let sender = std::thread::spawn(move || {
            for _ in 0..200 {
                if input.write_all(&pattern_bytes.repeat(256)).is_err() {
                    break;
                }
            }
        });
        let found = received.recv_timeout(Duration::from_secs(5)).is_ok();
        drop(playback);
        drop(capture);
        sender.join().unwrap();
        worker.join().unwrap();
        assert!(found, "native direction {index} failed");
    }
    s.close();
    assert!(s.error().is_none());
}

fn soak_profile(name: &'static str, channels: &[C], microphone: &[C], creation: u64) {
    let profile = AudioProfile::new(
        name,
        &[
            AudioStreamDescription::new(
                "playback",
                SampleDirection::HostToController,
                PcmFormat::new(48_000, channels).unwrap(),
            ),
            AudioStreamDescription::new(
                "microphone",
                SampleDirection::ControllerToHost,
                PcmFormat::new(48_000, microphone).unwrap(),
            ),
        ],
        "Emulated acceptance profile",
    );
    let pattern: Vec<i16> = [101, -202, 303, -404][..channels.len()].into();
    let bytes: Vec<u8> = pattern
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>()
        .repeat(256);
    for trial in 0..3 {
        let mut session = Session::open(
            &profile,
            AudioOptions::new(AudioExposure::Emulated),
            creation + trial,
        )
        .unwrap();
        let mut playback = playback_client(&session.endpoints()[0].host_node, channels.len());
        let mut stdin = playback.0.stdin.take().unwrap();
        let bytes = bytes.clone();
        let writer = std::thread::spawn(move || while stdin.write_all(&bytes).is_ok() {});
        let mut buffer = vec![0; 4096 * channels.len()];
        let startup = Instant::now();
        let mut first = None;
        // Drain during warm-up, excluding stream startup from steady-state metrics.
        while startup.elapsed() < Duration::from_secs(2) {
            let read = session.read_playback(&mut buffer).unwrap();
            if buffer[..read.frames * channels.len()]
                .chunks_exact(channels.len())
                .any(|f| f == pattern)
            {
                first.get_or_insert_with(|| startup.elapsed());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(first.is_some(), "{name}: playback never started");
        let baseline_loss = session.dropped_playback_frames();
        let start = Instant::now();
        let mut frames = 0_u64;
        let mut unexpected = 0_u64;
        let mut first_unexpected = None;
        let mut zero_frames = 0_u64;
        let mut gaps = 0_u64;
        while start.elapsed() < Duration::from_secs(60) {
            let read = session.read_playback(&mut buffer).unwrap();
            frames += u64::try_from(read.frames).unwrap();
            gaps += u64::from(read.discontinuity);
            for frame in buffer[..read.frames * channels.len()].chunks_exact(channels.len()) {
                unexpected += u64::from(frame != pattern);
                zero_frames += u64::from(frame.iter().all(|v| *v == 0));
                if frame != pattern && first_unexpected.is_none() {
                    first_unexpected = Some((start.elapsed(), frame.to_vec()));
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let elapsed = start.elapsed();
        let loss = session.dropped_playback_frames() - baseline_loss;
        // Deliberately stop servicing for longer than the bounded queue capacity.
        std::thread::sleep(Duration::from_millis(200));
        let stall_loss = session.dropped_playback_frames() - baseline_loss - loss;
        let mut saw_gap = false;
        let until = Instant::now() + Duration::from_secs(1);
        while Instant::now() < until {
            let read = session.read_playback(&mut buffer).unwrap();
            saw_gap |= read.discontinuity;
            std::thread::sleep(Duration::from_millis(1));
        }
        drop(playback);
        writer.join().unwrap();
        session.close();
        eprintln!(
            "{name} trial={trial} elapsed={elapsed:?} frames={frames} startup_to_first={first:?} dropped={loss} unexpected_frames={unexpected} zero_frames={zero_frames} first_unexpected={first_unexpected:?} discontinuities={gaps} deliberate_stall_loss={stall_loss}"
        );
        assert_eq!(loss, 0, "{name}: unexplained queue loss");
        assert_eq!(unexpected, 0, "{name}: corrupted or missing host frames");
        assert_eq!(gaps, 0, "{name}: steady-state discontinuity");
        assert!(
            (2_851_200..=2_908_800).contains(&frames),
            "{name}: unexpected steady-state frame rate"
        );
        assert!(
            stall_loss > 0 && saw_gap,
            "{name}: deliberate stall was not observable"
        );
        assert!(session.error().is_none());
    }
}
#[test]
#[ignore = "three 60-second PipeWire playback trials plus warm-up/stall checks"]
fn soak_dualsense() {
    soak_profile(
        "dualsense.emulated",
        &[
            C::AudibleLeft,
            C::AudibleRight,
            C::HapticLeft,
            C::HapticRight,
        ],
        &[C::MicrophoneLeft, C::MicrophoneRight],
        1100,
    );
}
#[test]
#[ignore = "three 60-second PipeWire playback trials plus warm-up/stall checks"]
fn soak_dualshock4() {
    soak_profile(
        "dualshock4.emulated",
        &[C::AudibleLeft, C::AudibleRight],
        &[C::Microphone],
        1200,
    );
}
#[test]
#[ignore = "three 60-second PipeWire playback trials plus warm-up/stall checks"]
fn soak_xbox360() {
    soak_profile(
        "xbox360.emulated",
        &[C::AudibleLeft, C::AudibleRight],
        &[C::Microphone],
        1300,
    );
}

/// Source timestamps are taken immediately before handing each block to pw-cat;
/// arrival is measured at the library sample read. This includes the client pipe
/// and graph buffers. It is an application-to-application measurement, not a
/// claim about physical converters or about the reverse direction.
#[test]
#[ignore = "requires PipeWire and pw-cat; measures host-to-library sample latency"]
fn latency_host_to_library_samples() {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    const BLOCKS: usize = 3000;
    const FRAMES: usize = 128;
    let mut session =
        Session::open(&profile(), AudioOptions::new(AudioExposure::Emulated), 1400).unwrap();
    let mut client = latency_playback_client(&session.endpoints()[0].host_node, 4);
    let mut input = client.0.stdin.take().unwrap();
    let stamps: Arc<Vec<AtomicU64>> = Arc::new((0..BLOCKS).map(|_| AtomicU64::new(0)).collect());
    let writer_stamps = stamps.clone();
    let started = Instant::now();
    let (primed_tx, primed_rx) = std::sync::mpsc::sync_channel(1);
    let writer = std::thread::spawn(move || {
        writer_stamps[0].store(1, Ordering::Release);
        // Link activation can discard initial graph buffers. Continue warm-up
        // until the receiver confirms flow; none of these frames are measured.
        prime_stream(&primed_rx, || {
            input
                .write_all(&1_i16.to_le_bytes().repeat(128 * 4))
                .unwrap();
        });
        let streaming = Instant::now();
        for block in 1..BLOCKS {
            let due = streaming
                + Duration::from_nanos(
                    u64::try_from((block - 1) * FRAMES).unwrap() * 1_000_000_000 / 48000,
                );
            std::thread::sleep(due.saturating_duration_since(Instant::now()));
            let marker = i16::try_from(block + 1).unwrap();
            let bytes = marker.to_le_bytes().repeat(FRAMES * 4);
            writer_stamps[block].store(
                u64::try_from(started.elapsed().as_nanos()).unwrap() + 1,
                Ordering::Release,
            );
            if input.write_all(&bytes).is_err() {
                break;
            }
        }
    });
    let mut latencies = Vec::new();
    let mut marker_counts = vec![0; BLOCKS];
    let mut buffer = [0_i16; 4096];
    let mut invalid = 0;
    let mut prime_frames = 0;
    while started.elapsed() < Duration::from_secs(11) && marker_counts[BLOCKS - 1] < 128 {
        let read = session.read_playback(&mut buffer).unwrap();
        let now = u64::try_from(started.elapsed().as_nanos()).unwrap();
        for frame in buffer[..read.frames * 4].chunks_exact(4) {
            if frame.iter().all(|sample| *sample == 0) {
                continue;
            }
            let Ok(marker) = usize::try_from(frame[0]) else {
                invalid += 1;
                continue;
            };
            if !(1..=BLOCKS).contains(&marker) || frame.iter().any(|sample| *sample != frame[0]) {
                if invalid < 4 {
                    eprintln!("invalid playback frame: {frame:?}");
                }
                invalid += 1;
                continue;
            }
            marker_counts[marker - 1] += 1;
            if marker == 1 {
                prime_frames += 1;
                if prime_frames == 1024 {
                    primed_tx.send(()).unwrap();
                }
                continue;
            }
            let stamp = stamps[marker - 1].load(Ordering::Acquire);
            if stamp == 0 || stamp - 1 > now {
                invalid += 1;
                continue;
            }
            if stamp > 2_000_000_000 {
                latencies.push(now - (stamp - 1));
            }
        }
        std::thread::sleep(Duration::from_micros(500));
    }
    drop(client);
    writer.join().unwrap();
    session.close();
    assert!(session.error().is_none());
    assert_direction_latency(
        "host_to_library_samples",
        &mut latencies,
        invalid,
        Some(session.dropped_playback_frames()),
        measured_marker_frames(&stamps),
        marker_frame_errors(&marker_counts, &stamps),
    );
}

fn assert_direction_latency(
    label: &str,
    latencies: &mut [u64],
    invalid: usize,
    dropped: Option<u64>,
    expected_frames: usize,
    marker_errors: (usize, usize),
) {
    eprintln!(
        "{label} missing_marker_frames={} duplicate_marker_frames={}",
        marker_errors.0, marker_errors.1
    );
    latencies.sort_unstable();
    assert!(
        latencies.len() > 200_000,
        "insufficient measured frames: {}",
        latencies.len()
    );
    let percentile = |p: usize| latencies[(latencies.len() * p).div_ceil(100) - 1];
    eprintln!(
        "{label} frames={} p50_us={} p95_us={} p99_us={} max_us={} invalid={} playback_queue_dropped={:?} expected_marker_frames={expected_frames} (application-to-application; includes client buffering and scheduling; continuity requires separate validation)",
        latencies.len(),
        percentile(50) / 1000,
        percentile(95) / 1000,
        percentile(99) / 1000,
        latencies.last().unwrap() / 1000,
        invalid,
        dropped
    );
    assert_eq!(invalid, 0);
    assert_eq!(marker_errors, (0, 0), "per-marker continuity failed");
    assert_eq!(
        latencies.len(),
        expected_frames,
        "missing or duplicate measured marker frames"
    );
    assert!(percentile(99) < 20_000_000, "p99 must be below 20ms");
}

/// Reverse-direction measurement. Disable the test client's stdio buffering so
/// a FILE buffer does not add unrelated 21 ms batches to the capture boundary.
/// Each read still includes up to one 128-frame block of client collection time.
#[test]
#[ignore = "requires PipeWire, pw-cat and stdbuf; measures library-to-host latency"]
fn latency_library_to_host_samples() {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    const BLOCKS: usize = 3000;
    let mut session =
        Session::open(&profile(), AudioOptions::new(AudioExposure::Emulated), 1401).unwrap();
    let mut client = unbuffered_capture_client(&session.endpoints()[1].host_node, 2);
    let output = client.0.stdout.take().unwrap();
    let stamps: Arc<Vec<AtomicU64>> = Arc::new((0..BLOCKS).map(|_| AtomicU64::new(0)).collect());
    let reader_stamps = stamps.clone();
    let started = Instant::now();
    let (primed_tx, primed_rx) = std::sync::mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        capture_latencies(output, &reader_stamps, started, &primed_tx, 2)
    });
    stamps[0].store(1, Ordering::Release);
    prime_stream(&primed_rx, || {
        session.write_microphone(&[1; 256]).unwrap();
    });
    let streaming = Instant::now();
    for block in 1..BLOCKS {
        let due = streaming
            + Duration::from_nanos(
                u64::try_from((block - 1) * 128).unwrap() * 1_000_000_000 / 48000,
            );
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let samples = [i16::try_from(block + 1).unwrap(); 256];
        stamps[block].store(
            u64::try_from(started.elapsed().as_nanos()).unwrap() + 1,
            Ordering::Release,
        );
        let mut offset = 0;
        let deadline = Instant::now() + Duration::from_secs(1);
        while offset < samples.len() {
            assert!(Instant::now() < deadline, "microphone consumer stalled");
            offset += session.write_microphone(&samples[offset..]).unwrap() * 2;
            if offset < samples.len() {
                std::thread::sleep(Duration::from_micros(500));
            }
        }
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while !reader.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(client);
    let (mut latencies, invalid, marker_counts) = reader.join().unwrap();
    session.close();
    assert!(session.error().is_none());
    assert_direction_latency(
        "library_to_host_samples",
        &mut latencies,
        invalid,
        None,
        measured_marker_frames(&stamps),
        marker_frame_errors(&marker_counts, &stamps),
    );
}

fn capture_latencies(
    mut output: std::process::ChildStdout,
    stamps: &[std::sync::atomic::AtomicU64],
    started: Instant,
    primed: &std::sync::mpsc::SyncSender<()>,
    channels: usize,
) -> (Vec<u64>, usize, Vec<usize>) {
    use std::{io::Read, sync::atomic::Ordering};
    let mut marker_counts = vec![0; stamps.len()];
    let mut bytes = vec![0; 128 * channels * 2];
    let mut prime_frames = 0;
    let mut latencies = Vec::new();
    let mut invalid = 0;
    while marker_counts[stamps.len() - 1] < 128 && output.read_exact(&mut bytes).is_ok() {
        let now = u64::try_from(started.elapsed().as_nanos()).unwrap();
        for frame in bytes.chunks_exact(channels * 2) {
            let value = i16::from_le_bytes([frame[0], frame[1]]);
            if frame.iter().all(|byte| *byte == 0) {
                continue;
            }
            let Ok(marker) = usize::try_from(value) else {
                invalid += 1;
                continue;
            };
            if !(1..=stamps.len()).contains(&marker)
                || frame.chunks_exact(2).any(|sample| sample != &frame[..2])
            {
                if invalid < 4 {
                    eprintln!("invalid captured frame bytes: {frame:?}");
                }
                invalid += 1;
                continue;
            }
            marker_counts[marker - 1] += 1;
            if marker == 1 {
                prime_frames += 1;
                if prime_frames == 1024 {
                    let _ = primed.send(());
                }
                continue;
            }
            let stamp = stamps[marker - 1].load(Ordering::Acquire);
            if stamp == 0 || stamp - 1 > now {
                invalid += 1;
                continue;
            }
            if stamp > 2_000_000_000 {
                latencies.push(now - (stamp - 1));
            }
        }
    }
    (latencies, invalid, marker_counts)
}

#[test]
#[ignore = "requires isolated low-quantum PipeWire, pw-cat and stdbuf"]
fn latency_native_host_to_controller() {
    native_latency(SampleDirection::HostToController);
}
#[test]
#[ignore = "requires isolated low-quantum PipeWire, pw-cat and stdbuf"]
fn latency_native_controller_to_host() {
    native_latency(SampleDirection::ControllerToHost);
}
fn native_latency(direction: SampleDirection) {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    const BLOCKS: usize = 3000;
    let mut session = Session::open(
        &profile(),
        AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(AudioAccess::NativeClient)
            .with_microphone_access(AudioAccess::NativeClient),
        1402,
    )
    .unwrap();
    let endpoint = session
        .endpoints()
        .iter()
        .find(|e| e.direction == direction)
        .unwrap();
    let caller = endpoint.caller_node.as_deref().unwrap();
    let (source, dest) = if direction == SampleDirection::HostToController {
        (endpoint.host_node.as_str(), caller)
    } else {
        (caller, endpoint.host_node.as_str())
    };
    let channels = endpoint.format.channels().len();
    let mut playback = latency_playback_client(source, channels);
    let mut input = playback.0.stdin.take().unwrap();
    let mut capture = unbuffered_capture_client(dest, channels);
    let output = capture.0.stdout.take().unwrap();
    let stamps: Arc<Vec<AtomicU64>> = Arc::new((0..BLOCKS).map(|_| AtomicU64::new(0)).collect());
    let reader_stamps = stamps.clone();
    let started = Instant::now();
    let (primed_tx, primed_rx) = std::sync::mpsc::sync_channel(1);
    let reader = std::thread::spawn(move || {
        capture_latencies(output, &reader_stamps, started, &primed_tx, channels)
    });
    stamps[0].store(1, Ordering::Release);
    prime_stream(&primed_rx, || {
        input
            .write_all(&1_i16.to_le_bytes().repeat(128 * channels))
            .unwrap();
    });
    let streaming = Instant::now();
    for block in 1..BLOCKS {
        let due = streaming
            + Duration::from_nanos(
                u64::try_from((block - 1) * 128).unwrap() * 1_000_000_000 / 48000,
            );
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let bytes = i16::try_from(block + 1)
            .unwrap()
            .to_le_bytes()
            .repeat(128 * channels);
        stamps[block].store(
            u64::try_from(started.elapsed().as_nanos()).unwrap() + 1,
            Ordering::Release,
        );
        input.write_all(&bytes).unwrap();
    }
    drop(input);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !reader.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(playback);
    drop(capture);
    let (mut latencies, invalid, marker_counts) = reader.join().unwrap();
    session.close();
    assert!(session.error().is_none());
    let label = if direction == SampleDirection::HostToController {
        "native_host_to_controller"
    } else {
        "native_controller_to_host"
    };
    let dropped =
        (direction == SampleDirection::HostToController).then(|| session.dropped_playback_frames());
    assert_direction_latency(
        label,
        &mut latencies,
        invalid,
        dropped,
        measured_marker_frames(&stamps),
        marker_frame_errors(&marker_counts, &stamps),
    );
}

fn measured_marker_frames(stamps: &[std::sync::atomic::AtomicU64]) -> usize {
    stamps
        .iter()
        .filter(|stamp| stamp.load(std::sync::atomic::Ordering::Acquire) > 2_000_000_000)
        .count()
        * 128
}

fn unbuffered_capture_client(target: &str, channels: usize) -> ChildGuard {
    ChildGuard(
        Command::new("stdbuf")
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
                &channels.to_string(),
                "--latency",
                "128",
                "--target",
                target,
                "-",
            ])
            .stdout(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

// Endpoint registration precedes link activation. A finite startup block can
// be consumed before the receiver joins; readiness needs an observed handshake.
fn prime_stream(ready: &std::sync::mpsc::Receiver<()>, mut publish: impl FnMut()) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match ready.try_recv() {
            Ok(()) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                panic!("receiver disappeared during warm-up")
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        assert!(Instant::now() < deadline, "audio warm-up deadline exceeded");
        publish();
        std::thread::sleep(Duration::from_nanos(128 * 1_000_000_000 / 48000));
    }
}

#[test]
fn warm_up_retries_until_receiver_observes_flow() {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let mut attempts = 0;
    prime_stream(&rx, || {
        attempts += 1;
        // Deterministic link startup: the first buffers have no consumer.
        if attempts == 3 {
            tx.send(()).unwrap();
        }
    });
    assert_eq!(attempts, 3);
}

fn latency_playback_client(target: &str, channels: usize) -> ChildGuard {
    ChildGuard(
        Command::new("stdbuf")
            .args([
                "--input=0",
                "pw-cat",
                "--playback",
                "--raw",
                "--format",
                "s16",
                "--rate",
                "48000",
                "--channels",
                &channels.to_string(),
                "--latency",
                "128",
                "--target",
                target,
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

// A total frame count alone can hide a dropped block replaced by a duplicate.
fn marker_frame_errors(
    counts: &[usize],
    stamps: &[std::sync::atomic::AtomicU64],
) -> (usize, usize) {
    counts
        .iter()
        .zip(stamps)
        .filter(|(_, stamp)| stamp.load(std::sync::atomic::Ordering::Acquire) > 2_000_000_000)
        .fold((0, 0), |(missing, duplicate), (count, _)| {
            (
                missing + 128_usize.saturating_sub(*count),
                duplicate + count.saturating_sub(128),
            )
        })
}

#[test]
fn marker_counts_reject_equal_total_with_loss_and_duplication() {
    use std::sync::atomic::AtomicU64;
    let stamps = [
        AtomicU64::new(1),
        AtomicU64::new(2_000_000_001),
        AtomicU64::new(2_000_000_002),
    ];
    assert_eq!(marker_frame_errors(&[900, 128, 128], &stamps), (0, 0));
    assert_eq!(marker_frame_errors(&[900, 0, 256], &stamps), (128, 128));
    assert_eq!(marker_frame_errors(&[900, 127, 129], &stamps), (1, 1));
}

#[path = "support/marker_source.rs"]
mod marker_source;

#[test]
#[ignore = "requires isolated PipeWire; graph-driven source without blocking client pipes"]
fn latency_graph_source_to_library_samples() {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let seconds: u64 = std::env::var("VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS")
        .ok()
        .map_or(8, |s| s.parse().expect("integer trial duration"));
    assert!((8..=62).contains(&seconds));
    let blocks = usize::try_from(seconds * 48000 / 128).unwrap();
    let mut session = Session::open(
        &closure_profile(),
        AudioOptions::new(AudioExposure::Emulated),
        1403,
    )
    .unwrap();
    let channels = session.endpoints()[0].format.channels().len();
    let stamps: Arc<Vec<AtomicU64>> = Arc::new((0..blocks).map(|_| AtomicU64::new(0)).collect());
    let started = Instant::now();
    let source = marker_source::Source::start(
        session.endpoints()[0].host_node.clone(),
        stamps.clone(),
        started,
        channels,
    );
    let mut counts = vec![0; blocks];
    let mut latencies = Vec::new();
    let mut invalid = 0;
    let mut buffer = [0; 4096];
    while started.elapsed() < Duration::from_secs(seconds + 5) && counts[blocks - 1] < 128 {
        let read = session.read_playback(&mut buffer).unwrap();
        let now = u64::try_from(started.elapsed().as_nanos()).unwrap();
        for frame in buffer[..read.frames * channels].chunks_exact(channels) {
            if frame.iter().all(|v| *v == 0) {
                continue;
            }
            let Ok(marker) = usize::try_from(frame[0]) else {
                invalid += 1;
                continue;
            };
            if !(1..=blocks).contains(&marker) || frame.iter().any(|v| *v != frame[0]) {
                invalid += 1;
                continue;
            }
            counts[marker - 1] += 1;
            let stamp = stamps[marker - 1].load(Ordering::Acquire);
            if stamp == 0 || stamp - 1 > now {
                invalid += 1;
                continue;
            }
            if stamp > 2_000_000_000 {
                latencies.push(now - (stamp - 1));
            }
        }
        std::thread::sleep(Duration::from_micros(500));
    }
    let missing: Vec<_> = counts
        .iter()
        .enumerate()
        .filter(|(i, n)| stamps[*i].load(Ordering::Acquire) > 2_000_000_000 && **n != 128)
        .take(12)
        .collect();
    eprintln!(
        "first incomplete graph markers: {missing:?}; clocks: {:?}",
        session.timings()
    );
    drop(source);
    session.close();
    assert!(session.error().is_none());
    assert_direction_latency(
        "graph_source_to_library",
        &mut latencies,
        invalid,
        Some(session.dropped_playback_frames()),
        measured_marker_frames(&stamps),
        marker_frame_errors(&counts, &stamps),
    );
}

#[test]
#[ignore = "isolated PipeWire; graph-driven native playback continuity and latency"]
fn latency_graph_native_playback() {
    native_graph_latency(SampleDirection::HostToController);
}
#[test]
#[ignore = "isolated PipeWire; graph-driven native microphone continuity and latency"]
fn latency_graph_native_microphone() {
    native_graph_latency(SampleDirection::ControllerToHost);
}
fn native_graph_latency(direction: SampleDirection) {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let seconds = trial_seconds();
    let blocks = usize::try_from(seconds * 48000 / 128).unwrap();
    let mut session = Session::open(
        &closure_profile(),
        AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(AudioAccess::NativeClient)
            .with_microphone_access(AudioAccess::NativeClient),
        1404,
    )
    .unwrap();
    let endpoint = session
        .endpoints()
        .iter()
        .find(|e| e.direction == direction)
        .unwrap();
    let channels = endpoint.format.channels().len();
    let (source_node, capture_node) = if direction == SampleDirection::HostToController {
        (
            endpoint.host_node.clone(),
            endpoint.caller_node.clone().unwrap(),
        )
    } else {
        (
            endpoint.caller_node.clone().unwrap(),
            endpoint.host_node.clone(),
        )
    };
    let stamps: Arc<Vec<AtomicU64>> = Arc::new((0..blocks).map(|_| AtomicU64::new(0)).collect());
    let observations = marker_source::Observations::new(blocks);
    let started = Instant::now();
    let capture = marker_source::capture(
        capture_node,
        stamps.clone(),
        started,
        channels,
        observations.clone(),
    );
    let source = marker_source::Source::start(source_node, stamps.clone(), started, channels);
    while started.elapsed() < Duration::from_secs(seconds + 5)
        && observations.counts[blocks - 1].load(Ordering::Acquire) < 128
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(source);
    drop(capture);
    let (mut latencies, counts, invalid) = observations.snapshot(&stamps);
    session.close();
    assert!(session.error().is_none());
    assert!(!session.failed());
    assert_direction_latency(
        if direction == SampleDirection::HostToController {
            "graph_native_playback"
        } else {
            "graph_native_microphone"
        },
        &mut latencies,
        invalid,
        Some(session.dropped_playback_frames()),
        measured_marker_frames(&stamps),
        marker_frame_errors(&counts, &stamps),
    );
}

#[test]
#[ignore = "PipeWire processing/close/rollback/recreate lifecycle regression"]
fn pipewire_close_during_processing_retains_clocks_and_recreates() {
    use std::sync::{Arc, atomic::AtomicU64};
    let mut sibling =
        Session::open(&profile(), AudioOptions::new(AudioExposure::Emulated), 1500).unwrap();
    for creation in 1501..1506 {
        let mut session = Session::open(
            &profile(),
            AudioOptions::new(AudioExposure::Emulated),
            creation,
        )
        .unwrap();
        let stamps = Arc::new((0..3000).map(|_| AtomicU64::new(0)).collect());
        let source = marker_source::Source::start(
            session.endpoints()[0].host_node.clone(),
            stamps,
            Instant::now(),
            4,
        );
        let deadline = Instant::now() + Duration::from_secs(3);
        while session.timings().is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!session.timings().is_empty());
        session.close();
        let retained = session.timings();
        std::thread::sleep(Duration::from_millis(10));
        session.close();
        assert_eq!(session.timings(), retained);
        assert!(session.error().is_none());
        assert!(!session.failed());
        assert!(session.read_playback(&mut [0; 4]).is_err());
        drop(source);
        assert!(!sibling.is_closed());
    }
    sibling.close();
    assert!(!sibling.failed());
}

fn trial_seconds() -> u64 {
    let seconds = std::env::var("VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS")
        .ok()
        .map_or(8, |s| s.parse().expect("integer trial duration"));
    assert!((8..=62).contains(&seconds));
    seconds
}
fn closure_profile() -> AudioProfile {
    use gr_curated_controllers::audio;
    match std::env::var("VIRTUALGAMEPAD_AUDIO_FAMILY")
        .as_deref()
        .unwrap_or("dualsense")
    {
        "dualsense" => audio::dualsense(AudioExposure::Emulated),
        "dualshock4" => audio::dualshock4(AudioExposure::Emulated),
        "xbox360" => audio::xbox360(AudioExposure::Emulated),
        _ => panic!("unknown compiled acceptance profile"),
    }
    .unwrap()
}

#[test]
#[ignore = "isolated PipeWire; library microphone to graph-driven capture"]
fn latency_graph_library_microphone() {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let seconds = trial_seconds();
    let blocks = usize::try_from(seconds * 48000 / 128).unwrap();
    let mut session = Session::open(
        &closure_profile(),
        AudioOptions::new(AudioExposure::Emulated),
        1405,
    )
    .unwrap();
    let channels = session.endpoints()[1].format.channels().len();
    let stamps: Arc<Vec<AtomicU64>> = Arc::new((0..blocks).map(|_| AtomicU64::new(0)).collect());
    let observations = marker_source::Observations::new(blocks);
    let started = Instant::now();
    let capture = marker_source::capture(
        session.endpoints()[1].host_node.clone(),
        stamps.clone(),
        started,
        channels,
        observations.clone(),
    );
    stamps[0].store(1, Ordering::Release);
    let priming = vec![1; 128 * channels];
    while observations.counts[0].load(Ordering::Acquire) < 1024 {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "capture readiness deadline"
        );
        session.write_microphone(&priming).unwrap();
        std::thread::sleep(Duration::from_nanos(128 * 1_000_000_000 / 48000));
    }
    let streaming = Instant::now();
    for block in 1..blocks {
        let due = streaming
            + Duration::from_nanos(
                u64::try_from((block - 1) * 128).unwrap() * 1_000_000_000 / 48000,
            );
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
        let samples = vec![i16::try_from(block + 1).unwrap(); 128 * channels];
        stamps[block].store(
            u64::try_from(started.elapsed().as_nanos()).unwrap() + 1,
            Ordering::Release,
        );
        let mut offset = 0;
        let deadline = Instant::now() + Duration::from_secs(1);
        while offset < samples.len() {
            assert!(Instant::now() < deadline);
            offset += session.write_microphone(&samples[offset..]).unwrap() * channels;
            if offset < samples.len() {
                std::thread::sleep(Duration::from_micros(500));
            }
        }
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while observations.counts[blocks - 1].load(Ordering::Acquire) < 128 && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(capture);
    let (mut latencies, counts, invalid) = observations.snapshot(&stamps);
    session.close();
    assert!(session.error().is_none());
    assert!(!session.failed());
    assert_direction_latency(
        "graph_library_microphone",
        &mut latencies,
        invalid,
        None,
        measured_marker_frames(&stamps),
        marker_frame_errors(&counts, &stamps),
    );
}
