#![forbid(unsafe_code)]
//! Generic Linux USB gadget transport realization provider.
#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;
#[derive(Default)]
pub struct LinuxUsbGadgetProvider;
impl NativeProviderFactory for LinuxUsbGadgetProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::for_target(LinuxTarget::UsbGadget, false)
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
        if self.state == ProviderState::Closed {
            return Err(ProviderError::Closed);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ProviderOpenRequest {
        ProviderOpenRequest {
            session: RealizationSessionId(1),
            selection: RealizationSelection {
                controller: ControllerId::new("test.usb"),
                target: LinuxTarget::UsbGadget,
                mode: RealizationMode::HardwareFaithful,
            },
            requirements: ProviderRequirements::default(),
            realization: NativeControllerRealization::Usb(NativeUsbRealization {
                descriptor: vec![0],
                input_endpoint: 1,
                reverse_endpoint: 2,
                device_name: "test".into(),
                manufacturer: "test".into(),
                serial_number: "test".into(),
                identity: NativeDeviceIdentity {
                    vendor_id: 1,
                    product_id: 2,
                    version: 3,
                },
                usb_version: 0x0200,
                maximum_power_ma: 100,
                report_length: 64,
            }),
        }
    }

    #[test]
    fn accepts_only_transport_frames_after_open() {
        let provider = LinuxUsbGadgetProvider;
        let mut session = provider.open(request()).expect("valid USB request");
        session.open().expect("opens without kernel access");
        session
            .send(ProviderFrame::Transport {
                endpoint: 1,
                bytes: vec![],
            })
            .expect("transport frame");
        assert!(matches!(
            session.send(ProviderFrame::HidFeature {
                report_id: 1,
                bytes: vec![]
            }),
            Err(ProviderError::Unsupported { .. })
        ));
        assert_eq!(session.diagnostics().frames_sent, 1);
        session.close().expect("close succeeds");
        assert_eq!(session.open(), Err(ProviderError::Closed));
    }
}
