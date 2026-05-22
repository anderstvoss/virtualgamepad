//! `backend-trace` fixture support.

use super::schema::{FixtureEnvelope, FixtureError};
use gr_backend_api::{BackendError, BackendFrame, BackendReverseEvent, EvdevEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceDirection {
    Outbound,
    Inbound,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TraceOperation {
    Open,
    Send,
    DrainReverseEvents,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendTrace {
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
            _ => Self::Failure {
                operation: TraceOperation::Send,
                error: "unsupported backend frame variant in trace encoder".to_string(),
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
            Self::ReverseEvent { .. } | Self::Failure { .. } => None,
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
