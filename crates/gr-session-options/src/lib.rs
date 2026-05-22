#![forbid(unsafe_code)]

//! Session option compilation for `virtualgamepad`.

use gr_config::{
    AcceptedUpdateKind, ConfigBackpressurePolicy, HostPlatformPreference, OutputHandlingMode,
    SessionConfig,
};
use gr_runtime_model::{
    BackpressurePolicy, HostPlatform, ProviderId, ReverseEventDeliveryPolicy,
    SessionOptionsSnapshot,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledSessionOptions {
    pub input_validation_policy: InputValidationPolicy,
    pub provider_hints: ProviderHints,
    pub delivery_policy: ReverseEventDeliveryPolicy,
    pub backpressure_policy: BackpressurePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputValidationPolicy {
    pub accepted_update_kinds: Vec<AcceptedUpdateKind>,
    pub unknown_field_policy: UnknownFieldPolicy,
    pub range_validation_policy: RangeValidationPolicy,
    pub coerce_integer_like_values: bool,
    pub allow_missing_optional_fields: bool,
    pub require_monotonic_sequence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderHints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_platform_preference: Option<HostPlatform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_provider: Option<ProviderId>,
    pub reject_unsupported_provider_preference: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnknownFieldPolicy {
    Reject,
    Warn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RangeValidationPolicy {
    Reject,
    Allow,
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("acceptedUpdateKinds must not be empty")]
    EmptyAcceptedUpdateKinds,
}

/// Compile validated config policy into immutable runtime session
/// options.
///
/// # Errors
///
/// Returns a [`CompileError`] if the config is structurally valid but
/// cannot be compiled into runtime policy values.
pub fn compile_session_options(
    config: &SessionConfig,
) -> Result<CompiledSessionOptions, CompileError> {
    if config.input.accepted_update_kinds.is_empty() {
        return Err(CompileError::EmptyAcceptedUpdateKinds);
    }

    Ok(CompiledSessionOptions {
        input_validation_policy: InputValidationPolicy {
            accepted_update_kinds: config.input.accepted_update_kinds.clone(),
            unknown_field_policy: if config.input.reject_unknown_fields {
                UnknownFieldPolicy::Reject
            } else {
                UnknownFieldPolicy::Warn
            },
            range_validation_policy: if config.input.reject_out_of_range_values {
                RangeValidationPolicy::Reject
            } else {
                RangeValidationPolicy::Allow
            },
            coerce_integer_like_values: config.input.coerce_integer_like_values,
            allow_missing_optional_fields: config.input.allow_missing_optional_fields,
            require_monotonic_sequence: config.input.require_monotonic_sequence,
        },
        provider_hints: ProviderHints {
            host_platform_preference: config
                .session
                .host_platform_preference
                .map(host_platform_from_config),
            preferred_provider: config
                .session
                .provider_preference
                .as_deref()
                .map(ProviderId::from),
            reject_unsupported_provider_preference: config
                .validation
                .reject_unsupported_provider_preference,
        },
        delivery_policy: delivery_policy_from_config(config),
        backpressure_policy: backpressure_policy_from_config(config),
    })
}

impl CompiledSessionOptions {
    #[must_use]
    pub fn snapshot(&self) -> SessionOptionsSnapshot {
        SessionOptionsSnapshot {
            accepted_update_kinds: self
                .input_validation_policy
                .accepted_update_kinds
                .iter()
                .map(ToString::to_string)
                .collect(),
            unknown_field_policy: serde_name(&self.input_validation_policy.unknown_field_policy),
            range_validation_policy: serde_name(
                &self.input_validation_policy.range_validation_policy,
            ),
            coerce_integer_like_values: self.input_validation_policy.coerce_integer_like_values,
            allow_missing_optional_fields: self
                .input_validation_policy
                .allow_missing_optional_fields,
            require_monotonic_sequence: self.input_validation_policy.require_monotonic_sequence,
            preferred_provider: self.provider_hints.preferred_provider.clone(),
            reject_unsupported_provider_preference: self
                .provider_hints
                .reject_unsupported_provider_preference,
            delivery_policy: self.delivery_policy.clone(),
            backpressure_policy: self.backpressure_policy.clone(),
        }
    }
}

fn host_platform_from_config(value: HostPlatformPreference) -> HostPlatform {
    match value {
        HostPlatformPreference::Linux => HostPlatform::Linux,
        HostPlatformPreference::Windows => HostPlatform::Windows,
        HostPlatformPreference::Macos => HostPlatform::Macos,
    }
}

fn delivery_policy_from_config(config: &SessionConfig) -> ReverseEventDeliveryPolicy {
    match config.output_handling.mode {
        OutputHandlingMode::Callback => ReverseEventDeliveryPolicy::Callback {
            callback_namespace: config
                .output_handling
                .callback_namespace
                .clone()
                .unwrap_or_else(|| "virtualGamepad".to_string()),
        },
        OutputHandlingMode::Channel => ReverseEventDeliveryPolicy::Channel {
            state_field_prefix: config.output_handling.state_field_prefix.clone(),
        },
        OutputHandlingMode::LogOnly => ReverseEventDeliveryPolicy::LogOnly,
        OutputHandlingMode::PassThroughToPhysicalDevice => {
            ReverseEventDeliveryPolicy::PassThroughToPhysicalDevice
        }
        OutputHandlingMode::Ignore => ReverseEventDeliveryPolicy::Ignore,
    }
}

fn backpressure_policy_from_config(config: &SessionConfig) -> BackpressurePolicy {
    match config.output_handling.backpressure_policy {
        ConfigBackpressurePolicy::DropNewest => BackpressurePolicy::DropNewest {
            log_dropped_outputs: config.output_handling.log_dropped_outputs,
            max_queue_depth: config.output_handling.max_queue_depth,
        },
        ConfigBackpressurePolicy::DropOldest => BackpressurePolicy::DropOldest {
            log_dropped_outputs: config.output_handling.log_dropped_outputs,
            max_queue_depth: config.output_handling.max_queue_depth,
        },
        ConfigBackpressurePolicy::BlockProducer => BackpressurePolicy::BlockProducer {
            log_dropped_outputs: config.output_handling.log_dropped_outputs,
            max_queue_depth: config.output_handling.max_queue_depth,
        },
    }
}

fn serde_name<T: Serialize>(value: &T) -> String {
    serde_yaml::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "<unknown>".to_string())
}

#[cfg(test)]
mod tests {
    use super::{CompileError, compile_session_options};
    use gr_config::{
        AcceptedUpdateKind, ConfigBackpressurePolicy, HostPlatformPreference, InputSection,
        OutputHandlingMode, OutputHandlingSection, SessionConfig, SessionSection,
        UnsupportedCapabilityPolicy, ValidationSection, load_and_validate_file,
    };
    use gr_core::{BackendLevel, FidelityTier, ProfileId};
    use insta::assert_snapshot;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../gr-config/fixtures")
            .join(name)
    }

    #[test]
    fn compiled_options_snapshot_is_stable() {
        let report = load_and_validate_file(fixture_path("valid-dualsense-identity.yaml"))
            .expect("config report");
        let config = report.config.expect("valid config");
        let compiled = compile_session_options(&config).expect("compile options");

        assert_snapshot!(
            "compiled_session_options",
            serde_yaml::to_string(&compiled).expect("yaml")
        );
    }

    #[test]
    fn compiled_snapshot_is_stable() {
        let report = load_and_validate_file(fixture_path("valid-dualsense-identity.yaml"))
            .expect("config report");
        let config = report.config.expect("valid config");
        let compiled = compile_session_options(&config).expect("compile options");

        assert_snapshot!(
            "compiled_session_options_snapshot",
            serde_yaml::to_string(&compiled.snapshot()).expect("yaml")
        );
    }

    #[test]
    fn empty_update_kinds_is_rejected() {
        let config = SessionConfig {
            session: SessionSection {
                session_id: None,
                profile_id: ProfileId::from("dualsense"),
                fidelity_tier: FidelityTier::IdentityAware,
                host_platform_preference: Some(HostPlatformPreference::Linux),
                backend_preference: Some(BackendLevel::Hid),
                provider_preference: Some("linux-uhid".to_string()),
            },
            input: InputSection {
                accepted_update_kinds: Vec::<AcceptedUpdateKind>::new(),
                reject_unknown_fields: true,
                reject_out_of_range_values: true,
                coerce_integer_like_values: false,
                allow_missing_optional_fields: true,
                require_monotonic_sequence: false,
            },
            output_handling: OutputHandlingSection {
                mode: OutputHandlingMode::Callback,
                callback_namespace: Some("virtualGamepad".to_string()),
                state_field_prefix: None,
                backpressure_policy: ConfigBackpressurePolicy::DropOldest,
                log_dropped_outputs: true,
                max_queue_depth: Some(8),
                bridge_capabilities: vec!["leftRumble".to_string()],
            },
            validation: ValidationSection {
                require_supported_profile: true,
                reject_unsupported_fidelity: true,
                reject_unsupported_provider_preference: true,
                reject_unknown_config_fields: false,
                unsupported_capability_policy: UnsupportedCapabilityPolicy::Report,
            },
        };
        let error = compile_session_options(&config).expect_err("compile should fail");
        assert!(matches!(error, CompileError::EmptyAcceptedUpdateKinds));
    }
}
