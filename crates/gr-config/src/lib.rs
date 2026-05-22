#![forbid(unsafe_code)]

//! Configuration models and validation for `virtualgamepad`.

use gr_core::{BackendLevel, FidelityTier, ProfileId, SessionId};
use gr_profiles::registry;
use serde::{Deserialize, Serialize, de::Error as _};
use serde_yaml::{Mapping, Value};
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionConfig {
    pub session: SessionSection,
    pub input: InputSection,
    #[serde(rename = "outputHandling")]
    pub output_handling: OutputHandlingSection,
    pub validation: ValidationSection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSection {
    #[serde(rename = "sessionId", default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(rename = "profileId")]
    pub profile_id: ProfileId,
    #[serde(rename = "fidelityTier")]
    pub fidelity_tier: FidelityTier,
    #[serde(
        rename = "hostPlatformPreference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub host_platform_preference: Option<HostPlatformPreference>,
    #[serde(
        rename = "backendPreference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub backend_preference: Option<BackendLevel>,
    #[serde(
        rename = "providerPreference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub provider_preference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct InputSection {
    #[serde(rename = "acceptedUpdateKinds")]
    pub accepted_update_kinds: Vec<AcceptedUpdateKind>,
    #[serde(rename = "rejectUnknownFields")]
    pub reject_unknown_fields: bool,
    #[serde(rename = "rejectOutOfRangeValues")]
    pub reject_out_of_range_values: bool,
    #[serde(rename = "coerceIntegerLikeValues", default)]
    pub coerce_integer_like_values: bool,
    #[serde(rename = "allowMissingOptionalFields")]
    pub allow_missing_optional_fields: bool,
    #[serde(rename = "requireMonotonicSequence", default)]
    pub require_monotonic_sequence: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AcceptedUpdateKind {
    Frame,
    Delta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputHandlingSection {
    pub mode: OutputHandlingMode,
    #[serde(
        rename = "callbackNamespace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub callback_namespace: Option<String>,
    #[serde(
        rename = "stateFieldPrefix",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub state_field_prefix: Option<String>,
    #[serde(rename = "backpressurePolicy")]
    pub backpressure_policy: ConfigBackpressurePolicy,
    #[serde(rename = "logDroppedOutputs", default)]
    pub log_dropped_outputs: bool,
    #[serde(
        rename = "maxQueueDepth",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_queue_depth: Option<u32>,
    #[serde(
        rename = "bridgeCapabilities",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub bridge_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputHandlingMode {
    Callback,
    Channel,
    LogOnly,
    PassThroughToPhysicalDevice,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigBackpressurePolicy {
    DropNewest,
    DropOldest,
    BlockProducer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct ValidationSection {
    #[serde(rename = "requireSupportedProfile")]
    pub require_supported_profile: bool,
    #[serde(rename = "rejectUnsupportedFidelity")]
    pub reject_unsupported_fidelity: bool,
    #[serde(rename = "rejectUnsupportedProviderPreference")]
    pub reject_unsupported_provider_preference: bool,
    #[serde(rename = "rejectUnknownConfigFields", default)]
    pub reject_unknown_config_fields: bool,
    #[serde(rename = "unsupportedCapabilityPolicy")]
    pub unsupported_capability_policy: UnsupportedCapabilityPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnsupportedCapabilityPolicy {
    Reject,
    Report,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostPlatformPreference {
    Linux,
    Windows,
    Macos,
}

impl fmt::Display for AcceptedUpdateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame => f.write_str("frame"),
            Self::Delta => f.write_str("delta"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigDiagnostic {
    pub severity: DiagnosticSeverity,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigValidationReport {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ConfigDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ConfigDiagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<SessionConfig>,
}

impl ConfigValidationReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug, Error)]
pub enum ConfigLoadError {
    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
}

const TOP_LEVEL_KEYS: &[&str] = &["session", "input", "outputHandling", "validation"];
const SESSION_KEYS: &[&str] = &[
    "sessionId",
    "profileId",
    "fidelityTier",
    "hostPlatformPreference",
    "backendPreference",
    "providerPreference",
];
const INPUT_KEYS: &[&str] = &[
    "acceptedUpdateKinds",
    "rejectUnknownFields",
    "rejectOutOfRangeValues",
    "coerceIntegerLikeValues",
    "allowMissingOptionalFields",
    "requireMonotonicSequence",
];
const OUTPUT_KEYS: &[&str] = &[
    "mode",
    "callbackNamespace",
    "stateFieldPrefix",
    "backpressurePolicy",
    "logDroppedOutputs",
    "maxQueueDepth",
    "bridgeCapabilities",
];
const VALIDATION_KEYS: &[&str] = &[
    "requireSupportedProfile",
    "rejectUnsupportedFidelity",
    "rejectUnsupportedProviderPreference",
    "rejectUnknownConfigFields",
    "unsupportedCapabilityPolicy",
];
const KNOWN_PROVIDER_IDS: &[&str] = &[
    "linux-uinput",
    "linux-uhid",
    "linux-transport-usb",
    "linux-transport-bluetooth",
    "windows-hid",
    "macos-hid",
];

/// Load, parse, and validate a configuration file from disk.
///
/// # Errors
///
/// Returns a [`ConfigLoadError`] if the file cannot be read or the
/// configuration document cannot be parsed as YAML/JSON.
pub fn load_and_validate_file(
    path: impl AsRef<Path>,
) -> Result<ConfigValidationReport, ConfigLoadError> {
    let contents = std::fs::read_to_string(path)?;
    validate_config_str(&contents)
}

/// Parse a configuration document from a string and validate it.
///
/// # Errors
///
/// Returns a [`ConfigLoadError`] if the document is not valid
/// YAML/JSON.
pub fn validate_config_str(contents: &str) -> Result<ConfigValidationReport, ConfigLoadError> {
    let value: Value = serde_yaml::from_str(contents)?;
    validate_config_value(&value)
}

/// Validate a parsed YAML configuration value.
///
/// # Errors
///
/// Returns a [`ConfigLoadError`] if the supplied value cannot be
/// interpreted as a configuration mapping or deserialized into the
/// typed config model.
pub fn validate_config_value(value: &Value) -> Result<ConfigValidationReport, ConfigLoadError> {
    let mut diagnostics = Vec::new();
    let root = value
        .as_mapping()
        .ok_or_else(|| serde_yaml::Error::custom("configuration root must be a mapping"))?;
    collect_unknown_keys(
        root,
        TOP_LEVEL_KEYS,
        "",
        DiagnosticSeverity::Error,
        &mut diagnostics,
    );

    let strict_unknown_fields = root
        .get(Value::String("validation".to_string()))
        .and_then(Value::as_mapping)
        .and_then(|mapping| mapping.get(Value::String("rejectUnknownConfigFields".to_string())))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let nested_unknown_severity = if strict_unknown_fields {
        DiagnosticSeverity::Error
    } else {
        DiagnosticSeverity::Warning
    };

    check_section_keys(
        root,
        "session",
        SESSION_KEYS,
        nested_unknown_severity,
        &mut diagnostics,
    );
    check_section_keys(
        root,
        "input",
        INPUT_KEYS,
        nested_unknown_severity,
        &mut diagnostics,
    );
    check_section_keys(
        root,
        "outputHandling",
        OUTPUT_KEYS,
        nested_unknown_severity,
        &mut diagnostics,
    );
    check_section_keys(
        root,
        "validation",
        VALIDATION_KEYS,
        nested_unknown_severity,
        &mut diagnostics,
    );

    let config: SessionConfig = serde_yaml::from_value(Value::Mapping(root.clone()))?;
    validate_semantics(&config, &mut diagnostics);

    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .cloned()
        .collect::<Vec<_>>();
    let warnings = diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
        .collect::<Vec<_>>();

    Ok(ConfigValidationReport {
        config: errors.is_empty().then_some(config),
        errors,
        warnings,
    })
}

fn check_section_keys(
    root: &Mapping,
    section_name: &str,
    allowed_keys: &[&str],
    severity: DiagnosticSeverity,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) {
    if let Some(mapping) = root
        .get(Value::String(section_name.to_string()))
        .and_then(Value::as_mapping)
    {
        collect_unknown_keys(mapping, allowed_keys, section_name, severity, diagnostics);
    }
}

fn collect_unknown_keys(
    mapping: &Mapping,
    allowed_keys: &[&str],
    prefix: &str,
    severity: DiagnosticSeverity,
    diagnostics: &mut Vec<ConfigDiagnostic>,
) {
    for key in mapping.keys().filter_map(Value::as_str) {
        if !allowed_keys.contains(&key) {
            let path = if prefix.is_empty() {
                key.to_string()
            } else {
                format!("{prefix}.{key}")
            };
            diagnostics.push(ConfigDiagnostic {
                severity,
                path,
                message: "unknown configuration field".to_string(),
            });
        }
    }
}

fn validate_semantics(config: &SessionConfig, diagnostics: &mut Vec<ConfigDiagnostic>) {
    if config.input.accepted_update_kinds.is_empty() {
        diagnostics.push(error(
            "input.acceptedUpdateKinds",
            "must declare at least one accepted update kind",
        ));
    }

    if matches!(config.output_handling.mode, OutputHandlingMode::Callback)
        && config.output_handling.callback_namespace.is_none()
    {
        diagnostics.push(error(
            "outputHandling.callbackNamespace",
            "outputHandling.mode is `callback`, so `outputHandling.callbackNamespace` must be set",
        ));
    }

    if matches!(config.output_handling.mode, OutputHandlingMode::Channel)
        && config.output_handling.state_field_prefix.is_none()
    {
        diagnostics.push(warning(
            "outputHandling.stateFieldPrefix",
            "channel mode usually sets a stateFieldPrefix for stable host field names",
        ));
    }

    validate_profile_and_fidelity(config, diagnostics);
    validate_provider_preference(config, diagnostics);
}

fn validate_profile_and_fidelity(config: &SessionConfig, diagnostics: &mut Vec<ConfigDiagnostic>) {
    let registry = registry();
    match registry.profile(config.session.profile_id.clone()) {
        Some(profile) => {
            if !profile
                .supported_fidelity
                .contains(&config.session.fidelity_tier)
            {
                let diagnostic = ConfigDiagnostic {
                    severity: if config.validation.reject_unsupported_fidelity {
                        DiagnosticSeverity::Error
                    } else {
                        DiagnosticSeverity::Warning
                    },
                    path: "session.fidelityTier".to_string(),
                    message: format!(
                        "profile `{}` does not declare fidelity `{}`",
                        config.session.profile_id.as_ref(),
                        config.session.fidelity_tier
                    ),
                };
                diagnostics.push(diagnostic);
            }
        }
        None if config.validation.require_supported_profile => diagnostics.push(error(
            "session.profileId",
            &format!(
                "unknown built-in profile `{}`",
                config.session.profile_id.as_ref()
            ),
        )),
        None => diagnostics.push(warning(
            "session.profileId",
            &format!(
                "unknown built-in profile `{}`; continuing because requireSupportedProfile is false",
                config.session.profile_id.as_ref()
            ),
        )),
    }
}

fn validate_provider_preference(config: &SessionConfig, diagnostics: &mut Vec<ConfigDiagnostic>) {
    let Some(provider) = config.session.provider_preference.as_deref() else {
        return;
    };

    if !KNOWN_PROVIDER_IDS.contains(&provider) {
        diagnostics.push(ConfigDiagnostic {
            severity: if config.validation.reject_unsupported_provider_preference {
                DiagnosticSeverity::Error
            } else {
                DiagnosticSeverity::Warning
            },
            path: "session.providerPreference".to_string(),
            message: format!(
                "unknown provider preference `{provider}`; provider preferences are hints and may be ignored"
            ),
        });
    }
}

fn error(path: &str, message: &str) -> ConfigDiagnostic {
    ConfigDiagnostic {
        severity: DiagnosticSeverity::Error,
        path: path.to_string(),
        message: message.to_string(),
    }
}

fn warning(path: &str, message: &str) -> ConfigDiagnostic {
    ConfigDiagnostic {
        severity: DiagnosticSeverity::Warning,
        path: path.to_string(),
        message: message.to_string(),
    }
}

impl FromStr for HostPlatformPreference {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "linux" => Ok(Self::Linux),
            "windows" => Ok(Self::Windows),
            "macos" => Ok(Self::Macos),
            _ => Err(format!("unknown host platform `{value}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigLoadError, DiagnosticSeverity, load_and_validate_file, validate_config_str};
    use insta::assert_snapshot;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn valid_config_snapshot_is_stable() {
        let report = load_and_validate_file(fixture_path("valid-dualsense-identity.yaml"))
            .expect("valid config");
        assert!(report.is_ok());
        assert_snapshot!(
            "valid_dualsense_identity",
            serde_yaml::to_string(&report).expect("yaml")
        );
    }

    #[test]
    fn broken_mode_fails_schema_validation() {
        let report =
            load_and_validate_file(fixture_path("invalid-broken-mode.yaml")).expect("report");
        assert!(!report.is_ok());
        assert!(
            report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.path == "outputHandling.callbackNamespace")
        );
        assert!(report.errors.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("outputHandling.mode is `callback`")
        }));
    }

    #[test]
    fn unknown_top_level_section_is_rejected() {
        let report =
            load_and_validate_file(fixture_path("invalid-unknown-top-level.yaml")).expect("report");
        assert!(!report.is_ok());
        assert!(
            report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.path == "mystery")
        );
    }

    #[test]
    fn unknown_nested_field_warns_by_default() {
        let report =
            load_and_validate_file(fixture_path("warn-unknown-session-key.yaml")).expect("report");
        assert!(report.is_ok());
        assert!(report.warnings.iter().any(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Warning
                && diagnostic.path == "session.unexpectedHint"
        }));
    }

    #[test]
    fn unknown_nested_field_rejects_when_strict() {
        let report = load_and_validate_file(fixture_path("strict-unknown-session-key.yaml"))
            .expect("report");
        assert!(!report.is_ok());
        assert!(
            report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.path == "session.unexpectedHint")
        );
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn provider_preference_warns_or_errors_based_on_strictness() {
        let warning_report = load_and_validate_file(fixture_path("warn-unknown-provider.yaml"))
            .expect("warning report");
        assert!(warning_report.is_ok());
        assert!(
            warning_report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.path == "session.providerPreference")
        );

        let error_report = load_and_validate_file(fixture_path("strict-unknown-provider.yaml"))
            .expect("error report");
        assert!(!error_report.is_ok());
        assert!(
            error_report
                .errors
                .iter()
                .any(|diagnostic| diagnostic.path == "session.providerPreference")
        );
    }

    #[test]
    fn unsupported_fidelity_is_configurable() {
        let report = load_and_validate_file(fixture_path("warn-unsupported-fidelity.yaml"))
            .expect("warning report");
        assert!(report.is_ok());
        assert!(
            report
                .warnings
                .iter()
                .any(|diagnostic| diagnostic.path == "session.fidelityTier")
        );
    }

    #[test]
    fn parse_error_bubbles_up() {
        let error = validate_config_str("session: [").expect_err("parse should fail");
        assert!(matches!(error, ConfigLoadError::Parse(_)));
    }
}
