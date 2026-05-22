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
    #[command(alias = "show-type")]
    ShowTypes,
    /// Run the automated portion of a phase gate and print the manual checklist.
    PhaseGate { phase: u8 },
    /// List the built-in controller profiles (Phase 2 deliverable).
    ListProfiles,
    /// Print declared capabilities for a built-in profile (Phase 2 deliverable).
    ShowCapabilities {
        /// Profile identifier (e.g. `dualsense`, `xbox360`).
        profile_id: String,
    },
    /// Validate a session config file (Phase 3 deliverable).
    ValidateConfig { path: std::path::PathBuf },
}

fn main() {
    let cli = Cli::parse();
    let result: Result<(), String> = match cli.command {
        Command::Info => {
            print_info();
            Ok(())
        }
        Command::ShowTypes => {
            print_show_types();
            Ok(())
        }
        Command::PhaseGate { phase } => phase_gate::run(phase).map_err(|e| e.to_string()),
        Command::ListProfiles => match gr_cli::list_profiles() {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        },
        Command::ShowCapabilities { profile_id } => match gr_cli::show_capabilities(&profile_id) {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        },
        Command::ValidateConfig { path } => match gr_cli::validate_config(path) {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        },
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
    println!("library status: through Phase 2 profiles and capability registry");
    println!("demo status:    gate runner, type catalog, profile review, and config validation");
}

fn print_show_types() {
    print!("{}", gr_core::render_type_catalog());
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;
    use insta::assert_snapshot;

    #[test]
    fn show_types_output_is_stable() {
        assert_snapshot!("show_types", gr_core::render_type_catalog());
    }

    #[test]
    fn show_types_subcommand_accepts_canonical_name() {
        let cli = Cli::parse_from(["vgpd-demo", "show-types"]);
        assert!(matches!(cli.command, Command::ShowTypes));
    }

    #[test]
    fn show_types_subcommand_accepts_singular_alias() {
        let cli = Cli::parse_from(["vgpd-demo", "show-type"]);
        assert!(matches!(cli.command, Command::ShowTypes));
    }

    #[test]
    fn list_profiles_subcommand_parses() {
        let cli = Cli::parse_from(["vgpd-demo", "list-profiles"]);
        assert!(matches!(cli.command, Command::ListProfiles));
    }

    #[test]
    fn show_capabilities_subcommand_parses() {
        let cli = Cli::parse_from(["vgpd-demo", "show-capabilities", "dualsense"]);
        assert!(matches!(
            cli.command,
            Command::ShowCapabilities { profile_id } if profile_id == "dualsense"
        ));
    }

    #[test]
    fn validate_config_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "validate-config",
            "samples/configs/dualsense-identity.yaml",
        ]);
        assert!(matches!(
            cli.command,
            Command::ValidateConfig { path } if path == std::path::Path::new("samples/configs/dualsense-identity.yaml")
        ));
    }
}
