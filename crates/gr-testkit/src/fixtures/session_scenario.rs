//! `session-scenario` fixture support across the Phase 4 and Phase 7
//! runtime surfaces.

use super::schema::{FixtureEnvelope, FixtureError};
use gr_backend_api::{BackendFrame, BackendOpenContext, BackendReverseEvent};
use gr_core::{
    BackendFamily, BackendId, FidelityTier, ProfileInputDelta, ProfileInputFrame,
    SemanticOutputFunction, SessionId,
};
use gr_runtime_model::{HostPlatform, SessionLifecycleState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionScenarioFixture {
    pub envelope: FixtureEnvelope,
    pub scenario: SessionScenarioDocument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionScenarioDocument {
    Legacy(LegacySessionScenario),
    Runtime(RuntimeSessionScenario),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacySessionScenario {
    pub session: BackendOpenContext,
    pub backend: ScenarioBackend,
    pub steps: Vec<LegacyScenarioStep>,
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
    OpenRefused,
    SendPermanentlyFails,
    ProviderPanic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LegacyScenarioStep {
    Send { frame: BackendFrame },
    DrainReverse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSessionScenario {
    pub session: RuntimeSessionConfig,
    pub backend: ScenarioBackend,
    pub steps: Vec<RuntimeScenarioStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSessionConfig {
    pub session_id: SessionId,
    pub profile_id: gr_core::ProfileId,
    pub fidelity_tier: FidelityTier,
    pub backend_level: gr_core::BackendLevel,
    pub host_platform: HostPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RuntimeScenarioStep {
    SendInput { frame: ProfileInputFrame },
    SendInputDelta { delta: ProfileInputDelta },
    InjectReverse { event: BackendReverseEvent },
    SleepMs { millis: u64 },
    CloseSession,
    AssertFramesWritten { at_least: usize },
    AssertCounter { key: String, at_least: u64 },
    AssertOutputCount { at_least: usize },
    AssertSessionState { state: SessionLifecycleState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum RawSessionScenario {
    Runtime(RuntimeSessionScenario),
    Legacy(LegacySessionScenario),
}

/// Decode a `session-scenario` fixture into either the legacy Phase 4
/// model or the richer runtime-oriented Phase 7 model.
///
/// # Errors
///
/// Returns [`FixtureError`] when the payload does not match either
/// supported scenario shape.
pub fn decode_session_scenario(
    envelope: FixtureEnvelope,
) -> Result<SessionScenarioFixture, FixtureError> {
    let scenario = serde_yaml::from_value::<RawSessionScenario>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    Ok(SessionScenarioFixture {
        envelope,
        scenario: match scenario {
            RawSessionScenario::Legacy(scenario) => SessionScenarioDocument::Legacy(scenario),
            RawSessionScenario::Runtime(scenario) => SessionScenarioDocument::Runtime(scenario),
        },
    })
}
