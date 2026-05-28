//! Transport enumeration state machine + canned-trace replay.
//!
//! Phase 10 owns the transport-tier state-machine contract. The types
//! and replay function live in `gr-testkit` so consumers like `gr-cli`
//! can drive canned traces without pulling in the OS-specific
//! `gr-provider-linux-transport` crate (Phase 11 turns that crate
//! Linux-only).

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportTraceBus {
    Usb,
    Bluetooth,
}

impl fmt::Display for TransportTraceBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usb => f.write_str("usb"),
            Self::Bluetooth => f.write_str("bluetooth"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportTraceState {
    Idle,
    Connected,
    DescriptorRead,
    EndpointsConfigured,
    Ready,
    Disconnected,
}

impl fmt::Display for TransportTraceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => f.write_str("idle"),
            Self::Connected => f.write_str("connected"),
            Self::DescriptorRead => f.write_str("descriptor-read"),
            Self::EndpointsConfigured => f.write_str("endpoints-configured"),
            Self::Ready => f.write_str("ready"),
            Self::Disconnected => f.write_str("disconnected"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportControlStep {
    Connect,
    ReadDescriptor,
    ConfigureEndpoints,
    ReadySignal,
    InputPacket,
    ReversePacket,
    Disconnect,
}

impl fmt::Display for TransportControlStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("connect"),
            Self::ReadDescriptor => f.write_str("read-descriptor"),
            Self::ConfigureEndpoints => f.write_str("configure-endpoints"),
            Self::ReadySignal => f.write_str("ready-signal"),
            Self::InputPacket => f.write_str("input-packet"),
            Self::ReversePacket => f.write_str("reverse-packet"),
            Self::Disconnect => f.write_str("disconnect"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportTraceStep {
    pub step: TransportControlStep,
    pub endpoint_id: Option<u8>,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportEndpoints {
    pub input: u8,
    pub reverse: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportReplaySummary {
    pub final_state: TransportTraceState,
    pub consumed_steps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportReplayError {
    InvalidEndpoint {
        step_index: usize,
        step: TransportControlStep,
        expected: u8,
        actual: u8,
    },
    MissingTransition {
        step_index: usize,
        step: TransportControlStep,
        current_state: TransportTraceState,
        required_step: TransportControlStep,
    },
    UnexpectedFinalState {
        expected: TransportTraceState,
        actual: TransportTraceState,
    },
}

impl fmt::Display for TransportReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint {
                step_index,
                step,
                expected,
                actual,
            } => write!(
                f,
                "transport trace step {step_index} `{step}` targeted endpoint 0x{actual:02x}, expected 0x{expected:02x}"
            ),
            Self::MissingTransition {
                step_index,
                step,
                current_state,
                required_step,
            } => write!(
                f,
                "transport trace step {step_index} `{step}` requires `{required_step}` before it; current state is `{current_state}`"
            ),
            Self::UnexpectedFinalState { expected, actual } => write!(
                f,
                "transport trace finished in `{actual}` but fixture expected `{expected}`"
            ),
        }
    }
}

impl std::error::Error for TransportReplayError {}

/// Replay a canned transport enumeration/control-flow trace through the
/// Phase 10 transport state machine.
///
/// `endpoints` is optional: when supplied, `InputPacket`/`ReversePacket`
/// steps must carry matching `endpoint_id` values. Pass `None` to skip
/// endpoint validation (useful for traces that don't pin endpoint
/// values).
///
/// # Errors
///
/// Returns [`TransportReplayError`] when a mandatory startup transition
/// is missing, an endpoint is inconsistent with the supplied
/// `endpoints`, or the final state does not match
/// `expected_final_state`.
pub fn replay_transport_trace(
    endpoints: Option<TransportEndpoints>,
    steps: &[TransportTraceStep],
    expected_final_state: Option<TransportTraceState>,
) -> Result<TransportReplaySummary, TransportReplayError> {
    let mut state = TransportTraceState::Idle;
    for (index, step) in steps.iter().enumerate() {
        let step_index = index + 1;
        if let (Some(eps), Some(endpoint_id)) = (endpoints, step.endpoint_id) {
            let expected_endpoint = match step.step {
                TransportControlStep::InputPacket => Some(eps.input),
                TransportControlStep::ReversePacket => Some(eps.reverse),
                _ => None,
            };
            if let Some(expected) = expected_endpoint
                && endpoint_id != expected
            {
                return Err(TransportReplayError::InvalidEndpoint {
                    step_index,
                    step: step.step,
                    expected,
                    actual: endpoint_id,
                });
            }
        }

        state = match (state, step.step) {
            (TransportTraceState::Idle, TransportControlStep::Connect) => {
                TransportTraceState::Connected
            }
            (TransportTraceState::Connected, TransportControlStep::ReadDescriptor) => {
                TransportTraceState::DescriptorRead
            }
            (TransportTraceState::DescriptorRead, TransportControlStep::ConfigureEndpoints) => {
                TransportTraceState::EndpointsConfigured
            }
            (TransportTraceState::EndpointsConfigured, TransportControlStep::ReadySignal)
            | (
                TransportTraceState::Ready,
                TransportControlStep::InputPacket | TransportControlStep::ReversePacket,
            ) => TransportTraceState::Ready,
            (
                TransportTraceState::Connected
                | TransportTraceState::DescriptorRead
                | TransportTraceState::EndpointsConfigured
                | TransportTraceState::Ready,
                TransportControlStep::Disconnect,
            ) => TransportTraceState::Disconnected,
            (current_state, step_kind) => {
                return Err(TransportReplayError::MissingTransition {
                    step_index,
                    step: step_kind,
                    current_state,
                    required_step: required_previous_step(step_kind),
                });
            }
        };
    }

    if let Some(expected) = expected_final_state
        && state != expected
    {
        return Err(TransportReplayError::UnexpectedFinalState {
            expected,
            actual: state,
        });
    }

    Ok(TransportReplaySummary {
        final_state: state,
        consumed_steps: steps.len(),
    })
}

/// The transition that must have happened immediately before `step` for
/// the state machine to accept it.
///
/// For chained-prerequisite steps (`InputPacket`, `ReversePacket`,
/// `Disconnect`) this names only the *immediate* predecessor — the
/// error message carries the full picture via `current_state`. Example:
/// `Disconnect` from `Idle` renders as "requires `connect` before it;
/// current state is `idle`", which is accurate end-to-end.
fn required_previous_step(step: TransportControlStep) -> TransportControlStep {
    match step {
        TransportControlStep::Connect
        | TransportControlStep::ReadDescriptor
        | TransportControlStep::Disconnect => TransportControlStep::Connect,
        TransportControlStep::ConfigureEndpoints => TransportControlStep::ReadDescriptor,
        TransportControlStep::ReadySignal => TransportControlStep::ConfigureEndpoints,
        TransportControlStep::InputPacket | TransportControlStep::ReversePacket => {
            TransportControlStep::ReadySignal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(kind: TransportControlStep) -> TransportTraceStep {
        TransportTraceStep {
            step: kind,
            endpoint_id: None,
            bytes: Vec::new(),
        }
    }

    fn endpoint_step(kind: TransportControlStep, endpoint_id: u8) -> TransportTraceStep {
        TransportTraceStep {
            step: kind,
            endpoint_id: Some(endpoint_id),
            bytes: Vec::new(),
        }
    }

    #[test]
    fn dualsense_usb_enumeration_reaches_ready() {
        let summary = replay_transport_trace(
            Some(TransportEndpoints {
                input: 0x01,
                reverse: 0x02,
            }),
            &[
                step(TransportControlStep::Connect),
                step(TransportControlStep::ReadDescriptor),
                step(TransportControlStep::ConfigureEndpoints),
                step(TransportControlStep::ReadySignal),
                endpoint_step(TransportControlStep::InputPacket, 0x01),
            ],
            Some(TransportTraceState::Ready),
        )
        .expect("replay");

        assert_eq!(summary.final_state, TransportTraceState::Ready);
        assert_eq!(summary.consumed_steps, 5);
    }

    #[test]
    fn missing_configure_endpoints_surfaces_step_index_and_required_step() {
        let error = replay_transport_trace(
            None,
            &[
                step(TransportControlStep::Connect),
                step(TransportControlStep::ReadDescriptor),
                step(TransportControlStep::ReadySignal),
            ],
            Some(TransportTraceState::Ready),
        )
        .expect_err("missing configure-endpoints should fail");

        assert!(matches!(
            error,
            TransportReplayError::MissingTransition {
                step_index: 3,
                step: TransportControlStep::ReadySignal,
                current_state: TransportTraceState::DescriptorRead,
                required_step: TransportControlStep::ConfigureEndpoints,
            }
        ));
    }

    #[test]
    fn disconnect_from_idle_renders_actionable_error_text() {
        let error = replay_transport_trace(None, &[step(TransportControlStep::Disconnect)], None)
            .expect_err("disconnect from idle should fail");

        assert!(matches!(
            error,
            TransportReplayError::MissingTransition {
                step_index: 1,
                step: TransportControlStep::Disconnect,
                current_state: TransportTraceState::Idle,
                required_step: TransportControlStep::Connect,
            }
        ));
        let rendered = error.to_string();
        assert!(
            rendered.contains("`disconnect`")
                && rendered.contains("requires `connect`")
                && rendered.contains("current state is `idle`"),
            "unexpected error rendering: {rendered}"
        );
    }

    #[test]
    fn endpoint_mismatch_is_caught_when_endpoints_are_supplied() {
        let error = replay_transport_trace(
            Some(TransportEndpoints {
                input: 0x01,
                reverse: 0x02,
            }),
            &[
                step(TransportControlStep::Connect),
                step(TransportControlStep::ReadDescriptor),
                step(TransportControlStep::ConfigureEndpoints),
                step(TransportControlStep::ReadySignal),
                endpoint_step(TransportControlStep::InputPacket, 0x05),
            ],
            None,
        )
        .expect_err("endpoint mismatch should fail");

        assert!(matches!(
            error,
            TransportReplayError::InvalidEndpoint {
                step_index: 5,
                step: TransportControlStep::InputPacket,
                expected: 0x01,
                actual: 0x05,
            }
        ));
    }

    #[test]
    fn unexpected_final_state_is_reported() {
        let error = replay_transport_trace(
            None,
            &[
                step(TransportControlStep::Connect),
                step(TransportControlStep::Disconnect),
            ],
            Some(TransportTraceState::Ready),
        )
        .expect_err("expected ready but reached disconnected");

        assert!(matches!(
            error,
            TransportReplayError::UnexpectedFinalState {
                expected: TransportTraceState::Ready,
                actual: TransportTraceState::Disconnected,
            }
        ));
    }
}
