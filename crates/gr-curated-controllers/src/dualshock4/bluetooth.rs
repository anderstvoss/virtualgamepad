//! Source-derived Bluetooth-bus UHID framing for `DualShock` 4.
//! Not a radio emulator, a physical descriptor capture or device fidelity proof.
use super::{DualShock4SessionState, ds4_frame};
use gr_hid::{ReplyError, Report, ReportType};
use gr_realization_api::{
    NativeControllerRealization, NativeDeviceIdentity, NativeHidRealization, ProviderFrame,
    RealizationTarget,
};

/// Synthetic report shape compatible with the pinned Linux/SDL consumer fields,
/// not a captured retail HID descriptor.
pub(super) const DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x30, 0x75, 0x08, 0x95, 0x09, 0x81, 0x02,
    0x06, 0x00, 0xff, 0x09, 0x01, 0x85, 0x11, 0x75, 0x08, 0x95, 0x4d, 0x81, 0x02, 0x85, 0x11, 0x75,
    0x08, 0x95, 0x4d, 0x91, 0x02, 0x85, 0x02, 0x75, 0x08, 0x95, 0x24, 0xb1, 0x02, 0x85, 0x05, 0x75,
    0x08, 0x95, 0x28, 0xb1, 0x02, 0x85, 0xa3, 0x75, 0x08, 0x95, 0x30, 0xb1, 0x02, 0xc0,
];

pub(super) fn realization() -> NativeControllerRealization {
    NativeControllerRealization::Uhid(NativeHidRealization {
        target: RealizationTarget::LINUX_UHID_BLUETOOTH,
        bus_type: 5,
        device_name: "Wireless Controller".into(),
        physical_path: "virtualgamepad/uhid/dualshock4".into(),
        unique_id: "virtualgamepad-dualshock4".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x054c,
            product_id: 0x05c4,
            version: 0x0120,
        },
        descriptor: DESCRIPTOR.to_vec(),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: true,
    })
}

/// Reflected IEEE CRC-32 with the Sony HIDP report-kind seed included.
fn crc32(seed: u8, bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in std::iter::once(seed).chain(bytes.iter().copied()) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (if crc & 1 == 1 { 0xedb8_8320 } else { 0 });
        }
    }
    !crc
}

pub(super) fn minimal_frame(state: &DualShock4SessionState) -> ProviderFrame {
    let ProviderFrame::HidInput { bytes, .. } = ds4_frame(state) else {
        unreachable!("DualShock 4 USB encoder produces a numbered HID frame");
    };
    ProviderFrame::HidInput {
        report_id: Some(0x01),
        bytes: bytes[..9].to_vec(),
    }
}

pub(super) fn full_frame(state: &DualShock4SessionState) -> ProviderFrame {
    let ProviderFrame::HidInput { bytes, .. } = ds4_frame(state) else {
        unreachable!("DualShock 4 USB encoder produces a numbered HID frame");
    };
    // 77 bytes excluding report ID: two BT prefix bytes, 32 common state
    // bytes, four nine-byte touch history slots, two reserved, four CRC.
    let mut payload = vec![0_u8; 77];
    payload[0] = 0xc0; // HID payload present
    payload[2..34].copy_from_slice(&bytes[..32]);
    // Wireless DS4 capacity is deciles without USB's cable-connected bit.
    payload[31] = (state.status.battery().level().percent() / 10).min(10);
    payload[34..44].copy_from_slice(&bytes[32..42]);
    let mut checksum_data = Vec::with_capacity(74);
    checksum_data.push(0x11);
    checksum_data.extend_from_slice(&payload[..73]);
    payload[73..].copy_from_slice(&crc32(0xa1, &checksum_data).to_le_bytes());
    ProviderFrame::HidInput {
        report_id: Some(0x11),
        bytes: payload,
    }
}

#[allow(clippy::cast_possible_truncation)] // DS4 timing/sequence counters wrap at wire width.
fn encode_at(state: &DualShock4SessionState, now: u64, sequence: u8, full: bool) -> Report {
    let mut wire = state.clone();
    wire.status.sequence = sequence;
    wire.status.sensor_timestamp = ((now / 16).wrapping_mul(3)) as u16;
    wire.status.touch_sequence = sequence;
    super::common::logical_input(if full {
        full_frame(&wire)
    } else {
        minimal_frame(&wire)
    })
}

pub(super) fn minimal_report(state: &DualShock4SessionState, now: u64, sequence: u8) -> Report {
    encode_at(state, now, sequence, false)
}

pub(super) fn full_report(state: &DualShock4SessionState, now: u64, sequence: u8) -> Report {
    encode_at(state, now, sequence, true)
}

pub(super) fn validate_output(report: &Report) -> Result<(), ReplyError> {
    if report.kind != ReportType::Output || report.id() != Some(0x11) {
        return Err(ReplyError::Unsupported);
    }
    let payload = report.payload();
    if payload.len() != 77 {
        return Err(ReplyError::Invalid);
    }
    let mut prefix = Vec::with_capacity(74);
    prefix.push(0x11);
    prefix.extend_from_slice(&payload[..73]);
    let expected = crc32(0xa2, &prefix).to_le_bytes();
    if payload[73..] != expected {
        return Err(ReplyError::Invalid);
    }
    Ok(())
}

