//! Reference consumer for the curated controller-native API.

mod gui;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vgpd-demo", version, about = "VirtualGamepad native API demo")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the local graphical controller debugger (Linux only).
    Gui,
    /// Print the supported curated controller families and realization model.
    Info,
}

fn main() {
    let result = match Cli::parse().command {
        Command::Gui => gui::run(),
        Command::Info => {
            print_info();
            Ok(())
        }
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn print_info() {
    println!(
        "vgpd-demo {}\n\ncurated controllers: Generic Gamepad, Xbox 360, DualSense, Steam Controller\ncreation: explicit Linux target; no provider fallback\ninput: normalized spatial controls plus explicit native control enums\nlifecycle: local mutable state with explicit commit()",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn native_api_commands_parse() {
        assert!(matches!(
            Cli::parse_from(["vgpd-demo", "gui"]).command,
            Command::Gui
        ));
        assert!(matches!(
            Cli::parse_from(["vgpd-demo", "info"]).command,
            Command::Info
        ));
    }
}
