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
    /// Run a fake backend session scenario (Phase 4 deliverable).
    SimulateSession { path: std::path::PathBuf },
    /// Spin up many concurrent fake-backed runtime sessions.
    ManySessions { count: usize },
    /// Run a one-shot Linux uinput smoke probe for a profile and print the report.
    RunUinputSmoke { profile_id: String },
    /// Generate the initial support-claim evidence report.
    SupportReport {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        tier: Option<String>,
    },
    /// Render a backend trace fixture (Phase 4 deliverable).
    ReplayTrace { path: std::path::PathBuf },
    /// Plan a session from a profile id and backend inventory fixture (Phase 5 deliverable).
    PlanSession {
        profile_id: String,
        #[arg(long)]
        goal: String,
        #[arg(long)]
        inventory: std::path::PathBuf,
        #[arg(long)]
        host_platform: Option<String>,
        #[arg(long)]
        backend_preference: Option<String>,
        #[arg(long)]
        provider_preference: Option<String>,
        /// Session id stamped on the resulting plan. Defaults to 1
        /// for snapshot stability; production callers assign per-session
        /// ids via the manager.
        #[arg(long)]
        session_id: Option<u64>,
    },
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
        Command::SimulateSession { path } => match gr_cli::simulate_session(path, None) {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        },
        Command::ManySessions { count } => match gr_cli::many_sessions(count) {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        },
        Command::RunUinputSmoke { profile_id } => match gr_cli::run_uinput_smoke(&profile_id) {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        },
        Command::SupportReport { profile, tier } => {
            match gr_cli::support_report(profile.as_deref(), tier.as_deref()) {
                Ok(output) => {
                    println!("{output}");
                    Ok(())
                }
                Err(error) => Err(error.to_string()),
            }
        }
        Command::ReplayTrace { path } => match gr_cli::replay_trace(path) {
            Ok(output) => {
                println!("{output}");
                Ok(())
            }
            Err(error) => Err(error.to_string()),
        },
        Command::PlanSession {
            profile_id,
            goal,
            inventory,
            host_platform,
            backend_preference,
            provider_preference,
            session_id,
        } => match gr_cli::plan_session(
            &profile_id,
            &goal,
            inventory,
            host_platform.as_deref(),
            backend_preference.as_deref(),
            provider_preference.as_deref(),
            session_id,
        ) {
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
    println!("library status: through Phase 7 session runtime and trace tooling");
    println!(
        "demo status:    gate runner, profile review, config validation, simulate-session, many-sessions, run-uinput-smoke, support-report, replay-trace, plan-session"
    );
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

    #[test]
    fn simulate_session_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "simulate-session",
            "crates/gr-testkit/fixtures/community/fake-session-rumble.yaml",
        ]);
        assert!(matches!(
            cli.command,
            Command::SimulateSession { path } if path == std::path::Path::new("crates/gr-testkit/fixtures/community/fake-session-rumble.yaml")
        ));
    }

    #[test]
    fn replay_trace_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "replay-trace",
            "crates/gr-testkit/fixtures/community/fake-trace-rumble.yaml",
        ]);
        assert!(matches!(
            cli.command,
            Command::ReplayTrace { path } if path == std::path::Path::new("crates/gr-testkit/fixtures/community/fake-trace-rumble.yaml")
        ));
    }

    #[test]
    fn plan_session_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "plan-session",
            "dualsense",
            "--goal",
            "identity-aware",
            "--inventory",
            "samples/inventories/linux-uhid-only.yaml",
        ]);
        assert!(matches!(
            cli.command,
            Command::PlanSession { profile_id, goal, inventory, .. }
                if profile_id == "dualsense"
                    && goal == "identity-aware"
                    && inventory == std::path::Path::new("samples/inventories/linux-uhid-only.yaml")
        ));
    }

    #[test]
    fn many_sessions_subcommand_parses() {
        let cli = Cli::parse_from(["vgpd-demo", "many-sessions", "4"]);
        assert!(matches!(cli.command, Command::ManySessions { count } if count == 4));
    }

    #[test]
    fn run_uinput_smoke_subcommand_parses() {
        let cli = Cli::parse_from(["vgpd-demo", "run-uinput-smoke", "generic-gamepad"]);
        assert!(matches!(
            cli.command,
            Command::RunUinputSmoke { profile_id } if profile_id == "generic-gamepad"
        ));
    }

    #[test]
    fn support_report_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "support-report",
            "--profile",
            "xbox360",
            "--tier",
            "compatibility",
        ]);
        assert!(matches!(
            cli.command,
            Command::SupportReport { profile, tier }
                if profile.as_deref() == Some("xbox360")
                    && tier.as_deref() == Some("compatibility")
        ));
    }
}
