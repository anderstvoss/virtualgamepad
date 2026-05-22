//! Minimal `session-scenario` fixture support for Phase 4.

use super::schema::{FixtureEnvelope, FixtureError};
use gr_backend_api::{BackendFrame, BackendOpenContext, BackendReverseEvent};
use gr_core::{BackendFamily, BackendId, FidelityTier, SemanticOutputFunction};
use gr_runtime_model::HostPlatform;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionScenarioFixture {
    pub envelope: FixtureEnvelope,
    pub scenario: SessionScenario,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionScenario {
    pub session: BackendOpenContext,
    pub backend: ScenarioBackend,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioBackend {
    pub backend_id: BackendId,
    pub family: BackendFamily,
    pub host_platform: HostPlatform,
    pub supported_fidelity_tiers: Vec<FidelityTier>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_output_functions: Vec<SemanticOutputFunction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reverse_events: Vec<BackendReverseEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<ScenarioFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioFailure {
    SendWouldBlock,
    DrainParseError,
    CloseFails,
    EventReadinessFlapping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ScenarioStep {
    Send { frame: BackendFrame },
    DrainReverse,
}

/// Decode a `session-scenario` fixture envelope into a typed scenario.
///
/// # Errors
///
/// Returns an error when the payload is not valid `session-scenario`
/// YAML for the Phase 4 fake-session surface.
pub fn decode_session_scenario(
    envelope: FixtureEnvelope,
) -> Result<SessionScenarioFixture, FixtureError> {
    let scenario = serde_yaml::from_value::<SessionScenario>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    Ok(SessionScenarioFixture { envelope, scenario })
}
