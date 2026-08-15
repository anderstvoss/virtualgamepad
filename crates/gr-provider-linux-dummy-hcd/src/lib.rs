#![forbid(unsafe_code)]
//! Client-side provider for the fixed-purpose privileged `dummy_hcd` broker.

#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;

#[derive(Default)]
pub struct LinuxDummyHcdProvider;

impl NativeProviderFactory for LinuxDummyHcdProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(RealizationTarget::DummyHcd, true)
    }
    fn preflight(&self, request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        if !cfg!(target_os = "linux") {
            return Err(ProviderPreflightError::UnsupportedPlatform {
                target: RealizationTarget::DummyHcd,
            });
        }
        if !matches!(
            request.realization,
            NativeControllerRealization::DummyHcd(NativeDummyHcdRealization {
                controller: CompiledControllerKind::DualSense
            })
        ) {
            return Err(ProviderPreflightError::MissingDeviceNode {
                target: RealizationTarget::DummyHcd,
                path: "/run/virtualgamepad/broker.sock".into(),
            });
        }
        Ok(())
    }
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        request
            .validate_against(self.capabilities())
            .map_err(|error| ProviderError::Unsupported {
                reason: error.to_string(),
            })?;
        self.preflight(&request)?;
        Ok(Box::new(Session {
            state: ProviderState::Open,
            sent: 0,
            realization_id: request.session,
        }))
    }
}

struct Session {
    state: ProviderState,
    sent: u64,
    realization_id: RealizationSessionId,
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            return Err(ProviderError::Closed);
        }
        match frame {
            ProviderFrame::DummyHcdInput(bytes) if bytes.len() == 64 => {
                self.sent += 1;
                Ok(())
            }
            _ => Err(ProviderError::Unsupported {
                reason: "dummy_hcd accepts only exact 64-byte DualSense input reports".into(),
            }),
        }
    }
    fn drain_reverse_events(
        &mut self,
        _: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        if self.state == ProviderState::Open {
            Err(ProviderError::WouldBlock)
        } else {
            Err(ProviderError::Closed)
        }
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
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
        let _ = self.realization_id;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_usb_sized_input() {
        let mut session = Session {
            state: ProviderState::Open,
            sent: 0,
            realization_id: RealizationSessionId(1),
        };
        assert!(
            session
                .send(ProviderFrame::DummyHcdInput(vec![0; 63]))
                .is_err()
        );
        assert!(
            session
                .send(ProviderFrame::DummyHcdInput(vec![0; 64]))
                .is_ok()
        );
    }
}
