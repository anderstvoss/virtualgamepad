//! Bounded ordinary-caller USB native-client signal-flow probe.
//! `cargo run --features experimental,audio-usbip,audio-pipewire --example usb_audio_native_probe -- dualsense --seconds 3`
//! Checks indexed markers in both directions through ALSA and `PipeWire`.
//! This is a signal-flow probe; directional latency is measured separately.
#[path = "support/usb_audio.rs"]
mod usb_audio;
use usb_audio::{Args, Attachment, marker_frame};

use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use virtualgamepad::{
    AudioAccess, AudioExposure, AudioOptions, ControllerAudio, CreationOptions, DualSenseOptions,
    DualShock4Options, RealizationId, SampleDirection, Xbox360Options, create_dualsense,
    create_dualshock4, create_xbox360,
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
fn playback_position(frame: &[u8]) -> Option<u64> {
    if !matches!(frame.len(), 4 | 8) {
        return None;
    }
    let high = i16::from_le_bytes([frame[0], frame[1]]);
    let low = i16::from_le_bytes([frame[2], frame[3]]);
    if high <= 0 || low <= 0 {
        return None;
    }
    if frame.len() == 8
        && (i16::from_le_bytes([frame[4], frame[5]]) != -high
            || i16::from_le_bytes([frame[6], frame[7]]) != -low)
    {
        return None;
    }
    Some((u64::try_from(high).ok()? - 1) * 32_767 + u64::try_from(low).ok()? - 1)
}

fn audit_capture(bytes: &[u8], channels: usize, expected_start: Option<u64>) -> CaptureAudit {
    let frame_bytes = channels * 2;
    let mut audit = CaptureAudit::default();
    let mut current = 0;
    let mut previous = None;
    let mut first_exact = None;
    let mut last_exact = None;
    for (index, frame) in bytes.chunks_exact(frame_bytes).enumerate() {
        let position = playback_position(frame).filter(|position| {
            expected_start.is_none_or(|start| *position == start + index as u64)
        });
        if let Some(position) = position {
            first_exact.get_or_insert(index);
            last_exact = Some(index);
            if previous != position.checked_sub(1) || current == 0 {
                audit.runs += 1;
                current = 0;
            }
            current += 1;
            audit.exact += 1;
            audit.longest = audit.longest.max(current);
            previous = Some(position);
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

// Locate the start of the fixed measured interval by marker index. Never slide
// the window forward to a later clean run or accept a replay in place of a loss.
fn measured_capture(bytes: &[u8], channels: usize, seconds: u64) -> Option<CaptureAudit> {
    const WARMUP_FRAMES: usize = 96_000;
    let frame_bytes = channels * 2;
    let (offset, position) = bytes
        .chunks_exact(frame_bytes)
        .enumerate()
        .find_map(|(offset, frame)| playback_position(frame).map(|position| (offset, position)))?;
    let begin = offset.checked_add(WARMUP_FRAMES.checked_sub(usize::try_from(position).ok()?)?)?;
    let end = begin.checked_add(usize::try_from(seconds).ok()?.checked_mul(48_000)?)?;
    Some(audit_capture(
        bytes.get(begin.checked_mul(frame_bytes)?..end.checked_mul(frame_bytes)?)?,
        channels,
        Some(WARMUP_FRAMES as u64),
    ))
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
        let count = virtualgamepad::experimental::audio_instrumentation(audio)
            .native_playback_underrun_frames()
            .unwrap_or(0);
        let dropped = virtualgamepad::experimental::audio_instrumentation(audio)
            .native_microphone_dropped_frames()
            .unwrap_or(0);
        let fill = virtualgamepad::experimental::audio_instrumentation(audio)
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
                for (sample, value) in samples
                    .chunks_exact_mut(2)
                    .zip(marker_frame(frame + index as u64, channels))
                {
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
    attachment: &Attachment,
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

    attachment.prepare(family)?;
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
        let status = attachment.checker(family, Some((seconds, 1))).status();
        done.store(true, Ordering::Release);
        let (underrun, dropped, queue) = sampler.join().unwrap_or((0, 0, (0, 0, 0)));
        (status, underrun, dropped, queue)
    });
    let status = status?;
    let bytes = clients.finish();
    let frame_bytes = playback_channels * 2;
    let capture_result = audit_capture(&bytes, playback_channels, None);
    let measured = measured_capture(&bytes, playback_channels, seconds);
    println!(
        "usb_native family={family} audit={capture_result:?} measured={measured:?} captured_pipe_frames={} dropped={} native_playback_underrun={:?} measured_underrun={} microphone_silence={} microphone_dropped={:?} native_microphone_graph_dropped={:?} measured_graph_drop={} native_microphone_queue={:?} queue_window={queue_window:?} bridge_schedule_us={:?} closed={} last_error={:?} graph={:?}",
        bytes.len() / frame_bytes,
        audio.dropped_playback_frames(),
        virtualgamepad::experimental::audio_instrumentation(audio)
            .native_playback_underrun_frames(),
        measured_underrun,
        audio.underrun_frames(),
        audio.dropped_microphone_frames(),
        virtualgamepad::experimental::audio_instrumentation(audio)
            .native_microphone_dropped_frames(),
        measured_graph_drop,
        virtualgamepad::experimental::audio_instrumentation(audio).native_microphone_queue_frames(),
        virtualgamepad::experimental::audio_instrumentation(audio).native_bridge_scheduling_us(),
        audio.is_closed(),
        audio.last_error(),
        virtualgamepad::experimental::audio_instrumentation(audio).stream_timings()
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
    let mut args = Args::parse()?;
    let seconds = args.number("--seconds", 3, 1..=60)?;
    args.finish()?;
    let audio = AudioOptions::new(AudioExposure::Emulated)
        .with_playback_access(AudioAccess::NativeClient)
        .with_microphone_access(AudioAccess::NativeClient);
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO).with_audio(audio);
    match args.family {
        "dualsense" => {
            let mut controller = create_dualsense(options, DualSenseOptions::default())?;
            let attachment = Attachment::discover(controller.association())?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                args.family,
                seconds,
                &attachment,
            );
            eprintln!(
                "usb_native_controller family=dualsense diagnostics={:?}",
                controller.diagnostics()
            );
            controller.close();
            result?;
        }
        "dualshock4" => {
            let mut controller = create_dualshock4(options, DualShock4Options::default())?;
            let attachment = Attachment::discover(controller.association())?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                args.family,
                seconds,
                &attachment,
            );
            eprintln!(
                "usb_native_controller family=dualshock4 diagnostics={:?}",
                controller.diagnostics()
            );
            controller.close();
            result?;
        }
        "xbox360" => {
            let mut controller = create_xbox360(options, Xbox360Options::default())?;
            let attachment = Attachment::discover(controller.association())?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                args.family,
                seconds,
                &attachment,
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

#[cfg(test)]
mod tests {
    use super::{CaptureAudit, audit_capture, marker_frame, measured_capture, playback_position};

    fn indexed_bytes(position: u64, channels: usize) -> Vec<u8> {
        marker_frame(position, channels)[..channels]
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect()
    }

    #[test]
    fn native_capture_audit_keeps_interior_silence_separate_from_edges() {
        let mut bytes = vec![0; 4];
        bytes.extend(indexed_bytes(0, 2));
        bytes.extend(indexed_bytes(1, 2));
        bytes.extend([0; 4]);
        bytes.extend(indexed_bytes(3, 2));
        bytes.extend([1, 0, 0, 0]);
        bytes.extend(indexed_bytes(5, 2));
        bytes.extend([0; 4]);
        assert_eq!(
            audit_capture(&bytes, 2, None),
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
        let mut bytes: Vec<u8> = (0..96_000 + 48_000)
            .flat_map(|position| indexed_bytes(position, 2))
            .collect();
        bytes[..4].fill(0); // The next marker identifies the startup offset.
        let measured = measured_capture(&bytes, 2, 1).unwrap();
        assert_eq!(
            (measured.exact, measured.zero, measured.other),
            (48_000, 0, 0)
        );
        bytes[96_500 * 4..96_501 * 4].fill(0);
        bytes[96_800 * 4..96_801 * 4].copy_from_slice(&indexed_bytes(96_799, 2));
        let measured = measured_capture(&bytes, 2, 1).unwrap();
        assert_eq!(
            (measured.exact, measured.zero, measured.other),
            (47_998, 1, 1)
        );
        assert_eq!(measured_capture(&bytes[..bytes.len() - 4], 2, 1), None);
        let mut four_channel = indexed_bytes(32_767, 4);
        assert_eq!(playback_position(&four_channel), Some(32_767));
        four_channel[4] ^= 1;
        assert_eq!(playback_position(&four_channel), None);
    }
}
