#![forbid(unsafe_code)]
//! Generic Linux USB gadget transport realization provider.
#![allow(clippy::wildcard_imports)]
use gr_realization_api::*;
#[cfg(target_os = "linux")]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Write},
};
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
        match &request.realization {
            NativeControllerRealization::UsbTransportValidation(specification) => {
                check_endpoint(
                    &specification.input_endpoint_path,
                    NativeUsbEndpointDirection::DeviceToHost,
                )?;
                if let Some(path) = &specification.reverse_endpoint_path {
                    check_endpoint(path, NativeUsbEndpointDirection::HostToDevice)?;
                }
            }
            NativeControllerRealization::UsbComposite(specification) => {
                for endpoint in &specification.endpoints {
                    check_endpoint(&endpoint.path, endpoint.direction)?;
                }
            }
            _ => {}
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
                | NativeControllerRealization::UsbComposite(_)
        ) {
            return Err(ProviderError::Unsupported {
                reason: "USB gadget requires USB realization".into(),
            });
        }
        let endpoints = match request.realization {
            NativeControllerRealization::UsbTransportValidation(specification) => {
                let mut endpoints = BTreeMap::new();
                let input = endpoint_open_options(NativeUsbEndpointDirection::DeviceToHost)
                    .open(&specification.input_endpoint_path)
                    .map_err(|error| ProviderError::Open {
                        reason: error.to_string(),
                    })?;
                endpoints.insert(
                    0x81,
                    EndpointFile {
                        file: input,
                        direction: NativeUsbEndpointDirection::DeviceToHost,
                        maximum_packet_length: specification.maximum_input_packet_length,
                    },
                );
                if let (Some(path), Some(maximum_packet_length)) = (
                    specification.reverse_endpoint_path,
                    specification.maximum_reverse_packet_length,
                ) {
                    let reverse = endpoint_open_options(NativeUsbEndpointDirection::HostToDevice)
                        .open(path)
                        .map_err(|error| ProviderError::Open {
                            reason: error.to_string(),
                        })?;
                    endpoints.insert(
                        0x01,
                        EndpointFile {
                            file: reverse,
                            direction: NativeUsbEndpointDirection::HostToDevice,
                            maximum_packet_length,
                        },
                    );
                }
                endpoints
            }
            NativeControllerRealization::UsbComposite(specification) => specification
                .endpoints
                .into_iter()
                .map(|endpoint| {
                    endpoint_open_options(endpoint.direction)
                        .open(&endpoint.path)
                        .map(|file| {
                            (
                                endpoint.address,
                                EndpointFile {
                                    file,
                                    direction: endpoint.direction,
                                    maximum_packet_length: endpoint.maximum_packet_length,
                                },
                            )
                        })
                        .map_err(|error| ProviderError::Open {
                            reason: error.to_string(),
                        })
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?,
            _ => unreachable!("USB realization was checked above"),
        };
        Ok(Box::new(Session {
            realization_id: request.session,
            state: ProviderState::Open,
            sent: 0,
            reverse_events_drained: 0,
            endpoints,
        }))
    }
}
fn endpoint_open_options(direction: NativeUsbEndpointDirection) -> OpenOptions {
    let mut options = OpenOptions::new();
    match direction {
        NativeUsbEndpointDirection::DeviceToHost => {
            options.write(true);
        }
        NativeUsbEndpointDirection::HostToDevice => {
            options.read(true);
        }
    }
    #[cfg(target_os = "linux")]
    options.custom_flags(0o4000); // O_NONBLOCK; avoid stalling controller polling.
    options
}

fn check_endpoint(
    path: &str,
    direction: NativeUsbEndpointDirection,
) -> Result<(), ProviderPreflightError> {
    endpoint_open_options(direction)
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
    realization_id: RealizationSessionId,
    state: ProviderState,
    sent: u64,
    reverse_events_drained: u64,
    endpoints: BTreeMap<u8, EndpointFile>,
}
struct EndpointFile {
    file: File,
    direction: NativeUsbEndpointDirection,
    maximum_packet_length: u16,
}
impl NativeProviderSession for Session {
    fn send(&mut self, frame: ProviderFrame) -> Result<(), ProviderError> {
        if self.state != ProviderState::Open {
            return Err(ProviderError::Closed);
        }
        let ProviderFrame::Transport { endpoint, bytes } = frame else {
            return Err(ProviderError::Unsupported {
                reason: "USB gadget accepts transport packets only".into(),
            });
        };
        if let Some(endpoint_file) = self.endpoints.get_mut(&endpoint) {
            if endpoint_file.direction != NativeUsbEndpointDirection::DeviceToHost {
                return Err(ProviderError::Unsupported {
                    reason: "USB transport endpoint is host-to-device".into(),
                });
            }
            if bytes.len() > usize::from(endpoint_file.maximum_packet_length) {
                return Err(ProviderError::Write {
                    reason: "USB transport packet exceeds declared endpoint maximum".into(),
                });
            }
            endpoint_file
                .file
                .write_all(&bytes)
                .map_err(|error| ProviderError::Write {
                    reason: error.to_string(),
                })?;
        } else if !self.endpoints.is_empty() {
            return Err(ProviderError::Unsupported {
                reason: "USB composite endpoint is not declared".into(),
            });
        }
        self.sent += 1;
        Ok(())
    }
    fn drain_reverse_events(
        &mut self,
        sink: &mut dyn ProviderReverseEventSink,
    ) -> Result<(), ProviderError> {
        let mut drained = 0_u64;
        for (address, endpoint) in &mut self.endpoints {
            if endpoint.direction != NativeUsbEndpointDirection::HostToDevice {
                continue;
            }
            let mut bytes = vec![0_u8; usize::from(endpoint.maximum_packet_length)];
            match endpoint.file.read(&mut bytes) {
                Ok(0) => {}
                Ok(length) => {
                    bytes.truncate(length);
                    sink.push(ProviderReverseEvent {
                        session: self.realization_id,
                        sequence: self.reverse_events_drained + drained,
                        event: RawReverseEvent::Transport {
                            endpoint: *address,
                            bytes,
                        },
                    });
                    drained += 1;
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) => {
                    return Err(ProviderError::Read {
                        reason: error.to_string(),
                    });
                }
            }
        }
        if drained == 0 {
            return Err(ProviderError::WouldBlock);
        }
        self.reverse_events_drained += drained;
        Ok(())
    }
    fn readiness(&self) -> EventReadiness {
        if self
            .endpoints
            .values()
            .any(|endpoint| endpoint.direction == NativeUsbEndpointDirection::HostToDevice)
        {
            EventReadiness::AlwaysPoll
        } else {
            EventReadiness::NoReverseEvents
        }
    }
    fn diagnostics(&self) -> ProviderDiagnostics {
        ProviderDiagnostics {
            state: self.state,
            frames_sent: self.sent,
            reverse_events_drained: self.reverse_events_drained,
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
            realization_id: RealizationSessionId(1),
            state: ProviderState::Open,
            sent: 0,
            reverse_events_drained: 0,
            endpoints: BTreeMap::new(),
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
        let error = check_endpoint(
            "/dev/virtualgamepad-test-missing",
            NativeUsbEndpointDirection::DeviceToHost,
        )
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
