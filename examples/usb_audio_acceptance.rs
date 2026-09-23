//! Ordinary-caller, root-API USB sample continuity check. Requires the installed
//! broker and a prepared VHCI host. This does not measure end-to-end latency.
//! `cargo run --features audio-usbip --example usb_audio_acceptance -- dualsense 3 1 0`
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use virtualgamepad::{
    AudioExposure, AudioOptions, AudioRead, ControllerAudio, CreationOptions, RealizationId,
    create_dualsense, create_dualshock4, create_xbox360,
};

struct Checker(Child);
impl Drop for Checker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn terminal_audio_error(
    audio: &ControllerAudio,
    child: &mut Checker,
    error: virtualgamepad::AudioError,
) -> Box<dyn std::error::Error> {
    eprintln!(
        "root_pcm_terminal error={error} retained={:?} closed={} dropped={} microphone_silence={}",
        audio.last_error(),
        audio.is_closed(),
        audio.dropped_playback_frames(),
        audio.underrun_frames()
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if child.0.try_wait().ok().flatten().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Box::new(error)
}

struct PatternAudit {
    trial_frames: u64,
    total_frames: u64,
    valid: u64,
    interior_bad: u64,
    idle_silence: u64,
    extra_pattern: u64,
    received: u64,
    discontinuities: u64,
    expected_position: Option<u64>,
}
impl PatternAudit {
    fn new(trial_frames: u64, trials: u64) -> Self {
        Self {
            trial_frames,
            total_frames: trial_frames * trials,
            valid: 0,
            interior_bad: 0,
            idle_silence: 0,
            extra_pattern: 0,
            received: 0,
            discontinuities: 0,
            expected_position: None,
        }
    }
    fn observe(&mut self, frame: &[i16], pattern: &[i16]) {
        if frame == pattern {
            if self.valid == self.total_frames {
                self.extra_pattern += 1;
            } else {
                self.valid += 1;
            }
        } else if self.valid < self.total_frames && self.valid % self.trial_frames != 0 {
            self.interior_bad += 1;
        } else if frame.iter().all(|sample| *sample == 0) {
            self.idle_silence += 1;
        } else {
            self.interior_bad += 1;
        }
    }
    fn observe_block(&mut self, read: AudioRead, samples: &[i16], pattern: &[i16]) {
        if read.frames == 0 {
            return;
        }
        if read.discontinuity
            || self
                .expected_position
                .is_some_and(|position| position != read.first_frame)
        {
            self.discontinuities += 1;
        }
        self.expected_position = Some(read.first_frame + read.frames as u64);
        self.received += read.frames as u64;
        for frame in samples[..read.frames * pattern.len()].chunks_exact(pattern.len()) {
            self.observe(frame, pattern);
        }
    }
}

fn refill_target(host_frames: u64, fill_frames: u64) -> Result<u64, &'static str> {
    host_frames
        .checked_add(fill_frames)
        .ok_or("microphone frame position overflow")
}

