//! Shared implementation for the `gr-cli` binary and other tooling.

use gr_core::{
    CoreError, ProfileId, ProfileInputDelta, ProfileInputDeltaPayload, ProfileInputFrame,
    ProfileInputPayload, SemanticInputFunction, SequenceId, Timestamp,
};
use gr_profiles::{
    CapabilityItem, CapabilityRegistry, ControllerProfile, OutputFunctionRef, RegistryError,
    SemanticRef, registry,
};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

const PHASE_0_COMMANDS: &[&[&str]] = &[
    &["cargo", "build", "--workspace", "--all-features"],
    &["cargo", "test", "--workspace", "--all-features"],
    &[
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ],
];

const PHASE_1_COMMANDS: &[&[&str]] = &[
    &["cargo", "test", "--workspace", "--all-features"],
    &["cargo", "insta", "test", "--check"],
    &[
        "cargo",
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ],
];

const PHASE_2_COMMANDS: &[&[&str]] = &[
    &["cargo", "test", "--workspace", "--all-features"],
    &["cargo", "insta", "test", "--check"],
    &["cargo", "run", "-p", "gr-cli", "--", "capability-coverage"],
];

/// Validate a fixture path and summarize the decoded envelope.
///
/// # Errors
///
/// Returns an error if the path cannot be read, the YAML cannot be
/// parsed, or the fixture envelope is invalid.
pub fn validate_fixture(path: impl AsRef<Path>) -> Result<String, CliError> {
    let path = path.as_ref();
    let fixture = load_fixture_summary(path).map_err(|source| CliError::Fixture {
        path: path.to_path_buf(),
        source,
    })?;

    match fixture {
        FixtureDocument::Envelope(fixture) => Ok(format!(
            "fixture: {}\nkind: {}\nid: {}\nprofile_id: {}\npayload_type: {}",
            fixture.fixture,
            fixture.kind,
            fixture.id,
            fixture.profile_id.as_deref().unwrap_or("<none>"),
            yaml_value_kind(&fixture.payload),
        )),
        FixtureDocument::InputFrame(fixture) => Ok(format!(
            "fixture: {}\nkind: {}\nid: {}\nprofile_id: {}\npayload_type: {}",
            fixture.envelope.fixture,
            fixture.envelope.kind,
            fixture.envelope.id,
            fixture.frame.profile_id,
            fixture.frame.payload.variant_name(),
        )),
        FixtureDocument::InputDelta(fixture) => Ok(format!(
            "fixture: {}\nkind: {}\nid: {}\nprofile_id: {}\npayload_type: {}",
            fixture.envelope.fixture,
            fixture.envelope.kind,
            fixture.envelope.id,
            fixture.delta.profile_id,
            fixture.delta.payload.variant_name(),
        )),
    }
}

/// List the built-in controller profiles.
///
/// # Errors
///
/// Returns an error if the YAML output cannot be serialized.
pub fn list_profiles() -> Result<String, CliError> {
    let profiles = registry()
        .profiles()
        .iter()
        .map(ProfileListEntry::from)
        .collect::<Vec<_>>();
    serde_yaml::to_string(&profiles).map_err(CliError::SerializeYaml)
}

/// Print the declared capabilities of a built-in profile by id.
///
/// # Errors
///
/// Returns an error if the profile id is unknown or the YAML output
/// cannot be serialized.
pub fn show_capabilities(profile_id: &str) -> Result<String, CliError> {
    let profile =
        registry()
            .profile_by_str(profile_id)
            .ok_or_else(|| CliError::UnknownProfile {
                profile_id: profile_id.to_string(),
            })?;
    serde_yaml::to_string(&ProfileCapabilitySummary::from(profile)).map_err(CliError::SerializeYaml)
}

