use super::*;
use crate::{DpadDirection, FaceButton};

fn assert_axis_values(frame: &ProviderFrame, axes: &[(u16, i32)]) {
    let ProviderFrame::Evdev(events) = frame else {
        panic!("expected an evdev input frame");
    };
    for &(code, value) in axes {
        assert!(
            events
                .iter()
                .any(|event| event.event_type == 3 && event.code == code && event.value == value),
            "missing axis {code} = {value}"
        );
    }
}

macro_rules! root_snapshot_case {
    (@refresh_motion $controller:ident, xbox360) => {};
    (@refresh_motion $controller:ident, switch_pro) => {};
    (@refresh_motion $controller:ident, $sony:ident) => {
        let before = $controller.state().clone();
        let dirty = $controller.is_dirty();
        assert!($controller.refresh_motion().is_err());
        assert_eq!($controller.state(), &before);
        assert_eq!($controller.is_dirty(), dirty);
    };
    (@assert_state dualshock4, $actual:ident, $candidate:ident) => {
        // DS4 advances its private touch sequence when it accepts the contacts.
        assert_eq!($actual.left_stick(), $candidate.left_stick());
        assert_eq!($actual.right_stick(), $candidate.right_stick());
        assert_eq!($actual.triggers(), $candidate.triggers());
        assert_eq!($actual.motion(), $candidate.motion());
        for control in [
            DualShock4Control::Cross, DualShock4Control::Circle, DualShock4Control::Square,
            DualShock4Control::Triangle, DualShock4Control::L1, DualShock4Control::R1,
            DualShock4Control::Share, DualShock4Control::Options, DualShock4Control::PlayStation,
            DualShock4Control::TouchpadClick, DualShock4Control::LeftStickPress,
            DualShock4Control::RightStickPress,
        ] {
            assert_eq!($actual.native_pressed(control), $candidate.native_pressed(control));
        }
        for direction in [DpadDirection::Up, DpadDirection::Down, DpadDirection::Left, DpadDirection::Right] {
            assert_eq!($actual.dpad_pressed(direction), $candidate.dpad_pressed(direction));
        }
        for slot in [DualShock4TouchSlot::First, DualShock4TouchSlot::Second] {
            assert_eq!($actual.touch(slot), $candidate.touch(slot));
        }
    };
    (@assert_state $module:ident, $actual:ident, $candidate:ident) => {
        assert_eq!($actual, $candidate);
    };
    ($test:ident, $module:ident, $controller:ident, $state:ty, $id:literal, $axes:expr,
     [$($part:expr),*], |$snapshot:ident, $extra_invalid:ident| $setup:block) => {
        mod $module {
            use super::*;
            #[test]
            fn transactional_root_inputs_preserve_rejected_and_unchanged_work() {
                let record = Arc::new(Mutex::new(Record::default()));
                let options = CreationOptions::new(RealizationId::LINUX_UINPUT).internal().unwrap();
                let inner = gr_curated_controllers::$module::test_controller(Box::new(Fake(record.clone()))).unwrap();
                let association = ControllerAssociation::single(
                    ControllerId::new($id), options, inner.association(), inner.surface().common(),
                );
                let mut controller = $controller::from_parts(association, inner, $($part),*);
                controller.commit().unwrap();
                let clean = controller.state().clone();
                let mut $snapshot = clean.clone();
                let mut $extra_invalid: Vec<$state> = Vec::new();
                $snapshot.set_digital(DigitalControlUpdate::FaceButton {
                    button: FaceButton::South, pressed: true,
                });
                $setup
                controller.edit_inputs(|_| Ok(())).unwrap();
                assert!(!controller.is_dirty());
                let error = ControlError::UnsupportedControl { control: "synthetic late root edit" };
                for dirty in [false, true] {
                    if dirty {
                        controller.edit_inputs(|candidate| {
                            *candidate = $snapshot.clone();
                            Ok(())
                        }).unwrap();
                    }
                    let accepted = controller.state().clone();
                    assert_eq!(controller.edit_inputs(|candidate| {
                        *candidate = clean.clone();
                        Err(error.clone())
                    }), Err(error.clone()));
                    assert_eq!(controller.state(), &accepted);
                    assert_eq!(controller.is_dirty(), dirty);
                    for invalid in &$extra_invalid {
                        assert!(controller.edit_inputs(|candidate| {
                            *candidate = invalid.clone();
                            Ok(())
                        }).is_err());
                        assert_eq!(controller.state(), &accepted);
                        assert_eq!(controller.is_dirty(), dirty);
                    }
                    controller.edit_inputs(|_| Ok(())).unwrap();
                    assert_eq!(controller.is_dirty(), dirty);
                }
                assert_eq!(record.lock().unwrap().sent.len(), 1);
                controller.commit().unwrap();
                let accepted = controller.state().clone();
                controller.edit_inputs(|_| Ok(())).unwrap();
                controller.set_inputs(accepted.clone()).unwrap();
                assert!(!controller.is_dirty());
                controller.close();
                controller.close();
                assert_eq!(controller.edit_inputs(|_| panic!("closed root edit executed")), Err(ControlError::Closed));
                assert_eq!(controller.state(), &accepted);
                assert_eq!(record.lock().unwrap().closes, 1);
            }
        }
        #[test]
        fn $test() {
            let record = Arc::new(Mutex::new(Record::default()));
            let options = CreationOptions::new(RealizationId::LINUX_UINPUT).internal().unwrap();
            let inner = gr_curated_controllers::$module::test_controller(
                Box::new(Fake(record.clone()))
            ).unwrap();
            let association = ControllerAssociation::single(
                ControllerId::new($id), options, inner.association(), inner.surface().common(),
            );
            let mut controller = $controller::from_parts(association, inner, $($part),*);
            let creation = controller.association().creation();
            let outputs = controller.host_outputs().unwrap();
            let mut $snapshot = controller.state().clone();
            let mut $extra_invalid: Vec<$state> = Vec::new();
            $snapshot.set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South, pressed: true,
            });
            for direction in [DpadDirection::Up, DpadDirection::Right] {
                $snapshot.set_digital(DigitalControlUpdate::Dpad { direction, pressed: true });
            }
            $setup
            let mut opposing = $snapshot.clone();
            opposing.set_digital(DigitalControlUpdate::Dpad {
                direction: DpadDirection::Down, pressed: true,
            });
            $extra_invalid.push(opposing);

            controller.commit().unwrap();
            root_snapshot_case!(@refresh_motion controller, $module);
            let neutral = controller.state().clone();
            let neutral_frame = record.lock().unwrap().sent[0].clone();
            for invalid in &$extra_invalid {
                assert!(controller.set_inputs(invalid.clone()).is_err());
                assert_eq!(controller.state(), &neutral);
                assert!(!controller.is_dirty());
            }

            controller.set_inputs($snapshot.clone()).unwrap();
            let accepted = controller.state().clone();
            root_snapshot_case!(@assert_state $module, accepted, $snapshot);
            assert!(controller.is_dirty());
            assert_eq!(record.lock().unwrap().sent.len(), 1);
            record.lock().unwrap().fail_write = true;
            assert!(controller.commit().is_err());
            for mut invalid in $extra_invalid {
                invalid.set_digital(DigitalControlUpdate::FaceButton {
                    button: FaceButton::South, pressed: false,
                });
                assert!(controller.set_inputs(invalid).is_err());
                assert_eq!(controller.state(), &accepted);
                assert!(controller.is_dirty());
            }
            controller.commit().unwrap();
            assert!(!controller.is_dirty());
            let populated_frame = record.lock().unwrap().sent[1].clone();
            assert_ne!(populated_frame, neutral_frame);
            assert_axis_values(&populated_frame, $axes);

            controller.set_inputs($snapshot.clone()).unwrap();
            assert_eq!(controller.state(), &accepted);
            assert!(!controller.is_dirty());
            controller.commit().unwrap();
            assert_eq!(record.lock().unwrap().sent.last(), Some(&populated_frame));

            controller.set_inputs(neutral.clone()).unwrap();
            controller.commit().unwrap();
            assert_eq!(record.lock().unwrap().sent.last(), Some(&neutral_frame));
            controller.set_inputs($snapshot.clone()).unwrap();
            controller.neutralize().unwrap();
            controller.commit().unwrap();
            assert_eq!(record.lock().unwrap().sent.last(), Some(&neutral_frame));
            controller.set_inputs($snapshot.clone()).unwrap();
            controller.commit().unwrap();
            assert_eq!(record.lock().unwrap().sent.last(), Some(&populated_frame));
            controller.service(&mut |_| {}).unwrap();
            assert_eq!(controller.association().creation(), creation);
            assert_eq!(controller.host_outputs().unwrap(), outputs);

            controller.set_inputs($snapshot).unwrap();
            controller.set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::North,
                pressed: !controller.state().face_pressed(FaceButton::North),
            }).unwrap();
            let before_close = controller.state().clone();
            record.lock().unwrap().events.push_back(RawReverseEvent::HidGetReportRequest {
                request_id: 17, report_id: 1, report_type: 0,
            });
            assert!(matches!(controller.service(&mut |_| {}), Err(ControllerError::InvalidRequest { .. })));
            assert!(controller.commit().is_err());
            assert_eq!(controller.set_inputs(neutral), Err(ControlError::Closed));
            assert_eq!(controller.edit_inputs(|_| panic!("closed root edit executed")), Err(ControlError::Closed));
            assert_eq!(controller.state(), &before_close);
            assert!(controller.is_dirty());
            controller.close();
            controller.close();
            assert_eq!(record.lock().unwrap().closes, 1);
        }
    };
}

