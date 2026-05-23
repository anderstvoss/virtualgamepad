//! Shared envelope parsing for fixture documents.

use gr_core::CoreError;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fmt;
use std::path::Path;

use super::{
    backend_inventory::{BackendInventoryFixture, decode_backend_inventory},
    backend_trace::{BackendTraceFixture, decode_backend_trace},
    input_frame::{InputDeltaFixture, InputFrameFixture, decode_input_delta, decode_input_frame},
    plan_snapshot::{PlanSnapshotFixture, decode_plan_snapshot},
    reverse_event::{ReverseEventFixture, decode_reverse_event},
    session_scenario::{SessionScenarioFixture, decode_session_scenario},
};

pub const FIXTURE_SCHEMA_VERSION: &str = "virtualgamepad/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureEnvelope {
    pub fixture: String,
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureDocument {
    Envelope(FixtureEnvelope),
    InputFrame(InputFrameFixture),
    InputDelta(InputDeltaFixture),
    BackendTrace(BackendTraceFixture),
    ReverseEvent(ReverseEventFixture),
    SessionScenario(SessionScenarioFixture),
    PlanSnapshot(PlanSnapshotFixture),
    BackendInventory(BackendInventoryFixture),
}

#[derive(Debug)]
pub enum FixtureError {
    Io(std::io::Error),
    Parse(serde_yaml::Error),
    UnsupportedVersion { actual: String },
    MissingProfileId,
    UnsupportedKind { kind: String },
    ProfilePayloadMismatch { source: CoreError },
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read fixture: {error}"),
            Self::Parse(error) => write!(f, "failed to parse fixture YAML: {error}"),
            Self::UnsupportedVersion { actual } => write!(
                f,
                "unsupported fixture version in `fixture` field: expected `{FIXTURE_SCHEMA_VERSION}`, got `{actual}`"
            ),
            Self::MissingProfileId => {
                write!(
                    f,
                    "fixture kind `input-frame` requires a `profile_id` field"
                )
            }
            Self::UnsupportedKind { kind } => {
                write!(f, "unsupported fixture kind `{kind}`")
            }
            Self::ProfilePayloadMismatch { source } => source.fmt(f),
        }
    }
}

impl std::error::Error for FixtureError {}

/// Load and validate a typed fixture document from disk.
///
/// # Errors
///
/// Returns an error if the file cannot be read, the YAML is malformed,
/// the `fixture` version does not match `virtualgamepad/v1`, or the
/// fixture kind cannot be decoded.
pub fn load_fixture(path: impl AsRef<Path>) -> Result<FixtureDocument, FixtureError> {
    let contents = std::fs::read_to_string(path).map_err(FixtureError::Io)?;
    let envelope: FixtureEnvelope = serde_yaml::from_str(&contents).map_err(FixtureError::Parse)?;
    if envelope.fixture != FIXTURE_SCHEMA_VERSION {
        return Err(FixtureError::UnsupportedVersion {
            actual: envelope.fixture.clone(),
        });
    }
    match envelope.kind.as_str() {
        "input-frame" => decode_input_frame(envelope).map(FixtureDocument::InputFrame),
        "input-delta" => decode_input_delta(envelope).map(FixtureDocument::InputDelta),
        "backend-trace" => decode_backend_trace(envelope).map(FixtureDocument::BackendTrace),
        "session-scenario" => {
            decode_session_scenario(envelope).map(FixtureDocument::SessionScenario)
        }
        "plan-snapshot" => decode_plan_snapshot(envelope).map(FixtureDocument::PlanSnapshot),
        "backend-inventory" => {
            decode_backend_inventory(envelope).map(FixtureDocument::BackendInventory)
        }
        "reverse-event" => decode_reverse_event(envelope).map(FixtureDocument::ReverseEvent),
        other => Err(FixtureError::UnsupportedKind {
            kind: other.to_owned(),
        }),
    }
}
