//! Phase-gate rendering for `vgpd-demo`.

use gr_cli::PhaseGateReport;
use std::fmt;
use std::path::PathBuf;

const PLAN_PATH: &str = "../docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md";

pub fn run(phase: u8) -> Result<(), PhaseGateError> {
    let report = gr_cli::run_phase_gate_auto(phase).map_err(PhaseGateError::Cli)?;
    let gate = load_gate(phase)?;

    println!("Phase {phase}: {}", gate.title);
    println!(
        "{}",
        "=".repeat(9 + phase.to_string().len() + gate.title.len())
    );
    println!("Automated checks:");
    render_automated_checks(&report);
    println!();
    if !report.all_passed() {
        println!("Manual review should wait until the automated checks are green.");
        println!();
    }
    println!("Manual checklist:");
    for item in &gate.manual {
        println!("  {item}");
    }
    println!();
    println!("When complete, sign off with:");
    println!("  {}", gate.sign_off);

    if report.all_passed() {
        Ok(())
    } else {
        Err(PhaseGateError::AutomatedChecksFailed { phase })
    }
}

#[derive(Debug)]
struct GateSection {
    title: String,
    manual: Vec<String>,
    sign_off: String,
}

#[derive(Debug)]
pub enum PhaseGateError {
    Cli(gr_cli::CliError),
    Io(std::io::Error),
    PhaseNotFound { phase: u8 },
    ExitGateNotFound { phase: u8 },
    SignOffNotFound { phase: u8 },
    AutomatedChecksFailed { phase: u8 },
}

impl fmt::Display for PhaseGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cli(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "failed to read implementation plan: {error}"),
            Self::PhaseNotFound { phase } => {
                write!(
                    f,
                    "could not find `## Phase {phase}:` in the implementation plan"
                )
            }
            Self::ExitGateNotFound { phase } => write!(
                f,
                "could not find the exit gate for Phase {phase} in the implementation plan"
            ),
            Self::SignOffNotFound { phase } => {
                write!(f, "could not find the sign-off line for Phase {phase}")
            }
            Self::AutomatedChecksFailed { phase } => {
                write!(f, "phase {phase} automated checks failed")
            }
        }
    }
}

impl std::error::Error for PhaseGateError {}

fn load_gate(phase: u8) -> Result<GateSection, PhaseGateError> {
    let path = plan_path();
    let contents = std::fs::read_to_string(path).map_err(PhaseGateError::Io)?;
    let lines: Vec<&str> = contents.lines().collect();
    let phase_header = format!("## Phase {phase}:");
    let Some(phase_start) = lines
        .iter()
        .position(|line| line.starts_with(&phase_header))
    else {
        return Err(PhaseGateError::PhaseNotFound { phase });
    };

    let phase_end = lines
        .iter()
        .enumerate()
        .skip(phase_start + 1)
        .find_map(|(index, line)| line.starts_with("## Phase ").then_some(index))
        .unwrap_or(lines.len());
    let phase_lines = &lines[phase_start..phase_end];
    let title = phase_lines[0].split_once(':').map_or_else(
        || format!("Phase {phase}"),
        |(_, rest)| rest.trim().to_string(),
    );

    let Some(exit_gate_start) = phase_lines
        .iter()
        .position(|line| line.trim() == "### Exit gate")
    else {
        return Err(PhaseGateError::ExitGateNotFound { phase });
    };

    let gate_lines = &phase_lines[exit_gate_start + 1..];
    let manual = collect_checklist(gate_lines, "Manual portion:", "Sign-off:");
    let Some(sign_off) = gate_lines
        .iter()
        .find_map(|line| line.strip_prefix("Sign-off: ").map(str::to_string))
    else {
        return Err(PhaseGateError::SignOffNotFound { phase });
    };

    Ok(GateSection {
        title,
        manual,
        sign_off,
    })
}

fn collect_checklist(lines: &[&str], start_marker: &str, end_marker: &str) -> Vec<String> {
    let Some(start) = lines.iter().position(|line| line.trim() == start_marker) else {
        return Vec::new();
    };

    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, line)| line.trim().starts_with(end_marker).then_some(index))
        .unwrap_or(lines.len());

    lines[start + 1..end]
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            (!trimmed.is_empty()).then_some(trimmed.to_string())
        })
        .collect()
}

fn render_automated_checks(report: &PhaseGateReport) {
    for check in &report.checks {
        let mark = if check.success { "✓" } else { "✗" };
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
        println!("  {mark} {}{exit_suffix}", check.command_display);

        if !check.success {
            if !check.stderr.trim().is_empty() {
                println!("    stderr:");
                for line in check.stderr.lines() {
                    println!("      {line}");
                }
            }
            if !check.stdout.trim().is_empty() {
                println!("    stdout:");
                for line in check.stdout.lines() {
                    println!("      {line}");
                }
            }
        }
    }
}

fn plan_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PLAN_PATH)
}

#[cfg(test)]
mod tests {
    use super::load_gate;

    #[test]
    fn phase_zero_manual_checklist_extracts() {
        let gate = load_gate(0).expect("phase 0 gate");
        assert_eq!(gate.manual.len(), 4);
        assert!(gate.manual[0].contains("vgpd-demo phase-gate 0"));
        assert!(gate.manual[3].contains("version field"));
    }

    #[test]
    fn phase_zero_sign_off_extracts() {
        let gate = load_gate(0).expect("phase 0 gate");
        assert_eq!(
            gate.sign_off,
            "`git commit --allow-empty -m \"chore(phase-gate): Phase 0 gate passed\"`"
        );
    }
}
