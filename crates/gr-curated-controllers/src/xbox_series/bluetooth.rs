use super::{XboxSeriesButton as Button, XboxSeriesState};
use gr_hid::{ReplyError, Report, ReportType};
use gr_realization_api::{
    NativeControllerRealization, NativeDeviceIdentity, NativeHidRealization, ProviderFrame,
    RealizationTarget,
};

// Physical 045e:0b13 report map, recorded in xbox-series-bt-reference.md.
const DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x01, 0xa1, 0x00, 0x09, 0x30, 0x09, 0x31,
    0x15, 0x00, 0x27, 0xff, 0xff, 0x00, 0x00, 0x95, 0x02, 0x75, 0x10, 0x81, 0x02, 0xc0, 0x09, 0x01,
    0xa1, 0x00, 0x09, 0x32, 0x09, 0x35, 0x15, 0x00, 0x27, 0xff, 0xff, 0x00, 0x00, 0x95, 0x02, 0x75,
    0x10, 0x81, 0x02, 0xc0, 0x05, 0x02, 0x09, 0xc5, 0x15, 0x00, 0x26, 0xff, 0x03, 0x95, 0x01, 0x75,
    0x0a, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00, 0x75, 0x06, 0x95, 0x01, 0x81, 0x03, 0x05, 0x02, 0x09,
    0xc4, 0x15, 0x00, 0x26, 0xff, 0x03, 0x95, 0x01, 0x75, 0x0a, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00,
    0x75, 0x06, 0x95, 0x01, 0x81, 0x03, 0x05, 0x01, 0x09, 0x39, 0x15, 0x01, 0x25, 0x08, 0x35, 0x00,
    0x46, 0x3b, 0x01, 0x66, 0x14, 0x00, 0x75, 0x04, 0x95, 0x01, 0x81, 0x42, 0x75, 0x04, 0x95, 0x01,
    0x15, 0x00, 0x25, 0x00, 0x35, 0x00, 0x45, 0x00, 0x65, 0x00, 0x81, 0x03, 0x05, 0x09, 0x19, 0x01,
    0x29, 0x0f, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x0f, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00,
    0x75, 0x01, 0x95, 0x01, 0x81, 0x03, 0x05, 0x0c, 0x0a, 0xb2, 0x00, 0x15, 0x00, 0x25, 0x01, 0x95,
    0x01, 0x75, 0x01, 0x81, 0x02, 0x15, 0x00, 0x25, 0x00, 0x75, 0x07, 0x95, 0x01, 0x81, 0x03, 0x05,
    0x0f, 0x09, 0x21, 0x85, 0x03, 0xa1, 0x02, 0x09, 0x97, 0x15, 0x00, 0x25, 0x01, 0x75, 0x04, 0x95,
    0x01, 0x91, 0x02, 0x15, 0x00, 0x25, 0x00, 0x75, 0x04, 0x95, 0x01, 0x91, 0x03, 0x09, 0x70, 0x15,
    0x00, 0x25, 0x64, 0x75, 0x08, 0x95, 0x04, 0x91, 0x02, 0x09, 0x50, 0x66, 0x01, 0x10, 0x55, 0x0e,
    0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x91, 0x02, 0x09, 0xa7, 0x15, 0x00, 0x26,
    0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x91, 0x02, 0x65, 0x00, 0x55, 0x00, 0x09, 0x7c, 0x15, 0x00,
    0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x01, 0x91, 0x02, 0xc0, 0xc0,
];

const BUTTONS: [(Button, u8); 11] = [
    (Button::A, 0),
    (Button::B, 1),
    (Button::X, 3),
    (Button::Y, 4),
    (Button::LeftShoulder, 6),
    (Button::RightShoulder, 7),
    (Button::View, 10),
    (Button::Menu, 11),
    (Button::Guide, 12),
    (Button::LeftStickPress, 13),
    (Button::RightStickPress, 14),
];

