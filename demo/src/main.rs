//! `vgpd-demo` — companion demo for the `virtualgamepad` workspace.
//!
//! Starts as a minimal CLI scaffold and grows alongside the library
//! buildout; see `demo/README.md` for the planned growth phases.

mod phase_gate;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vgpd-demo", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print demo and workspace scaffold information.
    Info,
    /// Run the automated portion of a phase gate and print the manual checklist.
    PhaseGate { phase: u8 },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Info => {
            print_info();
            Ok(())
        }
        Command::PhaseGate { phase } => phase_gate::run(phase),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn print_info() {
    println!("vgpd-demo {}", env!("CARGO_PKG_VERSION"));
    println!("companion demo for the virtualgamepad workspace");
    println!();
    println!("library status: Phase 0 workspace scaffold (see docs/spec/ for design)");
    println!("demo status:    gate runner and CLI scaffold");
}
