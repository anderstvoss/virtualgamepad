#![forbid(unsafe_code)]
//! Client-side provider for the fixed-purpose privileged btvirt broker.

#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;

#[derive(Default)]
pub struct LinuxBtvirtProvider;
impl NativeProviderFactory for LinuxBtvirtProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(RealizationTarget::Btvirt, true)
    }
    fn preflight(&self, request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
        if !cfg!(target_os = "linux") {
            return Err(ProviderPreflightError::UnsupportedPlatform {
                target: RealizationTarget::Btvirt,
            });
        }
        if !matches!(
            request.realization,
            NativeControllerRealization::Btvirt(NativeBtvirtRealization {
                controller: CompiledControllerKind::DualSense
            })
        ) {
            return Err(ProviderPreflightError::MissingDeviceNode {
                target: RealizationTarget::Btvirt,
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
        }))
    }
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
        match frame {
            ProviderFrame::BtvirtInput(bytes) if bytes.len() == 78 => {
                self.sent += 1;
                Ok(())
            }
            _ => Err(ProviderError::Unsupported {
                reason: "btvirt accepts only exact 78-byte DualSense Bluetooth input reports"
                    .into(),
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
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_bluetooth_sized_input() {
        let mut session = Session {
            state: ProviderState::Open,
            sent: 0,
        };
        assert!(
            session
                .send(ProviderFrame::BtvirtInput(vec![0; 64]))
                .is_err()
        );
        assert!(
            session
                .send(ProviderFrame::BtvirtInput(vec![0; 78]))
                .is_ok()
        );
    }
}
