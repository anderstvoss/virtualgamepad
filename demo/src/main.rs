//! `vgpd-demo` — companion demo for the `virtualgamepad` library.
//!
//! Starts as a minimal CLI scaffold and grows alongside the library
//! buildout; see `demo/README.md` for the planned growth phases.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vgpd-demo", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print demo and library scaffold information.
    Info,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Info => print_info(),
    }
}

fn print_info() {
    println!("vgpd-demo {}", env!("CARGO_PKG_VERSION"));
    println!("companion demo for the virtualgamepad library");
    println!();
    println!("library status: pre-API scaffold (see docs/spec/ for design)");
    println!("demo status:    CLI scaffold; richer commands and GUI land as the library grows");
}
