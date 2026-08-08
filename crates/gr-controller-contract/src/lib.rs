#![forbid(unsafe_code)]

//! Controller-semantic contracts with no controller-family or provider logic.

use gr_audio_contract::AudioSidecarRequirement;
use gr_realization_api::{
    ControllerId, DeploymentTarget, ProviderRequirements, RealizationSelection, RealizationTarget,
    RealizationTargetSet, TransportValidationTarget,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitalControlUpdate {
    FaceButton {
        button: FaceButton,
        pressed: bool,
    },
    Dpad {
        direction: DpadDirection,
        pressed: bool,
    },
}

/// Read-only Linux presentation for one digital controller input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DigitalControlSurface {
    pub control: &'static str,
    pub event_code: u16,
}

/// Read-only Linux absolute-axis presentation. Numeric values are target
/// presentation values, never a controller's semantic state domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbsoluteAxisSurface {
    pub control: &'static str,
    pub event_code: u16,
    pub minimum: i32,
    pub maximum: i32,
    pub neutral: i32,
    pub flat: i32,
}

/// Read-only output channel advertised by a prepared controller target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputSurface {
    pub name: &'static str,
    pub event_type: u16,
    pub event_code: u16,
}

/// Target-specific limitation documented by the owning controller package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetRestriction {
    pub feature: &'static str,
    pub reason: &'static str,
}

/// Common immutable portion of a concrete controller's target presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerSurface {
    pub target: RealizationTarget,
    pub digital_controls: &'static [DigitalControlSurface],
    pub axes: &'static [AbsoluteAxisSurface],
    pub outputs: &'static [OutputSurface],
    pub restrictions: &'static [TargetRestriction],
}

/// Implemented by concrete typed controller-surface descriptors.
pub trait ControllerSurfaceInfo {
    fn common_surface(&self) -> &ControllerSurface;
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
    #[error("operation is unavailable in {selected_target}; available in {available_in:?}")]
    UnavailableInRealization {
        selected_target: RealizationTarget,
        available_in: RealizationTargetSet,
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
    pub target: RealizationTarget,
    pub provider_requirements: ProviderRequirements,
    /// Optional host-audio stream contract independent of controller reports.
    pub audio_sidecar: Option<AudioSidecarRequirement>,
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

/// An exact manifest entry validated for one controller and Linux target.
///
/// Controller packages keep typed feature availability outside this generic
/// value. The prepared realization binds only the provider-neutral selection
/// and requirements needed before host I/O begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedRealization {
    selection: RealizationSelection,
    entry: RealizationManifestEntry,
}

impl PreparedRealization {
    #[must_use]
    pub const fn selection(&self) -> RealizationSelection {
        self.selection
    }

    #[must_use]
    pub const fn entry(&self) -> RealizationManifestEntry {
        self.entry
    }
}
#[allow(clippy::missing_errors_doc)]
pub trait TargetAwareControllerDriver: RealizationControllerDefinition {
    type State: Clone + Send + 'static;
    type Frame: Send + 'static;
    fn neutral_state(&self) -> Self::State;
    fn apply_digital(
        &self,
        state: &mut Self::State,
        update: DigitalControlUpdate,
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
        target: RealizationTarget,
    },
    #[error("controller `{controller}` has an invalid audio sidecar for {target}")]
    InvalidAudioSidecar {
        controller: ControllerId,
        target: RealizationTarget,
    },
    #[error("controller `{controller}` does not support {target}")]
    UnsupportedTarget {
        controller: ControllerId,
        target: RealizationTarget,
    },
    #[error(
        "prepared realization belongs to controller `{prepared_controller}`, not `{driver_controller}`"
    )]
    ControllerMismatch {
        prepared_controller: ControllerId,
        driver_controller: ControllerId,
    },
}
/// Prepare a realization for ordinary application deployment.
#[allow(clippy::missing_errors_doc)]
pub fn prepare_deployment_realization(
    definition: &dyn RealizationControllerDefinition,
    target: DeploymentTarget,
) -> Result<PreparedRealization, ManifestError> {
    prepare_realization(definition, target.realization_target())
}

/// Prepare a realization for explicit hardware validation.
#[allow(clippy::missing_errors_doc)]
pub fn prepare_transport_validation_realization(
    definition: &dyn RealizationControllerDefinition,
    target: TransportValidationTarget,
) -> Result<PreparedRealization, ManifestError> {
    prepare_realization(definition, target.realization_target())
}

#[allow(clippy::missing_errors_doc)]
pub fn prepare_realization(
    definition: &dyn RealizationControllerDefinition,
    target: RealizationTarget,
) -> Result<PreparedRealization, ManifestError> {
    let controller = definition.controller_id();
    let entries = definition.realization_manifest().entries();
    if entries.is_empty() {
        return Err(ManifestError::Empty { controller });
    }
    for (index, entry) in entries.iter().enumerate() {
        if entry
            .audio_sidecar
            .is_some_and(|sidecar| !sidecar.is_valid())
        {
            return Err(ManifestError::InvalidAudioSidecar {
                controller,
                target: entry.target,
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
    Ok(PreparedRealization {
        selection: RealizationSelection { controller, target },
        entry,
    })
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
                target: RealizationTarget::UsbTransportValidation,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                audio_sidecar: None,
            }];
            RealizationManifest::new(&ENTRIES)
        }
    }
    #[test]
    fn independent_hardware_mode_needs_no_lower_mode() {
        assert!(prepare_realization(&Hardware, RealizationTarget::UsbTransportValidation).is_ok());
        assert!(matches!(
            prepare_realization(&Hardware, RealizationTarget::Hid),
            Err(ManifestError::UnsupportedTarget { .. })
        ));
    }

    #[test]
    fn explicit_usb_validation_prepares_a_hardware_only_controller() {
        let prepared = prepare_transport_validation_realization(
            &Hardware,
            TransportValidationTarget::UsbGadget,
        )
        .expect("the explicit USB API admits its declared target");
        assert_eq!(
            prepared.selection().target,
            RealizationTarget::UsbTransportValidation
        );
    }

    #[test]
    fn invalid_audio_sidecar_prevents_preparation() {
        struct InvalidAudio;
        impl RealizationControllerDefinition for InvalidAudio {
            fn controller_id(&self) -> ControllerId {
                ControllerId::new("test.invalid-audio")
            }
            fn realization_manifest(&self) -> RealizationManifest {
                static ENTRIES: [RealizationManifestEntry; 1] = [RealizationManifestEntry {
                    target: RealizationTarget::Evdev,
                    provider_requirements: ProviderRequirements {
                        requires_reverse_output: false,
                    },
                    audio_sidecar: Some(gr_audio_contract::AudioSidecarRequirement {
                        streams: &[],
                    }),
                }];
                RealizationManifest::new(&ENTRIES)
            }
        }
        assert!(matches!(
            prepare_realization(&InvalidAudio, RealizationTarget::Evdev),
            Err(ManifestError::InvalidAudioSidecar { .. })
        ));
    }
}
