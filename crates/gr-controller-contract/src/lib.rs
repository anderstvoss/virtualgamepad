#![forbid(unsafe_code)]

//! Controller-semantic contracts with no controller-family or provider logic.

use gr_audio_contract::AudioSidecarRequirement;
use gr_realization_api::{
    ControllerId, ProviderRequirements, RealizationSelection, RealizationTarget,
    RealizationTargetSet,
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

/// Stable semantic identifier used to connect an input topology to a typed
/// controller adapter. It describes a physical control, not a Linux event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputControlId(&'static str);

impl InputControlId {
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputAxisRange {
    pub minimum: i32,
    pub maximum: i32,
    pub neutral: i32,
}

impl InputAxisRange {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.minimum < self.maximum && self.neutral >= self.minimum && self.neutral <= self.maximum
    }
}

/// Abstract placement within a cluster grid. Renderers choose the concrete
/// dimensions; cardinal defaults use a three-by-three grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClusterPlacement {
    pub column: i8,
    pub row: i8,
}

impl ClusterPlacement {
    pub const NORTH: Self = Self { column: 1, row: 0 };
    pub const SOUTH: Self = Self { column: 1, row: 2 };
    pub const EAST: Self = Self { column: 2, row: 1 };
    pub const WEST: Self = Self { column: 0, row: 1 };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceButtonInput {
    pub button: FaceButton,
    pub label: &'static str,
    pub placement: ClusterPlacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceButtonCluster {
    pub id: InputControlId,
    pub title: &'static str,
    pub buttons: &'static [FaceButtonInput],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpadPresentation {
    IndependentButtons,
    SnappingAxis,
}

/// How a held four-button cluster resolves simultaneous directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpadHoldBehavior {
    /// Model a physical hat: retain at most one vertical and one horizontal direction.
    AdjacentPair,
    /// Model independently wired controls: retain every pressed direction.
    IndependentButtons,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DpadCluster {
    pub id: InputControlId,
    pub title: &'static str,
    pub presentation: DpadPresentation,
    pub hold_behavior: DpadHoldBehavior,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuxiliaryButtonInput {
    pub id: InputControlId,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickInput {
    pub id: InputControlId,
    pub title: &'static str,
    pub x: InputAxisRange,
    pub y: InputAxisRange,
    pub press: Option<AuxiliaryButtonInput>,
    pub capacitive: Option<AuxiliaryButtonInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerInputKind {
    Button {
        id: InputControlId,
    },
    Axis {
        id: InputControlId,
        range: InputAxisRange,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerInput {
    pub label: &'static str,
    pub kind: TriggerInputKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerStack {
    pub id: InputControlId,
    pub title: &'static str,
    pub controls: &'static [TriggerInput],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchpadActuation {
    None,
    Button(AuxiliaryButtonInput),
    Deflection {
        id: InputControlId,
        range: InputAxisRange,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TouchpadInput {
    pub id: InputControlId,
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub contacts: u8,
    pub actuation: TouchpadActuation,
}

/// Exact, deterministic scaling applied by an input adapter. A denominator of
/// zero is invalid topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputScale {
    pub numerator: i32,
    pub denominator: u32,
}

impl InputScale {
    pub const IDENTITY: Self = Self {
        numerator: 1,
        denominator: 1,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionInput {
    pub id: InputControlId,
    pub title: &'static str,
    pub range: InputAxisRange,
    pub gyroscope_scale: [InputScale; 3],
    pub accelerometer_scale: [InputScale; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraAxisInput {
    OneDimensional {
        id: InputControlId,
        title: &'static str,
        range: InputAxisRange,
    },
    TwoDimensional {
        id: InputControlId,
        title: &'static str,
        x: InputAxisRange,
        y: InputAxisRange,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustomInputModule {
    pub id: InputControlId,
    pub title: &'static str,
}

/// GUI-agnostic semantic input topology for one target surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputTopology {
    pub auxiliary_buttons: &'static [AuxiliaryButtonInput],
    pub face_button_clusters: &'static [FaceButtonCluster],
    pub dpads: &'static [DpadCluster],
    pub sticks: &'static [StickInput],
    pub trigger_stacks: &'static [TriggerStack],
    pub touchpads: &'static [TouchpadInput],
    pub motion: &'static [MotionInput],
    pub extra_axes: &'static [ExtraAxisInput],
    pub custom_modules: &'static [CustomInputModule],
}

impl InputTopology {
    pub const EMPTY: Self = Self {
        auxiliary_buttons: &[],
        face_button_clusters: &[],
        dpads: &[],
        sticks: &[],
        trigger_stacks: &[],
        touchpads: &[],
        motion: &[],
        extra_axes: &[],
        custom_modules: &[],
    };

    #[allow(clippy::missing_errors_doc)]
    pub fn validate(&self) -> Result<(), InputTopologyError> {
        let mut ids = Vec::new();
        let mut add_id = |id: InputControlId| {
            if id.as_str().is_empty() {
                return Err(InputTopologyError::EmptyIdentifier);
            }
            if ids.contains(&id) {
                return Err(InputTopologyError::DuplicateIdentifier(id));
            }
            ids.push(id);
            Ok(())
        };

        for button in self.auxiliary_buttons {
            add_id(button.id)?;
        }
        for cluster in self.face_button_clusters {
            add_id(cluster.id)?;
            for (index, button) in cluster.buttons.iter().enumerate() {
                if button.placement.column < 0 || button.placement.row < 0 {
                    return Err(InputTopologyError::InvalidPlacement(cluster.id));
                }
                if cluster.buttons[..index]
                    .iter()
                    .any(|other| other.placement == button.placement)
                {
                    return Err(InputTopologyError::DuplicatePlacement(cluster.id));
                }
            }
        }
        for dpad in self.dpads {
            add_id(dpad.id)?;
        }
        for stick in self.sticks {
            add_id(stick.id)?;
            validate_range(stick.id, stick.x)?;
            validate_range(stick.id, stick.y)?;
            if let Some(press) = stick.press {
                add_id(press.id)?;
            }
            if let Some(capacitive) = stick.capacitive {
                add_id(capacitive.id)?;
            }
        }
        for stack in self.trigger_stacks {
            add_id(stack.id)?;
            for control in stack.controls {
                match control.kind {
                    TriggerInputKind::Button { id } => add_id(id)?,
                    TriggerInputKind::Axis { id, range } => {
                        add_id(id)?;
                        validate_range(id, range)?;
                    }
                }
            }
        }
        for touchpad in self.touchpads {
            add_id(touchpad.id)?;
            if touchpad.width == 0 || touchpad.height == 0 || touchpad.contacts == 0 {
                return Err(InputTopologyError::InvalidTouchpad(touchpad.id));
            }
            match touchpad.actuation {
                TouchpadActuation::None => {}
                TouchpadActuation::Button(button) => add_id(button.id)?,
                TouchpadActuation::Deflection { id, range } => {
                    add_id(id)?;
                    validate_range(id, range)?;
                }
            }
        }
        for motion in self.motion {
            add_id(motion.id)?;
            validate_range(motion.id, motion.range)?;
            if motion
                .gyroscope_scale
                .iter()
                .chain(&motion.accelerometer_scale)
                .any(|scale| scale.denominator == 0)
            {
                return Err(InputTopologyError::InvalidScale(motion.id));
            }
        }
        for axis in self.extra_axes {
            match *axis {
                ExtraAxisInput::OneDimensional { id, range, .. } => {
                    add_id(id)?;
                    validate_range(id, range)?;
                }
                ExtraAxisInput::TwoDimensional { id, x, y, .. } => {
                    add_id(id)?;
                    validate_range(id, x)?;
                    validate_range(id, y)?;
                }
            }
        }
        for module in self.custom_modules {
            add_id(module.id)?;
        }
        Ok(())
    }
}

fn validate_range(id: InputControlId, range: InputAxisRange) -> Result<(), InputTopologyError> {
    range
        .is_valid()
        .then_some(())
        .ok_or(InputTopologyError::InvalidAxisRange(id))
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum InputTopologyError {
    #[error("input topology contains an empty identifier")]
    EmptyIdentifier,
    #[error("input topology contains duplicate identifier `{0:?}`")]
    DuplicateIdentifier(InputControlId),
    #[error("face-button cluster `{0:?}` contains duplicate placement")]
    DuplicatePlacement(InputControlId),
    #[error("face-button cluster `{0:?}` contains a negative placement")]
    InvalidPlacement(InputControlId),
    #[error("input `{0:?}` has an invalid axis range")]
    InvalidAxisRange(InputControlId),
    #[error("touchpad `{0:?}` has invalid dimensions or contact count")]
    InvalidTouchpad(InputControlId),
    #[error("motion input `{0:?}` has a zero scaling denominator")]
    InvalidScale(InputControlId),
}

#[cfg(test)]
mod input_topology_tests {
    use super::*;

    const AXIS: InputAxisRange = InputAxisRange {
        minimum: -10,
        maximum: 10,
        neutral: 0,
    };

    #[test]
    fn empty_topology_is_valid() {
        assert_eq!(InputTopology::EMPTY.validate(), Ok(()));
    }

    #[test]
    fn topology_rejects_duplicate_ids_and_placements() {
        static DUPLICATE_BUTTONS: [AuxiliaryButtonInput; 2] = [
            AuxiliaryButtonInput {
                id: InputControlId::new("same"),
                label: "One",
            },
            AuxiliaryButtonInput {
                id: InputControlId::new("same"),
                label: "Two",
            },
        ];
        static DUPLICATE_FACE_BUTTONS: [FaceButtonInput; 2] = [
            FaceButtonInput {
                button: FaceButton::North,
                label: "N",
                placement: ClusterPlacement::NORTH,
            },
            FaceButtonInput {
                button: FaceButton::South,
                label: "S",
                placement: ClusterPlacement::NORTH,
            },
        ];
        static DUPLICATE_FACE_CLUSTERS: [FaceButtonCluster; 1] = [FaceButtonCluster {
            id: InputControlId::new("face"),
            title: "Face",
            buttons: &DUPLICATE_FACE_BUTTONS,
        }];
        static NEGATIVE_FACE_BUTTONS: [FaceButtonInput; 1] = [FaceButtonInput {
            button: FaceButton::North,
            label: "N",
            placement: ClusterPlacement { column: -1, row: 0 },
        }];
        static NEGATIVE_FACE_CLUSTERS: [FaceButtonCluster; 1] = [FaceButtonCluster {
            id: InputControlId::new("negative-face"),
            title: "Face",
            buttons: &NEGATIVE_FACE_BUTTONS,
        }];
        let duplicate_ids = InputTopology {
            auxiliary_buttons: &DUPLICATE_BUTTONS,
            ..InputTopology::EMPTY
        };
        assert_eq!(
            duplicate_ids.validate(),
            Err(InputTopologyError::DuplicateIdentifier(
                InputControlId::new("same")
            ))
        );
        let duplicate_placement = InputTopology {
            face_button_clusters: &DUPLICATE_FACE_CLUSTERS,
            ..InputTopology::EMPTY
        };
        assert_eq!(
            duplicate_placement.validate(),
            Err(InputTopologyError::DuplicatePlacement(InputControlId::new(
                "face"
            )))
        );
        let negative_placement = InputTopology {
            face_button_clusters: &NEGATIVE_FACE_CLUSTERS,
            ..InputTopology::EMPTY
        };
        assert_eq!(
            negative_placement.validate(),
            Err(InputTopologyError::InvalidPlacement(InputControlId::new(
                "negative-face"
            )))
        );
    }

    #[test]
    fn topology_validates_ranges_touchpads_and_scales() {
        static INVALID_AXES: [ExtraAxisInput; 1] = [ExtraAxisInput::OneDimensional {
            id: InputControlId::new("axis"),
            title: "Axis",
            range: InputAxisRange {
                minimum: 1,
                maximum: 1,
                neutral: 1,
            },
        }];
        static INVALID_TOUCHPADS: [TouchpadInput; 1] = [TouchpadInput {
            id: InputControlId::new("touch"),
            title: "Touch",
            width: 0,
            height: 10,
            contacts: 1,
            actuation: TouchpadActuation::None,
        }];
        static INVALID_MOTION: [MotionInput; 1] = [MotionInput {
            id: InputControlId::new("motion"),
            title: "Motion",
            range: AXIS,
            gyroscope_scale: [InputScale::IDENTITY; 3],
            accelerometer_scale: [
                InputScale {
                    numerator: 1,
                    denominator: 0,
                },
                InputScale::IDENTITY,
                InputScale::IDENTITY,
            ],
        }];
        let invalid_axis = InputTopology {
            extra_axes: &INVALID_AXES,
            ..InputTopology::EMPTY
        };
        assert!(matches!(
            invalid_axis.validate(),
            Err(InputTopologyError::InvalidAxisRange(_))
        ));
        let invalid_touch = InputTopology {
            touchpads: &INVALID_TOUCHPADS,
            ..InputTopology::EMPTY
        };
        assert!(matches!(
            invalid_touch.validate(),
            Err(InputTopologyError::InvalidTouchpad(_))
        ));
        let invalid_scale = InputTopology {
            motion: &INVALID_MOTION,
            ..InputTopology::EMPTY
        };
        assert!(matches!(
            invalid_scale.validate(),
            Err(InputTopologyError::InvalidScale(_))
        ));
    }
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

/// Confidence level for a controller's selected host presentation.
///
/// A realization is never promoted to physical fidelity merely because it
/// advertises a familiar identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RealizationValidationStatus {
    ResearchBacked,
    HostValidated,
    PhysicallyValidated,
}

/// Common immutable portion of a concrete controller's target presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerSurface {
    pub target: RealizationTarget,
    pub validation_status: RealizationValidationStatus,
    pub digital_controls: &'static [DigitalControlSurface],
    pub axes: &'static [AbsoluteAxisSurface],
    pub outputs: &'static [OutputSurface],
    pub restrictions: &'static [TargetRestriction],
    pub input_topology: &'static InputTopology,
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
#[non_exhaustive]
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
                target: RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
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
        assert!(prepare_realization(&Hardware, RealizationTarget::LINUX_DUMMY_HCD_USB_HID).is_ok());
        assert!(matches!(
            prepare_realization(&Hardware, RealizationTarget::LINUX_UHID_USB),
            Err(ManifestError::UnsupportedTarget { .. })
        ));
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
                    target: RealizationTarget::LINUX_UINPUT,
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
            prepare_realization(&InvalidAudio, RealizationTarget::LINUX_UINPUT),
            Err(ManifestError::InvalidAudioSidecar { .. })
        ));
    }
}