pub(super) fn valid_output(payload: &[u8]) -> bool {
    Report::new(ReportType::Output, Some(0x11), payload.to_vec())
        .is_ok_and(|report| validate_output(&report).is_ok())
}

pub(super) fn features(identity: [u8; 6]) -> Vec<(u8, Vec<u8>)> {
    let source = gr_controller_wire::dualshock4_feature_responses(identity);
    let usb_calibration = source
        .iter()
        .find(|(id, _)| *id == 2)
        .expect("compiled calibration")
        .1
        .clone();
    let mut bt_calibration = usb_calibration.clone();
    bt_calibration[0] = 0x05;
    // BT puts all positive gyro limits before all negative limits.
    for (destination, original) in [(7, 7), (9, 11), (11, 15), (13, 9), (15, 13), (17, 17)] {
        bt_calibration[destination..destination + 2]
            .copy_from_slice(&usb_calibration[original..original + 2]);
    }
    let mut data = vec![0x05];
    data.extend_from_slice(&bt_calibration[1..]);
    let checksum = crc32(0xa3, &data);
    data.extend_from_slice(&checksum.to_le_bytes());
    let firmware = source
        .into_iter()
        .find(|(id, _)| *id == 0xa3)
        .expect("compiled firmware");
    vec![(2, usb_calibration), (0x05, data), firmware]
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_realization_api::ProviderFrame;
    #[test]
    fn synthetic_descriptor_and_neutral_reports_match_consumer_lengths() {
        let NativeControllerRealization::Uhid(spec) = realization() else {
            panic!("BT UHID");
        };
        assert_eq!(spec.bus_type, 5);
        assert_eq!(
            (spec.identity.vendor_id, spec.identity.product_id),
            (0x054c, 0x05c4)
        );
        assert_eq!(spec.descriptor, DESCRIPTOR);
        assert_ne!(
            spec.descriptor,
            gr_controller_wire::DUALSHOCK4_USB_DESCRIPTOR
        );
        let state = DualShock4SessionState::default();
        let ProviderFrame::HidInput { report_id, bytes } = minimal_frame(&state) else {
            panic!("HID");
        };
        assert_eq!((report_id, bytes.len()), (Some(1), 9));
        assert_eq!(&bytes[..4], &[128; 4]);
        let ProviderFrame::HidInput { report_id, bytes } = full_frame(&state) else {
            panic!("HID");
        };
        assert_eq!((report_id, bytes.len()), (Some(0x11), 77));
        assert_eq!(bytes[0], 0xc0);
        assert_eq!(bytes[34], 1);
        // Independent synthetic golden checksum for the neutral full frame;
        // not a physical-device packet or a measured report.
        assert_eq!(&bytes[73..], &[0xef, 0x96, 0xe5, 0x8a]);
        let mut prefix = vec![0x11];
        prefix.extend_from_slice(&bytes[..73]);
        assert_eq!(&bytes[73..], &crc32(0xa1, &prefix).to_le_bytes());
    }
    #[test]
    fn bluetooth_battery_level_uses_wireless_deciles() {
        let mut state = DualShock4SessionState::default();
        state.status.battery.set_exposed(true);
        state
            .status
            .battery
            .set_level(crate::BatteryLevel::new(37).unwrap());
        let ProviderFrame::HidInput { bytes, .. } = full_frame(&state) else {
            panic!("BT HID");
        };
        assert_eq!(bytes[31], 3);
        assert_eq!(bytes[31] & 0x10, 0);
    }

    #[test]
    fn feature_calibration_and_host_output_crc() {
        let f = features(super::super::common::test_address(42));
        assert_eq!(
            f.iter()
                .map(|(id, data)| (*id, data.len()))
                .collect::<Vec<_>>(),
            vec![(2, 37), (5, 41), (0xa3, 49)]
        );
        let calibration = &f[1].1;
        assert_eq!(
            &calibration[37..],
            &crc32(0xa3, &calibration[..37]).to_le_bytes()
        );
        let mut payload = vec![0; 77];
        payload[0] = 0xc0;
        payload[2] = 7; // motor, lightbar and blink updates
        payload[5..12].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
        let mut prefix = vec![0x11];
        prefix.extend_from_slice(&payload[..73]);
        payload[73..].copy_from_slice(&crc32(0xa2, &prefix).to_le_bytes());
        let report = Report::new(ReportType::Output, Some(0x11), payload.clone()).unwrap();
        assert_eq!(validate_output(&report), Ok(()));
        payload[5] ^= 1;
        let report = Report::new(ReportType::Output, Some(0x11), payload).unwrap();
        assert_eq!(validate_output(&report), Err(ReplyError::Invalid));
    }
}
