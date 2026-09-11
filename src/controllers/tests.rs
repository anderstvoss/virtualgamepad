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
consumer_case!(dualsense_consumer_lifecycle, dualsense, DualSenseController, DualSenseControl, Cross, DualSenseOutputEvent, "virtualgamepad.dualsense", identity: None);
consumer_case!(ds4_consumer_lifecycle, dualshock4, DualShock4Controller, DualShock4Control, Cross, DualShock4OutputEvent, "virtualgamepad.dualshock4", identity: None);
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
    "virtualgamepad.xbox360"
);
