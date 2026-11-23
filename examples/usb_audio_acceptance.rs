//! Ordinary-caller, root-API USB sample continuity check. Requires the installed
//! broker and a prepared VHCI host. This does not measure end-to-end latency.
//! `cargo run --features audio-usbip --example usb_audio_acceptance -- dualsense --seconds 3 --trials 1 --mic-fill-ms 12`
use std::process::Child;
use std::time::{Duration, Instant};
use virtualgamepad::{
    AudioExposure, AudioOptions, AudioRead, ControllerAudio, CreationOptions, DualSenseOptions,
    DualShock4Options, RealizationId, Xbox360Options, create_dualsense, create_dualshock4,
    create_xbox360,
};

#[path = "support/usb_audio.rs"]
mod usb_audio;
use usb_audio::{Args, Attachment};

use usb_audio::marker_frame;

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
    microphone_fill_ms: u64,
    attachment: &Attachment,
) -> Result<(), Box<dyn std::error::Error>> {
    let playback_channels = if family == "dualsense" { 4 } else { 2 };
    let microphone_channels = if family == "dualsense" { 2 } else { 1 };
    attachment.prepare(family)?;
    let mut child = Checker(
        attachment
            .checker(family, Some((seconds, trials)))
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

fn run(
    family: &str,
    seconds: u64,
    trials: u64,
    microphone_fill_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO)
        .with_audio(AudioOptions::new(AudioExposure::Emulated));
    match family {
        "dualsense" => {
            let mut controller = create_dualsense(options, DualSenseOptions::default())?;
            let attachment = Attachment::discover(controller.association())?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                family,
                seconds,
                trials,
                microphone_fill_ms,
                &attachment,
            );
            if result.is_err() {
                eprintln!("controller_terminal {:?}", controller.diagnostics());
            }
            controller.close();
            result?;
        }
        "dualshock4" => {
            let mut controller = create_dualshock4(options, DualShock4Options::default())?;
            let attachment = Attachment::discover(controller.association())?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                family,
                seconds,
                trials,
                microphone_fill_ms,
                &attachment,
            );
            if result.is_err() {
                eprintln!("controller_terminal {:?}", controller.diagnostics());
            }
            controller.close();
            result?;
        }
        "xbox360" => {
            let mut controller = create_xbox360(options, Xbox360Options::default())?;
            let attachment = Attachment::discover(controller.association())?;
            let result = exercise(
                controller.audio().ok_or("missing audio")?,
                family,
                seconds,
                trials,
                microphone_fill_ms,
                &attachment,
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
    let mut args = Args::parse()?;
    let seconds = args.number("--seconds", 3, 1..=60)?;
    let trials = args.number("--trials", 1, 1..=3)?;
    let fill_ms = args.number("--mic-fill-ms", 12, 1..=16)?;
    args.finish()?;
    run(args.family, seconds, trials, fill_ms)
}

#[cfg(test)]
mod tests {
    use super::{PatternAudit, marker_frame, refill_target, trial_frames};
    #[test]
    fn trial_includes_startup_and_drain_edges() {
        assert_eq!(trial_frames(3), 288_000);
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
