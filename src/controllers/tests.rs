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
                ($controller::from_parts(association, inner, $($value,)*), record)
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
            ($controller::from_parts(association, inner, $($value,)* Some(audio)), record, audio_record)
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
    let mut switch = SwitchProController::from_parts(association, inner);
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
        let mut controller = DualSenseController::from_parts(
            association(inner.association(), inner.surface().common()),
            inner,
            None,
            None,
        );
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
        let mut controller = DualShock4Controller::from_parts(
            association(inner.association(), inner.surface().common()),
            inner,
            None,
            None,
        );
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
            let mut controller = Xbox360Controller::from_parts(
                association(inner.association(), inner.surface().common()),
                inner,
                None,
            );
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
                        DualSenseController::from_parts(
                            association(inner.association(), inner.surface().common()),
                            inner,
                            None,
                            None,
                        ),
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
                        DualShock4Controller::from_parts(
                            association(inner.association(), inner.surface().common()),
                            inner,
                            None,
                            None,
                        ),
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
                    LiveController::Xbox(Xbox360Controller::from_parts(
                        association(inner.association(), inner.surface().common()),
                        inner,
                        None,
                    )),
                    record,
                    6,
                )
            }
            _ => unreachable!(),
        }
    }

    fn selected_probe_request(
        selector: Option<&str>,
        image: Option<&std::ffi::OsStr>,
    ) -> Result<Option<u8>, &'static str> {
        let request = match selector {
            None => return Ok(None),
            Some("1") => b'R',
            Some("native-timer") => b'T',
            Some("native-timer-prebuilt") => b'N',
            Some("direct-rust") => b'D',
            Some("direct-c") => b'C',
            Some(_) => return Err("unknown public probe selector"),
        };
        if matches!(request, b'N' | b'D' | b'C') && image.is_none_or(std::ffi::OsStr::is_empty) {
            return Err("selected probe requires an explicit image");
        }
        Ok(Some(request))
    }

    #[test]
    fn public_probe_selection_rejects_unknown_modes_and_missing_images() {
        let image = Some(std::ffi::OsStr::new("synthetic-prebuilt-image"));
        assert_eq!(selected_probe_request(None, None), Ok(None));
        for (selector, request) in [
            ("1", b'R'),
            ("native-timer", b'T'),
            ("native-timer-prebuilt", b'N'),
            ("direct-rust", b'D'),
            ("direct-c", b'C'),
        ] {
            assert_eq!(
                selected_probe_request(Some(selector), image),
                Ok(Some(request))
            );
            if matches!(request, b'N' | b'D' | b'C') {
                assert!(selected_probe_request(Some(selector), None).is_err());
                assert!(
                    selected_probe_request(Some(selector), Some(std::ffi::OsStr::new(""))).is_err()
                );
            }
        }
        for invalid in [
            "",
            "0",
            "native-timer-prebuild",
            "direct-c ",
            "shell",
            "N",
            "1\n",
        ] {
            assert!(selected_probe_request(Some(invalid), image).is_err());
        }
    }

    #[test]
    #[ignore = "requires the bounded USB lab and an ordinary-user observer; no kernel input injection"]
    fn live_kernel_worker_outputs_reach_root_callbacks() {
        use std::io::{Read, Write};
        use std::os::linux::net::SocketAddrExt;
        use std::os::unix::net::SocketAddr;
        // Reject a requested but unrecognized scenario before connecting or creating resources.
        let selector = match std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_FACTORY_REQUEST") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(error) => panic!("invalid public probe selector: {error}"),
        };
        let image = std::env::var_os("VIRTUALGAMEPAD_INDEPENDENT_CONTROL_IMAGE");
        let request = selected_probe_request(selector.as_deref(), image.as_deref())
            .expect("valid explicit public probe selection");
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
        match request {
            Some(b'T') => send_native_timer_request(&mut peer),
            Some(b'R') => send_public_factory_image(&mut peer),
            Some(mode @ (b'N' | b'D' | b'C')) => {
                send_probe_image(
                    &mut peer,
                    mode,
                    std::path::Path::new(image.as_ref().unwrap()),
                );
            }
            None => {}
            Some(_) => unreachable!("validated probe request"),
        }
        if let Some(mode) = request {
            println!(
                "public_probe_completed request={} passed=true",
                char::from(mode)
            );
        } else {
            println!("public_probe_completed request=output-only passed=true");
        }
    }

    fn send_native_timer_request(peer: &mut UnixStream) {
        use std::io::{Read, Write};
        peer.write_all(b"T").unwrap();
        peer.set_read_timeout(Some(std::time::Duration::from_secs(40)))
            .unwrap();
        let mut status = [0];
        peer.read_exact(&mut status).unwrap();
        assert_eq!(
            status, *b"F",
            "timer execution failed; not audio qualification"
        );
    }

    #[test]
    fn native_timer_request_requires_exact_terminal_reply() {
        use std::io::{Read, Write};
        for reply in [b'F', b'E'] {
            let (mut client, mut server) = UnixStream::pair().unwrap();
            let worker = std::thread::spawn(move || {
                let mut command = [0];
                server.read_exact(&mut command).unwrap();
                assert_eq!(command, *b"T");
                server.write_all(&[reply]).unwrap();
            });
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                send_native_timer_request(&mut client);
            }));
            worker.join().unwrap();
            assert_eq!(result.is_ok(), reply == b'F');
        }
    }

    fn send_public_factory_image(peer: &mut UnixStream) {
        send_probe_image(peer, b'R', std::path::Path::new("/proc/self/exe"));
    }

    fn send_probe_image(peer: &mut UnixStream, request: u8, path: &std::path::Path) {
        use std::io::{Read, Write};
        use std::process::{Command, Stdio};
        assert!(matches!(request, b'R' | b'D' | b'C' | b'N'));
        peer.write_all(&[request]).unwrap();
        let image = std::fs::File::open(path).unwrap();
        let mut sender = Command::new("python3")
            .args(["-I", "-c", r"import array,fcntl,hashlib,os,socket,struct,sys
s=socket.socket(fileno=0)
f=os.fdopen(os.dup(1),'rb'); data=f.read(128*1024*1024+1)
assert 4<=len(data)<=128*1024*1024
image=os.memfd_create('virtualgamepad-public-probe',os.MFD_CLOEXEC|os.MFD_ALLOW_SEALING)
out=os.fdopen(os.dup(image),'wb'); out.write(data); out.flush(); os.fchmod(image,0o500)
fcntl.fcntl(image,fcntl.F_ADD_SEALS,fcntl.F_SEAL_WRITE|fcntl.F_SEAL_GROW|fcntl.F_SEAL_SHRINK|fcntl.F_SEAL_SEAL)
assert s.sendmsg([hashlib.sha256(data).digest()],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[image]))])==32
if sys.argv[1]=='C':
    def exact(size):
        parts=bytearray()
        while len(parts)<size:
            chunk=s.recv(size-len(parts))
            if not chunk:raise ValueError('truncated C ledger transport')
            parts.extend(chunk)
        return bytes(parts)
    s.settimeout(425)
    try:
        for trial in range(1,4):
            assert exact(1)==b'G'
            size=struct.unpack('!I',exact(4))[0]; assert 0<size<=1024*1024
            digest=exact(32); payload=exact(size)
            assert hashlib.sha256(payload).digest()==digest
            name=os.path.join(sys.argv[2],f'c-control.trial-{trial}.ledger.jsonl.gz')
            descriptor=os.open(name,os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_WRONLY,0o600)
            with os.fdopen(descriptor,'wb') as artifact:artifact.write(payload)
    finally:s.setblocking(True)
