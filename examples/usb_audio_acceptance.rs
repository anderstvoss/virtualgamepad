//! Ordinary-caller, root-API USB sample continuity check. Requires the installed
//! broker and a prepared VHCI host. This does not measure end-to-end latency.
//! `cargo run --features audio-usbip --example usb_audio_acceptance -- dualsense 3 1 0`
use std::process::{Child, Command};
use std::time::{Duration, Instant};
use virtualgamepad::{
    AudioExposure, AudioOptions, ControllerAudio, CreationOptions, RealizationId, create_dualsense,
    create_dualshock4, create_xbox360,
};

struct Checker(Child);
impl Drop for Checker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn exercise(
    audio: &mut ControllerAudio,
    family: &str,
    seconds: u64,
    trials: u64,
    port: u32,
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
    let mut received = 0_u64;
    let mut expected_position = None;
    let mut discontinuities = 0_u64;
    let mut bad_samples = 0_u64;
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
        let read = audio.read_playback(&mut playback)?;
        if read.frames > 0 {
            if read.discontinuity || expected_position.is_some_and(|p| p != read.first_frame) {
                discontinuities += 1;
            }
            expected_position = Some(read.first_frame + read.frames as u64);
            received += read.frames as u64;
            for frame in playback[..read.frames * playback_channels].chunks_exact(playback_channels)
            {
                if frame != &([101, -202, 303, -404][..playback_channels]) {
                    bad_samples += 1;
                }
            }
        }
        // The initial 8 ms fill is independent of the queue capacity. Use an
        // absolute monotonic target so small loop delays do not accumulate.
        let elapsed_us = u64::try_from(started.elapsed().as_micros())?;
        let target = (elapsed_us * 48 / 1_000 + 384).min((trials * (seconds + 2) + 10) * 48_000);
        if submitted < target {
            let frames = (target - submitted).min(128) as usize;
            for frame in 0..frames {
                for channel in 0..microphone_channels {
                    microphone[frame * microphone_channels + channel] = i16::try_from(
                        100 + (submitted + frame as u64) % 97 + 100 * channel as u64,
                    )?;
                }
            }
            let accepted = audio.write_microphone(&microphone[..frames * microphone_channels])?;
            submitted += accepted as u64;
        }
        std::thread::sleep(Duration::from_micros(500));
    };
    // The IPC pump can lag process exit by one bounded block.
    let drain_deadline = Instant::now() + Duration::from_millis(100);
    while received < trials * (seconds + 2) * 48_000 && Instant::now() < drain_deadline {
        let read = audio.read_playback(&mut playback)?;
        if read.frames == 0 {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        if read.discontinuity || expected_position.is_some_and(|p| p != read.first_frame) {
            discontinuities += 1;
        }
        expected_position = Some(read.first_frame + read.frames as u64);
        received += read.frames as u64;
        for frame in playback[..read.frames * playback_channels].chunks_exact(playback_channels) {
            if frame != &([101, -202, 303, -404][..playback_channels]) {
                bad_samples += 1;
            }
        }
    }
    let dropped = audio.dropped_playback_frames();
    let silence = audio.underrun_frames();
    println!(
        "root_pcm family={family} received={received} submitted={submitted} discontinuities={discontinuities} bad_samples={bad_samples} dropped={dropped} microphone_silence={silence}"
    );
    if !child_status.success()
        || received != trials * (seconds + 2) * 48_000
        || bad_samples != 0
        || discontinuities != 0
        || dropped != 0
        || silence != 0
    {
        return Err("root USB full-duplex acceptance failed".into());
    }
    Ok(())
}

fn run(
    family: &str,
    seconds: u64,
    trials: u64,
    port: u32,
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
            );
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
            );
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
            );
            controller.close();
            result?;
        }
        _ => return Err("family must be dualsense, dualshock4 or xbox360".into()),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: usb_audio_acceptance FAMILY SECONDS TRIALS PORT".into());
    }
    let seconds = args[1].parse::<u64>()?;
    let trials = args[2].parse::<u64>()?;
    let port = args[3].parse::<u32>()?;
    if !(1..=60).contains(&seconds) || !(1..=3).contains(&trials) {
        return Err("seconds must be 1..60 and trials 1..3".into());
    }
    run(&args[0], seconds, trials, port)
}
