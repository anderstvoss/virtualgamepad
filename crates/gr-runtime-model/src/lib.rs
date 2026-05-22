#![forbid(unsafe_code)]

//! Runtime contracts for `virtualgamepad`.

use gr_core::{
    BackendFamily, BackendLevel, FidelityTier, ProfileId, SemanticOutputFunction, SessionId,
    Timestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }
    };
}

string_newtype!(ProviderId);
string_newtype!(ProfileSpecificOutputFunctionId);
string_newtype!(ProfileSpecificOutputPayloadId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostPlatform {
    Linux,
    Windows,
    Macos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmulationGoal {
    Compatibility,
    IdentityAware,
    HardwareFaithful,
}

impl From<FidelityTier> for EmulationGoal {
    fn from(value: FidelityTier) -> Self {
        match value {
            FidelityTier::Compatibility => Self::Compatibility,
            FidelityTier::IdentityAware => Self::IdentityAware,
            FidelityTier::HardwareFaithful => Self::HardwareFaithful,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranslatorFamily {
    Unresolved,
    GenericGamepad,
    XboxStyle,
    DualSense,
    SteamController,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SessionHostMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_version: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReverseEventDeliveryPolicy {
    Callback {
        callback_namespace: String,
    },
    Channel {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state_field_prefix: Option<String>,
    },
    LogOnly,
    PassThroughToPhysicalDevice,
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackpressurePolicy {
    DropNewest {
        #[serde(default)]
        log_dropped_outputs: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_queue_depth: Option<u32>,
    },
    DropOldest {
        #[serde(default)]
        log_dropped_outputs: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_queue_depth: Option<u32>,
    },
    BlockProducer {
        #[serde(default)]
        log_dropped_outputs: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_queue_depth: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SessionOptionsSnapshot {
    pub accepted_update_kinds: Vec<String>,
    pub unknown_field_policy: String,
    pub range_validation_policy: String,
    pub coerce_integer_like_values: bool,
    pub allow_missing_optional_fields: bool,
    pub require_monotonic_sequence: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_provider: Option<ProviderId>,
    pub reject_unsupported_provider_preference: bool,
    pub delivery_policy: ReverseEventDeliveryPolicy,
    pub backpressure_policy: BackpressurePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRequest {
    pub profile_id: ProfileId,
    pub goal: EmulationGoal,
    pub requested_fidelity_tier: FidelityTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_platform_preference: Option<HostPlatform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_preference: Option<BackendLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_preference: Option<ProviderId>,
    #[serde(default)]
    pub host_metadata: SessionHostMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CapabilityNegotiationResult {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsupported_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DegradationReport {
    pub degraded: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeploymentRequirements {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SessionPlan {
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<ProfileId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_goal: Option<EmulationGoal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_fidelity_tier: Option<FidelityTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_level: Option<BackendLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_host_platform: Option<HostPlatform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_backend_family: Option<BackendFamily>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_provider_id: Option<ProviderId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_translator_family: Option<TranslatorFamily>,
    #[serde(default)]
    pub capability_result: CapabilityNegotiationResult,
    #[serde(default)]
    pub degradation: DegradationReport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<PlannerWarning>,
    #[serde(default)]
    pub deployment_requirements: DeploymentRequirements,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_options: Option<SessionOptionsSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DescriptorTemplateSummary {
    pub fidelity: Option<FidelityTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descriptor_len: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TranslationConstants {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PreparedTranslationContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_platform: Option<HostPlatform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_family: Option<BackendFamily>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<ProviderId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<BackendLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_options: Option<SessionOptionsSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub descriptor_template: Option<DescriptorTemplateSummary>,
    #[serde(default)]
    pub translation_constants: TranslationConstants,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputCommandType {
    StateUpdate,
    Notification,
    FeatureRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OutputFunctionRef {
    Semantic(SemanticOutputFunction),
    ProfileSpecific(ProfileSpecificOutputFunctionId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RumblePayload {
    pub strong: u16,
    pub weak: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub red: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub green: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blue: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_index: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerEffectPayload {
    pub mode: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioCommand {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureRequestPayload {
    pub request: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileSpecificOutputPayload {
    pub payload_id: ProfileSpecificOutputPayloadId,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum OutputPayload {
    Rumble(RumblePayload),
    Lighting(LightingPayload),
    TriggerEffect(TriggerEffectPayload),
    Audio(AudioCommand),
    FeatureRequest(FeatureRequestPayload),
    ProfileSpecific(ProfileSpecificOutputPayload),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerOutputCommand {
    pub session_id: SessionId,
    pub profile_id: ProfileId,
    pub timestamp: Timestamp,
    pub command_type: OutputCommandType,
    pub function: OutputFunctionRef,
    pub payload: OutputPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionLifecycleState {
    Created,
    Planned,
    Running,
    Closing,
    Closed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStatusSnapshot {
    pub state: SessionLifecycleState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<ProfileId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SessionDiagnosticsSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub counters: BTreeMap<String, u64>,
}

#[cfg(test)]
mod tests {
    use super::{
        AudioCommand, BackpressurePolicy, ControllerOutputCommand, EmulationGoal,
        OutputCommandType, OutputFunctionRef, OutputPayload, ReverseEventDeliveryPolicy,
        RumblePayload, SessionLifecycleState, SessionOptionsSnapshot, SessionPlan,
        SessionStatusSnapshot,
    };
    use gr_core::{FidelityTier, ProfileId, SessionId, Timestamp};
    use insta::assert_snapshot;

    #[test]
    fn session_plan_yaml_is_stable() {
        let plan = SessionPlan {
            session_id: Some(SessionId::new(7)),
            profile_id: Some(ProfileId::from("dualsense")),
            requested_goal: Some(EmulationGoal::IdentityAware),
            requested_fidelity_tier: Some(FidelityTier::IdentityAware),
            session_options: Some(SessionOptionsSnapshot {
                accepted_update_kinds: vec!["frame".to_string(), "delta".to_string()],
                unknown_field_policy: "reject".to_string(),
                range_validation_policy: "reject".to_string(),
                coerce_integer_like_values: false,
                allow_missing_optional_fields: true,
                require_monotonic_sequence: false,
                preferred_provider: Some("linux-uhid".into()),
                reject_unsupported_provider_preference: true,
                delivery_policy: ReverseEventDeliveryPolicy::Callback {
                    callback_namespace: "virtualGamepad".to_string(),
                },
                backpressure_policy: BackpressurePolicy::DropOldest {
                    log_dropped_outputs: true,
                    max_queue_depth: Some(8),
                },
            }),
            ..SessionPlan::default()
        };

        assert_snapshot!(
            "session_plan",
            serde_yaml::to_string(&plan).expect("session plan yaml")
        );
    }

    #[test]
    fn controller_output_command_yaml_is_stable() {
        let command = ControllerOutputCommand {
            session_id: SessionId::new(9),
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(123),
            command_type: OutputCommandType::Notification,
            function: OutputFunctionRef::Semantic(gr_core::SemanticOutputFunction::Audio),
            payload: OutputPayload::Audio(AudioCommand {
                action: "mute-toggle".to_string(),
                target: Some("speaker".to_string()),
            }),
        };

        assert_snapshot!(
            "controller_output_command",
            serde_yaml::to_string(&command).expect("command yaml")
        );
    }

    #[test]
    fn status_snapshot_yaml_is_stable() {
        let status = SessionStatusSnapshot {
            state: SessionLifecycleState::Running,
            session_id: Some(SessionId::new(1)),
            profile_id: Some(ProfileId::from("xbox360")),
            warnings: vec!["degraded to compatibility".to_string()],
        };

        assert_snapshot!(
            "session_status",
            serde_yaml::to_string(&status).expect("status yaml")
        );
    }

    #[test]
    fn payload_variants_serialize() {
        let payload = OutputPayload::Rumble(RumblePayload {
            strong: 10,
            weak: 20,
        });
        let yaml = serde_yaml::to_string(&payload).expect("payload yaml");
        assert!(yaml.contains("strong: 10"));
        assert!(yaml.contains("weak: 20"));
    }
}
