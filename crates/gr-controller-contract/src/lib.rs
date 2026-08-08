#![forbid(unsafe_code)]

//! Controller-semantic contracts with no controller-family or provider logic.

use gr_realization_api::{
    ControllerId, LinuxTarget, ProviderRequirements, RealizationMode, RealizationModeSet,
    RealizationSelection,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceButton {
    North,
    South,
    East,
    West,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DpadDirection {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stick {
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trigger {
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickPosition {
    pub x: i16,
    pub y: i16,
}
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControlError {
    #[error("unsupported control `{control}`")]
    UnsupportedControl { control: &'static str },
    #[error("value {value} for `{control}` exceeds {maximum}")]
    ValueOutOfRange {
        control: &'static str,
        value: u32,
        maximum: u32,
    },
    #[error("operation is unavailable in {selected_mode}; available in {available_in:?}")]
    UnavailableInRealizationMode {
        selected_mode: RealizationMode,
        available_in: RealizationModeSet,
    },
    #[error("controller is closed")]
    Closed,
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CommitError {
    #[error("controller is closed")]
    Closed,
    #[error("backend rejected state: {reason}")]
    Backend { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealizationManifestEntry {
    pub target: LinuxTarget,
    pub mode: RealizationMode,
    pub provider_requirements: ProviderRequirements,
    pub available_features: RealizationModeSet,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealizationManifest {
    entries: &'static [RealizationManifestEntry],
}
impl RealizationManifest {
    #[must_use]
    pub const fn new(entries: &'static [RealizationManifestEntry]) -> Self {
        Self { entries }
    }
    #[must_use]
    pub const fn entries(&self) -> &'static [RealizationManifestEntry] {
        self.entries
    }
}
pub trait RealizationControllerDefinition: Send + Sync + 'static {
    fn controller_id(&self) -> ControllerId;
    fn realization_manifest(&self) -> RealizationManifest;
}
#[allow(clippy::missing_errors_doc)]
pub trait ModeAwareControllerDriver: RealizationControllerDefinition {
    type State: Clone + Send + 'static;
    type Frame: Send + 'static;
    fn neutral_state(&self) -> Self::State;
    fn apply_normalized(
        &self,
        state: &mut Self::State,
        update: ControlUpdate,
    ) -> Result<(), ControlError>;
    fn validate_state(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<(), ControlError>;
    fn encode(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<Self::Frame, ControlError>;
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ManifestError {
    #[error("controller `{controller}` declares no realizations")]
    Empty { controller: ControllerId },
    #[error("target {target} is duplicated for `{controller}`")]
    DuplicateTarget {
        controller: ControllerId,
        target: LinuxTarget,
    },
    #[error("target {target} realizes {actual_mode}, not {mode}")]
    TargetModeMismatch {
        target: LinuxTarget,
        mode: RealizationMode,
        actual_mode: RealizationMode,
    },
    #[error("controller `{controller}` does not support {target}")]
    UnsupportedTarget {
        controller: ControllerId,
        target: LinuxTarget,
    },
}
#[allow(clippy::missing_errors_doc)]
pub fn select_realization(
    definition: &dyn RealizationControllerDefinition,
    target: LinuxTarget,
) -> Result<(RealizationSelection, RealizationManifestEntry), ManifestError> {
    let controller = definition.controller_id();
    let entries = definition.realization_manifest().entries();
    if entries.is_empty() {
        return Err(ManifestError::Empty { controller });
    }
    for (index, entry) in entries.iter().enumerate() {
        if entry.mode != entry.target.mode() {
            return Err(ManifestError::TargetModeMismatch {
                target: entry.target,
                mode: entry.mode,
                actual_mode: entry.target.mode(),
            });
        }
        if entries[..index]
            .iter()
            .any(|previous| previous.target == entry.target)
        {
            return Err(ManifestError::DuplicateTarget {
                controller,
                target: entry.target,
            });
        }
    }
    let entry = entries
        .iter()
        .copied()
        .find(|entry| entry.target == target)
        .ok_or(ManifestError::UnsupportedTarget { controller, target })?;
    Ok((
        RealizationSelection {
            controller,
            target,
            mode: entry.mode,
        },
        entry,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Hardware;
    impl RealizationControllerDefinition for Hardware {
        fn controller_id(&self) -> ControllerId {
            ControllerId::new("test.hardware")
        }
        fn realization_manifest(&self) -> RealizationManifest {
            static ENTRIES: [RealizationManifestEntry; 1] = [RealizationManifestEntry {
                target: LinuxTarget::UsbGadget,
                mode: RealizationMode::HardwareFaithful,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                available_features: RealizationModeSet::singleton(
                    RealizationMode::HardwareFaithful,
                ),
            }];
            RealizationManifest::new(&ENTRIES)
        }
    }
    #[test]
    fn independent_hardware_mode_needs_no_lower_mode() {
        assert!(select_realization(&Hardware, LinuxTarget::UsbGadget).is_ok());
        assert!(matches!(
            select_realization(&Hardware, LinuxTarget::Uhid),
            Err(ManifestError::UnsupportedTarget { .. })
        ));
    }
}
