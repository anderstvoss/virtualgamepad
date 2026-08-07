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

/// Immutable capabilities promised by one explicitly selected provider.
///
/// This is intentionally controller-agnostic: providers describe only their
/// transport surface, while compiled controller definitions describe what
/// they require. The compatibility decision is made once during creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub target: LinuxTarget,
    pub provides_identity: bool,
    pub provides_transport: bool,
    pub provides_reverse_output: bool,
}

/// Validate that a provider can realize a controller's complete declared
/// surface without compatibility fallback.
///
/// # Errors
///
/// Returns [`CreationError::UnsupportedTarget`] when any declared requirement
/// is absent from `provider`.
pub fn validate_realization(
    controller: &dyn ControllerDefinition,
    provider: ProviderCapabilities,
) -> Result<(), CreationError> {
    let requirements = controller.requirements();
    let missing = if requirements.requires_transport && !provider.provides_transport {
        Some("the controller requires transport-level realization")
    } else if requirements.requires_identity && !provider.provides_identity {
        Some("the controller requires an identity-aware realization")
    } else if requirements.requires_reverse_output && !provider.provides_reverse_output {
        Some("the controller requires reverse-output delivery")
    } else {
        None
    };
    missing.map_or(Ok(()), |reason| {
        Err(CreationError::UnsupportedTarget {
            controller: controller.kind(),
            target: provider.target,
            reason: reason.to_string(),
        })
    })
}

/// Static metadata used by the controller runtime before any session opens.
pub trait ControllerDefinition: Send + Sync + 'static {
    fn kind(&self) -> ControllerKind;
    fn requirements(&self) -> RealizationRequirements;
}

/// A compiled controller implementation that owns its typed state and report
/// encoder. The generic runtime invokes this contract without knowing a
/// controller family or report format.
pub trait ControllerDriver: ControllerDefinition {
    type State: Clone + Send + 'static;
    type Frame: Send + 'static;

    fn neutral_state(&self) -> Self::State;

    /// Apply a normalized update to a typed state.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] when the update is unsupported or invalid for
    /// this controller.
    fn apply_normalized(
        &self,
        state: &mut Self::State,
        update: ControlUpdate,
    ) -> Result<(), ControlError>;

    /// Validate a complete controller state before it replaces the last valid
    /// state or reaches a provider.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] when any controller-specific invariant is
    /// violated.
    fn validate_state(&self, state: &Self::State) -> Result<(), ControlError>;

    /// Encode the complete current state into a provider-ready typed frame.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] if the state cannot be represented by this
    /// controller's report contract.
    fn encode(&self, state: &Self::State) -> Result<Self::Frame, ControlError>;
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
    #[error("{control} value {value} is outside the supported range 0..={maximum}")]
    ValueOutOfRange {
        control: &'static str,
        value: u32,
        maximum: u32,
    },
    #[error("{control} index {index} is invalid; expected an index below {exclusive_maximum}")]
    InvalidIndex {
        control: &'static str,
        index: usize,
        exclusive_maximum: usize,
    },
    #[error("controller is closed")]
    Closed,
}

/// Failure to create an exact controller realization.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CreationError {
    #[error("{target} support is not compiled in; enable Cargo feature `{feature}`")]
    ProviderNotCompiled {
        target: LinuxTarget,
        feature: &'static str,
    },
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

/// Failure to register a reverse-output callback.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SubscriptionError {
    #[error("controller is closed")]
    Closed,
    #[error("output subscription capacity {capacity} has been reached")]
    Capacity { capacity: usize },
    #[error("output subscription state is unavailable")]
    Unavailable,
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
    use super::{
        ControllerDefinition, ControllerKind, LinuxTarget, ProviderCapabilities,
        RealizationRequirements, validate_realization,
    };

    struct IdentityController;
    impl ControllerDefinition for IdentityController {
        fn kind(&self) -> ControllerKind {
            ControllerKind::DualSense
        }
        fn requirements(&self) -> RealizationRequirements {
            RealizationRequirements {
                requires_identity: true,
                requires_transport: false,
                requires_reverse_output: true,
            }
        }
    }

    #[test]
    fn stable_display_names_are_human_readable() {
        assert_eq!(ControllerKind::DualSense.to_string(), "DualSense");
        assert_eq!(LinuxTarget::Uhid.to_string(), "linux UHID");
    }

    #[test]
    fn realization_validation_rejects_missing_declared_surface() {
        let error = validate_realization(
            &IdentityController,
            ProviderCapabilities {
                target: LinuxTarget::Uinput,
                provides_identity: false,
                provides_transport: false,
                provides_reverse_output: true,
            },
        )
        .expect_err("uinput lacks identity");
        assert!(error.to_string().contains("identity-aware"));
    }
}
