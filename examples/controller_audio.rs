//! Headless root-API consumer. Does not connect physical speakers/microphones.
//! Run with `--features audio-pipewire --example controller_audio -- dualsense 10`.
#[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    use virtualgamepad::{
        AudioAccess, AudioExposure, AudioOptions, CreationOptions, RealizationId,
    };
    let args: Vec<_> = std::env::args().skip(1).collect();
    let family = args.first().map_or("dualsense", String::as_str);
    let seconds: u64 = args.get(1).map_or(Ok(10), |s| s.parse())?;
    if !(1..=600).contains(&seconds) {
        return Err("duration must be 1..=600 seconds".into());
    }
    let native = args.get(2).is_some_and(|s| s == "native");
    let access = if native {
        AudioAccess::NativeClient
    } else {
        AudioAccess::Samples
    };
    let options = CreationOptions::new(RealizationId::LINUX_UHID_USB).with_audio(
        AudioOptions::new(AudioExposure::Emulated)
            .with_playback_access(access)
            .with_microphone_access(access),
    );
    macro_rules! exercise {
        ($create:path) => {{
            let mut controller = $create(options)?;
            let audio = controller.audio().ok_or("audio was not created")?;
            for endpoint in audio.endpoints() {
                println!(
                    "{:?}: host={} caller={:?} format={:?} access={:?}",
                    endpoint.direction(),
                    endpoint.host_node(),
                    endpoint.caller_node(),
                    endpoint.format(),
                    endpoint.access()
                );
            }
            println!("{}", audio.limitation());
            let playback_channels = audio.endpoints()[0].format().channels().len();
            let mut samples = vec![0; playback_channels * 4096];
            let mut frames = 0;
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(seconds) {
                controller.service(&mut |event| println!("{event:?}"))?;
                if !native {
                    frames += controller
                        .audio()
                        .ok_or("audio session missing")?
                        .read_playback(&mut samples)?
                        .frames;
                    // Mic underruns intentionally produce silence. An application
                    // can write its own samples; this example never opens a mic.
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            controller.close();
            let audio = controller
                .audio()
                .ok_or("retained audio diagnostics missing")?;
            println!(
                "frames_read={frames} dropped={} silence_frames={} audio_error={:?}",
                audio.dropped_playback_frames(),
                audio.underrun_frames(),
                audio.last_error()
            );
            for timing in audio.stream_timings() {
                println!("retained_graph_timing={timing:?}");
            }
            println!("controller={:?}", controller.diagnostics());
        }};
    }
    match family {
        "dualsense" => exercise!(virtualgamepad::create_dualsense),
        "dualshock4" => exercise!(virtualgamepad::create_dualshock4),
        "xbox360" => exercise!(virtualgamepad::create_xbox360),
        _ => return Err("family must be dualsense, dualshock4 or xbox360".into()),
    }
    Ok(())
}
#[cfg(not(all(target_os = "linux", feature = "audio-pipewire")))]
fn main() {
    eprintln!("Controller audio requires Linux and --features audio-pipewire");
}