pub(super) fn realization() -> NativeControllerRealization {
    NativeControllerRealization::Uhid(NativeHidRealization {
        target: RealizationTarget::LINUX_UHID_BLUETOOTH,
        bus_type: 5,
        device_name: "Xbox Wireless Controller".into(),
        physical_path: "virtualgamepad/uhid/xbox-series".into(),
        unique_id: "virtualgamepad-xbox-series".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x045e,
            product_id: 0x0b13,
            version: 0x0523,
        },
        descriptor: DESCRIPTOR.to_vec(),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: false,
    })
}

pub(super) fn frame(state: &XboxSeriesState) -> ProviderFrame {
    let mut bytes = Vec::with_capacity(16);
    for axis in state.sticks.into_iter().flatten() {
        bytes.extend((u16::from_le_bytes(axis.to_le_bytes()) ^ 0x8000).to_le_bytes());
    }
    for trigger in state.triggers() {
        bytes.extend(trigger.raw().to_le_bytes());
    }
    let x = i8::from(state.pressed(Button::DpadRight)) - i8::from(state.pressed(Button::DpadLeft));
    let y = i8::from(state.pressed(Button::DpadDown)) - i8::from(state.pressed(Button::DpadUp));
    bytes.push(match (x, y) {
        (0, -1) => 1,
        (1, -1) => 2,
        (1, 0) => 3,
        (1, 1) => 4,
        (0, 1) => 5,
        (-1, 1) => 6,
        (-1, 0) => 7,
        (-1, -1) => 8,
        _ => 0,
    });
    let buttons = BUTTONS.iter().fold(0u16, |mask, (button, bit)| {
        mask | (u16::from(state.pressed(*button)) << bit)
    });
    bytes.extend(buttons.to_le_bytes());
    bytes.push(u8::from(state.pressed(Button::Share)));
    ProviderFrame::HidInput {
        report_id: Some(1),
        bytes,
    }
}

pub(super) fn input(state: &XboxSeriesState, _: u64, _: u8) -> Report {
    crate::common::logical_input(frame(state))
}

/// Bluetooth HID force feedback in the controller's native units.
/// Amplitudes are HID logical magnitudes from 0 through 100. They differ from
/// uinput's 0 through 65535 magnitudes and do not encode perceived intensity.
/// Mask bits 0 through 3 select weak, strong, right trigger and left trigger.
/// Duration and delay are descriptor-defined 10 ms ticks; repeat is a raw count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct XboxSeriesBluetoothRumble {
    pub motor_mask: u8,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub strong: u8,
    pub weak: u8,
    pub duration: u8,
    pub delay: u8,
    pub repeat: u8,
}

pub(super) fn rumble(report: &Report) -> Result<XboxSeriesBluetoothRumble, ReplyError> {
    if report.kind != ReportType::Output || report.id() != Some(3) {
        return Err(ReplyError::Unsupported);
    }
    let &[
        motor_mask,
        left_trigger,
        right_trigger,
        strong,
        weak,
        duration,
        delay,
        repeat,
    ] = report.payload()
    else {
        return Err(ReplyError::Invalid);
    };
    if motor_mask & 0xf0 != 0
        || [left_trigger, right_trigger, strong, weak]
            .iter()
            .any(|&x| x > 100)
    {
        return Err(ReplyError::Invalid);
    }
    Ok(XboxSeriesBluetoothRumble {
        motor_mask,
        left_trigger,
        right_trigger,
        strong,
        weak,
        duration,
        delay,
        repeat,
    })
}