const BYTE_AXES: &[(u16, i32)] = &[
    (0, 0),
    (1, 255),
    (2, 255),
    (3, 255),
    (4, 0),
    (5, 23),
    (16, 1),
    (17, -1),
    (47, 0),
    (47, 1),
    (57, 127),
    (57, 7),
    (53, 1919),
    (54, 941),
];

root_snapshot_case!(
    dualsense_whole_state,
    dualsense,
    DualSenseController,
    DualSenseState,
    "virtualgamepad.dualsense",
    BYTE_AXES,
    [None, None],
    |snapshot, invalid| {
        let axis = DualSenseAxis::new;
        snapshot.set_sticks((axis(0), axis(255)), (axis(255), axis(0)));
        snapshot.set_triggers(DualSenseTrigger::new(255), DualSenseTrigger::new(23));
        snapshot.set_native(DualSenseControl::TouchpadClick, true);
        snapshot.set_touch(
            TouchSlot::First,
            Some(DualSenseTouchContact::new(127, 1919, 941).unwrap()),
        );
        snapshot.set_touch(
            TouchSlot::Second,
            Some(DualSenseTouchContact::new(7, 0, 0).unwrap()),
        );
        let mut with_motion = snapshot.clone();
        with_motion.set_motion(MotionSample {
            accelerometer: [i16::MIN, 2, 3],
            gyroscope: [4, 5, i16::MAX],
        });
        invalid.push(with_motion);
        let mut with_mute = snapshot.clone();
        with_mute.set_native(DualSenseControl::MicrophoneMute, true);
        invalid.push(with_mute);
    }
);
root_snapshot_case!(
    ds4_whole_state,
    dualshock4,
    DualShock4Controller,
    DualShock4State,
    "virtualgamepad.dualshock4",
    BYTE_AXES,
    [None, None],
    |snapshot, invalid| {
        let axis = DualShock4Axis::new;
        snapshot.set_sticks((axis(0), axis(255)), (axis(255), axis(0)));
        snapshot.set_triggers(DualShock4Trigger::new(255), DualShock4Trigger::new(23));
        snapshot.set_native(DualShock4Control::TouchpadClick, true);
        snapshot.set_touch(
            DualShock4TouchSlot::First,
            Some(DualShock4TouchContact::new(127, 1919, 941).unwrap()),
        );
        snapshot.set_touch(
            DualShock4TouchSlot::Second,
            Some(DualShock4TouchContact::new(7, 0, 0).unwrap()),
        );
        let mut with_motion = snapshot.clone();
        with_motion.set_motion(DualShock4MotionSample {
            accelerometer: [i16::MIN, 2, 3],
            gyroscope: [4, 5, i16::MAX],
        });
        invalid.push(with_motion);
        let mut with_touch = snapshot.clone();
        with_touch.set_touch(
            DualShock4TouchSlot::Second,
            Some(DualShock4TouchContact::new(128, 0, 0).unwrap()),
        );
        invalid.push(with_touch);
    }
);
root_snapshot_case!(
    switch_pro_whole_state,
    switch_pro,
    SwitchProController,
    SwitchProState,
    "virtualgamepad.switch-pro",
    &[
        (0, -32768),
        (1, 32767),
        (3, 32767),
        (4, -32768),
        (16, 1),
        (17, -1)
    ],
    [],
    |snapshot, invalid| {
        let axis = SwitchProAxis::new;
        snapshot.set_sticks(
            (axis(i16::MIN), axis(i16::MAX)),
            (axis(i16::MAX), axis(i16::MIN)),
        );
        snapshot.set_native(SwitchProControl::Capture, true);
        snapshot.set_native(SwitchProControl::Zl, true);
        snapshot.set_native(SwitchProControl::Zr, true);
        let mut with_motion = snapshot.clone();
        with_motion.set_motion(SwitchProMotionSample {
            accelerometer: [i16::MIN, 2, 3],
            gyroscope: [4, 5, i16::MAX],
        });
        invalid.push(with_motion);
    }
);
root_snapshot_case!(
    xbox360_whole_state,
    xbox360,
    Xbox360Controller,
    Xbox360State,
    "virtualgamepad.xbox360",
    &[
        (0, -32768),
        (1, 32767),
        (2, 255),
        (3, 32767),
        (4, -32768),
        (5, 23),
        (16, 1),
        (17, -1)
    ],
    [None],
    |snapshot, _invalid| {
        let axis = Xbox360Axis::new;
        snapshot.set_sticks(
            (axis(i16::MIN), axis(i16::MAX)),
            (axis(i16::MAX), axis(i16::MIN)),
        );
        snapshot.set_triggers(Xbox360Trigger::new(255), Xbox360Trigger::new(23));
        snapshot.set_native(Xbox360Control::Guide, true);
    }
);

