//! Controller-owned native interfaces. Record: wii-c5-evdev-consumer-shape.
//! uinput cannot carry bus5 or fuzz; these deviations are surfaced and tested.
use super::{RuntimeState, WiiExtension, WiiRemoteRevision};
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
        // hid-wiimote nodes sit on the Bluetooth HID device, so the emulated
        // native view advertises BUS_BLUETOOTH like the real remote does.
        bus_type: 0x05,
        physical_path: None,
        device_name: name.into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x057e,
            product_id: if revision == WiiRemoteRevision::Tr {
                0x0330
            } else {
                0x0306
            },
            version: native_version(revision),
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
pub(super) fn roles(state: &RuntimeState) -> Vec<Role> {
    let mut roles = vec![Role::Accel, Role::Ir];
    if state.motion_plus_attached() {
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
// until this explicit consumer conversion. Advertised ABS limits are not clamps.
// Record: wii-c5-evdev-native-values (source-only decoder/input-core semantics).
pub(super) fn frame(role: Role, state: &RuntimeState) -> ProviderFrame {
    let NativeControllerRealization::Evdev(spec) = specification(role, state.revision()) else {
        unreachable!()
    };
    let (keys, values): (Vec<bool>, Vec<i32>) = match role {
        Role::Core => (state.inputs.buttons.to_vec(), vec![]),
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
                    let (x, y) = p.map_or((1023, 1023), super::WiiIrPoint::coordinates);
                    [i32::from(x), i32::from(y)]
                })
                .collect(),
        ),
        Role::Nunchuk => {
            let n = match state.extension() {
                WiiExtension::Nunchuk(n) => n.clone(),
                _ => super::WiiNunchukInputs::default(),
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
                _ => super::WiiClassicInputs::default(),
            };
            // Consumer keys follow the native button array's order.
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
                value: v,
            }),
    );
    events.push(EvdevEvent {
        event_type: 0,
        code: 0,
        value: 0,
    });
    ProviderFrame::Evdev(events)
}
// P6: byte-for-byte retail RVL-036/RVL-CNT-01 HID descriptor captured in
// wii-p6-retail-plus-rvl-cnt-01-descriptor-observed and compared in
// wii-p6-retail-plus-rvl-cnt-01-descriptor-comparison. The compiled byte array
// is generated from the pinned physical research fixture, not transcribed.
// Reuse for Original and TR is a compatibility choice, not retail validation
// of either unmeasured revision. W8 speaker controls/data decode as before;
// speaker audio playback remains refused.
pub(super) const DESCRIPTOR: &[u8; 217] = &include!(
    "../../../../tests/fixtures/protocol-research/wii-p6-retail-plus-rvl-cnt-01-descriptor.rs"
);
/// HID/input version reported by a real RVL-CNT-01 over Bluetooth, measured on
/// one retail Original remote (#158 comment 7061259225). It is a model-level
/// value, not a per-unit identifier. No RVL-CNT-01-TR value has been measured,
/// so that revision keeps 0.
const ORIGINAL_NATIVE_VERSION: u16 = 0x8601;
fn native_version(revision: WiiRemoteRevision) -> u16 {
    if revision == WiiRemoteRevision::Tr {
        0
    } else {
        ORIGINAL_NATIVE_VERSION
    }
}
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
    spec.identity.version = native_version(revision);
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
    fn plus_native_roles_preserve_all_source_controls_with_plus_identity() {
        for role in [
            Role::Core,
            Role::Accel,
            Role::Ir,
            Role::MotionPlus,
            Role::Nunchuk,
            Role::Classic,
        ] {
            let NativeControllerRealization::Evdev(plus) =
                specification(role, WiiRemoteRevision::Tr)
            else {
                unreachable!()
            };
            let NativeControllerRealization::Evdev(original) =
                specification(role, WiiRemoteRevision::Original)
            else {
                unreachable!()
            };
            assert_eq!(plus.identity.product_id, 0x0330);
            assert_eq!(plus.identity.vendor_id, 0x057e);
            assert_eq!(plus.key_codes, original.key_codes);
            assert_eq!(plus.absolute_axes, original.absolute_axes);
            assert_eq!(plus.force_feedback_codes, original.force_feedback_codes);
        }
    }

    #[test]
    fn bluetooth_identity_numbering_and_retail_descriptor_are_exact() {
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
            assert_eq!(s.descriptor.len(), 217);
        }
    }
    #[test]
    fn bluetooth_descriptor_equals_pinned_retail_research_fixture() {
        // Research fixture wii-p6-retail-plus-rvl-cnt-01-descriptor, obtained
        // from the captured unit; no bytes originate from the interim model.
        let expected: Vec<u8> = include_str!(
            "../../../../tests/fixtures/protocol-research/wii-p6-retail-plus-rvl-cnt-01-descriptor.hex"
        )
        .split_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).unwrap())
        .collect();
        assert_eq!(expected.len(), 217);
        assert_eq!(&DESCRIPTOR[..], expected.as_slice());
        for revision in [WiiRemoteRevision::Original, WiiRemoteRevision::Tr] {
            let NativeControllerRealization::Uhid(spec) = hid(revision) else {
                unreachable!()
            };
            assert_eq!(spec.descriptor.as_slice(), expected.as_slice());
        }
    }
    #[test]
    fn native_mapping_retains_all_keys_and_source_conversions() {
        let mut s = RuntimeState {
            inputs: crate::wii_remote::WiiRemoteInputs {
                buttons: [true; 11],
                ..Default::default()
            },
            revision: super::WiiRemoteRevision::default(),
            motion_plus_attached: false,
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
        s.inputs.extension = WiiExtension::Classic(super::super::WiiClassicInputs {
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
        assert_eq!(axes, [-32, 31, -32, 30, 14, 6]);
        s.inputs.motion_plus.gyroscope = [8193, 8194, 8195];
        s.inputs.motion_plus.slow = [true; 3];
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

    #[test]
    fn native_consumer_values_are_not_clipped_to_advertised_abs_bounds() {
        fn values(role: Role, state: &RuntimeState) -> Vec<i32> {
            let ProviderFrame::Evdev(events) = frame(role, state) else {
                unreachable!()
            };
            events
                .into_iter()
                .filter(|e| e.event_type == 3)
                .map(|e| e.value)
                .collect()
        }
        let mut state = RuntimeState::default();
        state.set_accelerometer([0, 512, 1023]).unwrap();
        assert_eq!(values(Role::Accel, &state), [-512, 0, 511]);
        state.inputs.extension = WiiExtension::Nunchuk(super::super::WiiNunchukInputs {
            stick: [0, 255],
            accelerometer: [0, 512, 1023],
            ..Default::default()
        });
        assert_eq!(values(Role::Nunchuk, &state), [-128, 127, -512, 0, 511]);
        state.inputs.extension = WiiExtension::Classic(super::super::WiiClassicInputs {
            left: [0, 63],
            right: [0, 31],
            triggers: [31, 30],
            ..Default::default()
        });
        assert_eq!(values(Role::Classic, &state), [-32, 31, -32, 30, 60, 62]);
        state.inputs.motion_plus.gyroscope = [0, 8192, 16383];
        state.inputs.motion_plus.slow = [true; 3];
        assert_eq!(values(Role::MotionPlus, &state), [73719, 0, -73728]);
    }
    #[test]
    fn native_ir_missing_sentinel_keeps_all_slots_distinct_from_visible_edges() {
        for slot in 0..4 {
            let mut state = RuntimeState::default();
            let mut points = [None; 4];
            points[slot] = Some(super::super::WiiIrPoint::new(1023, 767).unwrap());
            state.set_ir(points);
            let ProviderFrame::Evdev(events) = frame(Role::Ir, &state) else {
                unreachable!()
            };
            let values: Vec<_> = events
                .iter()
                .filter(|e| e.event_type == 3)
                .map(|e| e.value)
                .collect();
            for i in 0..4 {
                assert_eq!(values[i * 2], 1023);
                assert_eq!(values[i * 2 + 1], if i == slot { 767 } else { 1023 });
            }
        }
    }
}

#[cfg(test)]
mod w2_tests;