/// Cross-check declared capabilities against Phase 2 registry rules.
///
/// # Errors
///
/// This operation is purely in-memory and only fails if the report
/// cannot be assembled, which does not currently happen.
pub fn capability_coverage() -> Result<CapabilityCoverageReport, CliError> {
    let registry = registry();
    let gaps = registry
        .profiles()
        .iter()
        .flat_map(|profile| collect_profile_gaps(registry, profile))
        .collect::<Vec<_>>();
    Ok(CapabilityCoverageReport { gaps })
}

/// Coverage report produced by [`capability_coverage`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CapabilityCoverageReport {
    pub gaps: Vec<CapabilityGap>,
}

impl CapabilityCoverageReport {
    #[must_use]
    pub fn all_covered(&self) -> bool {
        self.gaps.is_empty()
    }
}

/// A declared capability with no satisfying Phase 2 registry rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityGap {
    pub profile_id: String,
    pub capability: String,
    pub reason: String,
}

/// Execute the deterministic automated portion of a phase gate.
///
/// # Errors
///
/// Returns an error when `phase` is unsupported or the workspace root
/// cannot be resolved from the current crate location.
pub fn run_phase_gate_auto(phase: u8) -> Result<PhaseGateReport, CliError> {
    let commands = phase_gate_commands(phase)?;
    let repo_root = repo_root()?;
    let checks = commands
        .iter()
        .map(|command| run_phase_gate_command(&repo_root, command))
        .collect::<Vec<_>>();

    Ok(PhaseGateReport { phase, checks })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseGateReport {
    pub phase: u8,
    pub checks: Vec<PhaseGateCheckResult>,
}

impl PhaseGateReport {
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|check| check.success)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseGateCheckResult {
    pub command_display: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub enum CliError {
    Fixture {
        path: PathBuf,
        source: FixtureError,
    },
    UnknownPhase {
        phase: u8,
    },
    UnimplementedPhase {
        phase: u8,
    },
    UnknownProfile {
        profile_id: String,
    },
    SerializeYaml(serde_yaml::Error),
    WorkspaceRootNotFound {
        start: PathBuf,
    },
    CommandLaunch {
        command_display: String,
        source: std::io::Error,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture { path, source } => write!(f, "{}: {source}", path.display()),
            Self::UnknownPhase { phase } => {
                write!(f, "unknown phase `{phase}`; expected a value from 0 to 12")
            }
            Self::UnimplementedPhase { phase } => {
                write!(f, "automated gate not implemented for phase `{phase}` yet")
            }
            Self::UnknownProfile { profile_id } => write!(f, "unknown profile `{profile_id}`"),
            Self::SerializeYaml(source) => write!(f, "failed to serialize yaml output: {source}"),
            Self::WorkspaceRootNotFound { start } => write!(
                f,
                "could not locate workspace root from `{}`",
                start.display()
            ),
            Self::CommandLaunch {
                command_display,
                source,
            } => write!(f, "failed to launch `{command_display}`: {source}"),
        }
    }
}

impl std::error::Error for CliError {}

const FIXTURE_SCHEMA_VERSION: &str = "virtualgamepad/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FixtureEnvelope {
    fixture: String,
    kind: String,
    id: String,
    #[serde(default)]
    profile_id: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FixtureDocument {
    Envelope(FixtureEnvelope),
    InputFrame(InputFrameFixture),
    InputDelta(InputDeltaFixture),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RawInputFramePayload {
    timestamp: Timestamp,
    sequence: SequenceId,
    #[serde(flatten)]
    payload: ProfileInputPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputFrameFixture {
    envelope: FixtureEnvelope,
    frame: ProfileInputFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RawInputDeltaPayload {
    timestamp: Timestamp,
    sequence: SequenceId,
    #[serde(flatten)]
    payload: ProfileInputDeltaPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputDeltaFixture {
    envelope: FixtureEnvelope,
    delta: ProfileInputDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProfileListEntry {
    profile_id: String,
    display_name: &'static str,
    profile_family: String,
    supported_fidelity: Vec<String>,
}

impl From<&ControllerProfile> for ProfileListEntry {
    fn from(profile: &ControllerProfile) -> Self {
        Self {
            profile_id: profile.profile_id.to_string(),
            display_name: profile.display_name,
            profile_family: profile_family_name(profile.profile_family).to_string(),
            supported_fidelity: profile
                .supported_fidelity
                .iter()
                .map(ToString::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ProfileCapabilitySummary {
    profile_id: String,
    display_name: &'static str,
    profile_family: String,
    identity: gr_profiles::ProfileIdentity,
    supported_fidelity: Vec<String>,
    input_capabilities: Vec<CapabilitySummaryItem>,
    output_capabilities: Vec<CapabilitySummaryItem>,
    reverse_command_support: Vec<String>,
    input_contract: gr_profiles::ProfileInputContract,
    descriptor_templates: Vec<DescriptorTemplateSummary>,
}

impl From<&ControllerProfile> for ProfileCapabilitySummary {
    fn from(profile: &ControllerProfile) -> Self {
        Self {
            profile_id: profile.profile_id.to_string(),
            display_name: profile.display_name,
            profile_family: profile_family_name(profile.profile_family).to_string(),
            identity: profile.identity,
            supported_fidelity: profile
                .supported_fidelity
                .iter()
                .map(ToString::to_string)
                .collect(),
            input_capabilities: profile
                .capabilities
                .input
                .iter()
                .copied()
                .map(CapabilitySummaryItem::from)
                .collect(),
            output_capabilities: profile
                .capabilities
                .output
                .iter()
                .copied()
                .map(CapabilitySummaryItem::from)
                .collect(),
            reverse_command_support: profile
                .reverse_command_support
                .supported
                .iter()
                .copied()
                .map(output_function_name)
                .collect(),
            input_contract: profile.input_contract,
            descriptor_templates: profile
                .descriptor_templates
                .iter()
                .map(DescriptorTemplateSummary::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CapabilitySummaryItem {
    category: String,
    semantic: String,
    optionality: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    covered_fields: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<gr_profiles::ValueRange>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    range_applies_to: Vec<String>,
}

impl From<CapabilityItem> for CapabilitySummaryItem {
    fn from(capability: CapabilityItem) -> Self {
        let semantic = match capability.semantic {
            SemanticRef::Input(semantic) => semantic.to_string(),
            SemanticRef::Output(semantic) => semantic.to_string(),
        };
        let covered_fields = capability_fields(capability);
        let range_applies_to = if capability.range.is_some() {
            covered_fields.clone()
        } else {
            Vec::new()
        };

        Self {
            category: capability.category.to_string(),
            semantic,
            optionality: serde_name(&capability.optionality),
            covered_fields,
            range: capability.range,
            range_applies_to,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DescriptorTemplateSummary {
    fidelity: String,
    descriptor_len: usize,
}

impl From<&gr_profiles::DescriptorTemplate> for DescriptorTemplateSummary {
    fn from(template: &gr_profiles::DescriptorTemplate) -> Self {
        Self {
            fidelity: template.fidelity.to_string(),
            descriptor_len: template.descriptor.0.len(),
        }
    }
}

fn profile_family_name(family: gr_profiles::ProfileFamily) -> &'static str {
    match family {
        gr_profiles::ProfileFamily::GenericGamepad => "generic-gamepad",
        gr_profiles::ProfileFamily::Xbox360 => "xbox360",
        gr_profiles::ProfileFamily::DualSense => "dualsense",
        gr_profiles::ProfileFamily::SteamController => "steam-controller",
        _ => "unknown",
    }
}

fn yaml_value_kind(value: &serde_yaml::Value) -> &'static str {
    match value {
        serde_yaml::Value::Null => "null",
        serde_yaml::Value::Bool(_) => "bool",
        serde_yaml::Value::Number(_) => "number",
        serde_yaml::Value::String(_) => "string",
        serde_yaml::Value::Sequence(_) => "sequence",
        serde_yaml::Value::Mapping(_) => "mapping",
        serde_yaml::Value::Tagged(_) => "tagged",
    }
}

fn load_fixture_summary(path: impl AsRef<Path>) -> Result<FixtureDocument, FixtureError> {
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
        "backend-trace" | "reverse-event" | "plan-snapshot" | "session-scenario" => {
            Ok(FixtureDocument::Envelope(envelope))
        }
        other => Err(FixtureError::UnsupportedKind {
            kind: other.to_owned(),
        }),
    }
}

fn decode_input_frame(envelope: FixtureEnvelope) -> Result<InputFrameFixture, FixtureError> {
    let profile_id = envelope
        .profile_id
        .clone()
        .ok_or(FixtureError::MissingProfileId)?;
    let payload = serde_yaml::from_value::<RawInputFramePayload>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    let frame = ProfileInputFrame {
        profile_id: ProfileId::from(profile_id),
        timestamp: payload.timestamp,
        sequence: payload.sequence,
        payload: payload.payload,
    };
    frame.validate().map_err(|source| match source {
        CoreError::ProfilePayloadMismatch { .. } | CoreError::UnknownHumanName { .. } => {
            FixtureError::ProfilePayloadMismatch { source }
        }
    })?;

    Ok(InputFrameFixture { envelope, frame })
}

fn decode_input_delta(envelope: FixtureEnvelope) -> Result<InputDeltaFixture, FixtureError> {
    let profile_id = envelope
        .profile_id
        .clone()
        .ok_or(FixtureError::MissingProfileId)?;
    let payload = serde_yaml::from_value::<RawInputDeltaPayload>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    let delta = ProfileInputDelta {
        profile_id: ProfileId::from(profile_id),
        timestamp: payload.timestamp,
        sequence: payload.sequence,
        payload: payload.payload,
    };
    delta.validate().map_err(|source| match source {
        CoreError::ProfilePayloadMismatch { .. } | CoreError::UnknownHumanName { .. } => {
            FixtureError::ProfilePayloadMismatch { source }
        }
    })?;

    Ok(InputDeltaFixture { envelope, delta })
}

fn collect_profile_gaps(
    registry: CapabilityRegistry,
    profile: &ControllerProfile,
) -> Vec<CapabilityGap> {
    let mut gaps = Vec::new();

    if let Err(error) = registry.validate_profile_contract(profile) {
        gaps.push(capability_gap(profile, "registry", &error));
    }

    for capability in profile.capabilities.input {
        if !matches!(capability.semantic, SemanticRef::Input(_)) {
            gaps.push(CapabilityGap {
                profile_id: profile.profile_id.to_string(),
                capability: capability_label(*capability),
                reason: "input capability used output semantic".to_string(),
            });
        }
    }

    for capability in profile.capabilities.output {
        if !matches!(capability.semantic, SemanticRef::Output(_)) {
            gaps.push(CapabilityGap {
                profile_id: profile.profile_id.to_string(),
                capability: capability_label(*capability),
                reason: "output capability used input semantic".to_string(),
            });
        }
    }

    for function in profile.reverse_command_support.supported {
        let declared = profile.capabilities.output.iter().any(|capability| {
            matches!(
                (capability.semantic, function),
                (SemanticRef::Output(output), OutputFunctionRef::Semantic(expected))
                    if output == *expected
            )
        });
        if !declared {
            gaps.push(CapabilityGap {
                profile_id: profile.profile_id.to_string(),
                capability: output_function_name(*function),
                reason: "reverse support has no matching output capability".to_string(),
            });
        }
    }

    gaps
}

fn capability_gap(
    profile: &ControllerProfile,
    capability: &str,
    error: &RegistryError,
) -> CapabilityGap {
    CapabilityGap {
        profile_id: profile.profile_id.to_string(),
        capability: capability.to_string(),
        reason: error.to_string(),
    }
}

fn capability_label(capability: CapabilityItem) -> String {
    match capability.semantic {
        SemanticRef::Input(semantic) => format!("{}:{}", capability.category, semantic),
        SemanticRef::Output(semantic) => format!("{}:{}", capability.category, semantic),
    }
}

fn output_function_name(function: OutputFunctionRef) -> String {
    match function {
        OutputFunctionRef::Semantic(semantic) => semantic.to_string(),
        _ => "unknown".to_string(),
    }
}

fn capability_fields(capability: CapabilityItem) -> Vec<String> {
    match capability.semantic {
        SemanticRef::Input(SemanticInputFunction::LeftStick) => {
            vec!["sticks.left_x".to_string(), "sticks.left_y".to_string()]
        }
        SemanticRef::Input(SemanticInputFunction::RightStick) => {
            vec!["sticks.right_x".to_string(), "sticks.right_y".to_string()]
        }
        _ => Vec::new(),
    }
}

fn serde_name<T: Serialize>(value: &T) -> String {
    serde_yaml::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn run_phase_gate_command(repo_root: &Path, command: &[String]) -> PhaseGateCheckResult {
    let command_display = command.join(" ");
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(repo_root)
        .output();

    match output {
        Ok(output) => PhaseGateCheckResult {
            command_display,
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(source) => PhaseGateCheckResult {
            command_display: command_display.clone(),
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("failed to launch `{command_display}`: {source}"),
        },
    }
}

fn phase_gate_commands(phase: u8) -> Result<Vec<Vec<String>>, CliError> {
    match phase {
        0 => Ok(PHASE_0_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        1 => Ok(PHASE_1_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        2 => Ok(PHASE_2_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        3..=12 => Err(CliError::UnimplementedPhase { phase }),
        _ => Err(CliError::UnknownPhase { phase }),
    }
}

fn repo_root() -> Result<PathBuf, CliError> {
    let start = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    repo_root_from(&start)
}

fn repo_root_from(start: &Path) -> Result<PathBuf, CliError> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join("Cargo.toml").is_file() && path.join("demo").is_dir() {
            return Ok(path.to_path_buf());
        }
        current = path.parent();
    }

    Err(CliError::WorkspaceRootNotFound {
        start: start.to_path_buf(),
    })
}

#[must_use]
pub fn render_phase_gate_report(report: &PhaseGateReport) -> String {
    let mut lines = Vec::with_capacity(report.checks.len() * 4);
    for check in &report.checks {
        let status = if check.success { "PASS" } else { "FAIL" };
        let exit_suffix = if check.success {
            String::new()
        } else {
            format!(
                " (exit code {})",
                check
                    .exit_code
                    .map_or_else(|| "launch error".to_string(), |code| code.to_string())
            )
        };
        lines.push(format!("{status} {}{exit_suffix}", check.command_display));

        if !check.success {
            if !check.stderr.trim().is_empty() {
                lines.push("stderr:".to_string());
                lines.extend(check.stderr.lines().map(|line| format!("  {line}")));
            }
            if !check.stdout.trim().is_empty() {
                lines.push("stdout:".to_string());
                lines.extend(check.stdout.lines().map(|line| format!("  {line}")));
            }
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        PHASE_0_COMMANDS, PHASE_1_COMMANDS, PHASE_2_COMMANDS, capability_coverage, list_profiles,
        phase_gate_commands, repo_root, repo_root_from, show_capabilities, validate_fixture,
    };
    use insta::assert_snapshot;
    use std::path::Path;

    #[test]
    fn smoke() {}

    #[test]
    fn phase_zero_commands_match_expected_order() {
        let commands = phase_gate_commands(0).expect("phase 0 commands");
        let expected = PHASE_0_COMMANDS
            .iter()
            .map(|command| {
                command
                    .iter()
                    .map(|arg| (*arg).to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(commands, expected);
    }

    #[test]
    fn phase_one_commands_match_expected_order() {
        let commands = phase_gate_commands(1).expect("phase 1 commands");
        let expected = PHASE_1_COMMANDS
            .iter()
            .map(|command| {
                command
                    .iter()
                    .map(|arg| (*arg).to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(commands, expected);
    }

    #[test]
    fn phase_two_commands_match_expected_order() {
        let commands = phase_gate_commands(2).expect("phase 2 commands");
        let expected = PHASE_2_COMMANDS
            .iter()
            .map(|command| {
                command
                    .iter()
                    .map(|arg| (*arg).to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(commands, expected);
    }

    #[test]
    fn unimplemented_phase_errors_clearly() {
        let error = phase_gate_commands(3).expect_err("phase 3 should be unimplemented");
        assert_eq!(
            error.to_string(),
            "automated gate not implemented for phase `3` yet"
        );
    }

    #[test]
    fn list_profiles_output_is_stable() {
        let output = list_profiles().expect("list-profiles succeeds");
        assert_snapshot!("list_profiles", output);
    }

    #[test]
    fn show_capabilities_output_is_stable() {
        let output = show_capabilities("dualsense").expect("show-capabilities succeeds");
        assert_snapshot!("show_capabilities_dualsense", output);
    }

    #[test]
    fn xbox360_capability_output_is_stable() {
        let output = show_capabilities("xbox360").expect("show-capabilities succeeds");
        assert_snapshot!("show_capabilities_xbox360", output);
    }

    #[test]
    fn show_capabilities_makes_stick_axis_coverage_explicit() {
        let output = show_capabilities("dualsense").expect("show-capabilities succeeds");
        assert!(output.contains("covered_fields:"));
        assert!(output.contains("- sticks.left_x"));
        assert!(output.contains("- sticks.left_y"));
        assert!(output.contains("range_applies_to:"));
    }

    #[test]
    fn show_capabilities_rejects_unknown_profile() {
        let error = show_capabilities("not-a-profile").expect_err("unknown profile should fail");
        assert_eq!(error.to_string(), "unknown profile `not-a-profile`");
    }

    #[test]
    fn capability_coverage_report_is_clean() {
        let report = capability_coverage().expect("coverage report");
        assert!(report.all_covered());
        assert_snapshot!(
            "capability_coverage",
            serde_yaml::to_string(&report).expect("yaml")
        );
    }

    #[test]
    fn repo_root_resolves_to_workspace_root() {
        let root = repo_root().expect("workspace root");
        assert!(root.join("demo").is_dir());
        assert!(root.join("crates").is_dir());
        assert!(root.join("Cargo.toml").is_file());
    }

    #[test]
    fn repo_root_can_walk_up_from_nested_path() {
        let start = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let root = repo_root_from(&start).expect("workspace root");
        assert!(root.join("demo").is_dir());
        assert!(root.join("Cargo.toml").is_file());
    }

    #[test]
    fn phase_zero_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_zero = plan
            .split("## Phase 0:")
            .nth(1)
            .and_then(|section| section.split("## Phase 1:").next())
            .expect("phase 0 section");
        let automated = phase_zero
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in PHASE_0_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 0 automated section is missing {command}"
            );
        }
    }

    #[test]
    fn phase_one_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_one = plan
            .split("## Phase 1:")
            .nth(1)
            .and_then(|section| section.split("## Phase 2:").next())
            .expect("phase 1 section");
        let automated = phase_one
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in PHASE_1_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 1 automated section is missing {command}"
            );
        }
    }

    #[test]
    fn phase_two_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_two = plan
            .split("## Phase 2:")
            .nth(1)
            .and_then(|section| section.split("## Phase 3:").next())
            .expect("phase 2 section");
        let automated = phase_two
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in PHASE_2_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 2 automated section is missing {command}"
            );
        }
    }

    #[test]
    fn validate_fixture_summary_for_dualsense_fixture_is_stable() {
        let repo_root = repo_root().expect("workspace root");
        let fixture_path = repo_root.join("crates/gr-core/fixtures/payload-dualsense-neutral.yaml");
        let summary = validate_fixture(fixture_path).expect("fixture should validate");
        assert_snapshot!("validate_fixture_dualsense", summary);
    }
}
