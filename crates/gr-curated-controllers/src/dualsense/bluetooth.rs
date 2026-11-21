//! Source-backed, synthetic Bluetooth-bus UHID `DualSense` framing.
//!
//! Local UHID bus 5 is not a radio, pairing, or `BlueZ` implementation.
//! The HID descriptor is consumer-compatible and NOT a captured retail descriptor.
//! Linux: <https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L76-L95>
//! SDL: <https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/hidapi/SDL_hidapi_ps5.c#L1431-L1475>

use super::{DualSenseSessionState, dualsense_hid_input_report};
use gr_hid::{ReplyError, Report, ReportType};
use gr_realization_api::ProviderFrame;

pub(super) const INPUT_SEED: u8 = 0xa1;
pub(super) const OUTPUT_SEED: u8 = 0xa2;
pub(super) const FEATURE_SEED: u8 = 0xa3;
pub(super) const ENHANCED_ID: u8 = 0x31;
pub(super) const ENHANCED_LENGTH: usize = 78;

// Synthetic HID descriptor for the Linux/SDL consumer report contract.
// Report data lengths exclude each report ID. Only known feature IDs are
// advertised: byte-for-byte retail descriptor fidelity is NOT evidenced.
// The generic vendor usage fields intentionally make no physical control-map
// claim beyond the report sizes and IDs.
pub(super) const DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x06, 0x00, 0xff, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08,
    0x85, 0x01, 0x09, 0x01, 0x95, 0x09, 0x81, 0x02, 0x85, 0x31, 0x09, 0x02, 0x95, 0x4d, 0x81, 0x02,
    0x09, 0x03, 0x95, 0x4d, 0x91, 0x02, 0x85, 0x05, 0x09, 0x04, 0x95, 0x28, 0xb1, 0x02, 0x85, 0x09,
    0x09, 0x05, 0x95, 0x13, 0xb1, 0x02, 0x85, 0x20, 0x09, 0x06, 0x95, 0x3f, 0xb1, 0x02, 0xc0,
];

fn update_crc(mut crc: u32, byte: u8) -> u32 {
    crc ^= u32::from(byte);
    for _ in 0..8 {
        crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
    }
    crc
}

/// CRC-32/IEEE of the HIDP seed byte and report excluding its CRC trailer.
pub(super) fn checksum(seed: u8, report: &[u8]) -> [u8; 4] {
    let mut crc = update_crc(u32::MAX, seed);
    for &byte in report {
        crc = update_crc(crc, byte);
    }
    (!crc).to_le_bytes()
}

pub(super) fn sign(seed: u8, wire: &mut [u8]) {
    let offset = wire.len() - 4;
    let digest = checksum(seed, &wire[..offset]);
    wire[offset..].copy_from_slice(&digest);
}

pub(super) fn valid(seed: u8, wire: &[u8]) -> bool {
    wire.len() >= 5 && wire[wire.len() - 4..] == checksum(seed, &wire[..wire.len() - 4])
}

fn usb_common(state: &DualSenseSessionState) -> Vec<u8> {
    let ProviderFrame::HidInput {
        report_id: Some(1),
        bytes,
    } = dualsense_hid_input_report(state)
    else {
        unreachable!("DualSense common input must use USB report 1");
    };
    bytes
}

/// Simple Bluetooth input is SDL's reduced nine-byte state, not USB report 1.
pub(super) fn simple_input(state: &DualSenseSessionState) -> Report {
    let common = usb_common(state);
    let bytes = vec![
        common[0], common[1], common[2], common[3], common[7], common[8], common[9], common[4],
        common[5],
    ];
    Report::new(ReportType::Input, Some(1), bytes).expect("fixed synthetic report")
}

/// Enhanced Bluetooth input: ID, one prefix byte, common 63-byte state,
/// nine reserved bytes and the four-byte CRC (78 total).
pub(super) fn enhanced_input(state: &DualSenseSessionState) -> Report {
    let mut wire = vec![0u8; ENHANCED_LENGTH];
    wire[0] = ENHANCED_ID;
    wire[2..65].copy_from_slice(&usb_common(state));
    sign(INPUT_SEED, &mut wire);
    Report::from_wire(ReportType::Input, true, &wire).expect("fixed synthetic report")
}

