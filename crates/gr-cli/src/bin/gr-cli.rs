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
