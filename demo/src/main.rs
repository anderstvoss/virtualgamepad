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
    /// Run a runtime session scenario using the canonical scenario alias.
    RunScenario { path: std::path::PathBuf },
    /// Spin up many concurrent fake-backed runtime sessions.
    ManySessions { count: usize },
    /// Run a Linux uinput smoke probe for a profile; use `--interactive`
    /// to keep the device alive for host inspection.
    RunUinputSmoke {
        profile_id: String,
        #[arg(long)]
        interactive: bool,
        #[arg(long, default_value = "none")]
        script: String,
        #[arg(long, default_value_t = 750)]
        step_delay_ms: u64,
    },
    /// Run a Linux UHID smoke probe for a profile; use `--interactive`
    /// to keep the device alive for host inspection.
    RunUhidSmoke {
        profile_id: String,
        #[arg(long)]
        interactive: bool,
        #[arg(long, default_value = "usb")]
        bus: String,
    },
    /// Run a Linux transport smoke probe for the Phase 11 `DualSense` USB target.
    RunTransportSmoke {
        profile_id: String,
        #[arg(long)]
        interactive: bool,
    },
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
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match Cli::parse().command {
        Command::Info => {
            print_info();
            Ok(())
        }
        Command::ShowTypes => {
            print_show_types();
            Ok(())
        }
        Command::PhaseGate { phase } => phase_gate::run(phase).map_err(|error| error.to_string()),
        Command::ListProfiles => print_output(gr_cli::list_profiles()),
        Command::ShowCapabilities { profile_id } => {
            print_output(gr_cli::show_capabilities(&profile_id))
        }
        Command::ValidateConfig { path } => print_output(gr_cli::validate_config(path)),
        Command::SimulateSession { path } => print_output(gr_cli::simulate_session(path, None)),
        Command::RunScenario { path } => print_output(gr_cli::run_scenario(path, None)),
        Command::ManySessions { count } => print_output(gr_cli::many_sessions(count)),
        Command::RunUinputSmoke {
            profile_id,
            interactive,
            script,
            step_delay_ms,
        } => print_output(
            gr_cli::parse_uinput_smoke_options(interactive, &script, step_delay_ms)
                .and_then(|options| gr_cli::run_uinput_smoke(&profile_id, options)),
        ),
        Command::SupportReport { profile, tier } => {
            print_output(gr_cli::support_report(profile.as_deref(), tier.as_deref()))
        }
        Command::RunUhidSmoke {
            profile_id,
            interactive,
            bus,
        } => print_output(
            gr_cli::parse_uhid_smoke_options(interactive, &bus)
                .and_then(|options| gr_cli::run_uhid_smoke(&profile_id, options)),
        ),
        Command::RunTransportSmoke {
            profile_id,
            interactive,
        } => print_output(gr_cli::run_transport_smoke(&profile_id, interactive)),
        Command::ReplayTrace { path } => print_output(gr_cli::replay_trace(path)),
        Command::PlanSession {
            profile_id,
            goal,
            inventory,
            host_platform,
            backend_preference,
            provider_preference,
            session_id,
        } => print_output(gr_cli::plan_session(
            &profile_id,
            &goal,
            inventory,
            host_platform.as_deref(),
            backend_preference.as_deref(),
            provider_preference.as_deref(),
            session_id,
        )),
    }
}

fn print_output(result: Result<String, impl ToString>) -> Result<(), String> {
    let output = result.map_err(|error| error.to_string())?;
    println!("{output}");
    Ok(())
}

fn print_info() {
    println!("vgpd-demo {}", env!("CARGO_PKG_VERSION"));
    println!("companion demo for the virtualgamepad workspace");
    println!();
    println!("library status: through Phase 9 Linux uhid provider support");
    println!(
        "demo status:    gate runner, profile review, config validation, simulate-session, run-scenario, many-sessions, run-uinput-smoke, run-uhid-smoke, run-transport-smoke, support-report, replay-trace, plan-session"
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
    fn run_scenario_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "run-scenario",
            "samples/scenarios/dualsense-audio-mode.yaml",
        ]);
        assert!(matches!(
            cli.command,
            Command::RunScenario { path } if path == std::path::Path::new("samples/scenarios/dualsense-audio-mode.yaml")
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
    fn run_uhid_smoke_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "run-uhid-smoke",
            "dualsense",
            "--interactive",
            "--bus",
            "bluetooth",
        ]);
        assert!(matches!(
            cli.command,
            Command::RunUhidSmoke {
                profile_id,
                interactive,
                bus,
            } if profile_id == "dualsense"
                && interactive
                && bus == "bluetooth"
        ));
    }

    #[test]
    fn run_uinput_smoke_subcommand_parses() {
        let cli = Cli::parse_from(["vgpd-demo", "run-uinput-smoke", "generic-gamepad"]);
        assert!(matches!(
            cli.command,
            Command::RunUinputSmoke {
                profile_id,
                interactive,
                script,
                step_delay_ms,
            } if profile_id == "generic-gamepad"
                && !interactive
                && script == "none"
                && step_delay_ms == 750
        ));
    }

    #[test]
    fn run_uinput_smoke_interactive_flags_parse() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "run-uinput-smoke",
            "xbox360",
            "--interactive",
            "--script",
            "exercise",
            "--step-delay-ms",
            "600",
        ]);
        assert!(matches!(
            cli.command,
            Command::RunUinputSmoke {
                profile_id,
                interactive,
                script,
                step_delay_ms,
            } if profile_id == "xbox360"
                && interactive
                && script == "exercise"
                && step_delay_ms == 600
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

    #[test]
    fn run_transport_smoke_subcommand_parses() {
        let cli = Cli::parse_from([
            "vgpd-demo",
            "run-transport-smoke",
            "dualsense",
            "--interactive",
        ]);
        assert!(matches!(
            cli.command,
            Command::RunTransportSmoke {
                profile_id,
                interactive,
            } if profile_id == "dualsense" && interactive
        ));
    }
}
