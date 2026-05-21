//! Shared implementation for the `gr-cli` binary and other tooling.

use gr_testkit::fixtures::{FixtureDocument, FixtureError, load_fixture};
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

/// Validate a fixture path and summarize the decoded envelope.
///
/// # Errors
///
/// Returns an error if the path cannot be read, the YAML cannot be
/// parsed, or the fixture envelope is invalid.
pub fn validate_fixture(path: impl AsRef<Path>) -> Result<String, CliError> {
    let path = path.as_ref();
    let fixture = load_fixture(path).map_err(|source| CliError::Fixture {
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
        2..=12 => Err(CliError::UnimplementedPhase { phase }),
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
        PHASE_0_COMMANDS, PHASE_1_COMMANDS, phase_gate_commands, repo_root, repo_root_from,
        validate_fixture,
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
    fn unimplemented_phase_errors_clearly() {
        let error = phase_gate_commands(2).expect_err("phase 2 should be unimplemented");
        assert_eq!(
            error.to_string(),
            "automated gate not implemented for phase `2` yet"
        );
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

        let actual_commands = PHASE_0_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
            .collect::<Vec<_>>();

        for command in actual_commands {
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

        let actual_commands = PHASE_1_COMMANDS
            .iter()
            .map(|command| format!("`{}`", command.join(" ")))
            .collect::<Vec<_>>();

        for command in actual_commands {
            assert!(
                automated.contains(&command),
                "phase 1 automated section is missing {command}"
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
