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
    /// Parse and validate a session config file (Phase 3).
    ValidateConfig { path: PathBuf },
    /// Validate a fixture file and print the decoded envelope.
    ValidateFixture { path: PathBuf },
    /// Run a fake-backend-backed Phase 4 session scenario.
    SimulateSession {
        path: PathBuf,
        #[arg(long)]
        record: Option<PathBuf>,
        #[arg(long)]
        concurrency: Option<usize>,
    },
    /// Run a runtime session scenario using the canonical scenario alias.
    RunScenario {
        path: PathBuf,
        #[arg(long)]
        record: Option<PathBuf>,
    },
    /// Render a backend trace fixture.
    ReplayTrace { path: PathBuf },
    /// Plan a session from a profile id and backend inventory fixture.
    PlanSession {
        profile_id: String,
        #[arg(long)]
        goal: String,
        #[arg(long)]
        inventory: PathBuf,
        #[arg(long)]
        host_platform: Option<String>,
        #[arg(long)]
        backend_preference: Option<String>,
        #[arg(long)]
        provider_preference: Option<String>,
        /// Session id to stamp on the resulting plan. Defaults to 1
        /// for snapshot stability; production callers assign per-session
        /// ids via the manager.
        #[arg(long)]
        session_id: Option<u64>,
    },
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
    /// Spin up many fake-backed sessions and print their status.
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
    /// Compare the built-in Phase 9 UHID implementation against the
    /// descriptor and reverse-trace references.
    CompareRealDevice {
        #[arg(long)]
        profile: String,
        #[arg(long, default_value = "usb")]
        bus: String,
        #[arg(long, default_value = "identity")]
        layer: String,
    },
    /// Generate the initial support-claim evidence report.
    SupportReport {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        tier: Option<String>,
    },
}

#[derive(Args, Debug)]
struct PhaseGateArgs {
    /// Phase number from the implementation plan.
    phase: u8,
    /// Run only the deterministic automated portion.
    #[arg(long)]
    auto: bool,
}

#[allow(clippy::too_many_lines)]
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::ValidateConfig { path } => match gr_cli::validate_config(path) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::ValidateFixture { path } => match gr_cli::validate_fixture(path) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::SimulateSession {
            path,
            record,
            concurrency,
        } => {
            if let Some(count) = concurrency {
                match gr_cli::many_sessions(count) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("{error}");
                        std::process::exit(1);
                    }
                }
            } else {
                match gr_cli::simulate_session(path, record.as_deref()) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("{error}");
                        std::process::exit(1);
                    }
                }
            }
        }
        Command::RunScenario { path, record } => {
            match gr_cli::run_scenario(path, record.as_deref()) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
        Command::ManySessions { count } => match gr_cli::many_sessions(count) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::ReplayTrace { path } => match gr_cli::replay_trace(path) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
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
        Command::RunUinputSmoke {
            profile_id,
            interactive,
            script,
            step_delay_ms,
        } => match gr_cli::parse_uinput_smoke_options(interactive, &script, step_delay_ms)
            .and_then(|options| gr_cli::run_uinput_smoke(&profile_id, options))
        {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::RunUhidSmoke {
            profile_id,
            interactive,
            bus,
        } => match gr_cli::parse_uhid_smoke_options(interactive, &bus)
            .and_then(|options| gr_cli::run_uhid_smoke(&profile_id, options))
        {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::RunTransportSmoke {
            profile_id,
            interactive,
        } => match gr_cli::run_transport_smoke(&profile_id, interactive) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        },
        Command::CompareRealDevice {
            profile,
            bus,
            layer,
        } => {
            let Ok(bus_mode) = bus.parse() else {
                eprintln!("invalid `bus` value `{bus}`");
                std::process::exit(1);
            };
            let Ok(layer) = layer.parse() else {
                eprintln!("invalid `layer` value `{layer}`");
                std::process::exit(1);
            };
            match gr_cli::compare_real_device(&profile, bus_mode, layer) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
        Command::SupportReport { profile, tier } => {
            match gr_cli::support_report(profile.as_deref(), tier.as_deref()) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;
    use std::path::Path;

    #[test]
    fn run_uinput_smoke_subcommand_parses() {
        let cli = Cli::parse_from(["gr-cli", "run-uinput-smoke", "generic-gamepad"]);
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
            "gr-cli",
            "run-uinput-smoke",
            "xbox360",
            "--interactive",
            "--script",
            "exercise",
            "--step-delay-ms",
            "1200",
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
                && step_delay_ms == 1200
        ));
    }

    #[test]
    fn support_report_subcommand_parses() {
        let cli = Cli::parse_from([
            "gr-cli",
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
    fn run_uhid_smoke_subcommand_parses() {
        let cli = Cli::parse_from([
            "gr-cli",
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
    fn compare_real_device_subcommand_parses() {
        let cli = Cli::parse_from([
            "gr-cli",
            "compare-real-device",
            "--profile",
            "dualsense",
            "--bus",
            "usb",
            "--layer",
            "transport",
        ]);
        assert!(matches!(
            cli.command,
            Command::CompareRealDevice { profile, bus, layer }
                if profile == "dualsense" && bus == "usb" && layer == "transport"
        ));
    }

    #[test]
    fn run_transport_smoke_subcommand_parses() {
        let cli = Cli::parse_from([
            "gr-cli",
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

    #[test]
    fn simulate_session_subcommand_still_parses() {
        let cli = Cli::parse_from([
            "gr-cli",
            "simulate-session",
            "crates/gr-testkit/fixtures/community/fake-session-rumble.yaml",
        ]);
        assert!(matches!(
            cli.command,
            Command::SimulateSession { path, .. }
                if path == Path::new("crates/gr-testkit/fixtures/community/fake-session-rumble.yaml")
        ));
    }

    #[test]
    fn run_scenario_subcommand_parses() {
        let cli = Cli::parse_from([
            "gr-cli",
            "run-scenario",
            "samples/scenarios/dualsense-audio-mode.yaml",
        ]);
        assert!(matches!(
            cli.command,
            Command::RunScenario { path, .. }
                if path == Path::new("samples/scenarios/dualsense-audio-mode.yaml")
        ));
    }
}
