//! Ordinary-caller, root-API USB sample continuity check. Requires the installed
//! broker and a prepared VHCI host. This does not measure end-to-end latency.
//! Supply the prepared lab instance and expected broker generation; the checker
//! verifies serial, ancestry and isolation before streaming. See the CLI usage.
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use virtualgamepad::{
    AudioExposure, AudioOptions, AudioRead, ControllerAudio, CreationOptions, RealizationId,
    create_dualsense, create_dualshock4, create_xbox360,
};

#[derive(Debug)]
struct LabIdentity {
    instance: String,
    generation: u64,
}
impl LabIdentity {
    fn new(instance: String, generation: u64) -> Result<Self, &'static str> {
        if instance.is_empty()
            || instance.len() > 32
            || !instance
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || generation == 0
        {
            return Err("invalid expected lab session identity");
        }
        Ok(Self {
            instance,
            generation,
        })
    }
    fn checker_arguments(&self, family: &str, seconds: u64, trials: u64, port: u32) -> Vec<String> {
        vec![
            "scripts/validate-usb-audio-live.py".into(),
            "--reserve-owned-card".into(),
            "--profile".into(),
            family.into(),
            "--port".into(),
            port.to_string(),
            "--instance".into(),
            self.instance.clone(),
            "--generation".into(),
            self.generation.to_string(),
            "--seconds".into(),
            seconds.to_string(),
            "--trials".into(),
            trials.to_string(),
        ]
    }
}

fn marker_frame(position: u64, channels: usize) -> [i16; 4] {
    let index = if channels == 1 {
        position / 2
    } else {
        position
    };
    let high = i16::try_from(index / 32_767 + 1).expect("bounded trial marker high");
    let low = i16::try_from(index % 32_767 + 1).expect("bounded trial marker low");
    if channels == 1 {
        [if position % 2 == 0 { high } else { -low }, 0, 0, 0]
    } else {
        [high, low, -high, -low]
    }
}

fn trial_frames(seconds: u64) -> u64 {
    // The ALSA checker sends two startup seconds and one drain-edge second.
    (seconds + 3) * 48_000
}

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
    fn observe_block(&mut self, read: AudioRead, samples: &[i16], channels: usize) {
        self.observe_samples(
            read.frames,
            read.first_frame,
            read.discontinuity,
            samples,
            channels,
        );
    }
    fn observe_samples(
        &mut self,
        frames: usize,
        first_frame: u64,
        discontinuity: bool,
        samples: &[i16],
        channels: usize,
    ) {
        if frames == 0 {
            return;
        }
        if discontinuity
            || self
                .expected_position
                .is_some_and(|position| position != first_frame)
        {
            self.discontinuities += 1;
        }
        self.expected_position = Some(first_frame + frames as u64);
        self.received += frames as u64;
        for frame in samples[..frames * channels].chunks_exact(channels) {
            let pattern = marker_frame(self.valid % self.trial_frames, channels);
            self.observe(frame, &pattern[..channels]);
        }
    }
}

fn refill_target(host_frames: u64, fill_frames: u64) -> Result<u64, &'static str> {
    host_frames
        .checked_add(fill_frames)
        .ok_or("microphone frame position overflow")
}

fn refill_microphone(
    audio: &mut ControllerAudio,
    child: &mut Checker,
    microphone: &mut [i16; 2 * 128],
    channels: usize,
    submitted: &mut u64,
    target: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut accepted_any = false;
    while *submitted < target {
        let frames = (target - *submitted).min(128) as usize;
        for frame in 0..frames {
            for channel in 0..channels {
                microphone[frame * channels + channel] =
                    marker_frame(*submitted + frame as u64, channels)[channel];
            }
        }
        let accepted = match audio.write_microphone(&microphone[..frames * channels]) {
            Ok(accepted) => accepted,
            Err(error) => return Err(terminal_audio_error(audio, child, error)),
        };
        if accepted == 0 {
            break;
        }
        accepted_any = true;
        *submitted += accepted as u64;
    }
    Ok(accepted_any)
}

