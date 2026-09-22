#![cfg(all(target_os = "linux", feature = "audio-pipewire"))]
use virtualgamepad::{
    AudioExposure, AudioOptions, CreationOptions, RealizationId, create_dualsense,
    create_dualshock4, create_xbox360,
};

#[test]
#[ignore = "requires prepared UHID and PipeWire session"]
fn all_family_root_audio_creation_service_and_terminal_cleanup() {
    let options = CreationOptions::new(RealizationId::LINUX_UHID_USB)
        .with_audio(AudioOptions::new(AudioExposure::Emulated));
    macro_rules! exercise {
        ($create:ident, $channels:expr, $mic:expr) => {{
            let mut controller = $create(options).unwrap();
            let mut sibling = $create(options).unwrap();
            assert_ne!(
                controller.association().creation(),
                sibling.association().creation()
            );
            assert_eq!(controller.association().components().len(), 3);
            assert!(controller.association().components()[1].surface().is_none());
            assert!(
                controller.association().components()[1]
                    .audio_endpoint()
                    .is_some()
            );
            let audio = controller.audio().unwrap();
            assert_eq!(audio.endpoints()[0].format().channels().len(), $channels);
            assert_eq!(audio.endpoints()[1].format().channels().len(), $mic);
            assert!(!audio.is_closed());
            assert_eq!(audio.write_microphone(&[0; $mic * 16]).unwrap(), 16);
            controller.service(&mut |_| {}).unwrap();
            controller.neutralize().unwrap();
            controller.commit().unwrap();
            controller.close();
            controller.close();
            assert!(controller.audio().unwrap().is_closed());
            assert!(controller.audio().unwrap().last_error().is_none());
            assert!(
                controller
                    .audio()
                    .unwrap()
                    .write_microphone(&[0; $mic])
                    .is_err()
            );
            sibling.service(&mut |_| {}).unwrap();
            assert!(!sibling.audio().unwrap().is_closed());
            sibling.close();
        }};
    }
    exercise!(create_dualsense, 4, 2);
    exercise!(create_dualshock4, 2, 1);
    exercise!(create_xbox360, 2, 1);
}
