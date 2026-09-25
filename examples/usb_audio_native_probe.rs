//! Bounded ordinary-caller USB native-client signal-flow probe.
//! `cargo run --features audio-usbip,audio-pipewire --example usb_audio_native_probe -- dualsense 3 0`
//! This checks exact microphone capture and an unbroken host-playback pattern;
//! a constant playback pattern cannot establish frame-exact continuity or latency.
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use virtualgamepad::{
    AudioAccess, AudioExposure, AudioOptions, ControllerAudio, CreationOptions, RealizationId,
    SampleDirection, create_dualsense, create_dualshock4, create_xbox360,
};

struct Clients {
    capture: Child,
    microphone: Child,
    reader: Option<JoinHandle<Vec<u8>>>,
    writer: Option<JoinHandle<()>>,
    finished: bool,
}
impl Clients {
    fn finish(&mut self) -> Vec<u8> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;
        let _ = self.microphone.kill();
        let _ = self.capture.kill();
        let _ = self.microphone.wait();
        let _ = self.capture.wait();
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        self.reader
            .take()
            .map_or_else(Vec::new, |reader| reader.join().unwrap_or_default())
    }
}
impl Drop for Clients {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn client(command: &str, node: &str, channels: usize) -> Result<Child, std::io::Error> {
    Command::new("pw-cat")
        .args([
            command,
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
            node,
            "-",
        ])
        .stdin(if command == "--playback" {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(if command == "--record" {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .spawn()
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CaptureAudit {
    longest: u64,
    exact: u64,
    zero: u64,
    interior_zero: usize,
    other: u64,
    runs: u64,
}
fn audit_capture(bytes: &[u8], pattern: &[i16]) -> CaptureAudit {
    let frame_bytes = pattern.len() * 2;
    let mut audit = CaptureAudit::default();
    let mut current = 0_u64;
    let mut first_exact = None;
    let mut last_exact = None;
    for (index, frame) in bytes.chunks_exact(frame_bytes).enumerate() {
        let exact = frame
            .chunks_exact(2)
            .zip(pattern)
            .all(|(sample, expected)| i16::from_le_bytes([sample[0], sample[1]]) == *expected);
        if exact {
            first_exact.get_or_insert(index);
            last_exact = Some(index);
            if current == 0 {
                audit.runs += 1;
            }
            current += 1;
            audit.exact += 1;
            audit.longest = audit.longest.max(current);
        } else {
            if frame.iter().all(|sample| *sample == 0) {
                audit.zero += 1;
            } else {
                audit.other += 1;
            }
            current = 0;
        }
    }
    audit.interior_zero = first_exact.zip(last_exact).map_or(0, |(first, last)| {
        bytes
            .chunks_exact(frame_bytes)
            .skip(first)
            .take(last - first + 1)
            .filter(|frame| frame.iter().all(|sample| *sample == 0))
            .count()
    });
    audit
}
// The native source can be connected before ALSA activates the USB playback
// alternate setting. Score only the fixed post-warm-up interval; do not let a
// long clean run elsewhere conceal a gap in the requested measurement window.
fn measured_capture(bytes: &[u8], pattern: &[i16], seconds: u64) -> Option<CaptureAudit> {
    let frame_bytes = pattern.len() * 2;
    let first = bytes.chunks_exact(frame_bytes).position(|frame| {
        frame
            .chunks_exact(2)
            .zip(pattern)
            .all(|(sample, expected)| i16::from_le_bytes([sample[0], sample[1]]) == *expected)
    })?;
    let begin = first.checked_add(96_000)?;
    let end = begin.checked_add(usize::try_from(seconds).ok()?.checked_mul(48_000)?)?;
    Some(audit_capture(
        bytes.get(begin.checked_mul(frame_bytes)?..end.checked_mul(frame_bytes)?)?,
        pattern,
    ))
}
fn run_checker(family: &str, seconds: u64, port: u32) -> std::io::Result<std::process::ExitStatus> {
    let mut checker = Command::new("python3");
    checker.arg("scripts/validate-usb-audio-live.py").args([
        "--profile",
        family,
        "--port",
        &port.to_string(),
        "--seconds",
        &seconds.to_string(),
        "--trials",
        "1",
    ]);
    // The private PipeWire lab starts policy only: it does not enumerate ALSA
    // cards. The checker still verifies exact owned VHCI/ALSA ancestry, while
    // avoiding a wait for a PipeWire device that cannot exist in that graph.
    if std::env::var_os("VIRTUALGAMEPAD_AUDIO_LAB_QUANTUM").is_none() {
        checker.arg("--reserve-owned-card");
    }
    checker.status()
}

#[cfg(test)]
mod tests {
    use super::{CaptureAudit, audit_capture, measured_capture};
    #[test]
    fn native_capture_audit_keeps_interior_silence_separate_from_edges() {
        let mut bytes = Vec::new();
        for frame in [
            [0_i16, 0],
            [101, -202],
            [101, -202],
            [0, 0],
            [101, -202],
            [8, 9],
            [101, -202],
            [0, 0],
        ] {
            for sample in frame {
                bytes.extend(sample.to_le_bytes());
            }
        }
        assert_eq!(
            audit_capture(&bytes, &[101, -202]),
            CaptureAudit {
                longest: 2,
                exact: 4,
                zero: 3,
                interior_zero: 1,
                other: 1,
                runs: 3,
            }
        );
    }
    #[test]
    fn native_acceptance_requires_exact_post_warmup_window() {
        let mut samples = vec![101_i16; 96_000 + 144_000];
        samples[500] = 0; // Startup instability is excluded.
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let measured = measured_capture(&bytes, &[101], 3).unwrap();
        assert_eq!(measured.exact, 144_000);
        assert_eq!(measured.zero, 0);
        samples[96_500] = 0;
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        assert_eq!(measured_capture(&bytes, &[101], 3).unwrap().zero, 1);
        assert_eq!(measured_capture(&bytes[..bytes.len() - 2], &[101], 3), None);
    }
}

fn sample_window(
    audio: &ControllerAudio,
    done: &AtomicBool,
    seconds: u64,
) -> (u64, u64, (u64, u64, u64)) {
    let started = Instant::now();
    let mut before = None;
    let mut after = 0;
    let mut drop_before = None;
    let mut drop_after = 0;
    let mut queue_start = None;
    let mut queue_end = 0;
    let mut queue_peak = 0;
    while !done.load(Ordering::Acquire) {
        let elapsed = started.elapsed();
        let count = audio.native_playback_underrun_frames().unwrap_or(0);
        let dropped = audio.native_microphone_dropped_frames().unwrap_or(0);
        let fill = audio
            .native_microphone_queue_frames()
            .map_or(0, |(fill, _)| fill);
        if elapsed >= Duration::from_secs(2) {
            before.get_or_insert(count);
            drop_before.get_or_insert(dropped);
            queue_start.get_or_insert(fill);
        }
        if elapsed <= Duration::from_secs(seconds + 2) {
            after = count;
            drop_after = dropped;
            queue_end = fill;
            queue_peak = queue_peak.max(fill);
        }
        thread::sleep(Duration::from_millis(20));
    }
    (
        after.saturating_sub(before.unwrap_or(after)),
        drop_after.saturating_sub(drop_before.unwrap_or(drop_after)),
        (queue_start.unwrap_or(0), queue_end, queue_peak),
    )
}

fn write_microphone_pattern(
    mut stdin: impl Write + Send + 'static,
    channels: usize,
    seconds: u64,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut frame = 0_u64;
        let end = (seconds + 6) * 48_000;
        let mut bytes = vec![0_u8; 128 * channels * 2];
        while frame < end {
            for (index, samples) in bytes.chunks_exact_mut(channels * 2).enumerate() {
                for (channel, sample) in samples.chunks_exact_mut(2).enumerate() {
                    let value =
                        i16::try_from(100 + (frame + index as u64) % 97 + 100 * channel as u64)
                            .expect("bounded synthetic pattern");
                    sample.copy_from_slice(&value.to_le_bytes());
                }
            }
            if stdin.write_all(&bytes).is_err() {
                break;
            }
            frame += 128;
        }
    })
}

fn exercise(
    audio: &mut ControllerAudio,
    family: &str,
    seconds: u64,
    port: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let playback = audio
        .endpoints()
        .iter()
        .find(|endpoint| endpoint.direction() == SampleDirection::HostToController)
        .ok_or("missing playback endpoint")?;
    let microphone = audio
        .endpoints()
        .iter()
        .find(|endpoint| endpoint.direction() == SampleDirection::ControllerToHost)
        .ok_or("missing microphone endpoint")?;
    let playback_node = playback
        .caller()
        .and_then(virtualgamepad::AudioEndpointSelector::pipewire_node)
        .ok_or("missing native playback node")?;
    let microphone_node = microphone
        .caller()
        .and_then(virtualgamepad::AudioEndpointSelector::pipewire_node)
        .ok_or("missing native microphone node")?;
    let playback_channels = playback.format().channels().len();
    let microphone_channels = microphone.format().channels().len();

    let mut capture = client("--record", playback_node, playback_channels)?;
    let stdout = capture.stdout.take().ok_or("missing capture pipe")?;
    let reader = thread::spawn(move || {
        // The test reader must not repeatedly reallocate a multi-megabyte
        // capture while the graph is streaming; that would introduce an
        // avoidable scheduling stall into the continuity measurement.
        let capacity = usize::try_from(seconds + 6).unwrap() * 48_000 * playback_channels * 2;
        let mut bytes = Vec::with_capacity(capacity);
        let _ = std::io::BufReader::new(stdout).read_to_end(&mut bytes);
        bytes
    });
    let microphone = match client("--playback", microphone_node, microphone_channels) {
        Ok(microphone) => microphone,
        Err(error) => {
            let _ = capture.kill();
            let _ = capture.wait();
            let _ = reader.join();
            return Err(error.into());
        }
    };
    let mut clients = Clients {
        capture,
        microphone,
        reader: Some(reader),
        writer: None,
        finished: false,
    };
    let stdin = clients
        .microphone
        .stdin
        .take()
        .ok_or("missing microphone pipe")?;
    clients.writer = Some(write_microphone_pattern(
        stdin,
        microphone_channels,
        seconds,
    ));
    thread::sleep(Duration::from_millis(300));
    let done = AtomicBool::new(false);
    let (status, measured_underrun, measured_graph_drop, queue_window) = thread::scope(|scope| {
        let sampler = scope.spawn(|| sample_window(audio, &done, seconds));
        let status = run_checker(family, seconds, port);
        done.store(true, Ordering::Release);
        let (underrun, dropped, queue) = sampler.join().unwrap_or((0, 0, (0, 0, 0)));
        (status, underrun, dropped, queue)
    });
    let status = status?;
    let bytes = clients.finish();
    let pattern = &[101_i16, -202, 303, -404][..playback_channels];
    let frame_bytes = playback_channels * 2;
    let capture_result = audit_capture(&bytes, pattern);
    let measured = measured_capture(&bytes, pattern, seconds);
    println!(
        "usb_native family={family} audit={capture_result:?} measured={measured:?} captured_pipe_frames={} dropped={} native_playback_underrun={:?} measured_underrun={} microphone_silence={} microphone_dropped={:?} native_microphone_graph_dropped={:?} measured_graph_drop={} native_microphone_queue={:?} queue_window={queue_window:?} bridge_schedule_us={:?} closed={} last_error={:?} graph={:?}",
        bytes.len() / frame_bytes,
        audio.dropped_playback_frames(),
        audio.native_playback_underrun_frames(),
        measured_underrun,
        audio.underrun_frames(),
        audio.dropped_microphone_frames(),
        audio.native_microphone_dropped_frames(),
        measured_graph_drop,
        audio.native_microphone_queue_frames(),
        audio.native_bridge_scheduling_us(),
        audio.is_closed(),
        audio.last_error(),
        audio.stream_timings()
    );
    if !status.success()
        || measured.as_ref().is_none_or(|result| {
            result.exact != seconds * 48_000 || result.zero != 0 || result.other != 0
        })
        || audio.dropped_playback_frames() != 0
    {
        return Err("native USB signal-flow probe failed".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: usb_audio_native_probe FAMILY SECONDS PORT".into());
    }
    let seconds = args[1].parse::<u64>()?;
    let port = args[2].parse::<u32>()?;
    if !(1..=60).contains(&seconds) {
        return Err("seconds must be 1..60".into());
    }
    let audio = AudioOptions::new(AudioExposure::Emulated)
        .with_playback_access(AudioAccess::NativeClient)
        .with_microphone_access(AudioAccess::NativeClient);
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO).with_audio(audio);
    match args[0].as_str() {
        "dualsense" => {
            let mut controller = create_dualsense(options)?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                &args[0],
                seconds,
                port,
            );
            eprintln!(
                "usb_native_controller family=dualsense diagnostics={:?}",
                controller.diagnostics()
            );
            controller.close();
            result?;
        }
        "dualshock4" => {
            let mut controller = create_dualshock4(options)?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                &args[0],
                seconds,
                port,
            );
            eprintln!(
                "usb_native_controller family=dualshock4 diagnostics={:?}",
                controller.diagnostics()
            );
            controller.close();
            result?;
        }
        "xbox360" => {
            let mut controller = create_xbox360(options)?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                &args[0],
                seconds,
                port,
            );
            eprintln!(
                "usb_native_controller family=xbox360 diagnostics={:?}",
                controller.diagnostics()
            );
            controller.close();
            result?;
        }
        _ => return Err("unknown family".into()),
    }
    Ok(())
}
