//! The harness injects only provider I/O; assertions exercise application handles.
use super::*;
use crate::{ControllerStatus, ForceFeedbackEffect, ForceFeedbackEvent, RumbleEffect};
use gr_realization_api::{
    EventReadiness, NativeProviderSession, ProviderDiagnostics, ProviderError, ProviderFrame,
    ProviderReverseEvent, ProviderReverseEventSink, ProviderState, RawReverseEvent,
    RealizationSessionId,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct Record {
    events: VecDeque<RawReverseEvent>,
    sent: Vec<ProviderFrame>,
    closes: usize,
    fail_write: bool,
    fail_close: bool,
}
struct Fake(Arc<Mutex<Record>>);
impl NativeProviderSession for Fake {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        let mut record = self.0.lock().unwrap();
        if record.closes != 0 {
            return Err(ProviderError::Closed);
        }
        if std::mem::take(&mut record.fail_write) {
            return Err(ProviderError::WouldBlock);
        }
        record.sent.push(frame);
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut record = self.0.lock().unwrap();
        if record.closes != 0 {
            return Err(ProviderError::Closed);
        }
        if let Some(event) = record.events.pop_front() {
            out.push(ProviderReverseEvent {
                session: RealizationSessionId(99),
                sequence: 1,
                event,
            });
        }
        Ok(())
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        let record = self.0.lock().unwrap();
        ProviderDiagnostics {
            state: if record.closes == 0 {
                ProviderState::Open
            } else {
                ProviderState::Closed
            },
            frames_sent: record.sent.len() as u64,
            reverse_events_drained: 0,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        let mut record = self.0.lock().unwrap();
        record.closes += 1;
        if record.fail_close {
            Err(ProviderError::Write {
                reason: "synthetic cleanup failure".into(),
            })
        } else {
            Ok(())
        }
    }
}
fn effect() -> ForceFeedbackEffect {
    ForceFeedbackEffect::Rumble(RumbleEffect {
        id: 0,
        strong: 100,
        weak: 200,
        length_ms: 10,
        delay_ms: 0,
        trigger_button: 0,
        trigger_interval_ms: 0,
    })
}

#[test]
fn native_face_labels_preserve_spatial_encoding_and_readback() {
    use crate::FaceButton::{East, North, South, West};
    macro_rules! verify {
        ($module:ident, $control:ident, $pairs:expr) => {
            for (native, spatial) in $pairs {
                let native_record = Arc::new(Mutex::new(Record::default()));
                let spatial_record = Arc::new(Mutex::new(Record::default()));
                let mut native_controller = gr_curated_controllers::$module::test_controller(
                    Box::new(Fake(native_record.clone())),
                )
                .unwrap();
                let mut spatial_controller = gr_curated_controllers::$module::test_controller(
                    Box::new(Fake(spatial_record.clone())),
                )
                .unwrap();
                native_controller.set_native(native, true).unwrap();
                spatial_controller
                    .set_digital(DigitalControlUpdate::FaceButton {
                        button: spatial,
                        pressed: true,
                    })
                    .unwrap();
                native_controller.commit().unwrap();
                spatial_controller.commit().unwrap();
                assert_eq!(
                    native_record.lock().unwrap().sent,
                    spatial_record.lock().unwrap().sent
                );
                for button in [South, East, West, North] {
                    assert_eq!(
                        native_controller.state().face_pressed(button),
                        button == spatial
                    );
                }
                assert!(native_controller.state().native_pressed(native));
                native_controller.neutralize().unwrap();
                assert!(!native_controller.state().native_pressed(native));
            }
        };
    }
    verify!(
        dualshock4,
        DualShock4Control,
        [
            (DualShock4Control::Cross, South),
            (DualShock4Control::Circle, East),
            (DualShock4Control::Square, West),
            (DualShock4Control::Triangle, North),
        ]
    );
    verify!(
        switch_pro,
        SwitchProControl,
        [
            (SwitchProControl::B, South),
            (SwitchProControl::A, East),
            (SwitchProControl::Y, West),
            (SwitchProControl::X, North),
        ]
    );
}

macro_rules! consumer_case {
    ($test:ident, $module:ident, $controller:ident, $control:ident, $button:ident, $output:ident, $id:literal $(, $field:ident : $value:expr)*) => {
        #[test]
        fn $test() {
            fn create() -> ($controller, Arc<Mutex<Record>>) {
                let record = Arc::new(Mutex::new(Record::default()));
                let options = CreationOptions::new(RealizationId::LINUX_UINPUT).internal().unwrap();
                let inner = gr_curated_controllers::$module::test_controller(Box::new(Fake(record.clone()))).unwrap();
                let association = ControllerAssociation::single(ControllerId::new($id), options, inner.association(), inner.surface().common());
                ($controller { inner, association, $($field: $value,)* }, record)
            }
            let (mut controller, record) = create();
            let (mut other, other_record) = create();
            for (event, expected) in [
                (gr_hid::Lifecycle::Start { numbered_input: true, numbered_output: true, numbered_feature: true }, crate::HostLifecycle::Started),
                (gr_hid::Lifecycle::Open, crate::HostLifecycle::Opened),
                (gr_hid::Lifecycle::Close, crate::HostLifecycle::Closed),
                (gr_hid::Lifecycle::Stop, crate::HostLifecycle::Stopped),
            ] {
                record.lock().unwrap().events.push_back(RawReverseEvent::HidLifecycle(event));
                let mut seen = Vec::new();
                controller.service(&mut |output| if let $output::HostLifecycle(event) = output { seen.push(event); }).unwrap();
                assert_eq!(seen, [expected]);
                controller.service(&mut |_| panic!("lifecycle replayed")).unwrap();
            }

            assert_ne!(controller.association().creation(), other.association().creation());
            assert_eq!(controller.association().components().len(), 1);
            assert_eq!(controller.association().components()[0].role(), "primary");
            assert!(matches!(controller.readiness(), Some(ServiceReadiness::Poll)));
            controller.set_native($control::$button, true).unwrap();
            assert!(controller.state().native_pressed($control::$button));
            let accepted = controller.state().clone();
            record.lock().unwrap().fail_write = true;
            assert!(controller.commit().is_err());
            assert!(controller.is_dirty());
            assert_eq!(controller.state(), &accepted);
            controller.commit().unwrap();
            assert!(!controller.is_dirty());
            let request = RawReverseEvent::ForceFeedbackUpload { request_id: 17, effect: effect() };
            record.lock().unwrap().events.push_back(request);
            let mut observations = Vec::new();
            controller.service(&mut |event| observations.push(event)).unwrap();
            assert_eq!(record.lock().unwrap().sent.last(), Some(&ProviderFrame::ForceFeedbackUploadReply { request_id: 17, status: 0 }));
            assert!(matches!(observations.last(), Some($output::ForceFeedback(ForceFeedbackEvent::Uploaded { request_id: 17, status: 0, .. }))));
            let count = observations.len();
            for _ in 0..10 { controller.service(&mut |event| observations.push(event)).unwrap(); }
            assert_eq!(observations.len(), count);
            record.lock().unwrap().events.push_back(RawReverseEvent::ForceFeedbackErase { request_id: 18, effect_id: 0 });
            controller.service(&mut |_| {}).unwrap();
            assert_eq!(record.lock().unwrap().sent.last(), Some(&ProviderFrame::ForceFeedbackEraseReply { request_id: 18, status: 0 }));
            record.lock().unwrap().events.push_back(RawReverseEvent::ForceFeedbackUpload { request_id: 19, effect: ForceFeedbackEffect::Unsupported { id: 1, kind: 0x51 } });
            controller.service(&mut |_| {}).unwrap();
            assert_eq!(record.lock().unwrap().sent.last(), Some(&ProviderFrame::ForceFeedbackUploadReply { request_id: 19, status: -95 }));
            controller.neutralize().unwrap();
            assert!(!controller.state().native_pressed($control::$button));
            controller.commit().unwrap();
            record.lock().unwrap().fail_close = true;
            controller.close();
            controller.close();
            assert_eq!(controller.diagnostics().status(), ControllerStatus::Closed);
            assert!(controller.diagnostics().last_error().unwrap().contains("synthetic cleanup failure"));
            assert!(controller.readiness().is_none());
            assert!(controller.next_service_in().is_none());
            assert!(!controller.wants_write());
            assert!(matches!(controller.set_native($control::$button, true), Err(ControlError::Closed)));
            assert!(matches!(controller.service(&mut |_| {}), Err(ControllerError::Closed)));
            drop(controller);
            assert_eq!(record.lock().unwrap().closes, 1);
            other.set_native($control::$button, true).unwrap();
            other.commit().unwrap();
            other.service(&mut |_| {}).unwrap();
            assert_eq!(other_record.lock().unwrap().closes, 0);
            // A required request escaping the selected personality fails closed
            // within the same service cycle, rather than becoming caller work.
            other_record.lock().unwrap().events.push_back(RawReverseEvent::HidGetReportRequest { request_id: 20, report_id: 1, report_type: 0 });
            assert!(matches!(other.service(&mut |_| {}), Err(ControllerError::InvalidRequest { .. })));
            assert_eq!(other_record.lock().unwrap().closes, 1);
            let (replacement, replacement_record) = create();
            assert_ne!(replacement.association().creation(), other.association().creation());
            drop(replacement);
            assert_eq!(replacement_record.lock().unwrap().closes, 1);
        }
    };
}
consumer_case!(dualsense_consumer_lifecycle, dualsense, DualSenseController, DualSenseControl, Cross, DualSenseOutputEvent, "virtualgamepad.dualsense", identity: None, audio: None);
consumer_case!(ds4_consumer_lifecycle, dualshock4, DualShock4Controller, DualShock4Control, Cross, DualShock4OutputEvent, "virtualgamepad.dualshock4", identity: None, audio: None);
consumer_case!(
    switch_consumer_lifecycle,
    switch_pro,
    SwitchProController,
    SwitchProControl,
    L,
    SwitchProOutputEvent,
    "virtualgamepad.switch-pro"
);
consumer_case!(
    xbox_consumer_lifecycle,
    xbox360,
    Xbox360Controller,
    Xbox360Control,
    A,
    Xbox360OutputEvent,
    "virtualgamepad.xbox360", audio: None
);

macro_rules! audio_consumer_case {
    ($create:ident, $test:ident, $threaded:ident, $module:ident, $controller:ident, $id:literal $(, $field:ident : $value:expr)*) => {
        fn $create(access: crate::AudioAccess) -> ($controller, Arc<Mutex<Record>>, Arc<Mutex<crate::audio::fake::Record>>) {
            let record = Arc::new(Mutex::new(Record::default()));
            let options = CreationOptions::new(RealizationId::LINUX_UINPUT).internal().unwrap();
            let inner = gr_curated_controllers::$module::test_controller(Box::new(Fake(record.clone()))).unwrap();
            let (audio, audio_record) = crate::audio::fake::open(options.session.0, access);
            let association = ControllerAssociation::single(ControllerId::new($id), options, inner.association(), inner.surface().common()).with_audio(Some(&audio));
            ($controller { inner, association, audio: Some(audio), $($field: $value,)* }, record, audio_record)
        }
        #[test]
        fn $test() {
            for access in [crate::AudioAccess::Samples, crate::AudioAccess::NativeClient] {
                for audio_failed in [false, true] {
                    let (mut controller, hid, audio) = $create(access);
                    let (mut sibling, sibling_hid, sibling_audio) = $create(access);
                    let creation = controller.association().creation();
                    let association = controller.association().clone();
                    assert_eq!(association.components().len(), 3);
                    assert_eq!(association.components()[0].kind(), crate::ComponentKind::Input);
                    for component in &association.components()[1..] {
                        assert_eq!(component.kind(), crate::ComponentKind::Audio);
                        assert!(component.surface().is_none());
                        assert!(component.requested_unique_id().is_none());
                        assert!(component.requested_physical_path().is_none());
                        assert!(component.audio_endpoint().unwrap().host().pipewire_node().unwrap().contains(&creation.to_string()));
                    }
                    controller.neutralize().unwrap(); controller.commit().unwrap();
                    { let record = audio.lock().unwrap(); assert_eq!((record.playback_flushes,record.microphone_flushes),(0,0)); }
                    for _ in 0..8 { controller.service(&mut |_| {}).unwrap(); }
                    if access == crate::AudioAccess::Samples {
                        controller.audio().unwrap().flush_playback().unwrap();
                        controller.audio().unwrap().flush_microphone().unwrap();
                        assert_eq!(hid.lock().unwrap().closes,0);
                    }
                    hid.lock().unwrap().fail_close = true;
                    audio.lock().unwrap().cleanup_failure = true;
                    if audio_failed {
                        audio.lock().unwrap().failed = true;
                    } else {
                        // A required HID request escaping the personality fails
                        // closed in the same service cycle, including its audio.
                        hid.lock().unwrap().events.push_back(RawReverseEvent::HidGetReportRequest { request_id: 77, report_id: 1, report_type: 0 });
                    }
                    assert!(controller.service(&mut |_| {}).is_err());
                    controller.close(); controller.close();
                    let diagnostics = controller.diagnostics();
                    assert_eq!(diagnostics.status(),ControllerStatus::Failed);
                    let error = diagnostics.last_error().unwrap();
                    assert!(error.contains("synthetic cleanup failure"));
                    assert!(error.contains("synthetic audio cleanup failure"));
                    if audio_failed { assert!(error.contains("synthetic audio failure")); }
                    assert!(controller.readiness().is_none());
                    assert!(controller.next_service_in().is_none());
                    assert!(controller.audio().unwrap().diagnostics().is_closed());
                    assert_eq!(controller.audio().unwrap().read_playback(&mut [0;2]),Err(crate::AudioError::Closed));
                    assert_eq!(controller.diagnostics(), diagnostics);
                    drop(controller);
                    assert_eq!(hid.lock().unwrap().closes,1);
                    assert_eq!(audio.lock().unwrap().closes,1);
                    sibling.service(&mut |_| {}).unwrap();
                    assert_eq!(sibling_hid.lock().unwrap().closes,0);
                    assert_eq!(sibling_audio.lock().unwrap().closes,0);
                    let (replacement, _, replacement_audio) = $create(access);
                    assert_ne!(replacement.association().creation(),creation);
                    assert_ne!(replacement.association().components()[1].audio_endpoint().unwrap().host(),association.components()[1].audio_endpoint().unwrap().host());
                    drop(replacement);
                    assert_eq!(replacement_audio.lock().unwrap().closes,1);
                }
            }
        }
        #[test]
        fn $threaded() {
            use std::sync::Barrier;
            let (controller,hid,audio) = $create(crate::AudioAccess::Samples);
            let controller = Arc::new(Mutex::new(controller));
            let barrier = Arc::new(Barrier::new(2));
            std::thread::scope(|scope| {
                let pcm_controller = controller.clone();
                let pcm_barrier = barrier.clone();
                scope.spawn(move || {
                    for _ in 0..64 {
                        pcm_barrier.wait();
                        {
                            let mut controller = pcm_controller.lock().unwrap();
                            let audio = controller.audio().unwrap();
                            audio.read_playback(&mut [0;8]).unwrap();
                            audio.write_microphone(&[101,-202]).unwrap();
                        }
                        pcm_barrier.wait();
                    }
                });
                for _ in 0..64 {
                    barrier.wait();
                    {
                        let mut controller = controller.lock().unwrap();
                        controller.service(&mut |_| {}).unwrap();
                        assert!(controller.next_service_in().is_some());
                    }
                    barrier.wait();
                }
            });
            { let record = audio.lock().unwrap(); assert_eq!((record.reads,record.writes),(64,64)); }
            controller.lock().unwrap().close();
            assert_eq!(hid.lock().unwrap().closes,1);
            assert_eq!(audio.lock().unwrap().closes,1);
        }
    };
}
audio_consumer_case!(audio_dualsense, dualsense_required_audio_lifecycle, dualsense_borrowed_audio_threads, dualsense, DualSenseController, "virtualgamepad.dualsense", identity: None);
audio_consumer_case!(audio_ds4, ds4_required_audio_lifecycle, ds4_borrowed_audio_threads, dualshock4, DualShock4Controller, "virtualgamepad.dualshock4", identity: None);
audio_consumer_case!(
    audio_xbox,
    xbox_required_audio_lifecycle,
    xbox_borrowed_audio_threads,
    xbox360,
    Xbox360Controller,
    "virtualgamepad.xbox360"
);

#[test]
fn mixed_audio_and_no_audio_root_consumers_service_independently() {
    let (mut sony, _, sony_audio) = audio_dualsense(crate::AudioAccess::Samples);
    let (mut ds4, _, ds4_audio) = audio_ds4(crate::AudioAccess::NativeClient);
    let (mut xbox, _, xbox_audio) = audio_xbox(crate::AudioAccess::Samples);
    let record = Arc::new(Mutex::new(Record::default()));
    let inner = gr_curated_controllers::switch_pro::test_controller(Box::new(Fake(record.clone())))
        .unwrap();
    let options = CreationOptions::new(RealizationId::LINUX_UINPUT)
        .internal()
        .unwrap();
    let association = ControllerAssociation::single(
        ControllerId::new("virtualgamepad.switch-pro"),
        options,
        inner.association(),
        inner.surface().common(),
    );
    let mut switch = SwitchProController { inner, association };
    for _ in 0..64 {
        sony.service(&mut |_| {}).unwrap();
        ds4.service(&mut |_| {}).unwrap();
        xbox.service(&mut |_| {}).unwrap();
        switch.service(&mut |_| {}).unwrap();
        sony.audio().unwrap().read_playback(&mut [0; 8]).unwrap();
        xbox.audio()
            .unwrap()
            .write_microphone(&[101, -202])
            .unwrap();
    }
    ds4.close();
    sony.service(&mut |_| {}).unwrap();
    xbox.service(&mut |_| {}).unwrap();
    switch.service(&mut |_| {}).unwrap();
    assert_eq!(ds4_audio.lock().unwrap().closes, 1);
    assert_eq!(sony_audio.lock().unwrap().closes, 0);
    assert_eq!(xbox_audio.lock().unwrap().closes, 0);
    sony.close();
    xbox.close();
    switch.close();
}

#[cfg(all(target_os = "linux", feature = "audio-usbip"))]
mod worker_outputs {
    use super::*;
    use gr_audio_worker::client::Control;
    use gr_curated_controllers::WorkerBridge;
    use std::{marker::PhantomData, os::unix::net::UnixStream, thread, time::Duration};

    struct WireRecord {
        control: Control,
        retained: ProviderDiagnostics,
        closes: usize,
        live: bool,
        dropped: u64,
    }
    struct WireBridge<S>(Arc<Mutex<WireRecord>>, PhantomData<S>);
    impl<S: Send> WorkerBridge<S> for WireBridge<S> {
        fn update(&mut self, _: &S) -> Result<(), ProviderError> {
            panic!("output service must not synthesize an input update")
        }
        fn output(&mut self) -> Result<Option<RawReverseEvent>, ProviderError> {
            let mut record = self.0.lock().unwrap();
            let WireRecord {
                control, retained, ..
            } = &mut *record;
            crate::usb_audio::worker_output(control, retained)
        }
        fn diagnostics(&mut self) -> ProviderDiagnostics {
            let mut record = self.0.lock().unwrap();
            if record.live && record.retained.state == ProviderState::Open {
                match record.control.diagnostics() {
                    Ok(counters) => record.dropped = counters[0],
                    Err(error) => {
                        record.retained.state = ProviderState::Failed;
                        record.retained.last_error = Some(error.to_string());
                    }
                }
            }
            record.retained.clone()
        }
        fn dropped_output_events(&self) -> u64 {
            self.0.lock().unwrap().dropped
        }
        fn close(&mut self) -> Result<(), ProviderError> {
            let mut record = self.0.lock().unwrap();
            if record.retained.state == ProviderState::Closed {
                return Ok(());
            }
            record.closes += 1;
            let result = record.control.close();
            record.retained.state = ProviderState::Closed;
            result.map_err(|error| {
                let reason = error.to_string();
                record.retained.last_error = Some(match record.retained.last_error.take() {
                    Some(original) => format!("{original}; worker cleanup: {reason}"),
                    None => reason.clone(),
                });
                ProviderError::Write { reason }
            })
        }
    }

    type WireFixture<S> = (
        Box<dyn WorkerBridge<S>>,
        Arc<Mutex<WireRecord>>,
        thread::JoinHandle<()>,
    );
    fn wire<S: Send + 'static>(family: u8, outputs: Vec<Vec<u8>>) -> WireFixture<S> {
        let (client, mut server) = UnixStream::pair().unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut outputs = VecDeque::from(outputs);
        let task = thread::spawn(move || {
            loop {
                let (tag, generation) = match gr_privileged_broker::read_message(&mut server) {
                    Ok(value) => value,
                    Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(error) => panic!("unexpected worker request failure: {error}"),
                };
                assert_eq!(generation, 7_u64.to_le_bytes());
                let mut response = generation;
                match tag {
                    2 => response.extend(outputs.pop_front().unwrap_or_else(|| vec![0])),
                    4 => {}
                    _ => panic!("unexpected worker operation {tag}"),
                }
                gr_privileged_broker::write_message(&mut server, tag, &response).unwrap();
                if tag == 4 {
                    assert!(outputs.is_empty(), "closed before pending output delivery");
                    break;
                }
            }
        });
        let record = Arc::new(Mutex::new(WireRecord {
            control: Control::new(client, 7, family).unwrap(),
            retained: ProviderDiagnostics {
                state: ProviderState::Open,
                frames_sent: 0,
                reverse_events_drained: 0,
                write_failures: 0,
                lifecycle_events: 0,
                last_error: None,
            },
            closes: 0,
            live: false,
            dropped: 0,
        }));
        (
            Box::new(WireBridge(record.clone(), PhantomData)),
            record,
            task,
        )
    }
    fn association(
        inner: &gr_curated_controllers::ControllerAssociation,
        surface: &'static crate::ControllerSurface,
    ) -> ControllerAssociation {
        ControllerAssociation::single(
            ControllerId::new("test.usb.outputs"),
            gr_curated_controllers::CreationOptions {
                target: RealizationId::LINUX_USBIP_USB_AUDIO,
                session: RealizationSessionId(7),
            },
            inner,
            surface,
        )
    }

    fn dualsense_cases(live: bool) -> (Vec<Vec<u8>>, Vec<crate::DualSenseOutputEvent>) {
        let routes = [
            crate::DualSenseAudioPath::HeadphonesStereo,
            crate::DualSenseAudioPath::HeadphonesDualMono,
            crate::DualSenseAudioPath::HeadphonesLeftSpeakerRight,
            crate::DualSenseAudioPath::SpeakerRightOnly,
        ];
        let mut outputs = Vec::new();
        let mut expected = Vec::new();
        for (index, route) in routes.into_iter().enumerate() {
            let (right, left) = [(17, 33), (44, 66), (0, 0), (0, 0)][index];
            let mut raw = vec![0; 47];
            raw[..4].copy_from_slice(&[0xe1, 0x97, right, left]);
            raw[5..10].copy_from_slice(&[
                64,
                96,
                u8::try_from(index).unwrap() << 4,
                u8::from((index % 2 == 0) != live),
                if index % 2 == 1 { 0x10 } else { 0 },
            ]);
            raw[10..21].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
            raw[21..32].copy_from_slice(&[11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
            raw[37] = 0xfd;
            let red = 32
                + if live {
                    u8::try_from(index).unwrap()
                } else {
                    0
                };
            raw[43..47].copy_from_slice(&[0x15, red, 64, 128]);
            outputs.push([vec![1, 2], raw.clone()].concat());
            expected.push(crate::DualSenseOutputEvent::HidOutput(
                crate::DualSenseHidOutput::UsbOutput {
                    raw,
                    valid_flag0: 0xe1,
                    valid_flag1: 0x97,
                    valid_flag2: 0,
                    right_motor: Some(right),
                    left_motor: Some(left),
                    right_trigger_effect: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                    left_trigger_effect: [11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
                    mute_button_led: Some((index % 2 == 0) != live),
                    microphone_muted: Some(index % 2 == 1),
                    audio_path: Some(route),
                    speaker_volume: Some(64),
                    microphone_volume: Some(96),
                    speaker_preamp: Some(5),
                    player_leds: Some(0x15),
                    lightbar_rgb: Some([red, 64, 128]),
                },
            ));
        }
        for flag2 in [0, 4] {
            let mut raw = outputs[0][2..].to_vec();
            raw[0] = 0;
            raw[1] = 0;
            raw[38] = flag2;
            outputs.push([vec![1, 2], raw.clone()].concat());
            expected.push(crate::DualSenseOutputEvent::HidOutput(
                crate::DualSenseHidOutput::UsbOutput {
                    raw,
                    valid_flag0: 0,
                    valid_flag1: 0,
                    valid_flag2: flag2,
                    right_motor: (flag2 == 4).then_some(17),
                    left_motor: (flag2 == 4).then_some(33),
                    right_trigger_effect: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                    left_trigger_effect: [11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
                    mute_button_led: None,
                    microphone_muted: None,
                    audio_path: None,
                    speaker_volume: None,
                    microphone_volume: None,
                    speaker_preamp: None,
                    player_leds: None,
                    lightbar_rgb: None,
                },
            ));
        }
        (outputs, expected)
    }
    #[test]
    fn dualsense_worker_wire_reaches_root_typed_callbacks_in_order_without_idle_replay() {
        let (outputs, expected) = dualsense_cases(false);
        let (bridge, record, task) = wire(1, outputs);
        let inner = gr_curated_controllers::create_dualsense_usb_worker(bridge);
        let mut controller = DualSenseController {
            association: association(inner.association(), inner.surface().common()),
            inner,
            audio: None,
            identity: None,
        };
        let mut observed = Vec::new();
        controller
            .service(&mut |event| observed.push(event))
            .unwrap();
        assert_eq!(observed, expected);
        for _ in 0..5 {
            controller
                .service(&mut |_| panic!("idle callback replay"))
                .unwrap();
        }
        assert_eq!(record.lock().unwrap().retained.reverse_events_drained, 6);
        assert_eq!(controller.dropped_output_events(), 0);
        controller.close();
        controller.close();
        assert_eq!(record.lock().unwrap().closes, 1);
        task.join().unwrap();
    }

    fn ds4_cases(live: bool) -> (Vec<Vec<u8>>, Vec<crate::DualShock4OutputEvent>) {
        let mut outputs = Vec::new();
        let mut expected = Vec::new();
        for (index, (right, left, enabled)) in [(17, 33, true), (44, 66, false), (0, 0, true)]
            .into_iter()
            .enumerate()
        {
            let light = enabled;
            let red = 32
                + if live {
                    u8::try_from(index).unwrap()
                } else {
                    0
                };
            let mut raw = vec![0; 31];
            raw[0] = if light { 3 } else { 1 };
            raw[3..8].copy_from_slice(&[right, left, red, 64, 128]);
            outputs.push([vec![1, 5], raw.clone()].concat());
            expected.push(crate::DualShock4OutputEvent::HidOutput(
                crate::DualShock4HidOutput::UsbOutput {
                    raw,
                    right_motor: right,
                    left_motor: left,
                    lightbar_rgb: light.then_some([red, 64, 128]),
                },
            ));
        }
        (outputs, expected)
    }
    #[test]
    fn ds4_worker_wire_preserves_start_update_stop_and_lightbar_validity() {
        let (outputs, expected) = ds4_cases(false);
        let (bridge, record, task) = wire(2, outputs);
        let inner = gr_curated_controllers::create_dualshock4_usb_worker(bridge);
        let mut controller = DualShock4Controller {
            association: association(inner.association(), inner.surface().common()),
            inner,
            audio: None,
            identity: None,
        };
        let mut observed = Vec::new();
        controller
            .service(&mut |event| observed.push(event))
            .unwrap();
        assert_eq!(observed, expected);
        for _ in 0..5 {
            controller
                .service(&mut |_| panic!("idle callback replay"))
                .unwrap();
        }
        assert_eq!(record.lock().unwrap().retained.reverse_events_drained, 3);
        controller.close();
        controller.close();
        assert_eq!(record.lock().unwrap().closes, 1);
        task.join().unwrap();
    }

    #[test]
    fn rejected_worker_output_produces_no_root_callback_and_malformed_reply_closes_once() {
        for response in [vec![0], vec![0, 9], vec![2, 5]] {
            let malformed = response.len() != 1;
            let (bridge, record, task) = wire(3, vec![response]);
            let inner = gr_curated_controllers::create_xbox360_usb_worker(bridge);
            let mut controller = Xbox360Controller {
                association: association(inner.association(), inner.surface().common()),
                inner,
                audio: None,
            };
            let result =
                controller.service(&mut |_| panic!("rejected output reached root callback"));
            assert_eq!(result.is_err(), malformed);
            if malformed {
                assert!(
                    record
                        .lock()
                        .unwrap()
                        .retained
                        .last_error
                        .as_ref()
                        .unwrap()
                        .contains("invalid worker output")
                );
                assert!(
                    controller
                        .service(&mut |_| panic!("closed callback"))
                        .is_err()
                );
            } else {
                for _ in 0..5 {
                    controller
                        .service(&mut |_| panic!("idle callback"))
                        .unwrap();
                }
            }
            assert_eq!(record.lock().unwrap().retained.reverse_events_drained, 0);
            controller.close();
            controller.close();
            assert_eq!(record.lock().unwrap().closes, 1);
            task.join().unwrap();
        }
    }

    fn live_bridge<S: Send + 'static>(
        control: Control,
    ) -> (Box<dyn WorkerBridge<S>>, Arc<Mutex<WireRecord>>) {
        let record = Arc::new(Mutex::new(WireRecord {
            control,
            retained: ProviderDiagnostics {
                state: ProviderState::Open,
                frames_sent: 0,
                reverse_events_drained: 0,
                write_failures: 0,
                lifecycle_events: 0,
                last_error: None,
            },
            closes: 0,
            live: true,
            dropped: 0,
        }));
        (Box::new(WireBridge(record.clone(), PhantomData)), record)
    }

    enum LiveController {
        DualSense(DualSenseController, Vec<crate::DualSenseOutputEvent>),
        Ds4(DualShock4Controller, Vec<crate::DualShock4OutputEvent>),
        Xbox(Xbox360Controller),
    }
    impl LiveController {
        fn observe(&mut self, sequence: usize) {
            macro_rules! check {
                ($controller:expr, $expected:expr) => {{
                    let mut observed = Vec::new();
                    let deadline = std::time::Instant::now() + Duration::from_secs(1);
                    while observed.is_empty() && std::time::Instant::now() < deadline {
                        $controller
                            .service(&mut |event| observed.push(event))
                            .unwrap();
                        if observed.is_empty() {
                            thread::sleep(Duration::from_millis(1));
                        }
                    }
                    assert_eq!(observed, vec![$expected[sequence / 2].clone()]);
                    for _ in 0..5 {
                        $controller
                            .service(&mut |_| panic!("live typed callback replay"))
                            .unwrap();
                    }
                    assert_eq!($controller.dropped_output_events(), 0);
                }};
            }
            match self {
                Self::DualSense(controller, expected) => check!(controller, expected),
                Self::Ds4(controller, expected) => check!(controller, expected),
                Self::Xbox(controller) => {
                    for _ in 0..5 {
                        controller
                            .service(&mut |_| panic!("unsupported output became a callback"))
                            .unwrap();
                    }
                    assert_eq!(controller.dropped_output_events(), 0);
                }
            }
        }
        fn close(&mut self) {
            match self {
                Self::DualSense(controller, _) => {
                    controller.close();
                    controller.close();
                }
                Self::Ds4(controller, _) => {
                    controller.close();
                    controller.close();
                }
                Self::Xbox(controller) => {
                    controller.close();
                    controller.close();
                }
            }
        }
    }

    fn observer_channels(peer: &UnixStream) -> std::io::Result<[UnixStream; 3]> {
        let timeout = peer.read_timeout()?;
        let channels = gr_privileged_broker::audio_fds::receive(peer)?;
        // Descriptor receipt uses a short deadline; the next phase includes PCM
        // production and drain, so restore the caller's bounded phase deadline.
        peer.set_read_timeout(timeout)?;
        Ok(channels)
    }

    #[test]
    fn observer_handoff_restores_phase_deadline_and_preserves_channels() {
        use std::io::{Read, Write};
        let (tx, rx) = UnixStream::pair().unwrap();
        rx.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
        let pairs = [
            UnixStream::pair().unwrap(),
            UnixStream::pair().unwrap(),
            UnixStream::pair().unwrap(),
        ];
        let (channels, mut peers): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
        let channels: [UnixStream; 3] = channels.try_into().unwrap();
        gr_privileged_broker::audio_fds::send(&tx, &channels).unwrap();
        let mut received = observer_channels(&rx).unwrap();
        assert_eq!(rx.read_timeout().unwrap(), Some(Duration::from_secs(30)));
        for (index, channel) in received.iter_mut().enumerate() {
            let marker = [u8::try_from(index).unwrap()];
            peers[index].write_all(&marker).unwrap();
            let mut actual = [0];
            channel.read_exact(&mut actual).unwrap();
            assert_eq!(actual, marker);
        }
    }

    fn live_controller(
        control: Control,
        family: u8,
    ) -> (LiveController, Arc<Mutex<WireRecord>>, usize) {
        match family {
            1 => {
                let (bridge, record) = live_bridge(control);
                let inner = gr_curated_controllers::create_dualsense_usb_worker(bridge);
                (
                    LiveController::DualSense(
                        DualSenseController {
                            association: association(inner.association(), inner.surface().common()),
                            inner,
                            audio: None,
                            identity: None,
                        },
                        dualsense_cases(true).1,
                    ),
                    record,
                    12,
                )
            }
            2 => {
                let (bridge, record) = live_bridge(control);
                let inner = gr_curated_controllers::create_dualshock4_usb_worker(bridge);
                (
                    LiveController::Ds4(
                        DualShock4Controller {
                            association: association(inner.association(), inner.surface().common()),
                            inner,
                            audio: None,
                            identity: None,
                        },
                        ds4_cases(true).1,
                    ),
                    record,
                    6,
                )
            }
            3 => {
                let (bridge, record) = live_bridge(control);
                let inner = gr_curated_controllers::create_xbox360_usb_worker(bridge);
                (
                    LiveController::Xbox(Xbox360Controller {
                        association: association(inner.association(), inner.surface().common()),
                        inner,
                        audio: None,
                    }),
                    record,
                    6,
                )
            }
            _ => unreachable!(),
        }
    }

    #[test]
    #[ignore = "requires the bounded USB lab and an ordinary-user observer; no kernel input injection"]
    fn live_kernel_worker_outputs_reach_root_callbacks() {
        use std::io::{Read, Write};
        use std::os::linux::net::SocketAddrExt;
        use std::os::unix::net::SocketAddr;
        let instance =
            std::env::var("VIRTUALGAMEPAD_TYPED_OUTPUT_INSTANCE").expect("explicit lab instance");
        assert!(
            instance.starts_with("virtualgamepad-alpha-")
                && instance.len() <= 32
                && instance
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        );
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        assert!(
            status
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .unwrap()
                .split_whitespace()
                .skip(1)
                .all(|uid| uid != "0")
        );
        let address = SocketAddr::from_abstract_name(format!("vga-{instance}")).unwrap();
        let mut peer = UnixStream::connect_addr(&address).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        peer.set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        for family in 1..=3 {
            let mut metadata = [0; 9];
            peer.read_exact(&mut metadata).unwrap();
            assert_eq!(metadata[8], family);
            let generation = u64::from_le_bytes(metadata[..8].try_into().unwrap());
            assert_ne!(generation, 0);
            let [control, playback, microphone] = observer_channels(&peer).unwrap();
            let control = Control::new(control, generation, family).unwrap();
            let (mut controller, record, count) = live_controller(control, family);
            peer.write_all(b"A").unwrap();
            for sequence in 0..count {
                let mut command = [0];
                peer.read_exact(&mut command).unwrap();
                assert_eq!(command, b"N"[..]);
                controller.observe(sequence);
                peer.write_all(b"O").unwrap();
            }
            peer.write_all(b"P").unwrap();
            // PCM checks run in the owner while this observer makes no IPC calls.
            let mut command = [0];
            peer.read_exact(&mut command).unwrap();
            assert_eq!(command, b"D"[..]);
            assert_eq!(
                record.lock().unwrap().retained.reverse_events_drained,
                if family == 3 {
                    0
                } else {
                    u64::try_from(count).unwrap()
                }
            );
            assert_eq!(record.lock().unwrap().dropped, 0);
            controller.close();
            assert_eq!(record.lock().unwrap().closes, 1);
            assert!(record.lock().unwrap().retained.last_error.is_none());
            drop((controller, record, playback, microphone));
            peer.write_all(b"X").unwrap();
            println!(
                "typed_output_observer family={family} generation={generation} requests={count} passed=true"
            );
        }
        if std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_FACTORY_REQUEST").as_deref() == Ok("1") {
            send_public_factory_image(&mut peer);
        }
    }

    fn send_public_factory_image(peer: &mut UnixStream) {
        use std::io::{Read, Write};
        use std::process::{Command, Stdio};
        peer.write_all(b"R").unwrap();
        let image = std::fs::File::open("/proc/self/exe").unwrap();
        let mut sender = Command::new("python3")
            .args(["-I", "-c", "import array,hashlib,os,socket; s=socket.socket(fileno=0); s.settimeout(3); f=os.fdopen(os.dup(1),'rb'); h=hashlib.file_digest(f,'sha256').digest(); assert s.sendmsg([h],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[1]))])==32"])
            .stdin(Stdio::from(std::os::fd::OwnedFd::from(peer.try_clone().unwrap())))
            .stdout(Stdio::from(image))
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = sender.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                sender.kill().unwrap();
                sender.wait().unwrap();
                panic!("ordinary image sender timed out");
            }
            thread::sleep(Duration::from_millis(10));
        };
        assert!(status.success());
        peer.set_read_timeout(Some(Duration::from_secs(50)))
            .unwrap();
        let mut result = [0];
        peer.read_exact(&mut result).unwrap();
        assert_eq!(result, [b'F']);
    }

    #[test]
    #[ignore = "requires the prepared ordinary client mount and explicit sample factory lab opt-in"]
    fn live_public_usb_sample_factory() {
        assert_eq!(
            std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_LAB").as_deref(),
            Ok("1")
        );
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        assert!(
            status
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .unwrap()
                .split_whitespace()
                .skip(1)
                .all(|uid| uid != "0")
        );
        let options = crate::CreationOptions::new(crate::RealizationId::LINUX_USBIP_USB_AUDIO)
            .with_audio(crate::AudioOptions::new(crate::AudioExposure::Emulated));
        macro_rules! exercise {
            ($factory:ident, $family:literal, $playback:literal, $microphone:literal) => {{
                let mut controller = crate::$factory(options).unwrap();
                controller.service(&mut |_| {}).unwrap();
                controller.neutralize().unwrap();
                controller.commit().unwrap();
                let audio = controller.audio().unwrap();
                assert_eq!(audio.endpoints().len(), 2);
                for endpoint in audio.endpoints() {
                    assert_eq!(endpoint.access(), crate::AudioAccess::Samples);
                    assert_eq!(endpoint.format().sample_rate_hz(), 48_000);
                    assert!(endpoint.host().alsa_pcm().is_some());
                    assert!(endpoint.caller().is_none());
                }
                let mut invalid = [0; $playback - 1];
                assert_eq!(
                    audio.read_playback(&mut invalid),
                    Err(crate::AudioError::InvalidSampleBuffer)
                );
                if $microphone > 1 {
                    assert_eq!(
                        audio.write_microphone(&[0]),
                        Err(crate::AudioError::InvalidSampleBuffer)
                    );
                }
                let mut playback = [0; $playback * 16];
                assert_eq!(audio.read_playback(&mut playback).unwrap().frames, 0);
                assert_eq!(audio.write_microphone(&[0; $microphone * 16]).unwrap(), 16);
                audio.flush_playback().unwrap();
                audio.flush_microphone().unwrap();
                assert!(audio.last_error().is_none());
                controller.close();
                controller.close();
                let audio = controller.audio().unwrap();
                assert!(audio.is_closed());
                assert!(audio.last_error().is_none());
                assert_eq!(
                    audio.read_playback(&mut playback),
                    Err(crate::AudioError::Closed)
                );
                assert_eq!(
                    audio.write_microphone(&[0; $microphone]),
                    Err(crate::AudioError::Closed)
                );
                assert!(controller.diagnostics().last_error().is_none());
                println!("public_usb_sample_factory family={} passed=true", $family);
            }};
        }
        exercise!(create_dualsense, "dualsense", 4, 2);
        exercise!(create_dualshock4, "dualshock4", 2, 1);
        exercise!(create_xbox360, "xbox360", 2, 1);
    }
}
