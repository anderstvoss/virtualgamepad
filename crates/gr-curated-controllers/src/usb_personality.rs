//! Workspace implementation interface for trusted USB workers. This is not
//! re-exported by the application crate. Reuse the same controller-owned report
//! policy as UHID; a transport must not duplicate feature or output semantics.
use crate::{common::HidDriver, dualsense, dualshock4, xbox360};
use gr_hid::Protocol;
use gr_realization_api::{RawReverseEvent, RealizationSessionId};

#[must_use]
pub fn dualsense(
    identity: [u8; 6],
) -> impl Protocol<State = dualsense::DualSenseState, Output = RawReverseEvent> {
    dualsense::DualSenseDefinition.hid_protocol(RealizationSessionId(0), identity)
}
#[must_use]
pub fn dualshock4(
    identity: [u8; 6],
) -> impl Protocol<State = dualshock4::DualShock4State, Output = RawReverseEvent> {
    dualshock4::DualShock4Definition.hid_protocol(RealizationSessionId(0), identity)
}
#[must_use]
pub fn xbox360() -> impl Protocol<State = xbox360::Xbox360State, Output = RawReverseEvent> {
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
}
