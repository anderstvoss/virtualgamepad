//! `gr-cli` binary entrypoint.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gr-cli", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate a fixture file and print the decoded envelope.
    ValidateFixture { path: PathBuf },
    /// Run the automated portion of a phase gate.
    PhaseGate(PhaseGateArgs),
    /// List the built-in controller profiles (Phase 2).
    ListProfiles,
    /// Print declared capabilities for a built-in profile (Phase 2).
    ShowCapabilities {
        /// Profile identifier (e.g. `dualsense`, `xbox360`).
        profile_id: String,
    },
    /// Cross-check declared capabilities against translator coverage (Phase 2).
    CapabilityCoverage,
}

#[derive(Args, Debug)]
struct PhaseGateArgs {
    /// Phase number from the implementation plan.
    phase: u8,
    /// Run only the deterministic automated portion.
    #[arg(long)]
    auto: bool,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::ValidateFixture { path } => match gr_cli::validate_fixture(path) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::PhaseGate(args) => run_phase_gate(&args),
        Command::ListProfiles => match gr_cli::list_profiles() {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::ShowCapabilities { profile_id } => match gr_cli::show_capabilities(&profile_id) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::CapabilityCoverage => match gr_cli::capability_coverage() {
            Ok(report) => {
                match serde_yaml::to_string(&report) {
                    Ok(output) => print!("{output}"),
                    Err(error) => {
                        eprintln!("failed to serialize yaml output: {error}");
                        std::process::exit(1);
                    }
                }
                if !report.all_covered() {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
    }
}

fn run_phase_gate(args: &PhaseGateArgs) {
    if !args.auto {
        println!("pass `--auto` to run the automated phase-gate portion");
        return;
    }

    match gr_cli::run_phase_gate_auto(args.phase) {
        Ok(report) => {
            println!("{}", gr_cli::render_phase_gate_report(&report));
            if !report.all_passed() {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