fn exercise(
    audio: &mut ControllerAudio,
    family: &str,
    seconds: u64,
    trials: u64,
    port: u32,
    microphone_fill_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let playback_channels = if family == "dualsense" { 4 } else { 2 };
    let microphone_channels = if family == "dualsense" { 2 } else { 1 };
    let mut child = Checker(
        Command::new("python3")
            .arg("scripts/validate-usb-audio-live.py")
            .arg("--reserve-owned-card")
            .args(["--profile", family, "--port", &port.to_string()])
            .args([
                "--seconds",
                &seconds.to_string(),
                "--trials",
                &trials.to_string(),
            ])
            .spawn()?,
    );
    let started = Instant::now();
    let mut submitted = 0_u64;
    let mut pattern_audit = PatternAudit::new((seconds + 2) * 48_000, trials);
    let pattern = &[101, -202, 303, -404][..playback_channels];
    let mut playback = [0_i16; 4 * 128];
    let mut microphone = [0_i16; 2 * 128];
    let deadline = started + Duration::from_secs(trials * (seconds + 2) + 15);
    let child_status = loop {
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            return Err("ALSA checker exceeded bounded deadline".into());
        }
        let read = match audio.read_playback(&mut playback) {
            Ok(read) => read,
            Err(error) => return Err(terminal_audio_error(audio, &mut child, error)),
        };
        pattern_audit.observe_block(read, &playback, pattern);
        // Refill against the USB host's consumed position, not wall time.
        // Capacity is spare room, never a target operating fill.
        let host_frames = match audio.microphone_host_frames() {
            Ok(Some(host_frames)) => host_frames,
            Ok(None) => return Err("USB backend did not report microphone host frames".into()),
            Err(error) => return Err(terminal_audio_error(audio, &mut child, error)),
        };
        let target = refill_target(host_frames, microphone_fill_ms * 48)?;
        if submitted < target {
            let frames = (target - submitted).min(128) as usize;
            for frame in 0..frames {
                for channel in 0..microphone_channels {
                    microphone[frame * microphone_channels + channel] = i16::try_from(
                        100 + (submitted + frame as u64) % 97 + 100 * channel as u64,
                    )?;
                }
            }
            let accepted = match audio.write_microphone(&microphone[..frames * microphone_channels])
            {
                Ok(accepted) => accepted,
                Err(error) => return Err(terminal_audio_error(audio, &mut child, error)),
            };
            submitted += accepted as u64;
        }
        std::thread::sleep(Duration::from_millis(1));
    };
    // The IPC pump can lag process exit by one bounded block.
    let drain_deadline = Instant::now() + Duration::from_millis(100);
    while pattern_audit.valid < pattern_audit.total_frames && Instant::now() < drain_deadline {
        let read = match audio.read_playback(&mut playback) {
            Ok(read) => read,
            Err(error) => return Err(terminal_audio_error(audio, &mut child, error)),
        };
        if read.frames == 0 {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        pattern_audit.observe_block(read, &playback, pattern);
    }
    let dropped = audio.dropped_playback_frames();
    let silence = audio.underrun_frames();
    println!(
        "root_pcm family={family} fill_ms={microphone_fill_ms} received={} valid={} submitted={submitted} discontinuities={} interior_bad={} idle_silence={} extra_pattern={} dropped={dropped} microphone_silence={silence}",
        pattern_audit.received,
        pattern_audit.valid,
        pattern_audit.discontinuities,
        pattern_audit.interior_bad,
        pattern_audit.idle_silence,
        pattern_audit.extra_pattern
    );
    if !child_status.success()
        || pattern_audit.valid != pattern_audit.total_frames
        || pattern_audit.interior_bad != 0
        || pattern_audit.extra_pattern != 0
        || pattern_audit.discontinuities != 0
        || dropped != 0
    {
        return Err("root USB full-duplex acceptance failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PatternAudit, refill_target};
    #[test]
    fn silence_is_allowed_only_before_and_between_complete_trials() {
        let mut audit = PatternAudit::new(2, 2);
        let pattern = [101, -202];
        audit.observe(&[0, 0], &pattern);
        audit.observe(&pattern, &pattern);
        audit.observe(&[0, 0], &pattern);
        assert_eq!(audit.interior_bad, 1);
        audit.observe(&pattern, &pattern);
        audit.observe(&[0, 0], &pattern);
        audit.observe(&pattern, &pattern);
        audit.observe(&pattern, &pattern);
        audit.observe(&[0, 0], &pattern);
        assert_eq!((audit.valid, audit.idle_silence), (4, 3));
        audit.observe(&pattern, &pattern);
        assert_eq!(audit.extra_pattern, 1);
    }
    #[test]
    fn microphone_fill_tracks_host_timeline_including_underruns() {
        assert_eq!(refill_target(0, 576).unwrap(), 576);
        assert_eq!(refill_target(0, 576).unwrap(), 576);
        assert_eq!(refill_target(48, 576).unwrap(), 624);
        // The host timeline must advance through silence so the writer can
        // recover after an underrun instead of waiting on consumed samples.
        assert_eq!(refill_target(48 + 96, 576).unwrap(), 720);
    }
}

fn run(
    family: &str,
    seconds: u64,
    trials: u64,
    port: u32,
    microphone_fill_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO)
        .with_audio(AudioOptions::new(AudioExposure::Emulated));
    match family {
        "dualsense" => {
            let mut controller = create_dualsense(options)?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                family,
                seconds,
                trials,
                port,
                microphone_fill_ms,
            );
            if result.is_err() {
                eprintln!("controller_terminal {:?}", controller.diagnostics());
            }
            controller.close();
            result?;
        }
        "dualshock4" => {
            let mut controller = create_dualshock4(options)?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                family,
                seconds,
                trials,
                port,
                microphone_fill_ms,
            );
            if result.is_err() {
                eprintln!("controller_terminal {:?}", controller.diagnostics());
            }
            controller.close();
            result?;
        }
        "xbox360" => {
            let mut controller = create_xbox360(options)?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                family,
                seconds,
                trials,
                port,
                microphone_fill_ms,
            );
            if result.is_err() {
                eprintln!("controller_terminal {:?}", controller.diagnostics());
            }
            controller.close();
            result?;
        }
        _ => return Err("family must be dualsense, dualshock4 or xbox360".into()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(4..=5).contains(&args.len()) {
        return Err("usage: usb_audio_acceptance FAMILY SECONDS TRIALS PORT [MIC_FILL_MS]".into());
    }
    let seconds = args[1].parse::<u64>()?;
    let trials = args[2].parse::<u64>()?;
    let port = args[3].parse::<u32>()?;
    let microphone_fill_ms = args.get(4).map_or(Ok(12), |value| value.parse::<u64>())?;
    if !(1..=60).contains(&seconds)
        || !(1..=3).contains(&trials)
        || !(1..=16).contains(&microphone_fill_ms)
    {
        return Err("seconds must be 1..60, trials 1..3 and fill 1..16 ms".into());
    }
    run(&args[0], seconds, trials, port, microphone_fill_ms)
}
