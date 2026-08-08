#![forbid(unsafe_code)]
//! Generic Linux USB gadget transport realization provider.
use gr_realization_api::*;
#[derive(Default)]
pub struct LinuxUsbGadgetProvider;
impl NativeProviderFactory for LinuxUsbGadgetProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(LinuxTarget::UsbGadget, true)
    }
    fn open(
        &self,
        request: ProviderOpenRequest,
    ) -> Result<Box<dyn NativeProviderSession>, ProviderError> {
        request.validate().map_err(|e| ProviderError::Unsupported {
            reason: e.to_string(),
        })?;
        if !matches!(request.realization, NativeControllerRealization::Usb(_)) {
            return Err(ProviderError::Unsupported {
                reason: "USB gadget requires USB realization".into(),
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
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        self.state = ProviderState::Closed;
        Ok(())
    }
}