pub(super) fn validate_output(report: &Report) -> Result<(), ReplyError> {
    rumble(report).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::HidDriver;
    use crate::retained::RetainedProtocol;
    use gr_hid::{Lifecycle, Protocol, Reply, RequestKind};
    use gr_realization_api::RealizationSessionId;

    #[test]
    fn host_open_replays_current_snapshot_after_early_start_delivery() {
        let mut protocol = super::super::XboxSeriesDefinition::default()
            .hid_protocol(RealizationSessionId(1), [0; 6]);
        let mut peer = super::super::XboxSeriesDefinition::default()
            .hid_protocol(RealizationSessionId(2), [0; 6]);
        for p in [&mut protocol, &mut peer] {
            p.lifecycle(
                Lifecycle::Start {
                    numbered_input: true,
                    numbered_output: true,
                    numbered_feature: false,
                },
                0,
            );
            p.input(&XboxSeriesState::default(), 0).unwrap();
            assert_eq!(p.deadline(), None);
        }
        // A successful early UHID write can precede evdev registration. OPEN
        // must schedule the accepted snapshot again without an application edit.
        protocol.lifecycle(Lifecycle::Open, 20);
        assert_eq!(protocol.deadline(), Some(20));
        assert_eq!(peer.deadline(), None);
        assert_eq!(
            protocol.input(&XboxSeriesState::default(), 20).unwrap()[0].wire(),
            [1, 0, 128, 0, 128, 0, 128, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(protocol.deadline(), None);
        let mut state = XboxSeriesState::default();
        state.set_button(Button::Share, true);
        protocol.input(&state, 21).unwrap();
        protocol.lifecycle(Lifecycle::Close, 22);
        protocol.lifecycle(Lifecycle::Open, 23);
        assert_eq!(protocol.deadline(), Some(23));
        assert_eq!(protocol.input(&state, 23).unwrap()[0].wire()[16], 1);
        protocol.lifecycle(Lifecycle::Stop, 24);
        protocol.lifecycle(Lifecycle::Open, 25);
        assert_eq!(protocol.deadline(), None);
        protocol.lifecycle(Lifecycle::Stop, 26);
        assert_eq!(protocol.deadline(), None);
    }

    #[test]
    fn physical_input_shape_keeps_controls_independent_and_endpoints_exact() {
        let mut state = XboxSeriesState {
            sticks: [[i16::MIN, 0], [i16::MAX, -1]],
            ..XboxSeriesState::default()
        };
        state.set_triggers(1023, 756).unwrap();
        state.set_button(Button::DpadUp, true);
        state.set_button(Button::DpadRight, true);
        state.set_button(Button::Guide, true);
        state.set_button(Button::Share, true);
        assert_eq!(
            input(&state, 0, 0).wire(),
            [
                1, 0, 0, 0, 128, 255, 255, 255, 127, 255, 3, 244, 2, 2, 0, 16, 1
            ]
        );
        state.set_button(Button::Guide, false);
        assert_eq!(&input(&state, 0, 0).wire()[14..], &[0, 0, 1]);
        state.neutralize();
        assert_eq!(
            input(&state, 0, 0).wire(),
            [1, 0, 128, 0, 128, 0, 128, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn host_requests_reply_for_each_report_class_and_keep_valid_rumble() {
        let mut protocol = super::super::XboxSeriesDefinition::default()
            .hid_protocol(RealizationSessionId(1), [0; 6]);
        protocol.lifecycle(
            Lifecycle::Start {
                numbered_input: true,
                numbered_output: true,
                numbered_feature: false,
            },
            0,
        );
        protocol.lifecycle(Lifecycle::Open, 0);
        let state = XboxSeriesState::default();
        let input = protocol.input(&state, 0).unwrap().remove(0);
        for (kind, id, expected) in [
            (ReportType::Input, Some(1), Ok(input)),
            (ReportType::Input, Some(3), Err(ReplyError::Unsupported)),
            (ReportType::Output, Some(3), Err(ReplyError::Unsupported)),
            (ReportType::Feature, Some(1), Err(ReplyError::Unsupported)),
            (ReportType::Other(9), Some(3), Err(ReplyError::Unsupported)),
        ] {
            assert_eq!(
                protocol.request(&RequestKind::Get { kind, id }, 0).0,
                Reply::Get(expected)
            );
        }
        let payload = vec![15, 10, 20, 25, 50, 255, 0, 255];
        for (kind, id, bytes, expected) in [
            (ReportType::Output, Some(3), payload.clone(), Ok(())),
            (
                ReportType::Output,
                Some(3),
                vec![0; 7],
                Err(ReplyError::Invalid),
            ),
            (
                ReportType::Output,
                Some(1),
                payload.clone(),
                Err(ReplyError::Unsupported),
            ),
            (
                ReportType::Input,
                Some(1),
                vec![0; 16],
                Err(ReplyError::Unsupported),
            ),
            (
                ReportType::Feature,
                Some(3),
                payload,
                Err(ReplyError::Unsupported),
            ),
        ] {
            let report = Report::new(kind, id, bytes).unwrap();
            assert_eq!(
                protocol.request(&RequestKind::Set(report), 0).0,
                Reply::Set(expected)
            );
        }
        let retained = protocol.retained().bluetooth_rumble.unwrap();
        assert_eq!(
            (
                retained.left_trigger,
                retained.right_trigger,
                retained.strong,
                retained.weak
            ),
            (10, 20, 25, 50)
        );
        assert_eq!(protocol.retained().revision, 1);
        protocol.lifecycle(Lifecycle::Close, 1);
        protocol.lifecycle(Lifecycle::Stop, 1);
        assert!(protocol.input(&state, 1).unwrap().is_empty());
    }
    #[test]
    fn malformed_motor_bounds_preserve_the_accepted_snapshot() {
        let mut protocol = super::super::XboxSeriesDefinition::default()
            .hid_protocol(RealizationSessionId(1), [0; 6]);
        let valid = vec![15, 25, 50, 25, 50, 20, 3, 2];
        let report = Report::new(ReportType::Output, Some(3), valid.clone()).unwrap();
        assert_eq!(
            protocol.request(&RequestKind::Set(report), 0).0,
            Reply::Set(Ok(()))
        );
        let accepted = protocol.retained().clone();
        for (offset, value) in [(0, 16), (1, 101), (2, 255), (3, 101), (4, 255)] {
            let mut bytes = valid.clone();
            bytes[offset] = value;
            let report = Report::new(ReportType::Output, Some(3), bytes).unwrap();
            assert_eq!(
                protocol.request(&RequestKind::Set(report), 0).0,
                Reply::Set(Err(ReplyError::Invalid))
            );
            assert_eq!(protocol.retained(), &accepted);
        }
        let effect = accepted.bluetooth_rumble.unwrap();
        assert_eq!((effect.strong, effect.weak), (25, 50));
        assert_eq!((0x4000u16 / 655, 0x8000u16 / 655), (25, 50));
        assert_eq!((effect.duration, effect.delay, effect.repeat), (20, 3, 2));
    }

    #[test]
    fn repeated_lifecycle_and_host_requests_are_isolated_per_instance() {
        let definition = super::super::XboxSeriesDefinition::default();
        let mut first = definition.hid_protocol(RealizationSessionId(1), [0; 6]);
        let mut second = definition.hid_protocol(RealizationSessionId(2), [0; 6]);
        let start = Lifecycle::Start {
            numbered_input: true,
            numbered_output: true,
            numbered_feature: false,
        };
        let mut held = XboxSeriesState::default();
        held.set_button(Button::Share, true);
        first.input(&held, 0).unwrap();
        second.input(&XboxSeriesState::default(), 0).unwrap();
        let get = RequestKind::Get {
            kind: ReportType::Input,
            id: Some(1),
        };
        assert_ne!(first.request(&get, 0).0, second.request(&get, 0).0);
        let output = RequestKind::Set(
            Report::new(ReportType::Output, Some(3), vec![15, 25, 50, 0, 0, 1, 0, 0]).unwrap(),
        );
        first.request(&output, 0);
        assert_eq!(first.retained().revision, 1);
        assert_eq!(second.retained().revision, 0);
        for now in 1..4 {
            for lifecycle in [Lifecycle::Close, Lifecycle::Stop, Lifecycle::Stop] {
                first.lifecycle(lifecycle, now);
            }
            assert!(first.input(&held, now).unwrap().is_empty());
            assert_eq!(
                second
                    .input(&XboxSeriesState::default(), now)
                    .unwrap()
                    .len(),
                1
            );
            first.lifecycle(start, now);
            first.lifecycle(Lifecycle::Open, now);
            assert_eq!(first.input(&held, now).unwrap().len(), 1);
            assert_eq!(first.retained().revision, 1);
            assert_eq!(second.retained().revision, 0);
        }
    }
}
