//! Root-API smoke test for the administrator-installed USB/IP audio broker.
//! Run with `cargo run --features audio-usbip --example usb_audio_root`.
use virtualgamepad::{
    AudioAccess, AudioExposure, AudioOptions, CreationOptions, DualSenseControl, RealizationId,
    create_dualsense, create_dualshock4, create_xbox360,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let native = std::env::args().any(|arg| arg == "native");
    let mut audio = AudioOptions::new(AudioExposure::Emulated);
    if native {
        audio = audio
            .with_playback_access(AudioAccess::NativeClient)
            .with_microphone_access(AudioAccess::NativeClient);
    }
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO).with_audio(audio);
    let mut dualsense = create_dualsense(options)?;
    dualsense.set_native(DualSenseControl::Cross, true)?;
    dualsense.commit()?;
    dualsense.service(&mut |_| {})?;
    let endpoints = dualsense.audio().expect("enabled audio").endpoints();
    assert_eq!(endpoints.len(), 2);
    if native {
        assert!(endpoints.iter().all(|endpoint| {
            endpoint
                .caller()
                .and_then(virtualgamepad::AudioEndpointSelector::pipewire_node)
                .is_some()
        }));
    } else {
        let mut playback = [0_i16; 4 * 128];
        let _ = dualsense
            .audio()
            .expect("enabled audio")
            .read_playback(&mut playback)?;
    }
    dualsense.close();

    let mut ds4 = create_dualshock4(options)?;
    ds4.service(&mut |_| {})?;
    ds4.close();

    let mut xbox = create_xbox360(options)?;
    xbox.service(&mut |_| {})?;
    xbox.close();
    Ok(())
}
