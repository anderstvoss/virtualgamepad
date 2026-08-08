#![forbid(unsafe_code)]
//! Generic Linux uinput realization provider.
#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;
use std::fs::OpenOptions;
use std::io::ErrorKind;

#[derive(Default)]
pub struct LinuxUinputProvider;
impl NativeProviderFactory for LinuxUinputProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(LinuxTarget::Uinput, false)
    }
    fn preflight(&self) -> Result<(), ProviderPreflightError> {
        check_device_node(LinuxTarget::Uinput, "/dev/uinput")
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
        let NativeControllerRealization::Evdev(_) = request.realization else {
            return Err(ProviderError::Unsupported {
                reason: "uinput requires evdev realization".into(),
            });
        };
        Ok(Box::new(Session {
            id: request.session,
            state: ProviderState::NotOpen,
            sent: 0,
            failures: 0,
        }))
    }
}

fn check_device_node(target: LinuxTarget, path: &str) -> Result<(), ProviderPreflightError> {
    if !cfg!(target_os = "linux") {
        return Err(ProviderPreflightError::UnsupportedPlatform { target });
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map(|_| ())
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => ProviderPreflightError::MissingDeviceNode {
                target,
                path: path.into(),
            },
            _ => ProviderPreflightError::AccessDenied {
                target,
                path: path.into(),
            },
        })
}
struct Session {
    id: RealizationSessionId,
    state: ProviderState,
    sent: u64,
    failures: u64,
}
impl NativeProviderSession for Session {
    fn open(&mut self) -> Result<(), ProviderError> {
        if self.state == ProviderState::Closed {
            return Err(ProviderError::Closed);
        }
        self.state = ProviderState::Open;
        Ok(())
    }
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            self.failures += 1;
            return Err(ProviderError::Closed);
        }
        if !matches!(frame, ProviderFrame::Evdev(_)) {
            self.failures += 1;
            return Err(ProviderError::Unsupported {
                reason: "uinput accepts evdev frames only".into(),
            });
        }
        self.sent += 1;
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        _out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let _ = self.id;
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
            write_failures: self.failures,
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

    fn request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.uinput"),
                target: LinuxTarget::Uinput,
                mode: RealizationMode::HostCompatible,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Evdev(NativeEvdevRealization {
                device_name: "test".into(),
                identity: NativeDeviceIdentity {
                    vendor_id: 1,
                    product_id: 2,
                    version: 3,
                },
                event_codes: vec![],
                key_codes: vec![],
                absolute_axes: vec![],
                force_feedback_codes: vec![],
            }),
        }
    }

    #[test]
    fn accepts_only_evdev_frames_after_open() {
        let provider = LinuxUinputProvider;
        let mut session = provider.open(request()).expect("valid evdev request");
        assert_eq!(
            session.send(ProviderFrame::Evdev(vec![])),
            Err(ProviderError::Closed)
        );
        session.open().expect("opens without kernel access");
        session
            .send(ProviderFrame::Evdev(vec![]))
            .expect("evdev frame");
        assert!(matches!(
            session.send(ProviderFrame::Transport {
                endpoint: 1,
                bytes: vec![]
            }),
            Err(ProviderError::Unsupported { .. })
        ));
        assert_eq!(session.diagnostics().frames_sent, 1);
        session.close().expect("close is idempotent");
    }

    #[test]
    fn rejects_reverse_output_requirements_until_reverse_delivery_exists() {
        let mut request = request();
        request.requirements.requires_reverse_output = true;
        let Err(error) = LinuxUinputProvider.open(request) else {
            panic!("stub provider has no reverse delivery");
        };
        assert!(matches!(error, ProviderError::Unsupported { .. }));
    }

    #[test]
    fn missing_node_is_an_actionable_preflight_error() {
        let error = check_device_node(LinuxTarget::Uinput, "/dev/virtualgamepad-test-missing")
            .expect_err("test path must not exist");
        assert!(matches!(
            error,
            ProviderPreflightError::MissingDeviceNode { .. }
        ));
    }
}
