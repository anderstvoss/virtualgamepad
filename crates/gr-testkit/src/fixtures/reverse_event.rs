//! `reverse-event` fixture support.
//!
//! Standalone reverse-event fixtures complement `backend-trace` for
//! narrow tests and direct validation of one backend-originated event.
//! They use the exact same `BackendReverseEvent` serde contract as the
//! embedded `kind: reverse-event` payload inside `backend-trace`, so
//! translators and test harnesses can consume both forms without any
//! shape translation.

use super::schema::{FixtureEnvelope, FixtureError};
use gr_backend_api::BackendReverseEvent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseEventFixture {
    pub envelope: FixtureEnvelope,
    pub event: BackendReverseEvent,
}

/// Decode a `reverse-event` fixture envelope into a typed event.
///
/// # Errors
///
/// Returns an error if the payload is not valid `BackendReverseEvent`
/// YAML.
pub fn decode_reverse_event(
    envelope: FixtureEnvelope,
) -> Result<ReverseEventFixture, FixtureError> {
    let event = serde_yaml::from_value::<BackendReverseEvent>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    Ok(ReverseEventFixture { envelope, event })
}

#[cfg(test)]
mod tests {
    use super::decode_reverse_event;
    use crate::fixtures::{FixtureError, schema::FixtureEnvelope};
    use gr_backend_api::{BackendReverseEventKind, BackendReversePayload, BackendReverseTarget};
    use gr_core::SemanticOutputFunction;
    use serde_yaml::Value;

    fn rumble_envelope() -> FixtureEnvelope {
        let payload: Value = serde_yaml::from_str(
            r"
session_id: 7
profile_id: dualsense
timestamp: 12
sequence: 3
kind: hid-output-report
target:
  kind: semantic-output
  value: rumble
payload:
  kind: hid
  report_id: 5
  bytes: [10, 20]
",
        )
        .expect("reverse-event yaml");
        FixtureEnvelope {
            fixture: "virtualgamepad/v1".to_string(),
            kind: "reverse-event".to_string(),
            id: "dualsense-rumble-low".to_string(),
            profile_id: Some("dualsense".to_string()),
            notes: None,
            payload,
        }
    }

    #[test]
    fn reverse_event_decodes_to_typed_backend_event() {
        let fixture = decode_reverse_event(rumble_envelope()).expect("decode");
        assert_eq!(fixture.envelope.id, "dualsense-rumble-low");
        assert!(matches!(
            fixture.event.kind,
            BackendReverseEventKind::HidOutputReport
        ));
        assert!(matches!(
            fixture.event.target,
            Some(BackendReverseTarget::SemanticOutput(
                SemanticOutputFunction::Rumble
            ))
        ));
        assert!(matches!(
            fixture.event.payload,
            BackendReversePayload::Hid {
                report_id: Some(5),
                ..
            }
        ));
    }

    #[test]
    fn malformed_reverse_event_payload_fails_parse() {
        let payload: Value = serde_yaml::from_str(
            r"
session_id: nope
payload:
  kind: hid
  report_id: 5
  bytes: [10, 20]
",
        )
        .expect("yaml");
        let envelope = FixtureEnvelope {
            fixture: "virtualgamepad/v1".to_string(),
            kind: "reverse-event".to_string(),
            id: "broken-reverse-event".to_string(),
            profile_id: Some("dualsense".to_string()),
            notes: None,
            payload,
        };
        let error = decode_reverse_event(envelope).expect_err("should fail");
        assert!(matches!(error, FixtureError::Parse(_)));
    }
}