"])
            .arg(char::from(request).to_string())
            .arg(path.parent().expect("probe image parent"))
            .stdin(Stdio::from(std::os::fd::OwnedFd::from(peer.try_clone().unwrap())))
            .stdout(Stdio::from(image))
            .spawn()
            .unwrap();
        let deadline =
            std::time::Instant::now() + Duration::from_secs(if request == b'C' { 425 } else { 5 });
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
        peer.set_read_timeout(Some(Duration::from_secs(if request == b'C' {
            5
        } else if request == b'N' {
            25
        } else if request == b'D' {
            425
        } else {
            365
        })))
        .unwrap();
        let mut result = [0];
        peer.read_exact(&mut result).unwrap();
        assert_eq!(result, [b'F']);
    }

    #[test]
    fn prebuilt_native_timer_sender_requires_terminal_reply_and_sealed_image() {
        use std::os::fd::OwnedFd;
        use std::process::{Command, Stdio};
        for reply in ["F", "E"] {
            let (mut sender, receiver) = UnixStream::pair().unwrap();
            let mut child = Command::new("python3")
                .args([
                    "-I",
                    "-c",
                    r"import array,fcntl,hashlib,os,socket,sys
s=socket.socket(fileno=0); assert s.recv(1)==b'N'
data,ancillary,flags,_=s.recvmsg(32,socket.CMSG_SPACE(4),socket.MSG_CMSG_CLOEXEC)
assert len(data)==32 and len(ancillary)==1 and not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC)
fds=array.array('i'); fds.frombytes(ancillary[0][2]); assert len(fds)==1
required=fcntl.F_SEAL_WRITE|fcntl.F_SEAL_GROW|fcntl.F_SEAL_SHRINK|fcntl.F_SEAL_SEAL
assert fcntl.fcntl(fds[0],fcntl.F_GET_SEALS)&required==required
os.lseek(fds[0],0,os.SEEK_SET)
with os.fdopen(fds[0],'rb') as image: assert hashlib.sha256(image.read()).digest()==data
s.sendall(sys.argv[1].encode())
",
                ])
                .arg(reply)
                .stdin(Stdio::from(OwnedFd::from(receiver)))
                .spawn()
                .unwrap();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                send_probe_image(&mut sender, b'N', std::path::Path::new("/proc/self/exe"));
            }));
            assert_eq!(result.is_ok(), reply == "F");
            assert!(child.wait().unwrap().success());
            assert_eq!(
                sender.read_timeout().unwrap(),
                Some(Duration::from_secs(25))
            );
        }
    }

    #[test]
    fn ordinary_image_sender_preserves_peer_blocking_and_reaps_receiver() {
        use std::os::fd::{AsRawFd, OwnedFd};
        use std::process::{Command, Stdio};
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        let descriptor = sender.as_raw_fd();
        let flags = || {
            std::fs::read_to_string(format!("/proc/self/fdinfo/{descriptor}"))
                .unwrap()
                .lines()
                .find(|line| line.starts_with("flags:"))
                .unwrap()
                .to_owned()
        };
        let before = flags();
        let mut child = Command::new("python3")
            .args(["-I", "-c", "import array,os,socket; s=socket.socket(fileno=0); assert s.recv(1)==b'R'; data,ancillary,flags,_=s.recvmsg(32,socket.CMSG_SPACE(4),socket.MSG_CMSG_CLOEXEC); assert len(data)==32 and not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC) and len(ancillary)==1; fds=array.array('i'); fds.frombytes(ancillary[0][2]); assert len(fds)==1; os.close(fds[0]); s.sendall(b'F')"])
            .stdin(Stdio::from(OwnedFd::from(receiver)))
            .spawn()
            .unwrap();
        send_public_factory_image(&mut sender);
        assert!(child.wait().unwrap().success());
        assert_eq!(flags(), before);
    }

    #[test]
    fn independent_image_sender_uses_distinct_request_and_the_same_sealed_fd_contract() {
        use std::os::fd::OwnedFd;
        use std::process::{Command, Stdio};
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        let mut child = Command::new("python3")
            .args(["-I", "-c", "import array,fcntl,os,socket; s=socket.socket(fileno=0); assert s.recv(1)==b'D'; data,ancillary,flags,_=s.recvmsg(32,socket.CMSG_SPACE(4),socket.MSG_CMSG_CLOEXEC); assert len(data)==32 and not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC) and len(ancillary)==1; fds=array.array('i'); fds.frombytes(ancillary[0][2]); assert len(fds)==1; expected=fcntl.F_SEAL_WRITE|fcntl.F_SEAL_GROW|fcntl.F_SEAL_SHRINK|fcntl.F_SEAL_SEAL; assert fcntl.fcntl(fds[0],fcntl.F_GET_SEALS)&expected==expected; os.close(fds[0]); s.sendall(b'F')"])
            .stdin(Stdio::from(OwnedFd::from(receiver)))
            .spawn()
            .unwrap();
        send_probe_image(&mut sender, b'D', std::path::Path::new("/proc/self/exe"));
        assert!(child.wait().unwrap().success());
    }

    #[test]
    fn c_image_sender_preserves_three_bounded_ledgers_and_peer_blocking() {
        use std::os::fd::{AsRawFd, OwnedFd};
        use std::process::{Command, Stdio};
        struct Artifacts(std::path::PathBuf);
        impl Drop for Artifacts {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(self.0.join("control"));
                for trial in 1..=3 {
                    let _ = std::fs::remove_file(
                        self.0
                            .join(format!("c-control.trial-{trial}.ledger.jsonl.gz")),
                    );
                }
                let _ = std::fs::remove_dir(&self.0);
            }
        }
        let name = format!(
            "virtualgamepad-c-sender-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let artifacts = Artifacts(std::env::temp_dir().join(name));
        std::fs::create_dir(&artifacts.0).unwrap();
        let image = artifacts.0.join("control");
        // Transport-only synthetic image; execution acceptance is separate.
        std::fs::write(&image, b"\x7fELFsynthetic-control-transport").unwrap();
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        let descriptor = sender.as_raw_fd();
        let flags = || {
            std::fs::read_to_string(format!("/proc/self/fdinfo/{descriptor}"))
                .unwrap()
                .lines()
                .find(|line| line.starts_with("flags:"))
                .unwrap()
                .to_owned()
        };
        let before = flags();
        let mut child = Command::new("python3")
            .args([
                "-I",
                "-c",
                r"import array,fcntl,gzip,hashlib,os,socket,struct
s=socket.socket(fileno=0); assert s.recv(1)==b'C'
data,ancillary,flags,_=s.recvmsg(32,socket.CMSG_SPACE(4),socket.MSG_CMSG_CLOEXEC)
assert len(data)==32 and not flags&(socket.MSG_TRUNC|socket.MSG_CTRUNC) and len(ancillary)==1
fds=array.array('i'); fds.frombytes(ancillary[0][2]); assert len(fds)==1
expected=fcntl.F_SEAL_WRITE|fcntl.F_SEAL_GROW|fcntl.F_SEAL_SHRINK|fcntl.F_SEAL_SEAL
assert fcntl.fcntl(fds[0],fcntl.F_GET_SEALS)&expected==expected; os.close(fds[0])
for trial in range(1,4):
    payload=gzip.compress(f'synthetic-ledger-{trial}\n'.encode(),mtime=0)
    s.sendall(b'G'+struct.pack('!I',len(payload))+hashlib.sha256(payload).digest()+payload)
s.sendall(b'F')
",
            ])
            .stdin(Stdio::from(OwnedFd::from(receiver)))
            .spawn()
            .unwrap();
        send_probe_image(&mut sender, b'C', &image);
        assert!(child.wait().unwrap().success());
        assert_eq!(flags(), before);
        for trial in 1..=3 {
            let data = std::fs::read(
                artifacts
                    .0
                    .join(format!("c-control.trial-{trial}.ledger.jsonl.gz")),
            )
            .unwrap();
            assert!(data.starts_with(&[0x1f, 0x8b]));
            assert!(data.len() < 1024 * 1024);
        }
    }

    #[test]
    fn c_image_sender_rejects_damaged_ledgers_and_preserves_existing_evidence() {
        use std::os::fd::OwnedFd;
        use std::process::{Command, Stdio};
        struct Artifacts(std::path::PathBuf);
        impl Drop for Artifacts {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(self.0.join("control"));
                let _ = std::fs::remove_file(self.0.join("c-control.trial-1.ledger.jsonl.gz"));
                let _ = std::fs::remove_dir(&self.0);
            }
        }
        for mode in ["oversized", "truncated", "digest", "existing"] {
            let name = format!(
                "virtualgamepad-c-reject-{}-{}-{}",
                std::process::id(),
                mode,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let artifacts = Artifacts(std::env::temp_dir().join(name));
            std::fs::create_dir(&artifacts.0).unwrap();
            let image = artifacts.0.join("control");
            std::fs::write(&image, b"\x7fELFsynthetic-control-transport").unwrap();
            let first = artifacts.0.join("c-control.trial-1.ledger.jsonl.gz");
            if mode == "existing" {
                std::fs::write(&first, b"retained synthetic failure").unwrap();
            }
            let (mut sender, receiver) = UnixStream::pair().unwrap();
            let mut child = Command::new("python3")
                .args([
                    "-I",
                    "-c",
                    r"import array,hashlib,os,socket,struct,sys
s=socket.socket(fileno=0); assert s.recv(1)==b'C'
data,ancillary,flags,_=s.recvmsg(32,socket.CMSG_SPACE(4),socket.MSG_CMSG_CLOEXEC)
assert len(data)==32 and len(ancillary)==1
fds=array.array('i'); fds.frombytes(ancillary[0][2]); assert len(fds)==1; os.close(fds[0])
payload=b'synthetic'; digest=hashlib.sha256(payload).digest(); size=len(payload)
if sys.argv[1]=='oversized':size=1024*1024+1; payload=b''
if sys.argv[1]=='truncated':payload=payload[:1]
if sys.argv[1]=='digest':digest=bytes(32)
s.sendall(b'G'+struct.pack('!I',size)+digest+payload)
",
                ])
                .arg(mode)
                .stdin(Stdio::from(OwnedFd::from(receiver)))
                .spawn()
                .unwrap();
            let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                send_probe_image(&mut sender, b'C', &image);
            }));
            assert!(rejected.is_err(), "damaged ledger must reject: {mode}");
            assert!(child.wait().unwrap().success());
            if mode == "existing" {
                assert_eq!(
                    std::fs::read(&first).unwrap(),
                    b"retained synthetic failure"
                );
            } else {
                assert!(!first.exists());
            }
        }
    }

    fn public_factory_host_snapshot(stage: &str) {
        eprintln!("public_usb_factory_host_stage={stage}");
        for path in [
            "/proc/asound/cards",
            "/sys/devices/platform/vhci_hcd.0/status",
        ] {
            match std::fs::read_to_string(path) {
                Ok(snapshot) => eprintln!(
                    "public_usb_factory_host_state {path}={:?}",
                    snapshot.chars().take(4096).collect::<String>()
                ),
                Err(read_error) => {
                    eprintln!("public_usb_factory_host_state {path} unavailable={read_error}");
                }
            }
        }
        for entry in std::fs::read_dir("/sys/bus/usb/devices").unwrap() {
            let entry = entry.unwrap();
            if let Ok(serial) = std::fs::read_to_string(entry.path().join("serial")) {
                if serial.starts_with("vg-virtualgamepad-alpha-") {
                    eprintln!(
                        "public_usb_factory_owned_usb_node={} serial={}",
                        entry.file_name().to_string_lossy(),
                        serial.trim()
                    );
                }
            }
        }
    }

    fn report_public_factory_failure(family: &str, error: &crate::ControllerError) -> ! {
        public_factory_host_snapshot("factory failure");
        panic!("public USB factory {family}: {error}");
    }

    fn assert_ordinary_test_process() {
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
    }

    #[allow(clippy::too_many_lines)] // Keep caller ownership, graph identity and consumer lifecycle checks together.
    fn check_public_audio_ownership(
        audio: &mut crate::ControllerAudio,
        playback_channels: usize,
        microphone_channels: usize,
        playback_access: crate::AudioAccess,
        microphone_access: crate::AudioAccess,
    ) {
        assert_eq!(audio.endpoints().len(), 2);
        for endpoint in audio.endpoints() {
            let (access, channels) = match endpoint.direction() {
                crate::SampleDirection::HostToController => (playback_access, playback_channels),
                crate::SampleDirection::ControllerToHost => {
                    (microphone_access, microphone_channels)
                }
                direction => panic!("unexpected compiled endpoint direction: {direction:?}"),
            };
            assert_eq!(endpoint.access(), access);
            assert_eq!(endpoint.format().sample_rate_hz(), 48_000);
            assert_eq!(endpoint.format().channels().len(), channels);
            assert!(endpoint.host().alsa_pcm().is_some());
            if access == crate::AudioAccess::NativeClient {
                let node = endpoint
                    .caller()
                    .and_then(crate::AudioEndpointSelector::pipewire_node)
                    .expect("native caller must expose a graph node");
                // Observe the actual caller node while its controller is alive.
                // The prepared client owns a private graph, not the desktop graph.
                let observed = std::process::Command::new("/usr/bin/python3")
                    .args([
                        "-I",
                        "-c",
                        r"import json,os,subprocess,sys,tempfile
assert os.getuid()!=0
assert os.environ.get('PIPEWIRE_RUNTIME_DIR')==os.environ.get('XDG_RUNTIME_DIR')
with tempfile.TemporaryFile() as output:
 subprocess.run(['pw-dump'],check=True,timeout=3,stdout=output,stderr=subprocess.DEVNULL)
 output.seek(0); raw=output.read(1048577)
assert len(raw)<=1048576
objects=json.loads(raw)
assert isinstance(objects,list)
props=[item.get('info',{}).get('props',{}) for item in objects]
assert not any(item.get('device.api')=='alsa' for item in props)
assert sum(item.get('node.name')==sys.argv[1] for item in props)==1
",
                        node,
                    ])
                    .status()
                    .expect("private graph observer must start");
                assert!(
                    observed.success(),
                    "native endpoint missing from private graph: {node}"
                );
                if std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_MARKER_LAB").as_deref()
                    == Ok("1")
                {
                    let mode = if endpoint.direction() == crate::SampleDirection::HostToController {
                        "record"
                    } else {
                        "playback"
                    };
                    let lifecycle = std::process::Command::new("/usr/bin/python3")
                        .args([
                            "-I",
                            "-c",
                            include_str!("../../scripts/validate-usb-native-client.py"),
                            "--target",
                            node,
                            "--channels",
                            &channels.to_string(),
                            "--mode",
                            mode,
                        ])
                        .status()
                        .expect("owned native client helper must start");
                    assert!(
                        lifecycle.success(),
                        "native caller reconnect failed: {node}"
                    );
                    println!("public_usb_native_client_lifecycle_verified={node}");
                }
                println!("public_usb_native_node_verified={node}");
            } else {
                assert!(endpoint.caller().is_none());
            }
        }
        if playback_access == crate::AudioAccess::Samples {
            assert_eq!(
                audio.read_playback(&mut vec![0; playback_channels - 1]),
                Err(crate::AudioError::InvalidSampleBuffer)
            );
            assert_eq!(
                audio
                    .read_playback(&mut vec![0; playback_channels * 16])
                    .unwrap()
                    .frames,
                0
            );
            audio.flush_playback().unwrap();
        } else {
            assert_eq!(
                audio.read_playback(&mut vec![0; playback_channels]),
                Err(crate::AudioError::OwnershipMismatch)
            );
            assert_eq!(
                audio.flush_playback(),
                Err(crate::AudioError::OwnershipMismatch)
            );
        }
        if microphone_access == crate::AudioAccess::Samples {
            if microphone_channels > 1 {
                assert_eq!(
                    audio.write_microphone(&[0]),
                    Err(crate::AudioError::InvalidSampleBuffer)
                );
            }
            assert_eq!(
                audio
                    .write_microphone(&vec![0; microphone_channels * 16])
                    .unwrap(),
                16
            );
            audio.flush_microphone().unwrap();
        } else {
            assert_eq!(
                audio.write_microphone(&vec![0; microphone_channels]),
                Err(crate::AudioError::OwnershipMismatch)
            );
            assert_eq!(
                audio.flush_microphone(),
                Err(crate::AudioError::OwnershipMismatch)
            );
        }
        assert!(audio.last_error().is_none());
    }

    trait PublicSampleController {
        fn service_input(&mut self);
        fn sample_audio(&mut self) -> &mut crate::ControllerAudio;
    }

    macro_rules! public_sample_controller {
        ($kind:ty) => {
            impl PublicSampleController for $kind {
                fn service_input(&mut self) {
                    self.service(&mut |_| {}).unwrap();
                }
                fn sample_audio(&mut self) -> &mut crate::ControllerAudio {
                    self.audio().unwrap()
                }
            }
        };
    }
    public_sample_controller!(DualSenseController);
    public_sample_controller!(DualShock4Controller);
    public_sample_controller!(Xbox360Controller);

    fn public_marker(frame: usize, channels: usize) -> Vec<i16> {
        let index = if channels == 1 { frame / 2 } else { frame };
        let high = i16::try_from(index / 32_767 + 1).unwrap();
        let low = i16::try_from(index % 32_767 + 1).unwrap();
        match channels {
            1 => vec![if frame % 2 == 0 { high } else { -low }],
            2 => vec![high, low],
            4 => vec![high, low, -high, -low],
            _ => panic!("unsupported compiled marker layout"),
        }
    }

    fn public_marker_index(frame: &[i16]) -> Option<usize> {
        if !matches!(frame.len(), 2 | 4) || frame[0] <= 0 || frame[1] <= 0 {
            return None;
        }
        if frame.len() == 4 && (frame[2] != -frame[0] || frame[3] != -frame[1]) {
            return None;
        }
        Some(
            usize::try_from(frame[0] - 1).unwrap() * 32_767
                + usize::try_from(frame[1] - 1).unwrap(),
        )
    }

    #[test]
    fn public_usb_markers_retain_channel_integrity_and_distinct_frame_indices() {
        for channels in [2, 4] {
            for index in [0, 1, 32_766, 32_767, 96_000, 240_000] {
                assert_eq!(
                    public_marker_index(&public_marker(index, channels)),
                    Some(index)
                );
            }
        }
        assert_eq!(public_marker(0, 1), [1]);
        assert_eq!(public_marker(1, 1), [-1]);
        assert_eq!(public_marker(65_534, 1), [2]);
        assert_eq!(public_marker_index(&[0, 1]), None);
        assert_eq!(public_marker_index(&[1, 1, -2, -1]), None);
    }

    fn record_public_playback(seen: &mut [u8], frame: &[i16], begin: usize) -> bool {
        if let Some(index) = public_marker_index(frame) {
            if let Some(offset) = index.checked_sub(begin) {
                if let Some(count) = seen.get_mut(offset) {
                    *count = count.saturating_add(1);
                }
            }
            true
        } else {
            frame.iter().all(|sample| *sample == 0)
        }
    }

    #[test]
    fn public_usb_accounting_rejects_loss_balanced_by_replay_and_corruption() {
        let mut seen = [0; 5];
        for index in [96_000, 96_001, 96_001, 96_003, 96_004] {
            assert!(record_public_playback(
                &mut seen,
                &public_marker(index, 4),
                96_000
            ));
        }
        assert_eq!(seen, [1, 2, 0, 1, 1]);
        assert!(record_public_playback(&mut seen, &[0; 4], 96_000));
        assert!(!record_public_playback(&mut seen, &[1, 1, -2, -1], 96_000));
        assert!(record_public_playback(
            &mut seen,
            &public_marker(95_999, 4),
            96_000
        ));
        assert_eq!(seen, [1, 2, 0, 1, 1]);
    }

    fn public_microphone_credit(host: usize, silence: usize, produced: usize) -> usize {
        let consumed = host
            .checked_sub(silence)
            .expect("silence cannot exceed host frames");
        consumed
            .saturating_add(384)
            .saturating_sub(produced)
            .min(384)
    }

    #[test]
    fn public_usb_refill_catches_up_within_existing_fill_without_counting_silence() {
        assert_eq!(public_microphone_credit(0, 0, 0), 384);
        // A four-millisecond wake-up consumes 192 frames, exceeding a single
        // wire packet. Refill the operating fill rather than only one packet.
        assert_eq!(public_microphone_credit(192, 0, 384), 192);
        assert_eq!(public_microphone_credit(480, 96, 384), 384);
        assert_eq!(public_microphone_credit(480, 96, 768), 0);
        assert_eq!(public_microphone_credit(480, 0, 768), 96);
        assert_eq!(public_microphone_credit(288_000, 89_680, 198_320), 384);
    }

    fn public_sample_microphone_counter(
        native: bool,
        read: impl FnOnce() -> Result<Option<u64>, crate::AudioError>,
    ) -> Result<Option<u64>, crate::AudioError> {
        if native {
            // Sample counters cannot observe a native-owned microphone.
            Ok(None)
        } else {
            read()
        }
    }

    #[test]
    fn public_usb_native_accounting_never_reads_sample_only_counters() {
        assert_eq!(
            public_sample_microphone_counter(true, || panic!("sample counter on native endpoint")),
            Ok(None)
        );
        assert_eq!(
            public_sample_microphone_counter(false, || Ok(Some(96))),
            Ok(Some(96))
        );
        assert_eq!(
            public_sample_microphone_counter(false, || Err(crate::AudioError::OwnershipMismatch)),
            Err(crate::AudioError::OwnershipMismatch)
        );
    }

    fn public_counter_delta(before: Option<u64>, after: Option<u64>) -> Option<u64> {
        after?.checked_sub(before?)
    }

    #[test]
    fn public_usb_counter_delta_preserves_unavailable_and_rejects_regression() {
        assert_eq!(public_counter_delta(Some(1024), Some(1024)), Some(0));
        assert_eq!(public_counter_delta(Some(1024), Some(1104)), Some(80));
        assert_eq!(public_counter_delta(None, Some(80)), None);
        assert_eq!(public_counter_delta(Some(80), None), None);
        assert_eq!(public_counter_delta(Some(80), Some(79)), None);
    }

    fn spawn_public_host(mut command: std::process::Command) -> std::process::Child {
        // Consume the configuration so its parent copy of child stdout closes
        // immediately after spawning. Retaining it prevents socket EOF.
        command
            .spawn()
            .expect("owned public sample helper must start")
    }

    #[test]
    fn public_usb_host_receipt_reaches_eof_after_child_exit() {
        use std::io::Read;
        use std::os::fd::OwnedFd;
        use std::process::{Command, Stdio};
        let (mut output, writer) = UnixStream::pair().unwrap();
        output
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut command = Command::new("python3");
        command
            .args(["-I", "-c", "print('R',end='',flush=True)"])
            .stdin(Stdio::null())
            .stdout(Stdio::from(OwnedFd::from(writer)));
        let mut child = spawn_public_host(command);
        assert!(child.wait().unwrap().success());
        let mut receipt = Vec::new();
        output.read_to_end(&mut receipt).unwrap();
        assert_eq!(receipt, b"R");
    }

    fn public_exchange_passed(
        host_succeeded: bool,
        marker_errors: (usize, usize, usize),
        microphone_produced: usize,
        native_microphone: bool,
        backend_healthy: bool,
    ) -> bool {
        host_succeeded
            && marker_errors == (0, 0, 0)
            && (native_microphone || microphone_produced >= 240_000)
            && backend_healthy
    }

    fn public_native_playback_passed(
        host_markers_passed: bool,
        microphone_produced: usize,
        native_microphone: bool,
        backend_healthy: bool,
    ) -> bool {
        // Host status includes both native playback receipt and ALSA microphone
        // receipt. Never replace missing native marker evidence with zero errors.
        host_markers_passed
            && (native_microphone || microphone_produced >= 240_000)
            && backend_healthy
    }

    #[test]
    fn native_playback_requires_both_host_markers_source_and_backend_health() {
        assert!(public_native_playback_passed(true, 240_000, false, true));
        assert!(public_native_playback_passed(true, 0, true, true));
        assert!(!public_native_playback_passed(false, 240_000, false, true));
        assert!(!public_native_playback_passed(false, 0, true, true));
        assert!(!public_native_playback_passed(true, 239_999, false, true));
        assert!(!public_native_playback_passed(true, 240_000, false, false));
    }

    #[test]
    fn public_usb_exchange_collection_retains_every_acceptance_failure() {
        assert!(public_exchange_passed(
            true,
            (0, 0, 0),
            240_000,
            false,
            true
        ));
        assert!(public_exchange_passed(true, (0, 0, 0), 0, true, true));
        assert!(!public_exchange_passed(
            false,
            (0, 0, 0),
            240_000,
            false,
            true
        ));
        for errors in [(1, 0, 0), (0, 1, 0), (0, 0, 1), (1, 1, 0)] {
            assert!(!public_exchange_passed(true, errors, 240_000, false, true));
        }
        assert!(!public_exchange_passed(
            true,
            (0, 0, 0),
            239_999,
            false,
            true
        ));
        assert!(!public_exchange_passed(
            true,
            (0, 0, 0),
            240_000,
            false,
            false
        ));
    }

    #[allow(clippy::too_many_lines)] // Retain the owned ALSA child and controller through bounded drain.
    fn public_sample_exchange(
        controller: &mut impl PublicSampleController,
        family: &str,
        playback_channels: usize,
        microphone_channels: usize,
    ) -> bool {
        use std::io::Read;
        use std::os::fd::OwnedFd;
        use std::os::unix::process::CommandExt;
        use std::process::{Child, Command, Stdio};
        struct OwnedHost {
            child: Child,
            reaped: bool,
        }
        impl Drop for OwnedHost {
            fn drop(&mut self) {
                if !self.reaped {
                    // The unreaped group leader reserves this process identity.
                    // ALSA grandchildren inherit the dedicated process group.
                    let _ = Command::new("/usr/bin/kill")
                        .args(["-KILL", "--", &format!("-{}", self.child.id())])
                        .status();
                    let _ = self.child.kill();
                    self.child.wait().expect("owned ALSA helper must be reaped");
                }
            }
        }
        let audio = controller.sample_audio();
        let card = audio.endpoints()[0].host().alsa_pcm().unwrap().0.to_owned();
        let instance = std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_MARKER_INSTANCE").unwrap();
        // Bridge metrics are cached at 100 ms intervals. Record the pretrial
        // snapshot separately: prior lifecycle clients must not be mistaken
        // for marker-trial graph loss.
        #[cfg(feature = "experimental")]
        let graph_dropped_before =
            crate::experimental::audio_instrumentation(audio).native_microphone_dropped_frames();
        let native_playback = audio.endpoints()[0]
            .caller()
            .and_then(crate::AudioEndpointSelector::pipewire_node)
            .map(str::to_owned);
        let native_microphone = audio.endpoints()[1]
            .caller()
            .and_then(crate::AudioEndpointSelector::pipewire_node)
            .map(str::to_owned);
        let script = concat!(
            "__name__='captured_validator'\n",
            include_str!("../../scripts/validate-usb-audio-live.py"),
            "\n__name__='__main__'\n",
            include_str!("../../scripts/validate-usb-public-samples.py")
        );
        let (mut output, child_output) = UnixStream::pair().unwrap();
        output
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut command = Command::new("python3");
        command.args([
            "-I",
            "-c",
            script,
            "--family",
            family,
            "--card-id",
            &card,
            "--instance",
            &instance,
        ]);
        if let Some(node) = &native_playback {
            command.args(["--playback-node", node]);
        }
        if let Some(node) = &native_microphone {
            command.args(["--microphone-node", node]);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::from(OwnedFd::from(child_output)))
            .process_group(0);
        let child = spawn_public_host(command);
        let mut host = OwnedHost {
            child,
            reaped: false,
        };
        let mut ready = [0];
        output.read_exact(&mut ready).unwrap();
        assert_eq!(ready, [b'R']);
        let mut playback = vec![0; 256 * playback_channels];
        let mut seen = vec![0_u8; 144_000];
        let mut microphone_produced = 0;
        let mut invalid = 0;
        let deadline = std::time::Instant::now()
            + Duration::from_secs(if native_playback.is_some() { 24 } else { 14 });
        let mut status = None;
        let mut drain_until = None;
        let mut previous_cycle = std::time::Instant::now();
        let mut largest_delays = [[0_u64; 8]; 8];
        let mut maximum_service_us = 0;
        let mut maximum_read_us = 0;
        let mut previous_sleep_us = 0;
        let mut previous_write_us = 0;
        let timer_slack = std::fs::read_to_string("/proc/thread-self/timerslack_ns");
        let scheduling = std::fs::read_to_string("/proc/thread-self/sched")
            .map(|text| text.chars().take(2048).collect::<String>());
        println!(
            "public_usb_caller_scheduling family={family} timer_slack={timer_slack:?} sched={scheduling:?}"
        );
        loop {
            let cycle = std::time::Instant::now();
            let interval_us =
                u64::try_from(cycle.duration_since(previous_cycle).as_micros()).unwrap();
            previous_cycle = cycle;
            controller.service_input();
            let service_us = u64::try_from(cycle.elapsed().as_micros()).unwrap();
            maximum_service_us = maximum_service_us.max(service_us);
            let audio = controller.sample_audio();
            if native_playback.is_none() {
                let read_begin = std::time::Instant::now();
                let observation = audio.read_playback(&mut playback).unwrap();
                maximum_read_us =
                    maximum_read_us.max(u64::try_from(read_begin.elapsed().as_micros()).unwrap());
                for frame in playback[..observation.frames * playback_channels]
                    .chunks_exact(playback_channels)
                {
                    if !record_public_playback(&mut seen, frame, 96_000) {
                        invalid += 1;
                    }
                }
            }
            if status.is_none() && native_microphone.is_none() {
                let credit_begin = std::time::Instant::now();
                let consumed =
                    usize::try_from(audio.microphone_consumed_frames().unwrap().unwrap()).unwrap();
                let credit_us = u64::try_from(credit_begin.elapsed().as_micros()).unwrap();
                if interval_us > largest_delays[0][0] {
                    largest_delays[0] = [
                        interval_us,
                        service_us,
                        credit_us,
                        u64::try_from(consumed).unwrap(),
                        u64::try_from(microphone_produced).unwrap(),
                        audio.underrun_frames(),
                        previous_sleep_us,
                        previous_write_us,
                    ];
                    largest_delays.sort_unstable_by_key(|entry| entry[0]);
                }
                let available = public_microphone_credit(
                    consumed,
                    usize::try_from(audio.underrun_frames()).unwrap(),
                    microphone_produced,
                );
                if available > 0 {
                    let write_begin = std::time::Instant::now();
                    let samples: Vec<_> = (microphone_produced..microphone_produced + available)
                        .flat_map(|frame| public_marker(frame, microphone_channels))
                        .collect();
                    microphone_produced += audio.write_microphone(&samples).unwrap();
                    previous_write_us = u64::try_from(write_begin.elapsed().as_micros()).unwrap();
                } else {
                    previous_write_us = 0;
                }
            }
            if status.is_none() {
                if let Some(finished) = host.child.try_wait().unwrap() {
                    host.reaped = true;
                    status = Some(finished);
                    drain_until = Some(std::time::Instant::now() + Duration::from_secs(2));
                }
            } else if native_playback.is_some() || seen.iter().all(|count| *count == 1) {
                break;
            }
            if drain_until.is_some_and(|until| std::time::Instant::now() >= until) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "public sample exchange exceeded its deadline"
            );
            let sleep_begin = std::time::Instant::now();
            thread::sleep(Duration::from_millis(1));
            previous_sleep_us = u64::try_from(sleep_begin.elapsed().as_micros()).unwrap();
        }
        println!(
            "public_usb_host_exit family={family} playback={:?} microphone={:?} status={:?}",
            native_playback.as_ref().map(|_| "NativeClient"),
            native_microphone.as_ref().map(|_| "NativeClient"),
            status
        );
        let mut receipt = Vec::new();
        output.take(8193).read_to_end(&mut receipt).unwrap();
        assert!(receipt.len() <= 8192);
        let receipt = String::from_utf8(receipt).unwrap();
        let audio = controller.sample_audio();
        let microphone_consumed =
            public_sample_microphone_counter(native_microphone.is_some(), || {
                audio.microphone_consumed_frames()
            })
            .unwrap();
        let microphone_dropped =
            public_sample_microphone_counter(native_microphone.is_some(), || {
                audio.dropped_microphone_frames()
            })
            .unwrap();
        println!(
            "public_usb_sample_counters family={family} microphone_consumed={microphone_consumed:?} microphone_dropped={microphone_dropped:?} underrun={} playback_dropped={} maximum_service_us={maximum_service_us} maximum_read_us={:?} largest_cycle_delays_interval_service_credit_consumed_produced_silence_sleep_write={largest_delays:?}",
            audio.underrun_frames(),
            audio.dropped_playback_frames(),
            native_playback.is_none().then_some(maximum_read_us)
        );
        #[cfg(feature = "experimental")]
        if native_microphone.is_some() {
            let instrumentation = crate::experimental::audio_instrumentation(audio);
            println!(
                "public_usb_native_worker_progress family={family} progress={:?}",
                instrumentation.usb_microphone_progress()
            );
            println!(
                "public_usb_native_silence_trace family={family} trace={:?}",
                instrumentation.native_microphone_silence_trace()
            );
            println!(
                "public_usb_native_host_clock_progress family={family} interval_us_host_frames={:?}",
                instrumentation.native_microphone_host_clock_progress()
            );
            println!(
                "public_usb_native_graph_loss_trace family={family} trace={:?}",
                instrumentation.native_microphone_graph_loss_trace()
            );
            println!(
                "public_usb_native_microphone_counters family={family} graph_dropped={:?} cached_graph_dropped_before={graph_dropped_before:?} cached_trial_graph_drop_delta={:?} graph_loss_empty_corrupted_overflow={:?} queue_frames={:?} bridge_schedule_us={:?} stream_timings={:?}",
                instrumentation.native_microphone_dropped_frames(),
                public_counter_delta(
                    graph_dropped_before,
                    instrumentation.native_microphone_dropped_frames(),
                ),
                instrumentation.native_microphone_graph_loss_frames(),
                instrumentation.native_microphone_queue_frames(),
                instrumentation.native_bridge_scheduling_us(),
                instrumentation.stream_timings()
            );
        }
        println!("public_usb_sample_capture family={family} receipt={receipt}");
        let (missing, duplicate) = seen.iter().fold((0, 0), |(missing, duplicate), count| {
            (
                missing + usize::from(*count == 0),
                duplicate + usize::from(*count > 1),
            )
        });
        let passed = if native_playback.is_some() {
            println!(
                "public_usb_native_playback_marker_scope family={family} measured_frames=144000 receipt=owned_host_capture"
            );
            public_native_playback_passed(
                status.unwrap().success(),
                microphone_produced,
                native_microphone.is_some(),
                controller.sample_audio().last_error().is_none(),
            )
        } else {
            println!(
                "public_usb_sample_playback family={family} measured_frames=144000 missing={missing} duplicate={duplicate} invalid={invalid} microphone_produced={microphone_produced}"
            );
            public_exchange_passed(
                status.unwrap().success(),
                (missing, duplicate, invalid),
                microphone_produced,
                native_microphone.is_some(),
                controller.sample_audio().last_error().is_none(),
            )
        };
        if native_microphone.is_some() {
            println!("public_usb_native_microphone_exchange family={family} passed={passed}");
        } else {
            println!("public_usb_sample_exchange family={family} passed={passed}");
        }
        passed
    }

    fn collect_public_marker_case(run: impl FnOnce() -> bool) -> bool {
        // An acceptance-cell panic is a failed receipt, never permission to
        // skip cleanup or to omit independent cells from the matrix.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or(false)
    }

    #[test]
    fn terminal_service_failure_cleans_owned_case_and_preserves_next_receipt() {
        use std::cell::RefCell;
        struct OwnedCase<'a>(&'a RefCell<Vec<&'static str>>);
        impl OwnedCase<'_> {
            fn service(&self) -> Result<(), crate::ControllerError> {
                self.0.borrow_mut().push("service");
                Err(crate::ControllerError::Read {
                    reason: "synthetic broken pipe".into(),
                })
            }
        }
        impl Drop for OwnedCase<'_> {
            fn drop(&mut self) {
                self.0.borrow_mut().push("close-owned-case");
            }
        }
        let events = RefCell::new(Vec::new());
        assert!(!collect_public_marker_case(|| {
            let owned = OwnedCase(&events);
            events.borrow_mut().push("create");
            owned.service().unwrap();
            true
        }));
        assert_eq!(*events.borrow(), ["create", "service", "close-owned-case"]);
        assert!(collect_public_marker_case(|| {
            events.borrow_mut().push("next-case");
            true
        }));
        assert!(!collect_public_marker_case(|| false));
        assert_eq!(events.borrow().last(), Some(&"next-case"));
    }

    #[test]
    #[ignore = "requires the prepared ordinary client mount and explicit sample factory lab opt-in"]
    #[allow(clippy::too_many_lines)] // Keep ownership cases and their marker exchange in one fixture.
    fn live_public_usb_sample_factory() {
        assert_eq!(
            std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_LAB").as_deref(),
            Ok("1")
        );
        assert_ordinary_test_process();
        let mut failures = Vec::new();
        macro_rules! exercise {
            ($factory:ident, $family:literal, $playback:literal, $microphone:literal, $options:expr, $playback_access:expr, $microphone_access:expr) => {{
                let mut controller = crate::$factory($options)
                    .unwrap_or_else(|error| report_public_factory_failure($family, &error));
                controller.service(&mut |_| {}).unwrap();
                controller.neutralize().unwrap();
                controller.commit().unwrap();
                check_public_audio_ownership(
                    controller.audio().unwrap(),
                    $playback,
                    $microphone,
                    $playback_access,
                    $microphone_access,
                );
                controller.service(&mut |_| {}).unwrap();
                let mut exchange_passed = if std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_MARKER_LAB").as_deref()
                        == Ok("1")
                {
                    collect_public_marker_case(|| public_sample_exchange(&mut controller, $family, $playback, $microphone))
                } else { true };
                controller.close();
                controller.close();
                let audio = controller.audio().unwrap();
                assert!(audio.is_closed());
                if audio.last_error().is_some() {
                    eprintln!("public_usb_terminal_audio family={} retained={:?}", $family, audio.last_error());
                    exchange_passed = false;
                }
                assert_eq!(
                    audio.read_playback(&mut [0; $playback]),
                    Err(crate::AudioError::Closed)
                );
                assert_eq!(
                    audio.write_microphone(&[0; $microphone]),
                    Err(crate::AudioError::Closed)
                );
                let diagnostics = controller.diagnostics();
                if diagnostics.last_error().is_some() {
                    eprintln!("public_usb_terminal_controller family={} diagnostics={diagnostics:?}", $family);
                    exchange_passed = false;
                }
                if std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_SAMPLE_MARKER_LAB").as_deref() == Ok("1") {
                    println!("public_usb_marker_exchange family={} playback={:?} microphone={:?} passed={exchange_passed}",
                             $family, $playback_access, $microphone_access);
                }

                public_factory_host_snapshot($family);
                println!(
                    "public_usb_sample_factory family={} playback={:?} microphone={:?} passed={exchange_passed}",
                    $family, $playback_access, $microphone_access
                );
                if !exchange_passed {
                    failures.push(format!("{} {:?}/{:?}", $family, $playback_access, $microphone_access));
                }
            }};
        }
        let native =
            std::env::var("VIRTUALGAMEPAD_PUBLIC_USB_NATIVE_OWNERSHIP_LAB").as_deref() == Ok("1");
        // Execute outstanding mixed microphone cells first; a later sample
        // failure must not prevent their independent evidence from being collected.
        for playback in [
            crate::AudioAccess::Samples,
            crate::AudioAccess::NativeClient,
        ] {
            for microphone in [
                crate::AudioAccess::NativeClient,
                crate::AudioAccess::Samples,
            ] {
                if !native
                    && (playback != crate::AudioAccess::Samples
                        || microphone != crate::AudioAccess::Samples)
                {
                    continue;
                }
                let options =
                    crate::CreationOptions::new(crate::RealizationId::LINUX_USBIP_USB_AUDIO)
                        .with_audio(
                            crate::AudioOptions::new(crate::AudioExposure::Emulated)
                                .with_playback_access(playback)
                                .with_microphone_access(microphone),
                        );
                exercise!(
                    create_dualsense,
                    "dualsense",
                    4,
                    2,
                    options,
                    playback,
                    microphone
                );
                exercise!(
                    create_dualshock4,
                    "dualshock4",
                    2,
                    1,
                    options,
                    playback,
                    microphone
                );
                exercise!(
                    create_xbox360,
                    "xbox360",
                    2,
                    1,
                    options,
                    playback,
                    microphone
                );
            }
        }
        assert!(
            failures.is_empty(),
            "public USB marker cells failed: {failures:?}"
        );
    }
}

#[path = "retained_tests.rs"]
mod retained_outputs;
