#![forbid(unsafe_code)]
//! Client-side provider for the fixed-purpose privileged `dummy_hcd` broker.

#![allow(clippy::wildcard_imports)]
#![allow(clippy::needless_pass_by_value, clippy::struct_field_names)]
use gr_privileged_broker::{BROKER_SOCKET_PATH, BrokerClient, BrokerClientError, BrokerSession};
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
        require_dualsense(request)?;
        BrokerClient::connect().map(|_| ()).map_err(preflight_error)
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
        require_dualsense(&request).map_err(ProviderError::Preflight)?;
        let mut broker = BrokerClient::connect().map_err(open_error)?;
        let broker_session = broker
            .open(
                RealizationTarget::DummyHcd,
                CompiledControllerKind::DualSense,
            )
            .map_err(open_error)?;
        Ok(Box::new(Session {
            broker,
            broker_session,
            state: ProviderState::Open,
            sent: 0,
            reverse: 0,
            session: request.session,
        }))
    }
}

fn require_dualsense(request: &ProviderOpenRequest) -> Result<(), ProviderPreflightError> {
    if matches!(
        request.realization,
        NativeControllerRealization::DummyHcd(NativeDummyHcdRealization {
            controller: CompiledControllerKind::DualSense
        })
    ) {
        Ok(())
    } else {
        Err(ProviderPreflightError::MissingDeviceNode {
            target: RealizationTarget::DummyHcd,
            path: BROKER_SOCKET_PATH.into(),
        })
    }
}

fn preflight_error(error: BrokerClientError) -> ProviderPreflightError {
    match error {
        BrokerClientError::Unavailable(error)
            if error.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            ProviderPreflightError::AccessDenied {
                target: RealizationTarget::DummyHcd,
                path: BROKER_SOCKET_PATH.into(),
            }
        }
        _ => ProviderPreflightError::MissingDeviceNode {
            target: RealizationTarget::DummyHcd,
            path: BROKER_SOCKET_PATH.into(),
        },
    }
}
fn open_error(error: BrokerClientError) -> ProviderError {
    ProviderError::Open {
        reason: error.to_string(),
    }
}

struct Session {
    broker: BrokerClient,
    broker_session: BrokerSession,
    state: ProviderState,
    sent: u64,
    reverse: u64,
    session: RealizationSessionId,
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            return Err(ProviderError::Closed);
        }
        let ProviderFrame::DummyHcdInput(bytes) = frame else {
            return Err(ProviderError::Unsupported {
                reason: "dummy_hcd accepts only exact 64-byte DualSense input reports".into(),
            });
        };
        if bytes.len() != 64 {
            return Err(ProviderError::Unsupported {
                reason: "dummy_hcd accepts only exact 64-byte DualSense input reports".into(),
            });
        }
        self.broker
            .send_input(self.broker_session, &bytes)
            .map_err(|error| ProviderError::Write {
                reason: error.to_string(),
            })?;
        self.sent += 1;
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            return Err(ProviderError::Closed);
        }
        match self
            .broker
            .poll_reverse(self.broker_session)
            .map_err(|error| ProviderError::Read {
                reason: error.to_string(),
            })? {
            Some(bytes) => {
                self.reverse += 1;
                out.push(ProviderReverseEvent {
                    session: self.session,
                    sequence: self.reverse,
                    event: decode_hid_output(bytes)?,
                });
                Ok(())
            }
            None => Err(ProviderError::WouldBlock),
        }
    }
    fn readiness(&self) -> EventReadiness {
        EventReadiness::AlwaysPoll
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: self.state,
            frames_sent: self.sent,
            reverse_events_drained: self.reverse,
            write_failures: 0,
            lifecycle_events: 0,
            last_error: None,
        }
    }
    fn close(&mut self) -> Result<(), ProviderError> {
        if self.state == ProviderState::Closed {
            return Ok(());
        }
        self.broker
            .close(self.broker_session)
            .map_err(|error| ProviderError::Write {
                reason: error.to_string(),
            })?;
        self.state = ProviderState::Closed;
        Ok(())
    }
}

fn decode_hid_output(bytes: Vec<u8>) -> Result<RawReverseEvent, ProviderError> {
    let Some((&report_id, payload)) = bytes.split_first() else {
        return Err(ProviderError::Read {
            reason: "dummy_hcd output report omitted its report ID".into(),
        });
    };
    Ok(RawReverseEvent::HidOutput {
        report_id: Some(report_id),
        bytes: payload.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_hcd_output_separates_the_usb_report_id_from_its_payload() {
        assert_eq!(
            decode_hid_output(vec![0x02, 0x01, 0x04, 0x30]),
            Ok(RawReverseEvent::HidOutput {
                report_id: Some(0x02),
                bytes: vec![0x01, 0x04, 0x30],
            })
        );
        assert!(decode_hid_output(Vec::new()).is_err());
    }
}
