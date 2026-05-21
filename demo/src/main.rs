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
    /// Print the canonical Phase 1 type catalog.
    ShowTypes,
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
        Command::ShowTypes => {
            print_show_types();
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
    println!("library status: Phase 1 core domain model (see docs/spec/ for design)");
    println!("demo status:    gate runner, CLI scaffold, and type catalog");
}

fn print_show_types() {
    print!("{}", gr_core::render_type_catalog());
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    #[test]
    fn show_types_output_is_stable() {
        assert_snapshot!("show_types", gr_core::render_type_catalog());
    }
}
