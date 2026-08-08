#![forbid(unsafe_code)]
//! Generic Linux uinput realization provider.
use gr_realization_api::*;

#[derive(Default)]
pub struct LinuxUinputProvider;
impl NativeProviderFactory for LinuxUinputProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(LinuxTarget::Uinput, true)
    }
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        request.validate().map_err(|e| ProviderError::Unsupported {
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
