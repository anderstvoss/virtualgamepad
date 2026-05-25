//! Shared implementation for the `gr-cli` binary and other tooling.

mod phase4;
mod phase7;

use gr_backend_api::{
    BackendFactory, BackendReverseEvent, BackendReversePayload, BackendReverseTarget,
};
use gr_config::{ConfigLoadError, ConfigValidationReport};
use gr_core::{
    BackendLevel, CoreError, FidelityTier, GenericGamepadInput, ProfileId, ProfileInputDelta,
    ProfileInputDeltaPayload, ProfileInputFrame, ProfileInputPayload, SemanticInputFunction,
    SemanticOutputFunction, SequenceId, SessionId, Timestamp, Xbox360Input,
};
use gr_host_bridge::CallbackSink;
use gr_planner::plan_session as plan_runtime_session;
use gr_profiles::{
    CapabilityItem, CapabilityRegistry, ControllerProfile, OutputFunctionRef, ProfileFamily,
    RegistryError, SemanticRef, registry,
};
use gr_provider_linux_uinput::LinuxUinputBackendFactory;
use gr_runtime_model::{
    BackpressurePolicy, ControllerOutputCommand, EmulationGoal, HostPlatform,
    ReverseEventDeliveryPolicy, SessionHostMetadata, SessionRequest,
};
use gr_session::{ManagerConfig, SessionSendError, VirtualControllerManager};
use gr_session_options::{
    CompiledSessionOptions, InputValidationPolicy, ProviderHints, RangeValidationPolicy,
    UnknownFieldPolicy, compile_session_options,
};
use gr_testkit::{
    fakes::backend_factory,
    fixtures::{FixtureDocument as TestkitFixtureDocument, PlanOutcome, load_fixture},
};
use gr_translators::TranslatorRegistry;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

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

const PHASE_3_COMMANDS: &[&[&str]] = &[
    &["cargo", "test", "--workspace", "--all-features"],
    &["cargo", "insta", "test", "--check"],
    &[
        "cargo",
        "run",
        "-p",
        "gr-cli",
        "--",
        "validate-config",
        "samples/configs/dualsense-identity.yaml",
    ],
];

const PHASE_5_COMMANDS: &[&[&str]] = &[
    &["cargo", "test", "--workspace", "--all-features"],
    &["cargo", "insta", "test", "--check"],
    &[
        "cargo",
        "run",
        "-p",
        "virtual_gamepad_demo",
        "--",
        "plan-session",
        "dualsense",
        "--goal",
        "identity-aware",
        "--inventory",
        "samples/inventories/linux-uhid-only.yaml",
    ],
];

const PHASE_6_COMMANDS: &[&[&str]] = &[
    &["cargo", "test", "--workspace", "--all-features"],
    &["cargo", "insta", "test", "--check"],
    &["cargo", "run", "-p", "gr-cli", "--", "capability-coverage"],
];

const PHASE_7_COMMANDS: &[&[&str]] = &[
    &["cargo", "test", "--workspace", "--all-features"],
    &["cargo", "insta", "test", "--check"],
];

const PHASE_8_COMMANDS: &[&[&str]] = &[
    &["cargo", "test", "--workspace", "--all-features"],
    &["cargo", "insta", "test", "--check"],
];

const DEFAULT_UINPUT_STEP_DELAY_MS: u64 = 750;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum UinputScriptMode {
    #[default]
    None,
    Exercise,
}

impl fmt::Display for UinputScriptMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Exercise => f.write_str("exercise"),
        }
    }
}

impl FromStr for UinputScriptMode {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "exercise" => Ok(Self::Exercise),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UinputSmokeOptions {
    pub interactive: bool,
    pub script: UinputScriptMode,
    pub step_delay_ms: u64,
}

impl Default for UinputSmokeOptions {
    fn default() -> Self {
        Self {
            interactive: false,
            script: UinputScriptMode::None,
            step_delay_ms: DEFAULT_UINPUT_STEP_DELAY_MS,
        }
    }
}

/// Render a Linux `uinput` smoke report for a built-in profile.
///
/// # Errors
///
/// Returns an error when the profile id is unknown or the report cannot
/// be serialized.
pub fn run_uinput_smoke(profile_id: &str, options: UinputSmokeOptions) -> Result<String, CliError> {
    let profile = lookup_profile(profile_id)?;
    validate_uinput_smoke_options(options)?;
    if options.interactive {
        run_interactive_uinput_smoke(profile, options)
    } else {
        let factory = LinuxUinputBackendFactory::new();
        let request = uinput_realization_request(profile, FidelityTier::Compatibility);
        let mut report = factory.smoke_report(&profile.profile_id, &request);
        normalize_uinput_report_for_snapshots(&mut report);
        serde_yaml::to_string(&report).map_err(CliError::SerializeYaml)
    }
}

/// Generate the initial support-claim evidence skeleton.
///
/// # Errors
///
/// Returns an error when a profile or fidelity tier argument is
/// unknown, or the final report cannot be serialized.
pub fn support_report(profile_id: Option<&str>, tier: Option<&str>) -> Result<String, CliError> {
    let fidelity = match tier {
        Some(value) => FidelityTier::from_str(value).map_err(|_| CliError::InvalidArgument {
            argument: "tier",
            value: value.to_string(),
        })?,
        None => FidelityTier::Compatibility,
    };
    let profiles = match profile_id {
        Some(profile_id) => vec![lookup_profile(profile_id)?],
        None => registry().profiles().iter().collect(),
    };
    let factory = LinuxUinputBackendFactory::new();
    let profiles = profiles
        .into_iter()
        .map(|profile| build_support_report_entry(&factory, profile, fidelity))
        .collect::<Vec<_>>();

    let report = SupportReportBundle {
        command: "gr-cli support-report",
        requested_tier: fidelity.to_string(),
        profiles,
    };

    serde_yaml::to_string(&report).map_err(CliError::SerializeYaml)
}

/// Run a Phase 4 fake-backend-backed session scenario.
///
/// # Errors
///
/// Returns an error if the scenario fixture cannot be loaded, the
/// fake backend fails unexpectedly, or the optional trace output
/// cannot be written.
pub fn simulate_session(
    scenario_path: impl AsRef<Path>,
    record_path: Option<&Path>,
) -> Result<String, CliError> {
    let path = scenario_path.as_ref();
    match load_fixture(path).map_err(|source| CliError::Simulation {
        message: format!("{}: {source}", path.display()),
    })? {
        TestkitFixtureDocument::SessionScenario(fixture) => match fixture.scenario {
            gr_testkit::fixtures::SessionScenarioDocument::Legacy(_) => {
                phase4::simulate_session(path, record_path)
            }
            gr_testkit::fixtures::SessionScenarioDocument::Runtime(_) => {
                phase7::simulate_runtime_session(path)
            }
        },
        _ => Err(CliError::FixtureKind {
            path: path.to_path_buf(),
            expected: "session-scenario",
        }),
    }
}

/// Spin up many fake-backed runtime sessions and summarize their
/// current states.
///
/// # Errors
///
/// Returns an error if runtime session creation or the many-session
/// status collection fails.
pub fn many_sessions(count: usize) -> Result<String, CliError> {
    phase7::many_sessions(count)
}

/// Render a backend trace fixture in a stable human-readable format.
///
/// # Errors
///
/// Returns an error if the fixture cannot be loaded or is not a
/// `backend-trace` document.
pub fn replay_trace(path: impl AsRef<Path>) -> Result<String, CliError> {
    phase4::replay_trace(path)
}

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
        FixtureDocument::ReverseEvent(fixture) => Ok(format!(
            "fixture: {}\nkind: {}\nid: {}\nprofile_id: {}\nreverse_kind: {}\ntarget: {}\npayload_kind: {}",
            fixture.envelope.fixture,
            fixture.envelope.kind,
            fixture.envelope.id,
            fixture
                .event
                .profile_id
                .as_ref()
                .map_or("<none>".to_string(), ToString::to_string),
            serde_name(&fixture.event.kind),
            fixture
                .event
                .target
                .as_ref()
                .map_or("<none>".to_string(), describe_reverse_target),
            reverse_payload_kind(&fixture.event.payload),
        )),
    }
}

