#![forbid(unsafe_code)]
//! Generic Linux USB gadget transport realization provider.
#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;
use std::fs::OpenOptions;
use std::io::ErrorKind;
#[derive(Default)]
pub struct LinuxUsbGadgetProvider;
impl NativeProviderFactory for LinuxUsbGadgetProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(RealizationTarget::UsbTransportValidation, false)
    }
    fn preflight(&self, request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        if !cfg!(target_os = "linux") {
            return Err(ProviderPreflightError::UnsupportedPlatform {
                target: RealizationTarget::UsbTransportValidation,
            });
        }
        let NativeControllerRealization::UsbTransportValidation(specification) =
            &request.realization
        else {
            return Ok(());
        };
        check_endpoint(&specification.input_endpoint_path)?;
        if let Some(path) = &specification.reverse_endpoint_path {
            check_endpoint(path)?;
        }
        Ok(())
    }
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        request
            .validate_against(self.capabilities())
            .map_err(|e| ProviderError::Unsupported {
                reason: e.to_string(),
            })?;
        self.preflight(&request)?;
        if !matches!(
            request.realization,
            NativeControllerRealization::UsbTransportValidation(_)
        ) {
            return Err(ProviderError::Unsupported {
                reason: "USB gadget requires USB realization".into(),
            });
        }
        Ok(Box::new(Session {
            state: ProviderState::Open,
            sent: 0,
        }))
    }
}
fn check_endpoint(path: &str) -> Result<(), ProviderPreflightError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map(|_| ())
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => {
                ProviderPreflightError::MissingPreparedEndpoint { path: path.into() }
            }
            _ => ProviderPreflightError::PreparedEndpointAccessDenied { path: path.into() },
        })
}
struct Session {
    state: ProviderState,
    sent: u64,
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            return Err(ProviderError::Closed);
        }
        if !matches!(frame, ProviderFrame::Transport { .. }) {
            return Err(ProviderError::Unsupported {
                reason: "USB gadget accepts transport packets only".into(),
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
            lifecycle_events: 0,
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

    fn missing_endpoint_request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.usb"),
                target: RealizationTarget::UsbTransportValidation,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::UsbTransportValidation(
                NativeUsbTransportValidationRealization {
                    input_endpoint_path: "/dev/virtualgamepad-test-missing".into(),
                    reverse_endpoint_path: None,
                    device_name: "test".into(),
                    maximum_input_packet_length: 64,
                    maximum_reverse_packet_length: None,
                },
            ),
        }
    }

    #[test]
    fn accepts_only_transport_frames_after_open() {
        let mut session = Session {
            state: ProviderState::Open,
            sent: 0,
        };
        session
            .send(ProviderFrame::Transport {
                endpoint: 1,
                bytes: vec![],
            })
            .expect("transport frame");
        assert!(matches!(
            session.send(ProviderFrame::HidInput {
                report_id: None,
                bytes: vec![]
            }),
            Err(ProviderError::Unsupported { .. })
        ));
        assert_eq!(session.diagnostics().frames_sent, 1);
        session.close().expect("close succeeds");
    }

    #[test]
    fn missing_preprovisioned_endpoint_is_an_actionable_error() {
        let error = check_endpoint("/dev/virtualgamepad-test-missing")
            .expect_err("test endpoint must not exist");
        assert!(matches!(
            error,
            ProviderPreflightError::MissingPreparedEndpoint { .. }
        ));
    }

    #[test]
    fn open_returns_no_session_when_preflight_fails() {
        let Err(error) = LinuxUsbGadgetProvider.open(missing_endpoint_request()) else {
            panic!("missing endpoint must fail before a session exists");
        };
        assert!(matches!(
            error,
            ProviderError::Preflight(ProviderPreflightError::MissingPreparedEndpoint { .. })
        ));
    }
}
