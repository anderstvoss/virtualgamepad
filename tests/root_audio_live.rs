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

/// Short real-worker concurrency/lifetime probe, not continuity or latency acceptance.
#[test]
#[ignore = "requires prepared UHID and an isolated PipeWire session"]
fn all_family_root_audio_bounded_pcm_and_hid_threads() {
    use std::sync::{Arc, Barrier, Mutex};
    use std::time::Instant;
    let options = CreationOptions::new(RealizationId::LINUX_UHID_USB)
        .with_audio(AudioOptions::new(AudioExposure::Emulated));
    macro_rules! exercise {
        ($create:path, $channels:expr, $mic:expr) => {{
            let controller = Arc::new(Mutex::new($create(options).unwrap()));
            let barrier = Arc::new(Barrier::new(2));
            let start = Instant::now();
            let mut maximum_service_us = 0;
            std::thread::scope(|scope| {
                let pcm_owner = controller.clone();
                let pcm_barrier = barrier.clone();
                let pcm = scope.spawn(move || {
                    let mut received = 0;
                    let mut submitted = 0;
                    for _ in 0..64 {
                        pcm_barrier.wait();
                        {
                            let mut controller = pcm_owner.lock().unwrap();
                            let audio = controller.audio().unwrap();
                            received += audio.read_playback(&mut [0; $channels * 128]).unwrap().frames;
                            submitted += audio.write_microphone(&[0; $mic * 128]).unwrap();
                        }
                        pcm_barrier.wait();
                    }
                    (received, submitted)
                });
                for _ in 0..64 {
                    barrier.wait();
                    let service_start = Instant::now();
                    {
                        let mut controller = controller.lock().unwrap();
                        controller.service(&mut |_| {}).unwrap();
                        assert!(controller.next_service_in().is_some());
                    }
                    maximum_service_us = maximum_service_us.max(service_start.elapsed().as_micros());
                    barrier.wait();
                }
                let (received, submitted) = pcm.join().unwrap();
                println!("root-concurrency cycles=64 elapsed_us={} max_service_and_lock_us={} playback_frames={} microphone_frames={}",start.elapsed().as_micros(),maximum_service_us,received,submitted);
            });
            let mut controller = controller.lock().unwrap();
            controller.close(); controller.close();
            assert!(controller.audio().unwrap().diagnostics().is_closed());
            assert!(controller.audio().unwrap().last_error().is_none());
        }};
    }
    exercise!(create_dualsense, 4, 2);
    exercise!(create_dualshock4, 2, 1);
    exercise!(create_xbox360, 2, 1);
}