#[test]
fn root_sony_refresh_commits_a_new_motion_sample() {
    struct Bridge<S> {
        samples: Arc<Mutex<Vec<S>>>,
        provider: Fake,
    }
    impl<S: Clone + Send> gr_curated_controllers::WorkerBridge<S> for Bridge<S> {
        fn update(&mut self, state: &S) -> Result<(), ProviderError> {
            self.samples.lock().unwrap().push(state.clone());
            Ok(())
        }
        fn output(&mut self) -> Result<Option<RawReverseEvent>, ProviderError> {
            Ok(None)
        }
        fn diagnostics(&mut self) -> ProviderDiagnostics {
            self.provider.diagnostics()
        }
        fn dropped_output_events(&self) -> u64 {
            0
        }
        fn close(&mut self) -> Result<(), ProviderError> {
            self.provider.close()
        }
    }
    macro_rules! check {
        ($create:ident, $handle:ident, $id:literal) => {{
            let samples = Arc::new(Mutex::new(Vec::new()));
            let io = Arc::new(Mutex::new(Record::default()));
            let inner = gr_curated_controllers::$create(Box::new(Bridge {
                samples: samples.clone(),
                provider: Fake(io.clone()),
            }));
            let association = ControllerAssociation::single(
                ControllerId::new($id),
                gr_curated_controllers::CreationOptions {
                    target: RealizationId::LINUX_USBIP_USB_AUDIO,
                    session: RealizationSessionId(7),
                },
                inner.association(),
                inner.surface().common(),
            );
            let mut controller = $handle::from_parts(association, inner, None, None);
            controller.commit().unwrap();
            let previous = controller.state().clone();
            let motion = previous.motion();
            controller.set_motion(motion).unwrap();
            assert_eq!(controller.state(), &previous);
            controller.refresh_motion().unwrap();
            let refreshed = controller.state().clone();
            assert_ne!(refreshed, previous);
            assert_eq!(refreshed.motion(), motion);
            assert_eq!(*samples.lock().unwrap(), vec![previous, refreshed.clone()]);
            assert!(!controller.is_dirty());
            controller.close();
            controller.close();
            assert!(controller.refresh_motion().is_err());
            assert_eq!(controller.state(), &refreshed);
            assert_eq!(io.lock().unwrap().closes, 1);
        }};
    }
    check!(
        create_dualsense_usb_worker,
        DualSenseController,
        "virtualgamepad.dualsense"
    );
    check!(
        create_dualshock4_usb_worker,
        DualShock4Controller,
        "virtualgamepad.dualshock4"
    );
}
