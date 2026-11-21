//! Synthetic-only P10b device-data regression tests. No provider or hardware access.
use virtualgamepad::{
    ControllerError, CreationOptions, DualSenseDeviceData, DualSenseOptions,
    DualShock4DeviceData, DualShock4Options, RealizationId, create_dualsense,
    create_dualshock4,
};

#[test]
fn sony_reports_are_validated_and_retained_without_pairing_identity() {
    let ds = DualSenseDeviceData::default();
    let mut ds_firmware = ds.feature_report(0x20).unwrap().to_vec();
    ds_firmware[10] = 0x5a;
    let supplied = ds.with_feature_report(0x20, &ds_firmware).unwrap();
    assert_eq!(supplied.feature_report(0x20), Some(ds_firmware.as_slice()));
    assert_eq!(supplied.feature_report(0x05), ds.feature_report(0x05));
    assert!(supplied.feature_report(0x09).is_none());

    let ds4 = DualShock4DeviceData::default();
    let mut ds4_firmware = ds4.feature_report(0xa3).unwrap().to_vec();
    ds4_firmware[10] = 0x5a;
    let supplied = ds4.with_feature_report(0xa3, &ds4_firmware).unwrap();
    assert_eq!(supplied.feature_report(0xa3), Some(ds4_firmware.as_slice()));
    assert_eq!(supplied.feature_report(0x02), ds4.feature_report(0x02));
    assert!(supplied.feature_report(0x12).is_none());
}

#[test]
fn sony_reports_reject_malformed_ids_lengths_and_zero_denominators() {
    let ds = DualSenseDeviceData::default();
    assert!(ds.with_feature_report(0x09, &[0x09]).is_err());
    assert!(ds.with_feature_report(0x20, &[0x20]).is_err());
    let mut cal = ds.feature_report(0x05).unwrap().to_vec();
    cal[19..23].fill(0);
    assert!(ds.with_feature_report(0x05, &cal).is_err());

    let ds4 = DualShock4DeviceData::default();
    assert!(ds4.with_feature_report(0x12, &[0x12]).is_err());
    assert!(ds4.with_feature_report(0xa3, &[0xa3]).is_err());
    let mut cal = ds4.feature_report(0x02).unwrap().to_vec();
    cal[19..23].fill(0);
    assert!(ds4.with_feature_report(0x02, &cal).is_err());
}

#[test]
fn caller_selected_files_roundtrip_without_identity() {
    let ds = DualSenseDeviceData::default();
    let path = std::env::temp_dir().join(format!(
        "vg-p10b-dualsense-{}.sonydata",
        std::process::id()
    ));
    ds.save_file(&path).unwrap();
    assert_eq!(DualSenseDeviceData::load_file(&path).unwrap(), ds);
    assert!(DualShock4DeviceData::load_file(&path).is_err());
    std::fs::remove_file(path).unwrap();

    let ds4 = DualShock4DeviceData::default();
    let path = std::env::temp_dir().join(format!(
        "vg-p10b-dualshock4-{}.sonydata",
        std::process::id()
    ));
    ds4.save_file(&path).unwrap();
    assert_eq!(DualShock4DeviceData::load_file(&path).unwrap(), ds4);
    assert!(DualSenseDeviceData::load_file(&path).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn explicit_sony_device_data_is_typed_unsupported_on_usbip_without_opening_worker() {
    let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO);
    let ds = create_dualsense(
        options,
        DualSenseOptions::default().with_device_data(DualSenseDeviceData::default()),
    );
    assert!(matches!(
        ds,
        Err(ControllerError::Unsupported { reason })
            if reason.contains("DualSense") && reason.contains("USB/IP")
    ));

    let ds4 = create_dualshock4(
        options,
        DualShock4Options::default().with_device_data(DualShock4DeviceData::default()),
    );
    assert!(matches!(
        ds4,
        Err(ControllerError::Unsupported { reason })
            if reason.contains("DualShock 4") && reason.contains("USB/IP")
    ));
}
