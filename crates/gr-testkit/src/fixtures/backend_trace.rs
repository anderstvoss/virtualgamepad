//! `backend-trace` fixture support.

use super::schema::{FixtureEnvelope, FixtureError};
use super::transport_state_machine::{
    TransportControlStep, TransportEndpoints, TransportTraceBus, TransportTraceState,
};
use gr_backend_api::{BackendError, BackendFrame, BackendReverseEvent, EvdevEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceDirection {
    Outbound,
    Inbound,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceOperation {
    Open,
    Send,
    DrainReverseEvents,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BackendTrace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<gr_core::BackendId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<gr_core::BackendFamily>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<TransportTraceSpec>,
    #[serde(default)]
    pub steps: Vec<BackendTraceStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendTraceFixture {
    pub envelope: FixtureEnvelope,
    pub trace: BackendTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendTraceStep {
    pub direction: TraceDirection,
    #[serde(flatten)]
    pub payload: BackendTracePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportTraceSpec {
    pub bus: TransportTraceBus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_final_state: Option<TransportTraceState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<TransportEndpoints>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BackendTracePayload {
    HidInputReport {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    HidFeatureReport {
        report_id: u8,
        bytes: Vec<u8>,
    },
    TransportPacket {
        endpoint_id: u8,
        bytes: Vec<u8>,
    },
    TransportControl {
        step: TransportControlStep,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        endpoint_id: Option<u8>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        bytes: Vec<u8>,
    },
    EvdevEvents {
        events: Vec<EvdevEvent>,
    },
    ReverseEvent {
        event: BackendReverseEvent,
    },
    Failure {
        operation: TraceOperation,
        error: String,
    },
    /// A `BackendFrame` variant that the trace encoder did not recognize.
    /// Recorders emit this when a forward frame is added to `BackendFrame`
    /// without the trace encoder being updated; replay surfaces it as a
    /// `BackendError::Unsupported`.
    Unsupported {
        frame_kind: String,
    },
}

impl BackendTracePayload {
    #[must_use]
    pub fn from_frame(frame: BackendFrame) -> Self {
        match frame {
            BackendFrame::HidInputReport { report_id, bytes } => {
                Self::HidInputReport { report_id, bytes }
            }
            BackendFrame::HidFeatureReport { report_id, bytes } => {
                Self::HidFeatureReport { report_id, bytes }
            }
            BackendFrame::TransportPacket { endpoint_id, bytes } => {
                Self::TransportPacket { endpoint_id, bytes }
            }
            BackendFrame::EvdevEvents { events } => Self::EvdevEvents { events },
            other => Self::Unsupported {
                frame_kind: format!("{other:?}")
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string(),
            },
        }
    }

    #[must_use]
    pub fn as_frame(&self) -> Option<BackendFrame> {
        match self {
            Self::HidInputReport { report_id, bytes } => Some(BackendFrame::HidInputReport {
                report_id: *report_id,
                bytes: bytes.clone(),
            }),
            Self::HidFeatureReport { report_id, bytes } => Some(BackendFrame::HidFeatureReport {
                report_id: *report_id,
                bytes: bytes.clone(),
            }),
            Self::TransportPacket { endpoint_id, bytes } => Some(BackendFrame::TransportPacket {
                endpoint_id: *endpoint_id,
                bytes: bytes.clone(),
            }),
            Self::EvdevEvents { events } => Some(BackendFrame::EvdevEvents {
                events: events.clone(),
            }),
            Self::TransportControl { .. }
            | Self::ReverseEvent { .. }
            | Self::Failure { .. }
            | Self::Unsupported { .. } => None,
        }
    }

    /// Stable display label for the trace step kind. Useful in error
    /// messages and human-readable trace renderers.
    #[must_use]
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::HidInputReport { .. } => "hid-input-report",
            Self::HidFeatureReport { .. } => "hid-feature-report",
            Self::TransportPacket { .. } => "transport-packet",
            Self::TransportControl { .. } => "transport-control",
            Self::EvdevEvents { .. } => "evdev-events",
            Self::ReverseEvent { .. } => "reverse-event",
            Self::Failure { .. } => "failure",
            Self::Unsupported { .. } => "unsupported-frame",
        }
    }

    #[must_use]
    pub fn as_reverse_event(&self) -> Option<BackendReverseEvent> {
        match self {
            Self::ReverseEvent { event } => Some(event.clone()),
            _ => None,
        }
    }
}

impl From<&BackendError> for BackendTracePayload {
    fn from(error: &BackendError) -> Self {
        Self::Failure {
            operation: TraceOperation::DrainReverseEvents,
            error: error.to_string(),
        }
    }
}

/// Decode a `backend-trace` fixture envelope into a typed trace.
///
/// # Errors
///
/// Returns an error when the payload is not valid `backend-trace` YAML.
pub fn decode_backend_trace(
    envelope: FixtureEnvelope,
) -> Result<BackendTraceFixture, FixtureError> {
    let trace = serde_yaml::from_value::<BackendTrace>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    Ok(BackendTraceFixture { envelope, trace })
}

#[cfg(test)]
mod tests {
    use super::{
        BackendTrace, BackendTracePayload, BackendTraceStep, TraceDirection, TransportControlStep,
        TransportTraceBus, TransportTraceSpec, TransportTraceState,
    };

    #[test]
    fn unsupported_payload_round_trips_through_yaml() {
        let payload = BackendTracePayload::Unsupported {
            frame_kind: "FuturisticReport".to_string(),
        };
        let yaml = serde_yaml::to_string(&payload).expect("yaml");
        let decoded: BackendTracePayload = serde_yaml::from_str(&yaml).expect("decode");
        assert_eq!(payload, decoded);
        assert_eq!(payload.kind_label(), "unsupported-frame");
        assert!(payload.as_frame().is_none());
    }

    #[test]
    fn trace_decodes_without_backend_identity_for_back_compat() {
        let yaml = r"
steps:
  - direction: outbound
    kind: hid-input-report
    report_id: 1
    bytes: [1, 2, 3]
";
        let trace: BackendTrace = serde_yaml::from_str(yaml).expect("decode");
        assert!(trace.backend_id.is_none());
        assert!(trace.family.is_none());
        assert!(trace.transport.is_none());
        assert_eq!(trace.steps.len(), 1);
        assert!(matches!(
            trace.steps[0],
            BackendTraceStep {
                direction: TraceDirection::Outbound,
                payload: BackendTracePayload::HidInputReport { .. },
            }
        ));
    }

    #[test]
    fn transport_trace_decodes_additive_phase10_fields() {
        let yaml = r"
transport:
  bus: usb
  expected_final_state: ready
steps:
  - direction: outbound
    kind: transport-control
    step: connect
  - direction: outbound
    kind: transport-control
    step: configure-endpoints
    endpoint_id: 1
    bytes: [1, 2]
";
        let trace: BackendTrace = serde_yaml::from_str(yaml).expect("decode");
        assert_eq!(
            trace.transport,
            Some(TransportTraceSpec {
                bus: TransportTraceBus::Usb,
                expected_final_state: Some(TransportTraceState::Ready),
                endpoints: None,
            })
        );
        assert!(matches!(
            trace.steps[0],
            BackendTraceStep {
                direction: TraceDirection::Outbound,
                payload: BackendTracePayload::TransportControl {
                    step: TransportControlStep::Connect,
                    endpoint_id: None,
                    ..
                },
            }
        ));
    }
}
