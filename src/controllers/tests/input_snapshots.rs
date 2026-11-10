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
    ($test:ident, $module:ident, $controller:ident, $id:literal, $axes:expr,
     [$($part:expr),*], |$snapshot:ident, $extra_invalid:ident| $setup:block) => {
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
            let mut $extra_invalid = Vec::new();
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
            assert!(controller.is_dirty());
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
            let before_close = controller.state().clone();
            record.lock().unwrap().events.push_back(RawReverseEvent::HidGetReportRequest {
                request_id: 17, report_id: 1, report_type: 0,
            });
            assert!(matches!(controller.service(&mut |_| {}), Err(ControllerError::InvalidRequest { .. })));
            assert!(controller.commit().is_err());
            assert_eq!(controller.set_inputs(neutral), Err(ControlError::Closed));
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
