#![forbid(unsafe_code)]
//! Generic Linux UHID realization provider.
#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;
#[derive(Default)]
pub struct LinuxUhidProvider;
impl NativeProviderFactory for LinuxUhidProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(LinuxTarget::Uhid, true)
    }
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        request.validate().map_err(|e| ProviderError::Unsupported {
            reason: e.to_string(),
        })?;
        if !matches!(request.realization, NativeControllerRealization::Hid(_)) {
            return Err(ProviderError::Unsupported {
                reason: "UHID requires HID realization".into(),
            });
        }
        Ok(Box::new(Session {
            state: ProviderState::NotOpen,
            sent: 0,
        }))
    }
}
struct Session {
    state: ProviderState,
    sent: u64,
}
impl NativeProviderSession for Session {
    fn open(&mut self) -> Result<(), ProviderError> {
        self.state = ProviderState::Open;
        Ok(())
    }
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            return Err(ProviderError::Closed);
        }
        if !matches!(
            frame,
            ProviderFrame::HidInput { .. } | ProviderFrame::HidFeature { .. }
        ) {
            return Err(ProviderError::Unsupported {
                reason: "UHID accepts HID frames only".into(),
            });
        }
        self.sent += 1;
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        _: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        Err(ProviderError::WouldBlock)
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::NoReverseEvents
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: self.state,
            frames_sent: self.sent,
            reverse_events_drained: 0,
            write_failures: 0,
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        self.state = ProviderState::Closed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.uhid"),
                target: LinuxTarget::Uhid,
                mode: RealizationMode::IdentityAccurate,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Hid(NativeHidRealization {
                bus_type: 3,
                device_name: "test".into(),
                physical_path: String::new(),
                unique_id: String::new(),
                identity: NativeDeviceIdentity {
                    vendor_id: 1,
                    product_id: 2,
                    version: 3,
                },
                descriptor: vec![0],
                numbered_output_reports: false,
                numbered_feature_reports: false,
                feature_report_responses: BTreeMap::new(),
            }),
        }
    }

    #[test]
    fn accepts_only_hid_frames_after_open() {
        let provider = LinuxUhidProvider;
        let mut session = provider.open(request()).expect("valid HID request");
        session.open().expect("opens without kernel access");
        session
            .send(ProviderFrame::HidInput {
                report_id: None,
                bytes: vec![],
            })
            .expect("HID frame");
        assert!(matches!(
            session.send(ProviderFrame::Evdev(vec![])),
            Err(ProviderError::Unsupported { .. })
        ));
        assert_eq!(session.diagnostics().frames_sent, 1);
        session.close().expect("close succeeds");
    }
}
