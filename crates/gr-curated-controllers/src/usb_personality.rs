//! Workspace implementation interface for trusted USB workers. This is not
//! re-exported by the application crate. Reuse the same controller-owned report
//! policy as UHID; a transport must not duplicate feature or output semantics.
use crate::{common::HidDriver, dualsense, dualshock4, xbox360};
use gr_hid::Protocol;
use gr_realization_api::{RawReverseEvent, RealizationSessionId};

pub mod outputs;
use crate::retained::RetainedProtocol;
use outputs::WorkerOutputs;

/// Controller protocol used by the installed USB worker.
pub trait WorkerProtocol: Protocol {
    fn retained_outputs(&self) -> WorkerOutputs;
}

macro_rules! retained_protocol {
    ($protocol:ty, $family:ident) => {
        impl WorkerProtocol for $protocol {
            fn retained_outputs(&self) -> WorkerOutputs {
                WorkerOutputs::$family(self.retained().clone())
            }
        }
    };
}
retained_protocol!(dualsense::DualSenseUsbProtocol, DualSense);
retained_protocol!(
    crate::common::SnapshotProtocol<dualshock4::DualShock4SessionState, crate::retained::DualShock4OutputState>,
    DualShock4
);
retained_protocol!(
    crate::common::SnapshotProtocol<xbox360::Xbox360SessionState, crate::retained::Xbox360OutputState>,
    Xbox360
);

#[must_use]
pub fn dualsense(
    identity: [u8; 6],
) -> impl WorkerProtocol<State = dualsense::DualSenseSessionState, Output = RawReverseEvent> {
    dualsense::DualSenseDefinition::default().hid_protocol(RealizationSessionId(0), identity)
}

/// Pure protocol test seam; USB/IP worker creation accepts synthetic data only.
#[cfg(test)]
#[must_use]
fn dualsense_with_device_data(
    identity: [u8; 6],
    data: dualsense::DualSenseDeviceData,
) -> impl WorkerProtocol<State = dualsense::DualSenseSessionState, Output = RawReverseEvent> {
    dualsense::DualSenseUsbProtocol::with_device_data(identity, Some(data))
}

#[must_use]
pub fn dualshock4(
    identity: [u8; 6],
) -> impl WorkerProtocol<State = dualshock4::DualShock4SessionState, Output = RawReverseEvent> {
    dualshock4::DualShock4Definition::default().hid_protocol(RealizationSessionId(0), identity)
}

/// Pure protocol test seam; not an installed USB/IP worker configuration API.
#[cfg(test)]
#[must_use]
fn dualshock4_with_device_data(
    identity: [u8; 6],
    data: dualshock4::DualShock4DeviceData,
) -> impl WorkerProtocol<State = dualshock4::DualShock4SessionState, Output = RawReverseEvent> {
    dualshock4::DualShock4Definition::configured(data)
        .hid_protocol(RealizationSessionId(0), identity)
}

#[must_use]
pub fn xbox360()
-> impl WorkerProtocol<State = xbox360::Xbox360SessionState, Output = RawReverseEvent> {
    xbox360::Xbox360Definition.hid_protocol(RealizationSessionId(0), [0; 6])
}

#[cfg(test)]
mod tests {
    use super::*;
    use gr_hid::{Reply, ReplyError, Report, ReportType, RequestKind};

