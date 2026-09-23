//! Bounded ordinary-caller USB native-client signal-flow probe.
//! `cargo run --features audio-usbip,audio-pipewire --example usb_audio_native_probe -- dualsense 3 0`
//! This checks exact microphone capture and an unbroken host-playback pattern;
//! a constant playback pattern cannot establish frame-exact continuity or latency.
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    thread::{self, JoinHandle},
    time::Duration,
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
fn run_checker(family: &str, seconds: u64, port: u32) -> std::io::Result<std::process::ExitStatus> {
    Command::new("python3")
        .arg("scripts/validate-usb-audio-live.py")
        .args([
            "--reserve-owned-card",
            "--profile",
            family,
            "--port",
            &port.to_string(),
            "--seconds",
            &seconds.to_string(),
            "--trials",
            "1",
        ])
        .status()
}

#[cfg(test)]
mod tests {
    use super::{CaptureAudit, audit_capture};
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
}

fn exercise(
    audio: &ControllerAudio,
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
        let mut bytes = Vec::new();
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
    let mut stdin = clients
        .microphone
        .stdin
        .take()
        .ok_or("missing microphone pipe")?;
    clients.writer = Some(thread::spawn(move || {
        let mut frame = 0_u64;
        let end = (seconds + 6) * 48_000;
        let mut bytes = vec![0_u8; 128 * microphone_channels * 2];
        while frame < end {
            for (index, samples) in bytes.chunks_exact_mut(microphone_channels * 2).enumerate() {
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
    }));
    thread::sleep(Duration::from_millis(300));
    let status = run_checker(family, seconds, port)?;
    let bytes = clients.finish();
    let pattern = &[101_i16, -202, 303, -404][..playback_channels];
    let frame_bytes = playback_channels * 2;
    let capture_result = audit_capture(&bytes, pattern);
    println!(
        "usb_native family={family} audit={capture_result:?} captured_pipe_frames={} dropped={} native_playback_underrun={:?} microphone_silence={} graph={:?}",
        bytes.len() / frame_bytes,
        audio.dropped_playback_frames(),
        audio.native_playback_underrun_frames(),
        audio.underrun_frames(),
        audio.stream_timings()
    );
    if !status.success()
        || capture_result.longest < seconds * 48_000
        || capture_result.interior_zero != 0
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
            controller.close();
            result?;
        }
        _ => return Err("unknown family".into()),
    }
    Ok(())
}
