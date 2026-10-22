//! Controller-owned native interfaces. Record: wii-c5-evdev-consumer-shape.
//! uinput cannot carry bus5 or fuzz; these deviations are surfaced and tested.
use super::{WiiExtension, WiiRemoteRevision, WiiRemoteState};
use gr_realization_api::{
    EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization, NativeDeviceIdentity,
    NativeEvdevRealization, ProviderFrame, RealizationTarget,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Core,
    Accel,
    Ir,
    Nunchuk,
    Classic,
    MotionPlus,
}
impl Role {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Accel => "accel",
            Self::Ir => "ir",
            Self::Nunchuk => "nunchuk",
            Self::Classic => "classic",
            Self::MotionPlus => "motionplus",
        }
    }
}
// wii-kernel-evdev-base-keys / extension consumer table, not device firmware.
type CapabilityTable = (
    &'static str,
    &'static [u16],
    &'static [(u16, i32, i32, i32)],
    &'static [u16],
);
#[allow(clippy::too_many_lines)] // One source-linked capability table, verified independently below.
pub(super) fn specification(
    role: Role,
    revision: WiiRemoteRevision,
) -> NativeControllerRealization {
    let (name, keys, axes, ff): CapabilityTable = match role {
        Role::Core => (
            "Nintendo Wii Remote",
            &[105, 106, 103, 108, 407, 412, 257, 258, 304, 305, 316],
            &[],
            &[80],
        ),
        Role::Accel => (
            "Nintendo Wii Remote Accelerometer",
            &[],
            &[(3, -500, 500, 4), (4, -500, 500, 4), (5, -500, 500, 4)],
            &[],
        ),
        Role::Ir => (
            "Nintendo Wii Remote IR",
            &[],
            &[
                (16, 0, 1023, 4),
                (17, 0, 767, 4),
                (18, 0, 1023, 4),
                (19, 0, 767, 4),
                (20, 0, 1023, 4),
                (21, 0, 767, 4),
                (22, 0, 1023, 4),
                (23, 0, 767, 4),
            ],
            &[],
        ),
        Role::Nunchuk => (
            "Nintendo Wii Remote Nunchuk",
            &[306, 309],
            &[
                (16, -120, 120, 4),
                (17, -120, 120, 4),
                (3, -500, 500, 4),
                (4, -500, 500, 4),
                (5, -500, 500, 4),
            ],
            &[],
        ),
        Role::Classic => (
            "Nintendo Wii Remote Classic Controller",
            &[
                304, 305, 307, 308, 312, 313, 407, 412, 316, 105, 106, 103, 108, 310, 311,
            ],
            &[
                (18, -30, 30, 1),
                (19, -30, 30, 1),
                (20, -30, 30, 1),
                (21, -30, 30, 1),
                (22, -30, 30, 1),
                (23, -30, 30, 1),
            ],
            &[],
        ),
        Role::MotionPlus => (
            "Nintendo Wii Remote Motion Plus",
            &[],
            &[
                (3, -16000, 16000, 8),
                (4, -16000, 16000, 8),
                (5, -16000, 16000, 8),
            ],
            &[],
        ),
    };
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        physical_path: None,
        device_name: name.into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x057e,
            product_id: if revision == WiiRemoteRevision::Tr {
                0x0330
            } else {
                0x0306
            },
            version: 0,
        },
        event_codes: {
            let mut v = Vec::new();
            if !keys.is_empty() {
                v.push(1);
            }
            if !axes.is_empty() {
                v.push(3);
            }
            if !ff.is_empty() {
                v.push(21);
            }
            v
        },
        key_codes: keys.to_vec(),
        absolute_axes: axes
            .iter()
            .map(|&(code, minimum, maximum, flat)| NativeAbsoluteAxis {
                code,
                minimum,
                maximum,
                flat,
            })
            .collect(),
        relative_axes: vec![],
        led_codes: vec![],
        switch_codes: vec![],
        force_feedback_codes: ff.to_vec(),
    })
}
pub(super) fn roles(state: &WiiRemoteState) -> Vec<Role> {
    let mut roles = vec![Role::Accel, Role::Ir];
    if state.motion_plus().attached {
        roles.push(Role::MotionPlus);
    }
    match state.extension() {
        WiiExtension::Nunchuk(_) => roles.push(Role::Nunchuk),
        WiiExtension::Classic(_) | WiiExtension::ClassicPro(_) => roles.push(Role::Classic),
        WiiExtension::None => {}
    }
    roles
}
// wii-c5-report-layouts / wii-kernel consumer decoders. Native units are kept
// until this explicit consumer conversion, with declared evdev range clipping.
pub(super) fn frame(role: Role, state: &WiiRemoteState) -> ProviderFrame {
    let NativeControllerRealization::Evdev(spec) = specification(role, state.revision()) else {
        unreachable!()
    };
    let (keys, values): (Vec<bool>, Vec<i32>) = match role {
        Role::Core => (state.buttons.to_vec(), vec![]),
        Role::Accel => (
            vec![],
            state.accelerometer().map(|v| i32::from(v) - 512).to_vec(),
        ),
        Role::Ir => (
            vec![],
            state
                .ir()
                .iter()
                .flat_map(|p| {
                    let (x, y) = p.map_or((1023, 767), super::WiiIrPoint::coordinates);
                    [i32::from(x), i32::from(y)]
                })
                .collect(),
        ),
        Role::Nunchuk => {
            let n = match state.extension() {
                WiiExtension::Nunchuk(n) => n.clone(),
                _ => super::WiiNunchukState::default(),
            };
            (
                vec![n.c, n.z],
                vec![
                    i32::from(n.stick[0]) - 128,
                    i32::from(n.stick[1]) - 128,
                    i32::from(n.accelerometer[0]) - 512,
                    i32::from(n.accelerometer[1]) - 512,
                    i32::from(n.accelerometer[2]) - 512,
                ],
            )
        }
        Role::Classic => {
            let c = match state.extension() {
                WiiExtension::Classic(c) | WiiExtension::ClassicPro(c) => c.clone(),
                _ => super::WiiClassicState::default(),
            };
            // Consumer key order differs from native packed button order.
            let order = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
            (
                order.map(|i| c.buttons[i]).to_vec(),
                vec![
                    i32::from(c.left[0]) - 32,
                    i32::from(c.left[1]) - 32,
                    (i32::from(c.right[0]) - 16) * 2,
                    (i32::from(c.right[1]) - 16) * 2,
                    i32::from(c.triggers[1]) * 2,
                    i32::from(c.triggers[0]) * 2,
                ],
            )
        }
        Role::MotionPlus => {
            let m = state.motion_plus();
            let indices = [2, 1, 0];
            (
                vec![],
                indices
                    .map(|i| {
                        let raw = i32::from(m.gyroscope[i]) - 8192;
                        if m.slow[i] {
                            raw * 9
                        } else {
                            raw * 2000 * 9 / 440
                        }
                    })
                    .to_vec(),
            )
        }
    };
    let mut events: Vec<_> = spec
        .key_codes
        .iter()
        .zip(keys)
        .map(|(&code, value)| EvdevEvent {
            event_type: 1,
            code,
            value: i32::from(value),
        })
        .collect();
    events.extend(
        spec.absolute_axes
            .iter()
            .zip(values)
            .map(|(a, v)| EvdevEvent {
                event_type: 3,
                code: a.code,
                value: v.clamp(a.minimum, a.maximum),
            }),
    );
    events.push(EvdevEvent {
        event_type: 0,
        code: 0,
        value: 0,
    });
    ProviderFrame::Evdev(events)
}
// Record wii-c5-functional-descriptor: synthetic vendor-page byte arrays,
// not a retail descriptor. Every report ID/size is sourced in report-layouts.
pub(super) const DESCRIPTOR: &[u8] = &[
    0x06, 0x00, 0xff, 0x09, 0x01, 0xa1, 0x01, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x85, 0x20,
    0x09, 0x01, 0x95, 0x06, 0x81, 0x02, 0x85, 0x21, 0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x22,
    0x09, 0x01, 0x95, 0x04, 0x81, 0x02, 0x85, 0x30, 0x09, 0x01, 0x95, 0x02, 0x81, 0x02, 0x85, 0x31,
    0x09, 0x01, 0x95, 0x05, 0x81, 0x02, 0x85, 0x32, 0x09, 0x01, 0x95, 0x0a, 0x81, 0x02, 0x85, 0x33,
    0x09, 0x01, 0x95, 0x11, 0x81, 0x02, 0x85, 0x34, 0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x35,
    0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x36, 0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x37,
    0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x3d, 0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x3e,
    0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x3f, 0x09, 0x01, 0x95, 0x15, 0x81, 0x02, 0x85, 0x10,
    0x09, 0x01, 0x95, 0x01, 0x91, 0x02, 0x85, 0x11, 0x09, 0x01, 0x95, 0x01, 0x91, 0x02, 0x85, 0x12,
    0x09, 0x01, 0x95, 0x02, 0x91, 0x02, 0x85, 0x13, 0x09, 0x01, 0x95, 0x01, 0x91, 0x02, 0x85, 0x15,
    0x09, 0x01, 0x95, 0x01, 0x91, 0x02, 0x85, 0x16, 0x09, 0x01, 0x95, 0x15, 0x91, 0x02, 0x85, 0x17,
    0x09, 0x01, 0x95, 0x06, 0x91, 0x02, 0x85, 0x1a, 0x09, 0x01, 0x95, 0x01, 0x91, 0x02, 0xc0,
];
pub(super) fn hid(revision: WiiRemoteRevision) -> NativeControllerRealization {
    let mut realization = crate::common::hid_realization(
        if revision == WiiRemoteRevision::Tr {
            "Nintendo RVL-CNT-01-TR"
        } else {
            "Nintendo RVL-CNT-01"
        },
        0x057e,
        if revision == WiiRemoteRevision::Tr {
            0x0330
        } else {
            0x0306
        },
    );
    let NativeControllerRealization::Uhid(spec) = &mut realization else {
        unreachable!()
    };
    spec.target = RealizationTarget::LINUX_UHID_BLUETOOTH;
    spec.bus_type = 5;
    spec.identity.version = 0;
    spec.descriptor = DESCRIPTOR.to_vec();
    spec.numbered_input_reports = true;
    spec.numbered_output_reports = true;
    realization
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_native_node_matches_the_independently_extracted_kernel_capability_set() {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/protocol-research/wii-c5-evdev-golden.json"
        ))
        .unwrap();
        for (index, role) in [
            Role::Core,
            Role::Accel,
            Role::Ir,
            Role::Nunchuk,
            Role::Classic,
            Role::MotionPlus,
        ]
        .into_iter()
        .enumerate()
        {
            let NativeControllerRealization::Evdev(spec) =
                specification(role, WiiRemoteRevision::Original)
            else {
                unreachable!()
            };
            let source = &golden["manifest"]["sources"][index];
            assert_eq!(spec.device_name, source["reported_name"].as_str().unwrap());
            assert_eq!(
                u64::from(spec.identity.vendor_id),
                source["vendor_id"].as_u64().unwrap()
            );
            assert_eq!(
                u64::from(spec.identity.product_id),
                source["product_id"].as_u64().unwrap()
            );
            let expected: Vec<_> = golden["controls"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["source_index"] == index)
                .collect();
            let actual_count =
                spec.key_codes.len() + spec.absolute_axes.len() + spec.force_feedback_codes.len();
            assert_eq!(actual_count, expected.len());
            for c in expected {
                let code = u16::try_from(c["code"].as_u64().unwrap()).unwrap();
                match c["event_type"].as_u64().unwrap() {
                    1 => assert!(spec.key_codes.contains(&code)),
                    21 => assert!(spec.force_feedback_codes.contains(&code)),
                    3 => {
                        let a = spec.absolute_axes.iter().find(|a| a.code == code).unwrap();
                        assert_eq!(
                            i64::from(a.minimum),
                            c["absolute_range"]["minimum"].as_i64().unwrap()
                        );
                        assert_eq!(
                            i64::from(a.maximum),
                            c["absolute_range"]["maximum"].as_i64().unwrap()
                        );
                        assert_eq!(
                            i64::from(a.flat),
                            c["absolute_range"]["flat"].as_i64().unwrap()
                        );
                    }
                    _ => panic!("unmodelled control"),
                }
            }
            assert!(
                spec.relative_axes.is_empty()
                    && spec.led_codes.is_empty()
                    && spec.switch_codes.is_empty()
            );
        }
    }
    #[test]
    fn bluetooth_identity_numbering_and_synthetic_descriptor_are_exact() {
        for (revision, name, pid) in [
            (WiiRemoteRevision::Original, "Nintendo RVL-CNT-01", 0x0306),
            (WiiRemoteRevision::Tr, "Nintendo RVL-CNT-01-TR", 0x0330),
        ] {
            let NativeControllerRealization::Uhid(s) = hid(revision) else {
                unreachable!()
            };
            assert_eq!(s.target, RealizationTarget::LINUX_UHID_BLUETOOTH);
            assert_eq!(s.bus_type, 5);
            assert_eq!(s.device_name, name);
            assert_eq!(s.identity.product_id, pid);
            assert!(
                s.numbered_input_reports
                    && s.numbered_output_reports
                    && !s.numbered_feature_reports
            );
            let expected: Vec<_> = include_str!(
                "../../../../tests/fixtures/protocol-research/wii-c5-functional-descriptor.hex"
            )
            .split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).unwrap())
            .collect();
            assert_eq!(s.descriptor, expected);
        }
    }
    #[test]
    fn native_mapping_retains_all_keys_and_clips_only_at_declared_consumer_ranges() {
        let mut s = WiiRemoteState {
            buttons: [true; 11],
            ..Default::default()
        };
        let ProviderFrame::Evdev(events) = frame(Role::Core, &s) else {
            unreachable!()
        };
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == 1 && e.value == 1)
                .count(),
            11
        );
        s.extension = WiiExtension::Classic(super::super::WiiClassicState {
            left: [0, 63],
            right: [0, 31],
            triggers: [3, 7],
            buttons: [true; 15],
        });
        let ProviderFrame::Evdev(events) = frame(Role::Classic, &s) else {
            unreachable!()
        };
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == 1 && e.value == 1)
                .count(),
            15
        );
        let axes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == 3)
            .map(|e| e.value)
            .collect();
        assert_eq!(axes, [-30, 30, -30, 30, 14, 6]);
        s.motion_plus.gyroscope = [8193, 8194, 8195];
        s.motion_plus.slow = [true; 3];
        let ProviderFrame::Evdev(events) = frame(Role::MotionPlus, &s) else {
            unreachable!()
        };
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == 3)
                .map(|e| e.value)
                .collect::<Vec<_>>(),
            [27, 18, 9]
        );
    }
}