fn exercise(
    audio: &mut ControllerAudio,
    family: &str,
    seconds: u64,
    trials: u64,
    port: u32,
    microphone_fill_ms: u64,
    identity: &LabIdentity,
) -> Result<(), Box<dyn std::error::Error>> {
    let playback_channels = if family == "dualsense" { 4 } else { 2 };
    let microphone_channels = if family == "dualsense" { 2 } else { 1 };
    let mut child = Checker(
        Command::new("python3")
            .args(identity.checker_arguments(family, seconds, trials, port))
            .spawn()?,
    );
    let started = Instant::now();
    let mut submitted = 0_u64;
    let mut pattern_audit = PatternAudit::new(trial_frames(seconds), trials);
    let mut playback = [0_i16; 4 * 128];
    let mut microphone = [0_i16; 2 * 128];
    let mut previous_loop = Instant::now();
    let mut previous_host_frames = None;
    let mut maximum_loop_gap_us = 0_u128;
    let mut maximum_host_advance = 0_u64;
    let mut maximum_refill_deficit = 0_u64;
    let deadline = started + Duration::from_secs(trials * (seconds + 3) + 15);
    let child_status = loop {
        let now = Instant::now();
        maximum_loop_gap_us =
            maximum_loop_gap_us.max(now.duration_since(previous_loop).as_micros());
        previous_loop = now;
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
        pattern_audit.observe_block(read, &playback, playback_channels);
        // Refill against the USB host's consumed position, not wall time.
        // Capacity is spare room, never a target operating fill.
        let host_frames = match audio.microphone_consumed_frames() {
            Ok(Some(host_frames)) => host_frames,
            Ok(None) => return Err("USB backend did not report microphone host frames".into()),
            Err(error) => return Err(terminal_audio_error(audio, &mut child, error)),
        };
        if let Some(previous) = previous_host_frames {
            maximum_host_advance = maximum_host_advance.max(host_frames.saturating_sub(previous));
        }
        previous_host_frames = Some(host_frames);
        maximum_refill_deficit = maximum_refill_deficit.max(host_frames.saturating_sub(submitted));
        let target = refill_target(host_frames, microphone_fill_ms * 48)?;
        let accepted_any = refill_microphone(
            audio,
            &mut child,
            &mut microphone,
            microphone_channels,
            &mut submitted,
            target,
        )?;
        if read.frames == 0 && !accepted_any {
            std::thread::sleep(Duration::from_millis(1));
        }
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
        pattern_audit.observe_block(read, &playback, playback_channels);
    }
    let dropped = audio.dropped_playback_frames();
    let silence = audio.underrun_frames();
    println!(
        "root_pcm family={family} fill_ms={microphone_fill_ms} received={} valid={} submitted={submitted} discontinuities={} interior_bad={} idle_silence={} extra_pattern={} dropped={dropped} microphone_silence={silence} max_loop_gap_us={maximum_loop_gap_us} max_host_advance_frames={maximum_host_advance} max_refill_deficit_frames={maximum_refill_deficit}",
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
    use super::{LabIdentity, PatternAudit, marker_frame, refill_target, trial_frames};
    #[test]
    fn checker_receives_exact_expected_session_identity_and_trial_bounds() {
        let identity = LabIdentity::new("fixture-lab".into(), 7).unwrap();
        assert_eq!(
            identity.checker_arguments("dualshock4", 3, 2, 1),
            [
                "scripts/validate-usb-audio-live.py",
                "--reserve-owned-card",
                "--profile",
                "dualshock4",
                "--port",
                "1",
                "--instance",
                "fixture-lab",
                "--generation",
                "7",
                "--seconds",
                "3",
                "--trials",
                "2",
            ]
        );
        assert_eq!(trial_frames(3), 288_000);
        for instance in ["", "../foreign", "UPPER", "with space"] {
            assert!(LabIdentity::new(instance.into(), 7).is_err());
        }
        assert!(LabIdentity::new("fixture".into(), 0).is_err());
    }
    #[test]
    fn indexed_markers_match_checker_channel_and_wrap_contract() {
        assert_eq!(marker_frame(0, 4), [1, 1, -1, -1]);
        assert_eq!(marker_frame(32_766, 2)[..2], [1, 32_767]);
        assert_eq!(marker_frame(32_767, 2)[..2], [2, 1]);
        assert_eq!(marker_frame(65_534, 1)[0], 2);
        assert_eq!(marker_frame(65_535, 1)[0], -1);
    }
    #[test]
    fn indexed_playback_rejects_loss_offset_by_duplication_and_channel_corruption() {
        for positions in [[0, 1, 2, 3], [0, 1, 1, 3]] {
            let samples: Vec<_> = positions
                .into_iter()
                .flat_map(|position| marker_frame(position, 4))
                .collect();
            let mut audit = PatternAudit::new(4, 1);
            audit.observe_samples(4, 0, false, &samples, 4);
            if positions == [0, 1, 2, 3] {
                assert_eq!((audit.valid, audit.interior_bad), (4, 0));
            } else {
                assert!(audit.valid < 4 && audit.interior_bad > 0);
            }
        }
        let mut samples = marker_frame(0, 4);
        samples[3] = 1;
        let mut audit = PatternAudit::new(1, 1);
        audit.observe_samples(1, 0, false, &samples, 4);
        assert_eq!((audit.valid, audit.interior_bad), (0, 1));
    }
    #[test]
    fn indexed_mono_carries_pair_phase_across_reads_and_restarts_each_trial() {
        let mut audit = PatternAudit::new(4, 2);
        for frame in 0..8 {
            let samples = marker_frame(frame % 4, 1);
            audit.observe_samples(1, frame, false, &samples[..1], 1);
        }
        assert_eq!(
            (audit.valid, audit.interior_bad, audit.discontinuities),
            (8, 0, 0)
        );
    }
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
    identity: &LabIdentity,
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
                identity,
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
                identity,
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
                identity,
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
    if !(6..=7).contains(&args.len()) {
        return Err("usage: usb_audio_acceptance FAMILY SECONDS TRIALS PORT INSTANCE GENERATION [MIC_FILL_MS]".into());
    }
    let seconds = args[1].parse::<u64>()?;
    let trials = args[2].parse::<u64>()?;
    let port = args[3].parse::<u32>()?;
    let identity = LabIdentity::new(args[4].clone(), args[5].parse()?)?;
    let microphone_fill_ms = args.get(6).map_or(Ok(12), |value| value.parse::<u64>())?;
    if !(1..=60).contains(&seconds)
        || !(1..=3).contains(&trials)
        || !(1..=16).contains(&microphone_fill_ms)
    {
        return Err("seconds must be 1..60, trials 1..3 and fill 1..16 ms".into());
    }
    run(
        &args[0],
        seconds,
        trials,
        port,
        microphone_fill_ms,
        &identity,
    )
}