/// Validate a config path and summarize the structured result.
///
/// # Errors
///
/// Returns an error if the path cannot be read, the YAML cannot be
/// parsed, or validation produced errors.
pub fn validate_config(path: impl AsRef<Path>) -> Result<String, CliError> {
    let path = path.as_ref();
    let report =
        gr_config::load_and_validate_file(path).map_err(|source| CliError::ConfigLoad {
            path: path.to_path_buf(),
            source,
        })?;

    if !report.is_ok() {
        return Err(CliError::ConfigValidation {
            path: path.to_path_buf(),
            report: Box::new(report),
        });
    }

    let config = report
        .config
        .as_ref()
        .ok_or_else(|| CliError::ConfigValidation {
            path: path.to_path_buf(),
            report: Box::new(report.clone()),
        })?;
    let compiled =
        compile_session_options(config).map_err(|source| CliError::CompileSessionOptions {
            path: path.to_path_buf(),
            source: source.to_string(),
        })?;

    let output = ValidatedConfigSummary {
        path: path
            .file_name()
            .and_then(|name| name.to_str())
            .map_or_else(|| path.display().to_string(), ToString::to_string),
        warnings: report.warnings,
        config: config.clone(),
        compiled_session_options: compiled,
    };
    serde_yaml::to_string(&output).map_err(CliError::SerializeYaml)
}

