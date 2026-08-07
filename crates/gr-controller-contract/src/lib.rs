#![forbid(unsafe_code)]

//! Controller-agnostic public contracts.
//!
//! This crate deliberately contains no controller-family identifiers, report
//! formats, or provider I/O. A curated controller implementation supplies
//! those details through [`ControllerDefinition`].

use std::fmt;
use thiserror::Error;

/// A curated controller family known to the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerKind {
    GenericGamepad,
    Xbox360,
    DualSense,
    SteamController,
}

impl fmt::Display for ControllerKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::GenericGamepad => "generic gamepad",
            Self::Xbox360 => "Xbox 360",
            Self::DualSense => "DualSense",
            Self::SteamController => "Steam Controller",
        })
    }
}

/// The Linux realization target selected explicitly by an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinuxTarget {
    Uinput,
    Uhid,
    UsbTransport,
}

impl fmt::Display for LinuxTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Uinput => "linux uinput",
            Self::Uhid => "linux UHID",
            Self::UsbTransport => "linux USB transport",
        })
    }
}

/// A normalized face-button position, independent of printed device labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceButton {
    North,
    South,
    East,
    West,
}

/// A normalized D-pad direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DpadDirection {
    Up,
    Down,
    Left,
    Right,
}

/// A normalized stick identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stick {
    Left,
    Right,
}

/// A normalized analog-trigger identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    Left,
    Right,
}

/// A two-dimensional signed stick position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickPosition {
    pub x: i16,
    pub y: i16,
}

/// A normalized control update suitable for heterogeneous controller handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlUpdate {
    FaceButton {
        button: FaceButton,
        pressed: bool,
    },
    Dpad {
        direction: DpadDirection,
        pressed: bool,
    },
    Stick {
        stick: Stick,
        position: StickPosition,
    },
    Trigger {
        trigger: Trigger,
        value: u16,
    },
}

/// Static requirements a controller exposes to a Linux realization target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealizationRequirements {
    pub requires_identity: bool,
    pub requires_transport: bool,
    pub requires_reverse_output: bool,
}

/// Static metadata used by the controller runtime before any session opens.
pub trait ControllerDefinition: Send + Sync + 'static {
    fn kind(&self) -> ControllerKind;
    fn requirements(&self) -> RealizationRequirements;
}

/// Errors caused by an invalid or incompatible control update.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ControlError {
    #[error("controller `{controller}` does not support normalized control `{control}")]
    UnsupportedControl {
        controller: ControllerKind,
        control: &'static str,
    },
    #[error("native control `{control}` does not belong to controller `{controller}")]
    UnsupportedNativeControl {
        controller: ControllerKind,
        control: &'static str,
    },
    #[error("trigger value {value} is outside the supported range 0..={maximum}")]
    OutOfRange { value: u16, maximum: u16 },
    #[error("controller is closed")]
    Closed,
}

/// Failure to create an exact controller realization.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CreationError {
    #[error("{controller} cannot be realized through {target}: {reason}")]
    UnsupportedTarget {
        controller: ControllerKind,
        target: LinuxTarget,
        reason: String,
    },
    #[error("failed to open {target} for {controller}: {reason}")]
    ProviderOpen {
        controller: ControllerKind,
        target: LinuxTarget,
        reason: String,
    },
}

/// Failure to submit the latest valid controller state.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CommitError {
    #[error("controller is closed")]
    Closed,
    #[error("backend did not accept the controller state: {reason}")]
    Backend { reason: String },
}

#[cfg(test)]
mod tests {
    use super::{ControllerKind, LinuxTarget};

    #[test]
    fn stable_display_names_are_human_readable() {
        assert_eq!(ControllerKind::DualSense.to_string(), "DualSense");
        assert_eq!(LinuxTarget::Uhid.to_string(), "linux UHID");
    }
}