/// Validate the full host Bluetooth output before decoding its common block.
/// Linux uses a three-byte header, SDL uses a two-byte header. Both carry
/// the same 47-byte effects model; neither proves actuator behavior.
pub(super) fn normalized_output(report: &Report) -> Result<Report, ReplyError> {
    if report.kind != ReportType::Output || report.id() != Some(ENHANCED_ID) {
        return Err(ReplyError::Unsupported);
    }
    let wire = report.wire();
    if wire.len() != ENHANCED_LENGTH || !valid(OUTPUT_SEED, &wire) {
        return Err(ReplyError::Invalid);
    }
    let common_start = if wire[1] == 0x02 {
        2 // SDL HIDAPI prefix
    } else if wire[2] == 0x10 {
        3 // Linux hid-playstation tag
    } else {
        return Err(ReplyError::Invalid);
    };
    Report::new(
        ReportType::Output,
        Some(2),
        wire[common_start..common_start + 47].to_vec(),
    )
    .map_err(|_| ReplyError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dualsense::{
        DualSenseAxis, DualSenseControl, DualSenseInputs, DualSenseTouchContact, MotionSample,
        TouchSlot,
    };

    #[test]
    fn synthetic_descriptor_declares_bluetooth_only_report_shapes() {
        // Each size is payload-only; IDs are explicitly numbered.
        for declaration in [
            &[0x85, 0x01, 0x09, 0x01, 0x95, 0x09, 0x81, 0x02][..],
            &[0x85, 0x31, 0x09, 0x02, 0x95, 0x4d, 0x81, 0x02],
            &[0x09, 0x03, 0x95, 0x4d, 0x91, 0x02],
            &[0x85, 0x05, 0x09, 0x04, 0x95, 0x28, 0xb1, 0x02],
            &[0x85, 0x09, 0x09, 0x05, 0x95, 0x13, 0xb1, 0x02],
            &[0x85, 0x20, 0x09, 0x06, 0x95, 0x3f, 0xb1, 0x02],
        ] {
            assert!(
                DESCRIPTOR
                    .windows(declaration.len())
                    .any(|w| w == declaration)
            );
        }
        assert_ne!(DESCRIPTOR, gr_controller_wire::dualsense::USB_DESCRIPTOR);
    }

    #[test]
    fn neutral_and_known_state_use_distinct_simple_and_enhanced_framing() {
        let neutral = DualSenseSessionState::default();
        assert_eq!(
            simple_input(&neutral).wire(),
            [1, 128, 128, 128, 128, 8, 0, 0, 0, 0]
        );
        let enhanced = enhanced_input(&neutral).wire();
        assert_eq!(enhanced.len(), 78);
        assert_eq!(&enhanced[..10], &[0x31, 0, 128, 128, 128, 128, 0, 0, 1, 8]);
        assert!(valid(INPUT_SEED, &enhanced));
        assert_eq!(enhanced[2 + 52], 0x20);

        let mut s = neutral;
        let mut input = DualSenseInputs::default();
        input.set_sticks(
            (DualSenseAxis::new(12), DualSenseAxis::new(34)),
            (DualSenseAxis::new(56), DualSenseAxis::new(78)),
        );
        input.set_native(DualSenseControl::Cross, true);
        input.set_touch(
            TouchSlot::First,
            Some(DualSenseTouchContact::new(3, 100, 200).unwrap()),
        );
        input.set_motion(MotionSample {
            gyroscope: [1, 2, 3],
            accelerometer: [4, 5, 6],
        });
        s.inputs = input;
        let simple = simple_input(&s).wire();
        assert_eq!(simple, [1, 12, 34, 56, 78, 0x28, 0, 0, 0, 0]);
        let full = enhanced_input(&s).wire();
        assert_eq!(&full[..10], &[0x31, 0, 12, 34, 56, 78, 0, 0, 1, 0x28]);
        assert!(valid(INPUT_SEED, &full));
        assert_ne!(full[34], 0x80); // first touch active
    }

    #[test]
    fn output_crc_and_headers_normalize_to_one_common_effect() {
        for start in [2, 3] {
            let mut wire = vec![0; ENHANCED_LENGTH];
            wire[0] = ENHANCED_ID;
            if start == 2 {
                wire[1] = 0x02;
            } else {
                wire[2] = 0x10;
            }
            wire[start] = 0x01; // compatible motor flag
            wire[start + 2] = 13;
            wire[start + 3] = 37;
            wire[start + 1] = 0x14; // RGB and player-indicator validity
            wire[start + 43] = 0x1f;
            wire[start + 44..start + 47].copy_from_slice(&[8, 9, 10]);
            sign(OUTPUT_SEED, &mut wire);
            let r = Report::from_wire(ReportType::Output, true, &wire).unwrap();
            let common = normalized_output(&r).unwrap();
            assert_eq!(common.id(), Some(2));
            assert_eq!(common.payload()[..4], [1, 0x14, 13, 37]);
            assert_eq!(&common.payload()[44..47], &[8, 9, 10]);
            let mut bad = wire;
            bad[12] ^= 0x40;
            let r = Report::from_wire(ReportType::Output, true, &bad).unwrap();
            assert_eq!(normalized_output(&r), Err(ReplyError::Invalid));
        }
    }

    #[test]
    fn seeded_crc_domains_are_distinct() {
        let mut feature = vec![0u8; 41];
        feature[0] = 5;
        sign(FEATURE_SEED, &mut feature);
        assert!(valid(FEATURE_SEED, &feature));
        assert!(!valid(INPUT_SEED, &feature));
        assert!(!valid(OUTPUT_SEED, &feature));
    }
}