    fn feature(protocol: &mut impl Protocol, id: u8) -> Report {
        let (Reply::Get(Ok(report)), _) = protocol.request(
            &RequestKind::Get {
                kind: ReportType::Feature,
                id: Some(id),
            },
            0,
        ) else {
            panic!("compiled identity report missing")
        };
        report
    }
    #[test]
    fn worker_factories_reuse_controller_identity_and_report_policy() {
        let mut ds = dualsense([2, 1, 2, 3, 4, 5]);
        let mut ds_other = dualsense([2, 1, 2, 3, 4, 6]);
        assert_ne!(feature(&mut ds, 9), feature(&mut ds_other, 9));
        let mut ds4 = dualshock4([2, 1, 2, 3, 4, 5]);
        let mut alternate_ds4 = dualshock4([2, 1, 2, 3, 4, 6]);
        assert_ne!(feature(&mut ds4, 0x12), feature(&mut alternate_ds4, 0x12));
        for (reply, _) in [
            ds.request(
                &RequestKind::Set(Report::new(ReportType::Output, Some(2), vec![0; 47]).unwrap()),
                0,
            ),
            ds4.request(
                &RequestKind::Set(Report::new(ReportType::Output, Some(5), vec![0; 31]).unwrap()),
                0,
            ),
        ] {
            assert_eq!(reply, Reply::Set(Ok(())));
        }
        let mut xbox = xbox360();
        let reports = xbox.input(&xbox.neutral(), 0).unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].id(), None);
        let (reply, _) = xbox.request(
            &RequestKind::Get {
                kind: ReportType::Feature,
                id: None,
            },
            0,
        );
        assert_eq!(reply, Reply::Get(Err(ReplyError::Unsupported)));
    }

    #[test]
    fn sony_synthetic_usb_baseline_matches_compiled_wire_fixtures() {
        let identity = crate::common::test_address(7);
        let pairing: [u8; 5] = identity[1..6].try_into().unwrap();
        let ds = gr_controller_wire::dualsense::feature_responses(pairing);
        let ds4 = gr_controller_wire::dualshock4_feature_responses(identity);
        let mut sense = dualsense(identity);
        let mut shock = dualshock4(identity);
        for (id, expected) in ds {
            assert_eq!(feature(&mut sense, id).payload(), &expected[1..]);
        }
        for (id, expected) in ds4 {
            assert_eq!(feature(&mut shock, id).payload(), &expected[1..]);
        }
    }

    #[test]
    fn sony_supplied_usb_reports_survive_recreation_and_pairing_stays_separate() {
        use crate::{dualsense::DualSenseDeviceData, dualshock4::DualShock4DeviceData};
        let identity = crate::common::test_address(13);
        let mut cal = DualSenseDeviceData::default()
            .feature_report(0x05)
            .unwrap()
            .to_vec();
        let mut fw = DualSenseDeviceData::default()
            .feature_report(0x20)
            .unwrap()
            .to_vec();
        cal[3] = 17;
        fw[28] = 3;
        let data = DualSenseDeviceData::from_feature_reports(&cal, &fw).unwrap();
        let mut original = dualsense_with_device_data(identity, data);
        assert_eq!(feature(&mut original, 0x05).payload(), &cal[1..]);
        assert_eq!(feature(&mut original, 0x20).payload(), &fw[1..]);
        // Recreating a session from the same typed, validated value is exact.
        let mut restored = dualsense_with_device_data(identity, data);
        assert_eq!(feature(&mut restored, 0x05).payload(), &cal[1..]);
        assert_eq!(feature(&mut restored, 0x20).payload(), &fw[1..]);
        assert_eq!(feature(&mut original, 0x09), feature(&mut restored, 0x09));

        let mut cal4 = DualShock4DeviceData::default()
            .feature_report(0x02)
            .unwrap()
            .to_vec();
        let mut fw4 = DualShock4DeviceData::default()
            .feature_report(0xa3)
            .unwrap()
            .to_vec();
        cal4[3] = 19;
        fw4[2] = 23;
        let data4 = DualShock4DeviceData::from_feature_reports(&cal4, &fw4).unwrap();
        let mut first = dualshock4_with_device_data(identity, data4);
        let mut second = dualshock4_with_device_data(identity, data4);
        for (id, expected) in [(0x02, &cal4), (0xa3, &fw4)] {
            assert_eq!(feature(&mut first, id).payload(), &expected[1..]);
            assert_eq!(feature(&mut second, id).payload(), &expected[1..]);
        }
        assert_eq!(feature(&mut first, 0x12), feature(&mut second, 0x12));
    }

    #[test]
    fn sony_invalid_profiles_are_rejected_before_any_provider_is_opened() {
        use crate::{dualsense::DualSenseDeviceData, dualshock4::DualShock4DeviceData};
        let ds = DualSenseDeviceData::default();
        let cal = ds.feature_report(0x05).unwrap().to_vec();
        let fw = ds.feature_report(0x20).unwrap().to_vec();
        assert!(DualSenseDeviceData::from_feature_reports(&cal[..40], &fw).is_err());
        assert!(ds.with_feature_report(0x09, &[]).is_err());
        assert!(ds.with_feature_report(0x05, &fw).is_err());
        let mut bad = cal.clone();
        bad[7] = bad[9];
        bad[8] = bad[10];
        assert!(ds.with_feature_report(0x05, &bad).is_err());
        assert_eq!(ds.feature_report(0x05), Some(cal.as_slice()));
        let ds4 = DualShock4DeviceData::default();
        let cal4 = ds4.feature_report(0x02).unwrap().to_vec();
        let fw4 = ds4.feature_report(0xa3).unwrap().to_vec();
        assert!(DualShock4DeviceData::from_feature_reports(&cal4, &fw4[..48]).is_err());
        assert!(ds4.with_feature_report(0x12, &[]).is_err());
        assert!(ds4.with_feature_report(0x81, &[]).is_err());
        let mut bad4 = cal4.clone();
        bad4[19] = 0;
        bad4[20] = 0;
        bad4[21] = 0;
        bad4[22] = 0;
        assert!(ds4.with_feature_report(0x02, &bad4).is_err());
    }

    #[test]
    fn sony_runtime_files_round_trip_synthetic_reports_only() {
        use crate::{dualsense::DualSenseDeviceData, dualshock4::DualShock4DeviceData};
        let path = std::env::temp_dir().join(format!(
            "vg-p10b-synthetic-{}-sense.sonydata",
            std::process::id()
        ));
        let ds = DualSenseDeviceData::default();
        ds.save_file(&path).unwrap();
        assert_eq!(DualSenseDeviceData::load_file(&path).unwrap(), ds);
        assert!(DualShock4DeviceData::load_file(&path).is_err());
        std::fs::remove_file(&path).unwrap();

        let path = std::env::temp_dir().join(format!(
            "vg-p10b-synthetic-{}-shock.sonydata",
            std::process::id()
        ));
        let ds4 = DualShock4DeviceData::default();
        ds4.save_file(&path).unwrap();
        assert_eq!(DualShock4DeviceData::load_file(&path).unwrap(), ds4);
        assert!(DualSenseDeviceData::load_file(&path).is_err());
        std::fs::remove_file(&path).unwrap();
    }
}

/// Implementation-only native state transactions for the installed worker.
pub mod state;
