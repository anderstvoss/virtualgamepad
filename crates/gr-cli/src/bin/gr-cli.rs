//! `gr-cli` controller-native inspection entrypoint.

use clap::{Parser, Subcommand, ValueEnum};
use gr_controller_contract::{ControllerKind, LinuxTarget};

#[derive(Parser, Debug)]
#[command(name = "gr-cli", version, about = "Curated controller API inspector")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print the fixed curated controller set.
    ListControllers,
    /// Validate one exact controller/target realization pairing.
    ValidateTarget {
        controller: ControllerArg,
        target: TargetArg,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ControllerArg {
    GenericGamepad,
    Xbox360,
    Dualsense,
    SteamController,
}

impl From<ControllerArg> for ControllerKind {
    fn from(value: ControllerArg) -> Self {
        match value {
            ControllerArg::GenericGamepad => Self::GenericGamepad,
            ControllerArg::Xbox360 => Self::Xbox360,
            ControllerArg::Dualsense => Self::DualSense,
            ControllerArg::SteamController => Self::SteamController,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum TargetArg {
    Uinput,
    Uhid,
    UsbTransport,
}

impl From<TargetArg> for LinuxTarget {
    fn from(value: TargetArg) -> Self {
        match value {
            TargetArg::Uinput => Self::Uinput,
            TargetArg::Uhid => Self::Uhid,
            TargetArg::UsbTransport => Self::UsbTransport,
        }
    }
}

fn main() {
    let result = match Cli::parse().command {
        Command::ListControllers => Ok(gr_cli::list_controllers()),
        Command::ValidateTarget { controller, target } => {
            gr_cli::validate_target(controller.into(), target.into())
                .map_err(|error| error.to_string())
        }
    };
    match result {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