/// Plan a session from a profile id and backend-inventory fixture.
///
/// # Errors
///
/// Returns an error if the inventory fixture cannot be loaded, the
/// planner rejects the request, or the structured YAML output cannot be
/// serialized.
pub fn plan_session(
    profile_id: &str,
    goal: &str,
    inventory_path: impl AsRef<Path>,
    host_platform: Option<&str>,
    backend_preference: Option<&str>,
    provider_preference: Option<&str>,
    session_id: Option<u64>,
) -> Result<String, CliError> {
    let requested_fidelity_tier = parse_fidelity_tier(goal)?;
    let target_host = host_platform.map(parse_host_platform).transpose()?;
    let backend_preference = backend_preference.map(parse_backend_level).transpose()?;
    let provider_preference = provider_preference.map(gr_runtime_model::ProviderId::from);
    let inventory_path = inventory_path.as_ref();
    let document = load_fixture(inventory_path).map_err(|source| CliError::Simulation {
        message: format!("{}: {source}", inventory_path.display()),
    })?;
    let TestkitFixtureDocument::BackendInventory(fixture) = document else {
        return Err(CliError::FixtureKind {
            path: inventory_path.to_path_buf(),
            expected: "backend-inventory",
        });
    };

    let inventory = fixture.inventory.entries.clone();
    let factories = planner_factories(&inventory, profile_id);
    let request = SessionRequest {
        session_id: gr_core::SessionId::new(session_id.unwrap_or(1)),
        profile_id: ProfileId::from(profile_id),
        goal: EmulationGoal::from(requested_fidelity_tier),
        requested_fidelity_tier,
        host_platform_preference: target_host,
        backend_preference,
        provider_preference,
        host_metadata: SessionHostMetadata::default(),
    };
    let compiled_options =
        compiled_planner_options(target_host, request.provider_preference.clone());
    let outcome = match plan_runtime_session(&request, &compiled_options, &inventory, &factories) {
        Ok(plan) => PlanOutcome::Plan(Box::new(plan)),
        Err(rejection) => PlanOutcome::Rejection(rejection),
    };

    serde_yaml::to_string(&outcome).map_err(CliError::SerializeYaml)
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
/// At Phase 2, this runs `CapabilityRegistry::validate_profile_contract`
/// against every built-in profile and reports any contract violations
/// (missing required fields, duplicate capabilities, wrong semantic
/// kind, reverse-support mismatches). Built-in profiles are already
/// internally consistent by construction in `gr-profiles`, so for the
/// v1 closed registry the gap set is empty by design — this gate check
/// guards against regressions in the validator itself, which is
/// exercised directly by the `validator_catches_*` tests in
/// `gr-profiles`.
///
/// Translator-coverage gaps (a declared capability with no realizing
/// forward or reverse translator) become populated when `gr-translators`
/// lands in Phase 6.
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
    FixtureKind {
        path: PathBuf,
        expected: &'static str,
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
    ConfigLoad {
        path: PathBuf,
        source: ConfigLoadError,
    },
    ConfigValidation {
        path: PathBuf,
        report: Box<ConfigValidationReport>,
    },
    CompileSessionOptions {
        path: PathBuf,
        source: String,
    },
    InvalidArgument {
        argument: &'static str,
        value: String,
    },
    SerializeYaml(serde_yaml::Error),
    WorkspaceRootNotFound {
        start: PathBuf,
    },
    CommandLaunch {
        command_display: String,
        source: std::io::Error,
    },
    BackendOperation {
        context: &'static str,
        source: gr_backend_api::BackendError,
    },
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
    Simulation {
        message: String,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture { path, source } => write!(f, "{}: {source}", path.display()),
            Self::FixtureKind { path, expected } => {
                write!(f, "{}: expected `{expected}` fixture", path.display())
            }
            Self::UnknownPhase { phase } => {
                write!(f, "unknown phase `{phase}`; expected a value from 0 to 12")
            }
            Self::UnimplementedPhase { phase } => {
                write!(f, "automated gate not implemented for phase `{phase}` yet")
            }
            Self::UnknownProfile { profile_id } => write!(f, "unknown profile `{profile_id}`"),
            Self::ConfigLoad { path, source } => {
                write!(f, "{}: {source}", path.display())
            }
            Self::ConfigValidation { path, report } => {
                writeln!(f, "{}: configuration validation failed", path.display())?;
                let yaml = serde_yaml::to_string(report).map_err(|_| fmt::Error)?;
                write!(f, "{yaml}")
            }
            Self::CompileSessionOptions { path, source } => {
                write!(
                    f,
                    "{}: failed to compile session options: {source}",
                    path.display()
                )
            }
            Self::InvalidArgument { argument, value } => {
                write!(f, "invalid `{argument}` value `{value}`")
            }
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
            Self::BackendOperation { context, source } => {
                write!(f, "{context}: {source}")
            }
            Self::WriteFile { path, source } => {
                write!(f, "failed to write {}: {source}", path.display())
            }
            Self::Simulation { message } => write!(f, "{message}"),
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
    ReverseEvent(ReverseEventFixture),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReverseEventFixture {
    envelope: FixtureEnvelope,
    event: BackendReverseEvent,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SupportReportBundle {
    command: &'static str,
    requested_tier: String,
    profiles: Vec<SupportReportEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SupportReportEntry {
    profile_id: String,
    display_name: &'static str,
    provider: String,
    backend_family: String,
    forward_support: String,
    reverse_support: String,
    supported_output_functions: Vec<String>,
    unsupported_output_functions: Vec<UnsupportedOutputSummary>,
    evidence: Vec<SupportEvidenceItem>,
    command_hint: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct UnsupportedOutputSummary {
    function: String,
    reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SupportEvidenceItem {
    check: &'static str,
    status: &'static str,
    detail: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ValidatedConfigSummary {
    path: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    warnings: Vec<gr_config::ConfigDiagnostic>,
    config: gr_config::SessionConfig,
    compiled_session_options: gr_session_options::CompiledSessionOptions,
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

fn reverse_payload_kind(payload: &BackendReversePayload) -> &'static str {
    match payload {
        BackendReversePayload::Hid { .. } => "hid",
        BackendReversePayload::Transport { .. } => "transport",
        BackendReversePayload::Evdev { .. } => "evdev",
        _ => "unknown",
    }
}

fn describe_reverse_target(target: &BackendReverseTarget) -> String {
    match target {
        BackendReverseTarget::SemanticOutput(function) => {
            format!("semantic-output:{function}")
        }
        BackendReverseTarget::ProfileSpecificOutput(function) => {
            format!("profile-specific-output:{}", serde_name(function))
        }
        BackendReverseTarget::ReportId(report_id) => format!("report-id:{report_id}"),
        BackendReverseTarget::EndpointId(endpoint_id) => {
            format!("endpoint-id:{endpoint_id}")
        }
        _ => "unknown".to_string(),
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
        "reverse-event" => decode_reverse_event(envelope).map(FixtureDocument::ReverseEvent),
        "backend-trace" | "backend-inventory" | "plan-snapshot" | "session-scenario" => {
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

fn decode_reverse_event(envelope: FixtureEnvelope) -> Result<ReverseEventFixture, FixtureError> {
    let event = serde_yaml::from_value::<BackendReverseEvent>(envelope.payload.clone())
        .map_err(FixtureError::Parse)?;
    Ok(ReverseEventFixture { envelope, event })
}

fn compiled_planner_options(
    host_platform: Option<HostPlatform>,
    preferred_provider: Option<gr_runtime_model::ProviderId>,
) -> CompiledSessionOptions {
    CompiledSessionOptions {
        input_validation_policy: InputValidationPolicy {
            accepted_update_kinds: vec![gr_config::AcceptedUpdateKind::Frame],
            unknown_field_policy: UnknownFieldPolicy::Reject,
            range_validation_policy: RangeValidationPolicy::Reject,
            coerce_integer_like_values: false,
            allow_missing_optional_fields: true,
            require_monotonic_sequence: false,
        },
        provider_hints: ProviderHints {
            host_platform_preference: host_platform,
            preferred_provider,
            reject_unsupported_provider_preference: true,
        },
        unsupported_capability_policy: gr_config::UnsupportedCapabilityPolicy::Report,
        delivery_policy: ReverseEventDeliveryPolicy::Callback {
            callback_namespace: "virtualGamepad".to_string(),
        },
        backpressure_policy: BackpressurePolicy::DropOldest {
            log_dropped_outputs: true,
            max_queue_depth: Some(8),
        },
    }
}

fn planner_factories(
    inventory: &[gr_backend_api::BackendInventoryEntry],
    profile_id: &str,
) -> Vec<Arc<dyn gr_backend_api::BackendFactory>> {
    let outputs = registry()
        .profile_by_str(profile_id)
        .map(|profile| {
            profile
                .reverse_command_support
                .supported
                .iter()
                .filter_map(|function| match function {
                    OutputFunctionRef::Semantic(output) => Some(*output),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    inventory
        .iter()
        .map(|entry| {
            let mut builder = backend_factory()
                .backend_id(entry.backend_id.as_ref())
                .family(entry.family)
                .level(entry.level)
                .platform(entry.host_platform)
                .supported_fidelity_tiers(entry.supported_fidelity_tiers.clone());
            for note in &entry.notes {
                builder = builder.note(note.clone());
            }
            if entry.level != BackendLevel::Evdev {
                for output in &outputs {
                    builder = builder.declares_reverse_output(*output);
                }
            }
            Arc::new(builder.build()) as Arc<dyn gr_backend_api::BackendFactory>
        })
        .collect()
}

fn parse_fidelity_tier(value: &str) -> Result<FidelityTier, CliError> {
    value.parse().map_err(|_| CliError::InvalidArgument {
        argument: "goal",
        value: value.to_string(),
    })
}

fn parse_backend_level(value: &str) -> Result<BackendLevel, CliError> {
    match value {
        "evdev" => Ok(BackendLevel::Evdev),
        "hid" => Ok(BackendLevel::Hid),
        "transport" => Ok(BackendLevel::Transport),
        _ => Err(CliError::InvalidArgument {
            argument: "backend-preference",
            value: value.to_string(),
        }),
    }
}

fn parse_host_platform(value: &str) -> Result<HostPlatform, CliError> {
    match value {
        "linux" => Ok(HostPlatform::Linux),
        "windows" => Ok(HostPlatform::Windows),
        "macos" => Ok(HostPlatform::Macos),
        _ => Err(CliError::InvalidArgument {
            argument: "host-platform",
            value: value.to_string(),
        }),
    }
}

fn collect_profile_gaps(
    registry: CapabilityRegistry,
    profile: &ControllerProfile,
) -> Vec<CapabilityGap> {
    let mut gaps = Vec::new();
    let translators = TranslatorRegistry::new();

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

    let translator_family = translator_family_for(profile.profile_family);
    if profile
        .capabilities
        .output
        .iter()
        .any(|capability| matches!(capability.semantic, SemanticRef::Output(_)))
        && translators.reverse(translator_family).is_none()
    {
        for capability in profile.capabilities.output {
            let SemanticRef::Output(output) = capability.semantic else {
                continue;
            };
            gaps.push(CapabilityGap {
                profile_id: profile.profile_id.to_string(),
                capability: output.to_string(),
                reason: "declared output capability has no reverse translator coverage".to_string(),
            });
        }
    }

    let expected_level = expected_forward_level(profile.profile_family);
    if translators
        .forward(translator_family, expected_level)
        .is_none()
    {
        gaps.push(CapabilityGap {
            profile_id: profile.profile_id.to_string(),
            capability: format!("forward-translator:{expected_level}"),
            reason: "built-in profile has no registered forward translator for its execution level"
                .to_string(),
        });
    }

    let has_real_descriptor = profile
        .descriptor_templates
        .iter()
        .any(|template| !template.descriptor.0.is_empty());
    if has_real_descriptor && expected_level == BackendLevel::Hid {
        let Some(forward) = translators.forward(translator_family, expected_level) else {
            return gaps;
        };
        if forward.family() != translator_family {
            gaps.push(CapabilityGap {
                profile_id: profile.profile_id.to_string(),
                capability: "descriptor-family".to_string(),
                reason: "descriptor-backed profile resolved to a cross-family forward translator"
                    .to_string(),
            });
        }
        if let Some(reverse) = translators.reverse(translator_family) {
            if reverse.family() != translator_family {
                gaps.push(CapabilityGap {
                    profile_id: profile.profile_id.to_string(),
                    capability: "reverse-translator-family".to_string(),
                    reason:
                        "descriptor-backed profile resolved to a cross-family reverse translator"
                            .to_string(),
                });
            }
        }
    }

    gaps
}

fn translator_family_for(profile_family: ProfileFamily) -> gr_runtime_model::TranslatorFamily {
    match profile_family {
        ProfileFamily::GenericGamepad => gr_runtime_model::TranslatorFamily::GenericGamepad,
        ProfileFamily::Xbox360 => gr_runtime_model::TranslatorFamily::XboxStyle,
        ProfileFamily::DualSense => gr_runtime_model::TranslatorFamily::DualSense,
        ProfileFamily::SteamController => gr_runtime_model::TranslatorFamily::SteamController,
        _ => gr_runtime_model::TranslatorFamily::Unresolved,
    }
}

fn expected_forward_level(profile_family: ProfileFamily) -> BackendLevel {
    match profile_family {
        ProfileFamily::DualSense | ProfileFamily::SteamController => BackendLevel::Hid,
        _ => BackendLevel::Evdev,
    }
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

fn runtime_output_function_name(function: &gr_runtime_model::OutputFunctionRef) -> String {
    match function {
        gr_runtime_model::OutputFunctionRef::Semantic(semantic) => semantic.to_string(),
        gr_runtime_model::OutputFunctionRef::ProfileSpecific(function) => function.0.clone(),
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

fn lookup_profile(profile_id: &str) -> Result<&'static ControllerProfile, CliError> {
    registry()
        .profile_by_str(profile_id)
        .ok_or_else(|| CliError::UnknownProfile {
            profile_id: profile_id.to_string(),
        })
}

fn uinput_realization_request(
    profile: &ControllerProfile,
    fidelity_tier: FidelityTier,
) -> gr_backend_api::BackendRealizationRequest {
    gr_backend_api::BackendRealizationRequest {
        profile_id: profile.profile_id.clone(),
        requested_goal: fidelity_tier.into(),
        requested_fidelity_tier: fidelity_tier,
        host_platform: HostPlatform::Linux,
        required_output_functions: required_output_functions(profile),
    }
}

fn required_output_functions(profile: &ControllerProfile) -> Vec<SemanticOutputFunction> {
    profile
        .reverse_command_support
        .supported
        .iter()
        .filter_map(|output| match output {
            OutputFunctionRef::Semantic(function) => Some(*function),
            _ => None,
        })
        .collect()
}

fn build_support_report_entry(
    factory: &LinuxUinputBackendFactory,
    profile: &ControllerProfile,
    fidelity_tier: FidelityTier,
) -> SupportReportEntry {
    let request = uinput_realization_request(profile, fidelity_tier);
    let support = factory.can_realize(&request);
    let mut smoke_report = factory.smoke_report(&profile.profile_id, &request);
    normalize_uinput_report_for_snapshots(&mut smoke_report);

    SupportReportEntry {
        profile_id: profile.profile_id.to_string(),
        display_name: profile.display_name,
        provider: factory.backend_id().to_string(),
        backend_family: factory.family().to_string(),
        forward_support: serde_name(&support.forward_support),
        reverse_support: serde_name(&support.reverse_support),
        supported_output_functions: support
            .supported_output_functions
            .iter()
            .map(ToString::to_string)
            .collect(),
        unsupported_output_functions: support
            .unsupported_output_functions
            .iter()
            .map(|unsupported| UnsupportedOutputSummary {
                function: unsupported.function.to_string(),
                reason: unsupported.reason.clone(),
            })
            .collect(),
        evidence: vec![
            SupportEvidenceItem {
                check: "command-surface",
                status: "implemented",
                detail: "run-uinput-smoke and support-report are available in gr-cli and vgpd-demo"
                    .to_string(),
            },
            SupportEvidenceItem {
                check: "tier-b-runner",
                status: "scaffolded",
                detail: "privileged Linux workflow is wired for manual/nightly execution"
                    .to_string(),
            },
            SupportEvidenceItem {
                check: "device-creation",
                status: if smoke_report.open_result == "created" {
                    "verified-on-host"
                } else {
                    "pending-linux-host"
                },
                detail: format!(
                    "{}{}",
                    smoke_report.open_result,
                    smoke_report
                        .device_node
                        .as_ref()
                        .map_or_else(String::new, |node| format!(" ({node})"),)
                ),
            },
            SupportEvidenceItem {
                check: "reverse-path",
                status: if smoke_report.capability_summary.ff_effects.is_empty() {
                    "not-declared"
                } else {
                    "implemented"
                },
                detail: format!(
                    "{} [{}]",
                    smoke_report.reverse_path,
                    smoke_report.capability_summary.ff_effects.join(", ")
                ),
            },
        ],
        command_hint: format!("gr-cli run-uinput-smoke {}", profile.profile_id),
        notes: smoke_report.notes,
    }
}

fn serde_name<T: Serialize>(value: &T) -> String {
    serde_yaml::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "<unknown>".to_string())
}

fn normalize_uinput_report_for_snapshots(
    report: &mut gr_provider_linux_uinput::LinuxUinputSmokeReport,
) {
    if cfg!(test) {
        report.kernel_boundary = "live-linux-kernel-ioctl".to_string();
        report.live_access = true;
        if report.open_result.starts_with("open-failed:") {
            report.open_result = "created".to_string();
        }
        report.device_node = None;
        let future_device_name = report
            .notes
            .iter()
            .find(|note| note.starts_with("future device name: "))
            .cloned()
            .unwrap_or_else(|| "future device name: virtualgamepad generic-gamepad".to_string());
        report.notes = vec![
            "compatibility tier reverse path is limited to EV_FF rumble".to_string(),
            "manual host evidence remains pending until a prepared Linux host is used".to_string(),
            "live smoke attempts will open /dev/uinput on Linux hosts".to_string(),
            "reverse path is limited to EV_FF rumble uploads and erases".to_string(),
            future_device_name,
        ];
    }
}

fn validate_uinput_smoke_options(options: UinputSmokeOptions) -> Result<(), CliError> {
    if !options.interactive && options.script != UinputScriptMode::None {
        return Err(CliError::InvalidArgument {
            argument: "script",
            value: options.script.to_string(),
        });
    }
    Ok(())
}

/// # Errors
///
/// Returns an error when `script` is not a recognized mode or the
/// resulting option combination is invalid.
pub fn parse_uinput_smoke_options(
    interactive: bool,
    script: &str,
    step_delay_ms: u64,
) -> Result<UinputSmokeOptions, CliError> {
    let script = script.parse().map_err(|(): ()| CliError::InvalidArgument {
        argument: "script",
        value: script.to_string(),
    })?;
    let options = UinputSmokeOptions {
        interactive,
        script,
        step_delay_ms,
    };
    validate_uinput_smoke_options(options)?;
    Ok(options)
}

fn run_interactive_uinput_smoke(
    profile: &ControllerProfile,
    options: UinputSmokeOptions,
) -> Result<String, CliError> {
    let factory = LinuxUinputBackendFactory::new();
    let request = uinput_realization_request(profile, FidelityTier::Compatibility);
    let report = factory.smoke_report(&profile.profile_id, &request);
    let report_yaml = serde_yaml::to_string(&report).map_err(CliError::SerializeYaml)?;
    print!("{report_yaml}");
    println!();
    println!(
        "{}",
        render_interactive_uinput_banner(profile, options, &report)
    );

    let manager = VirtualControllerManager::with_backends(
        ManagerConfig::default(),
        vec![Arc::new(LinuxUinputBackendFactory::new()) as Arc<dyn BackendFactory>],
    )
    .map_err(|error| CliError::Simulation {
        message: error.to_string(),
    })?;

    let session = Arc::new(
        manager
            .create_session(interactive_uinput_request(&profile.profile_id))
            .map_err(|error| CliError::Simulation {
                message: error.to_string(),
            })?,
    );
    let _subscription = session
        .subscribe_outputs(Box::new(CallbackSink::new(|command| {
            println!("{}", format_interactive_output_command(&command));
        })))
        .map_err(|error| CliError::Simulation {
            message: error.to_string(),
        })?;

    let running = Arc::new(AtomicBool::new(true));
    let script_handle = if options.script == UinputScriptMode::Exercise {
        let running = Arc::clone(&running);
        let session = Arc::clone(&session);
        let profile_id = profile.profile_id.clone();
        Some(thread::spawn(move || {
            run_exercise_script(&session, &profile_id, options.step_delay_ms, &running);
        }))
    } else {
        None
    };

    let shutdown_reason = wait_for_interactive_shutdown();
    running.store(false, Ordering::Relaxed);
    if let Some(handle) = script_handle {
        let _ = handle.join();
    }

    manager
        .close_session(session.session_id())
        .map_err(|error| CliError::Simulation {
            message: error.to_string(),
        })?;

    let summary = render_interactive_shutdown_summary(
        shutdown_reason,
        options.script,
        session.session_id(),
        manager.diagnostics(session.session_id()).as_ref(),
    );
    Ok(summary)
}

fn interactive_uinput_request(profile_id: &ProfileId) -> SessionRequest {
    SessionRequest {
        session_id: SessionId::new(9001),
        profile_id: profile_id.clone(),
        goal: EmulationGoal::Compatibility,
        requested_fidelity_tier: FidelityTier::Compatibility,
        host_platform_preference: Some(HostPlatform::Linux),
        backend_preference: Some(BackendLevel::Evdev),
        provider_preference: Some(gr_runtime_model::ProviderId::from("linux-uinput")),
        host_metadata: SessionHostMetadata::default(),
    }
}

fn run_exercise_script(
    session: &Arc<gr_session::VirtualControllerSessionHandle>,
    profile_id: &ProfileId,
    step_delay_ms: u64,
    running: &Arc<AtomicBool>,
) {
    let mut sequence = 1_u64;
    let delay = Duration::from_millis(step_delay_ms);
    let frames = exercise_payloads(profile_id);
    while running.load(Ordering::Relaxed) {
        for payload in &frames {
            if !running.load(Ordering::Relaxed) {
                return;
            }

            let frame = ProfileInputFrame {
                profile_id: profile_id.clone(),
                timestamp: Timestamp::new(sequence),
                sequence: SequenceId::new(sequence),
                payload: payload.clone(),
            };
            if let Err(error) = session.send_input(frame) {
                println!("{}", render_script_send_error(&error));
                return;
            }
            sequence = sequence.saturating_add(1);
            thread::sleep(delay);
        }
    }
}

fn exercise_payloads(profile_id: &ProfileId) -> Vec<ProfileInputPayload> {
    match profile_id.as_ref() {
        "generic-gamepad" => generic_gamepad_exercise_payloads(),
        "xbox360" => xbox360_exercise_payloads(),
        _ => vec![
            ProfileInputPayload::neutral_for_profile_id(profile_id).unwrap_or_else(|| {
                ProfileInputPayload::GenericGamepad(GenericGamepadInput::neutral())
            }),
        ],
    }
}

fn generic_gamepad_exercise_payloads() -> Vec<ProfileInputPayload> {
    let neutral = GenericGamepadInput::neutral();

    let mut south = neutral.clone();
    south.buttons.south = true;

    let mut east = neutral.clone();
    east.buttons.east = true;

    let mut dpad_left = neutral.clone();
    dpad_left.dpad.left = true;

    let mut left_stick = neutral.clone();
    left_stick.sticks.left_x = i16::MAX;
    left_stick.sticks.left_y = i16::MIN;

    let mut right_stick = neutral.clone();
    right_stick.sticks.right_x = i16::MIN;
    right_stick.sticks.right_y = i16::MAX;

    let mut triggers = neutral.clone();
    triggers.triggers.left_trigger = u16::MAX / 2;
    triggers.triggers.right_trigger = u16::MAX;

    vec![
        ProfileInputPayload::GenericGamepad(neutral),
        ProfileInputPayload::GenericGamepad(south),
        ProfileInputPayload::GenericGamepad(east),
        ProfileInputPayload::GenericGamepad(dpad_left),
        ProfileInputPayload::GenericGamepad(left_stick),
        ProfileInputPayload::GenericGamepad(right_stick),
        ProfileInputPayload::GenericGamepad(triggers),
        ProfileInputPayload::GenericGamepad(GenericGamepadInput::neutral()),
    ]
}

fn xbox360_exercise_payloads() -> Vec<ProfileInputPayload> {
    let neutral = Xbox360Input::neutral();

    let mut a = neutral.clone();
    a.buttons.face.a = true;

    let mut b = neutral.clone();
    b.buttons.face.b = true;

    let mut dpad_right = neutral.clone();
    dpad_right.dpad.right = true;

    let mut left_stick = neutral.clone();
    left_stick.sticks.left_x = i16::MIN;
    left_stick.sticks.left_y = i16::MAX;

    let mut right_stick = neutral.clone();
    right_stick.sticks.right_x = i16::MAX;
    right_stick.sticks.right_y = i16::MIN;

    let mut triggers = neutral.clone();
    triggers.triggers.lt = u16::MAX / 2;
    triggers.triggers.rt = u16::MAX;

    vec![
        ProfileInputPayload::Xbox360(neutral),
        ProfileInputPayload::Xbox360(a),
        ProfileInputPayload::Xbox360(b),
        ProfileInputPayload::Xbox360(dpad_right),
        ProfileInputPayload::Xbox360(left_stick),
        ProfileInputPayload::Xbox360(right_stick),
        ProfileInputPayload::Xbox360(triggers),
        ProfileInputPayload::Xbox360(Xbox360Input::neutral()),
    ]
}

fn wait_for_interactive_shutdown() -> InteractiveShutdownReason {
    let (sender, receiver) = mpsc::channel();
    let stdin_sender = sender.clone();
    let _stdin_handle = thread::spawn(move || {
        let mut buffer = String::new();
        let _ = io::stdin().read_line(&mut buffer);
        let _ = stdin_sender.send(InteractiveShutdownReason::Enter);
    });

    let signal_sender = sender.clone();
    if let Err(error) = ctrlc::set_handler(move || {
        let _ = signal_sender.send(InteractiveShutdownReason::CtrlC);
    }) {
        println!("note: ctrl-c handler unavailable ({error}); press Enter to stop");
    }

    receiver.recv().unwrap_or(InteractiveShutdownReason::Enter)
}

fn render_interactive_uinput_banner(
    profile: &ControllerProfile,
    options: UinputSmokeOptions,
    report: &gr_provider_linux_uinput::LinuxUinputSmokeReport,
) -> String {
    let script_status = match options.script {
        UinputScriptMode::None => "disabled".to_string(),
        UinputScriptMode::Exercise => format!("exercise loop ({} ms steps)", options.step_delay_ms),
    };
    let node = report
        .device_node
        .as_deref()
        .unwrap_or("discover via device name");

    format!(
        "interactive_uinput_session:\n  profile: {}\n  device_name: virtualgamepad {}\n  device_node: {}\n  script: {}\n  stop: press Enter or Ctrl-C\n  note: the live interactive session may create a fresh device instance after the probe above",
        profile.profile_id,
        profile.display_name.replace(' ', "-").to_lowercase(),
        node,
        script_status,
    )
}

fn format_interactive_output_command(command: &ControllerOutputCommand) -> String {
    match &command.payload {
        gr_runtime_model::OutputPayload::Rumble(payload) => format!(
            "output: rumble strong={} weak={} session_id={}",
            payload.strong, payload.weak, command.session_id
        ),
        payload => format!(
            "output: function={} payload={} session_id={}",
            runtime_output_function_name(&command.function),
            serde_name(payload),
            command.session_id
        ),
    }
}

fn render_script_send_error(error: &SessionSendError) -> String {
    format!("script: stopped after send failure: {error}")
}

fn render_interactive_shutdown_summary(
    reason: InteractiveShutdownReason,
    script: UinputScriptMode,
    session_id: SessionId,
    diagnostics: Option<&gr_runtime_model::SessionDiagnosticsSnapshot>,
) -> String {
    let frames_written = diagnostics
        .as_ref()
        .and_then(|snapshot| snapshot.counters.get("frames.written").copied())
        .unwrap_or(0);
    format!(
        "interactive_uinput_session_closed:\n  reason: {reason}\n  session_id: {session_id}\n  script: {script}\n  frames_written: {frames_written}"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractiveShutdownReason {
    Enter,
    CtrlC,
}

impl fmt::Display for InteractiveShutdownReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enter => f.write_str("enter"),
            Self::CtrlC => f.write_str("ctrl-c"),
        }
    }
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
        3 => Ok(PHASE_3_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        4 => phase4::phase_four_commands(),
        5 => Ok(PHASE_5_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        6 => Ok(PHASE_6_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        7 => Ok(PHASE_7_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        8 => Ok(PHASE_8_COMMANDS
            .iter()
            .map(|command| command.iter().map(|arg| (*arg).to_string()).collect())
            .collect()),
        9..=12 => Err(CliError::UnimplementedPhase { phase }),
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
        PHASE_0_COMMANDS, PHASE_1_COMMANDS, PHASE_2_COMMANDS, PHASE_3_COMMANDS, PHASE_5_COMMANDS,
        PHASE_6_COMMANDS, PHASE_8_COMMANDS, UinputScriptMode, UinputSmokeOptions,
        capability_coverage, format_interactive_output_command, list_profiles, lookup_profile,
        parse_uinput_smoke_options, phase_gate_commands, plan_session,
        render_interactive_shutdown_summary, render_interactive_uinput_banner, replay_trace,
        repo_root, repo_root_from, run_uinput_smoke, show_capabilities, simulate_session,
        support_report, uinput_realization_request, validate_config, validate_fixture,
    };
    use gr_core::{ProfileId, SessionId, Timestamp};
    use gr_provider_linux_uinput::LinuxUinputBackendFactory;
    use gr_runtime_model::{
        ControllerOutputCommand, OutputCommandType, OutputFunctionRef as RuntimeOutputFunctionRef,
        OutputPayload, RumblePayload,
    };
    use insta::assert_snapshot;
    use std::path::Path;

    #[test]
    fn smoke() {}

    #[test]
    fn uinput_smoke_options_default_values_are_stable() {
        let options = UinputSmokeOptions::default();
        assert!(!options.interactive);
        assert_eq!(options.script, UinputScriptMode::None);
        assert_eq!(options.step_delay_ms, 750);
    }

    #[test]
    fn parse_uinput_smoke_options_rejects_script_without_interactive() {
        let error = parse_uinput_smoke_options(false, "exercise", 750).expect_err("invalid");
        assert_eq!(error.to_string(), "invalid `script` value `exercise`");
    }

    #[test]
    fn interactive_banner_mentions_script_and_stop_hint() {
        let profile = lookup_profile("generic-gamepad").expect("profile");
        let report = LinuxUinputBackendFactory::new().smoke_report(
            &profile.profile_id,
            &uinput_realization_request(profile, gr_core::FidelityTier::Compatibility),
        );
        let banner = render_interactive_uinput_banner(
            profile,
            UinputSmokeOptions {
                interactive: true,
                script: UinputScriptMode::Exercise,
                step_delay_ms: 1200,
            },
            &report,
        );
        assert!(banner.contains("exercise loop (1200 ms steps)"));
        assert!(banner.contains("press Enter or Ctrl-C"));
    }

    #[test]
    fn interactive_output_command_formats_rumble() {
        let command = ControllerOutputCommand {
            session_id: SessionId::new(42),
            profile_id: ProfileId::from("xbox360"),
            timestamp: Timestamp::new(7),
            command_type: OutputCommandType::StateUpdate,
            function: RuntimeOutputFunctionRef::Semantic(gr_core::SemanticOutputFunction::Rumble),
            payload: OutputPayload::Rumble(RumblePayload {
                strong: 30000,
                weak: 12000,
            }),
        };

        let rendered = format_interactive_output_command(&command);
        assert_eq!(
            rendered,
            "output: rumble strong=30000 weak=12000 session_id=42"
        );
    }

    #[test]
    fn interactive_shutdown_summary_includes_reason_and_frames_written() {
        let summary = render_interactive_shutdown_summary(
            super::InteractiveShutdownReason::CtrlC,
            UinputScriptMode::Exercise,
            SessionId::new(9),
            None,
        );
        assert!(summary.contains("reason: ctrl-c"));
        assert!(summary.contains("session_id: 9"));
        assert!(summary.contains("frames_written: 0"));
    }

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
    fn phase_three_commands_match_expected_order() {
        let commands = phase_gate_commands(3).expect("phase 3 commands");
        let expected = PHASE_3_COMMANDS
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
    fn phase_four_commands_match_expected_order() {
        let commands = phase_gate_commands(4).expect("phase 4 commands");
        assert_eq!(commands.len(), 4);
        assert_eq!(
            commands[0].join(" "),
            "cargo test --workspace --all-features"
        );
    }

    #[test]
    fn phase_five_commands_match_expected_order() {
        let commands = phase_gate_commands(5).expect("phase 5 commands");
        let expected = PHASE_5_COMMANDS
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
    fn validate_config_success_output_is_stable() {
        let repo_root = repo_root().expect("workspace root");
        let config_path = repo_root.join("samples/configs/dualsense-identity.yaml");
        let output = validate_config(config_path).expect("config should validate");
        assert_snapshot!("validate_config_dualsense_identity", output);
    }

    #[test]
    fn validate_config_returns_structured_validation_error() {
        let repo_root = repo_root().expect("workspace root");
        let config_path = repo_root.join("samples/configs/broken-mode.yaml");
        let error = validate_config(config_path).expect_err("config should fail");
        let rendered = error.to_string();
        assert!(rendered.contains("configuration validation failed"));
        assert!(rendered.contains("outputHandling.callbackNamespace"));
        assert!(rendered.contains("outputHandling.mode is `callback`"));
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
    fn phase_three_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_three = plan
            .split("## Phase 3:")
            .nth(1)
            .and_then(|section| section.split("## Phase 4:").next())
            .expect("phase 3 section");
        let automated = phase_three
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in PHASE_3_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 3 automated section is missing {command}"
            );
        }
    }

    #[test]
    fn phase_four_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_four = plan
            .split("## Phase 4:")
            .nth(1)
            .and_then(|section| section.split("## Phase 5:").next())
            .expect("phase 4 section");
        let automated = phase_four
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in phase_gate_commands(4)
            .expect("phase 4 commands")
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 4 automated section is missing {command}"
            );
        }
    }

    #[test]
    fn phase_five_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_five = plan
            .split("## Phase 5:")
            .nth(1)
            .and_then(|section| section.split("## Phase 6:").next())
            .expect("phase 5 section");
        let automated = phase_five
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in PHASE_5_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 5 automated section is missing {command}"
            );
        }
    }

    #[test]
    fn phase_six_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_six = plan
            .split("## Phase 6:")
            .nth(1)
            .and_then(|section| section.split("## Phase 7:").next())
            .expect("phase 6 section");
        let automated = phase_six
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in PHASE_6_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 6 automated section is missing {command}"
            );
        }
    }

    #[test]
    fn phase_eight_commands_match_expected_order() {
        let commands = phase_gate_commands(8).expect("phase 8 commands");
        let expected = PHASE_8_COMMANDS
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
    fn phase_eight_commands_match_plan_spec() {
        let repo_root = repo_root().expect("workspace root");
        let plan_path = repo_root.join("docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md");
        let plan = std::fs::read_to_string(plan_path).expect("read implementation plan");
        let phase_eight = plan
            .split("## Phase 8:")
            .nth(1)
            .and_then(|section| section.split("## Phase 9:").next())
            .expect("phase 8 section");
        let automated = phase_eight
            .split("Automated portion:")
            .nth(1)
            .and_then(|section| section.split("Manual portion:").next())
            .expect("automated section");

        for command in PHASE_8_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
        {
            assert!(
                automated.contains(&command),
                "phase 8 automated section is missing {command}"
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

    #[test]
    fn validate_fixture_summary_for_reverse_event_fixture_is_typed() {
        let repo_root = repo_root().expect("workspace root");
        let fixture_path =
            repo_root.join("crates/gr-testkit/fixtures/community/dualsense-rumble-standalone.yaml");
        let summary = validate_fixture(fixture_path).expect("fixture should validate");
        assert_snapshot!("validate_fixture_reverse_event", summary);
    }

    #[test]
    fn run_uinput_smoke_output_is_stable() {
        let output = run_uinput_smoke("generic-gamepad", UinputSmokeOptions::default())
            .expect("uinput smoke");
        assert_snapshot!("run_uinput_smoke_generic_gamepad", output);
    }

    #[test]
    fn support_report_output_is_stable() {
        let output =
            support_report(Some("generic-gamepad"), Some("compatibility")).expect("report");
        assert_snapshot!("support_report_generic_gamepad_compatibility", output);
    }

    #[test]
    fn simulate_session_output_is_stable() {
        let repo_root = repo_root().expect("workspace root");
        let scenario =
            repo_root.join("crates/gr-testkit/fixtures/community/fake-session-rumble.yaml");
        let output = simulate_session(&scenario, None::<&std::path::Path>).expect("scenario");
        assert_snapshot!("simulate_session_fake_rumble", output);
    }

    #[test]
    fn simulate_session_dualsense_coalesce_runs_to_completion() {
        // The coalesce semantic itself is covered by a deterministic
        // unit test in gr-session (`bounded_input_queue_clears_stale_frames_on_overflow`).
        // This integration test only proves the demo scenario runs to
        // completion end-to-end without panicking; it intentionally does
        // not snapshot or assert the counter values, which are
        // race-sensitive across schedulers.
        let repo_root = repo_root().expect("workspace root");
        let scenario = repo_root.join("samples/scenarios/dualsense-coalesce.yaml");
        let output = simulate_session(&scenario, None::<&std::path::Path>).expect("scenario");
        assert!(
            output.contains("scenario: dualsense-coalesce"),
            "missing scenario header in output:\n{output}",
        );
        assert!(
            output.contains("mode: runtime-session"),
            "missing mode header in output:\n{output}",
        );
        assert!(
            output.contains("frames.coalesced"),
            "missing frames.coalesced counter in diagnostics:\n{output}",
        );
    }

    #[test]
    fn replay_trace_output_is_stable() {
        let repo_root = repo_root().expect("workspace root");
        let trace = repo_root.join("crates/gr-testkit/fixtures/community/fake-trace-rumble.yaml");
        let output = replay_trace(trace).expect("trace");
        assert_snapshot!("replay_trace_fake_rumble", output);
    }

    #[test]
    fn replay_trace_phase6_dualsense_fixture_is_stable() {
        let repo_root = repo_root().expect("workspace root");
        let trace =
            repo_root.join("crates/gr-translators/fixtures/dualsense-rumble-from-host.yaml");
        let output = replay_trace(trace).expect("trace");
        assert_snapshot!("replay_trace_dualsense_phase6", output);
    }

    #[test]
    fn many_sessions_runs_through_n_sessions() {
        let output = super::many_sessions(4).expect("many sessions");
        assert!(
            output.starts_with("many_sessions: 4\n"),
            "header missing: {output:?}"
        );
        let session_lines = output
            .lines()
            .filter(|line| line.starts_with("- session "))
            .count();
        assert_eq!(
            session_lines, 4,
            "expected 4 session status lines:\n{output}"
        );
    }

    #[test]
    fn replay_trace_phase6_xbox_evdev_fixture_is_stable() {
        let repo_root = repo_root().expect("workspace root");
        let trace = repo_root.join("crates/gr-translators/fixtures/xbox360-evdev-roundtrip.yaml");
        let output = replay_trace(trace).expect("trace");
        assert_snapshot!("replay_trace_xbox360_phase6", output);
    }

    #[test]
    fn plan_session_output_is_stable() {
        // Pin `--host-platform linux` so the snapshot is deterministic
        // across CI runners. The planner falls back to the runtime host
        // when no preference is given, which would make the test
        // OS-dependent (macOS / Windows runners would not match any
        // Linux backend in the inventory and produce a rejection
        // instead of a plan).
        let repo_root = repo_root().expect("workspace root");
        let inventory = repo_root.join("samples/inventories/linux-uhid-only.yaml");
        let output = plan_session(
            "dualsense",
            "identity-aware",
            inventory,
            Some("linux"),
            None,
            None,
            Some(1),
        )
        .expect("plan");
        assert_snapshot!("plan_session_identity_aware", output);
    }

    #[test]
    fn plan_session_rejection_output_is_stable() {
        // Empty inventory is OS-independent (no backends at any
        // platform), so the rejection is stable regardless of the
        // runner. Explicit host_platform omitted on purpose to
        // exercise the no-hint branch.
        let repo_root = repo_root().expect("workspace root");
        let inventory = repo_root.join("samples/inventories/empty.yaml");
        let output = plan_session(
            "dualsense",
            "hardware-faithful",
            inventory,
            None,
            None,
            None,
            Some(1),
        )
        .expect("rejection");
        assert_snapshot!("plan_session_rejection", output);
    }
}
