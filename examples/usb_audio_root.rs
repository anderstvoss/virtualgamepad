//! Root-API smoke test for the administrator-installed USB/IP audio broker.
//! `cargo run --features audio-usbip --example usb_audio_root -- dualsense`
//! Add `--native` and the `audio-pipewire` feature to exercise `NativeClient` ownership.

#[path = "support/usb_audio.rs"]
mod usb_audio;
use usb_audio::{Args, Attachment};
use virtualgamepad::{
    AudioAccess, AudioExposure, AudioOptions, CreationOptions, DualSenseControl, DualSenseOptions,
    DualShock4Options, RealizationId, Xbox360Options, create_dualsense, create_dualshock4,
    create_xbox360,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse()?;
    let native = args.flag("--native");
    args.finish()?;
    let mut audio = AudioOptions::new(AudioExposure::Emulated);
    if native {
        audio = audio
            .with_playback_access(AudioAccess::NativeClient)
            .with_microphone_access(AudioAccess::NativeClient);
    }
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO).with_audio(audio);
    macro_rules! smoke {
        ($controller:ident, $setup:block) => {{
            let result = (|| {
                Attachment::discover($controller.association())?.prepare(args.family)?;
                $setup
                $controller.service(&mut |_| {})?;
                let audio = $controller.audio().expect("enabled audio");
                assert_eq!(audio.endpoints().len(), 2);
                if native {
                    assert!(audio.endpoints().iter().all(|endpoint| {
                        endpoint
                            .caller()
                            .and_then(virtualgamepad::AudioEndpointSelector::pipewire_node)
                            .is_some()
                    }));
                } else {
                    let mut playback = [0_i16; 4 * 128];
                    let _ = audio.read_playback(&mut playback)?;
                }
                Ok::<_, Box<dyn std::error::Error>>(())
            })();
            $controller.close();
            result?;
        }};
    }
    match args.family {
        "dualsense" => {
            let mut controller = create_dualsense(options, DualSenseOptions::default())?;
            smoke!(controller, {
                controller.set_native(DualSenseControl::Cross, true)?;
                controller.commit()?;
            });
        }
        "dualshock4" => {
            let mut controller = create_dualshock4(options, DualShock4Options::default())?;
            smoke!(controller, {});
        }
        "xbox360" => {
            let mut controller = create_xbox360(options, Xbox360Options::default())?;
            smoke!(controller, {});
        }
        _ => unreachable!("family was validated by the shared CLI"),
    }
    Ok(())
}
