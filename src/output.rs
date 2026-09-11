//! Controller-native observations. Required protocol requests never belong to callers.
use crate::{
    ControllerError, DualSenseHidOutput, DualShock4HidOutput, ForceFeedbackEvent, SwitchProRumble,
};

/// Host lifecycle observation, not controller destruction or automatic reopening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostLifecycle {
    Started,
    Stopped,
    Opened,
    Closed,
}
fn lifecycle(event: gr_hid::Lifecycle) -> HostLifecycle {
    match event {
        gr_hid::Lifecycle::Start { .. } => HostLifecycle::Started,
        gr_hid::Lifecycle::Stop => HostLifecycle::Stopped,
        gr_hid::Lifecycle::Open => HostLifecycle::Opened,
        gr_hid::Lifecycle::Close => HostLifecycle::Closed,
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DualSenseOutputEvent {
    HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    HidOutput(DualSenseHidOutput),
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DualShock4OutputEvent {
    HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    HidOutput(DualShock4HidOutput),
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Xbox360OutputEvent {
    HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    /// Native output payload retained without claiming an XUSB interpretation.
    HidOutput {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SwitchProOutputEvent {
    HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    /// Native payload retains modes/subcommands beyond the compressed motor words.
    Output {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
}
impl SwitchProOutputEvent {
    /// Extract encoded motor words; amplitude/frequency decoding is unvalidated.
    #[must_use]
    pub fn rumble(&self) -> Option<SwitchProRumble> {
        let Self::Output { report_id, bytes } = self else {
            return None;
        };
        gr_curated_controllers::SwitchProOutputEvent::Output {
            report_id: *report_id,
            bytes: bytes.clone(),
        }
        .rumble()
    }
}
fn request_leaked() -> ControllerError {
    ControllerError::InvalidRequest {
        reason: "required protocol request escaped controller ownership; session closed".into(),
    }
}
pub(crate) fn dualsense(
    event: gr_curated_controllers::DualSenseOutputEvent,
) -> Result<Option<DualSenseOutputEvent>, ControllerError> {
    use gr_curated_controllers::DualSenseOutputEvent as E;
    Ok(match event {
        E::HidLifecycle(e) => Some(DualSenseOutputEvent::HostLifecycle(lifecycle(e))),
        E::ForceFeedback(e) => Some(DualSenseOutputEvent::ForceFeedback(e)),
        E::HidOutput(e) => Some(DualSenseOutputEvent::HidOutput(e)),
        E::HidGetReportRequest { .. } | E::HidSetReportRequest { .. } => {
            return Err(request_leaked());
        }
        E::Other | E::ProviderEvent(_) => None,
    })
}
pub(crate) fn dualshock4(
    event: gr_curated_controllers::DualShock4OutputEvent,
) -> Result<Option<DualShock4OutputEvent>, ControllerError> {
    use gr_curated_controllers::DualShock4OutputEvent as E;
    Ok(match event {
        E::HidLifecycle(e) => Some(DualShock4OutputEvent::HostLifecycle(lifecycle(e))),
        E::ForceFeedback(e) => Some(DualShock4OutputEvent::ForceFeedback(e)),
        E::HidOutput(e) => Some(DualShock4OutputEvent::HidOutput(e)),
        E::HostRequest { .. } => return Err(request_leaked()),
        E::Other | E::ProviderEvent(_) => None,
    })
}
pub(crate) fn switch_pro(
    event: gr_curated_controllers::SwitchProOutputEvent,
) -> Result<Option<SwitchProOutputEvent>, ControllerError> {
    use gr_curated_controllers::SwitchProOutputEvent as E;
    Ok(match event {
        E::HidLifecycle(e) => Some(SwitchProOutputEvent::HostLifecycle(lifecycle(e))),
        E::ForceFeedback(e) => Some(SwitchProOutputEvent::ForceFeedback(e)),
        E::Output { report_id, bytes } => Some(SwitchProOutputEvent::Output { report_id, bytes }),
        E::HostRequest { .. } => return Err(request_leaked()),
        E::Other | E::ProviderEvent(_) => None,
    })
}
pub(crate) fn xbox360(
    event: gr_curated_controllers::Xbox360OutputEvent,
) -> Result<Option<Xbox360OutputEvent>, ControllerError> {
    use gr_curated_controllers::Xbox360OutputEvent as E;
    Ok(match event {
        E::HidLifecycle(e) => Some(Xbox360OutputEvent::HostLifecycle(lifecycle(e))),
        E::ForceFeedback(e) => Some(Xbox360OutputEvent::ForceFeedback(e)),
        E::HidOutput { report_id, bytes } => {
            Some(Xbox360OutputEvent::HidOutput { report_id, bytes })
        }
        E::HidGetReportRequest { .. } | E::HidSetReportRequest { .. } => {
            return Err(request_leaked());
        }
        E::Other | E::ProviderEvent(_) => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sony_native_output_is_preserved_without_provider_requests() {
        let output = DualSenseHidOutput::UsbOutput {
            raw: vec![7; 47],
            valid_flag0: 1,
            valid_flag1: 0x14,
            valid_flag2: 4,
            right_motor: Some(9),
            left_motor: None,
            right_trigger_effect: [11; 11],
            left_trigger_effect: [22; 11],
            mute_button_led: Some(true),
            player_leds: Some(3),
            lightbar_rgb: Some([1, 2, 3]),
        };
        assert_eq!(
            dualsense(gr_curated_controllers::DualSenseOutputEvent::HidOutput(
                output.clone()
            ))
            .unwrap(),
            Some(DualSenseOutputEvent::HidOutput(output))
        );
        let output = DualShock4HidOutput::UsbOutput {
            raw: vec![1, 2, 3],
            right_motor: 17,
            left_motor: 42,
            lightbar_rgb: None,
        };
        assert_eq!(
            dualshock4(gr_curated_controllers::DualShock4OutputEvent::HidOutput(
                output.clone()
            ))
            .unwrap(),
            Some(DualShock4OutputEvent::HidOutput(output))
        );
    }
    #[test]
    fn switch_rumble_and_unknown_native_payloads_survive_conversion() {
        for id in [None, Some(1), Some(0x10), Some(0xff)] {
            for length in [0, 8, 9, 10, 63, 64] {
                let original = gr_curated_controllers::SwitchProOutputEvent::Output {
                    report_id: id,
                    bytes: vec![3; length],
                };
                let rumble = original.rumble();
                let converted = switch_pro(original).unwrap().unwrap();
                assert_eq!(converted.rumble(), rumble);
                assert_eq!(
                    converted,
                    SwitchProOutputEvent::Output {
                        report_id: id,
                        bytes: vec![3; length]
                    }
                );
            }
        }
    }
    #[test]
    fn neither_get_nor_set_request_becomes_optional_application_work() {
        use gr_curated_controllers::{DualSenseOutputEvent as D, Xbox360OutputEvent as X};
        for event in [
            D::HidGetReportRequest {
                request_id: 1,
                report_id: 2,
                report_type: 0,
            },
            D::HidSetReportRequest {
                request_id: 1,
                report_id: 2,
                report_type: 0,
                bytes: vec![],
            },
        ] {
            assert!(dualsense(event).is_err());
        }
        for event in [
            X::HidGetReportRequest {
                request_id: 1,
                report_id: 2,
                report_type: 0,
            },
            X::HidSetReportRequest {
                request_id: 1,
                report_id: 2,
                report_type: 0,
                bytes: vec![],
            },
        ] {
            assert!(xbox360(event).is_err());
        }
        assert!(
            dualshock4(gr_curated_controllers::DualShock4OutputEvent::HostRequest {
                request_id: 1,
                report_id: 2,
                report_type: 0
            })
            .is_err()
        );
        assert!(
            switch_pro(gr_curated_controllers::SwitchProOutputEvent::HostRequest {
                request_id: 1,
                report_id: 2,
                report_type: 0
            })
            .is_err()
        );
    }
}
