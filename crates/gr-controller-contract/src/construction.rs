//! Unstable curated-package construction SPI; not re-exported by the root.
use super::{
    AbsoluteAxisSurface, AuxiliaryButtonInput, ControllerSurface, CustomInputModule,
    DigitalControlSurface, DpadCluster, DpadHoldBehavior, DpadPresentation, ExtraAxisInput,
    FaceButtonCluster, FaceButtonInput, InputAxisRange, InputControlId, InputScale, InputTopology,
    MotionInput, OutputSurface, RealizationTarget, RealizationValidationStatus, StickInput,
    TargetRestriction, TouchpadActuation, TouchpadInput, TriggerInput, TriggerStack,
};

/// Construction record for `FaceButtonCluster`; outside the application contract.
#[derive(Clone, Copy)]
pub struct FaceButtonClusterSpec {
    pub id: InputControlId,
    pub title: &'static str,
    pub buttons: &'static [FaceButtonInput],
}
impl FaceButtonClusterSpec {
    #[must_use]
    pub const fn build(self) -> FaceButtonCluster {
        FaceButtonCluster {
            id: self.id,
            title: self.title,
            buttons: self.buttons,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: FaceButtonCluster) -> Self {
        Self {
            id: value.id,
            title: value.title,
            buttons: value.buttons,
        }
    }
}

/// Construction record for `DpadCluster`; outside the application contract.
#[derive(Clone, Copy)]
pub struct DpadClusterSpec {
    pub id: InputControlId,
    pub title: &'static str,
    pub presentation: DpadPresentation,
    pub hold_behavior: DpadHoldBehavior,
}
impl DpadClusterSpec {
    #[must_use]
    pub const fn build(self) -> DpadCluster {
        DpadCluster {
            id: self.id,
            title: self.title,
            presentation: self.presentation,
            hold_behavior: self.hold_behavior,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: DpadCluster) -> Self {
        Self {
            id: value.id,
            title: value.title,
            presentation: value.presentation,
            hold_behavior: value.hold_behavior,
        }
    }
}

/// Construction record for `StickInput`; outside the application contract.
#[derive(Clone, Copy)]
pub struct StickInputSpec {
    pub id: InputControlId,
    pub title: &'static str,
    pub x: InputAxisRange,
    pub y: InputAxisRange,
    pub press: Option<AuxiliaryButtonInput>,
    pub capacitive: Option<AuxiliaryButtonInput>,
}
impl StickInputSpec {
    #[must_use]
    pub const fn build(self) -> StickInput {
        StickInput {
            id: self.id,
            title: self.title,
            x: self.x,
            y: self.y,
            press: self.press,
            capacitive: self.capacitive,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: StickInput) -> Self {
        Self {
            id: value.id,
            title: value.title,
            x: value.x,
            y: value.y,
            press: value.press,
            capacitive: value.capacitive,
        }
    }
}

/// Construction record for `TriggerStack`; outside the application contract.
#[derive(Clone, Copy)]
pub struct TriggerStackSpec {
    pub id: InputControlId,
    pub title: &'static str,
    pub controls: &'static [TriggerInput],
}
impl TriggerStackSpec {
    #[must_use]
    pub const fn build(self) -> TriggerStack {
        TriggerStack {
            id: self.id,
            title: self.title,
            controls: self.controls,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: TriggerStack) -> Self {
        Self {
            id: value.id,
            title: value.title,
            controls: value.controls,
        }
    }
}

/// Construction record for `TouchpadInput`; outside the application contract.
#[derive(Clone, Copy)]
pub struct TouchpadInputSpec {
    pub id: InputControlId,
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub contacts: u8,
    pub actuation: TouchpadActuation,
}
impl TouchpadInputSpec {
    #[must_use]
    pub const fn build(self) -> TouchpadInput {
        TouchpadInput {
            id: self.id,
            title: self.title,
            width: self.width,
            height: self.height,
            contacts: self.contacts,
            actuation: self.actuation,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: TouchpadInput) -> Self {
        Self {
            id: value.id,
            title: value.title,
            width: value.width,
            height: value.height,
            contacts: value.contacts,
            actuation: value.actuation,
        }
    }
}

/// Construction record for `MotionInput`; outside the application contract.
#[derive(Clone, Copy)]
pub struct MotionInputSpec {
    pub id: InputControlId,
    pub title: &'static str,
    pub range: InputAxisRange,
    pub gyroscope_scale: [InputScale; 3],
    pub accelerometer_scale: [InputScale; 3],
}
impl MotionInputSpec {
    #[must_use]
    pub const fn build(self) -> MotionInput {
        MotionInput {
            id: self.id,
            title: self.title,
            range: self.range,
            gyroscope_scale: self.gyroscope_scale,
            accelerometer_scale: self.accelerometer_scale,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: MotionInput) -> Self {
        Self {
            id: value.id,
            title: value.title,
            range: value.range,
            gyroscope_scale: value.gyroscope_scale,
            accelerometer_scale: value.accelerometer_scale,
        }
    }
}

/// Construction record for `InputTopology`; outside the application contract.
#[derive(Clone, Copy)]
pub struct InputTopologySpec {
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
impl InputTopologySpec {
    #[must_use]
    pub const fn build(self) -> InputTopology {
        InputTopology {
            auxiliary_buttons: self.auxiliary_buttons,
            face_button_clusters: self.face_button_clusters,
            dpads: self.dpads,
            sticks: self.sticks,
            trigger_stacks: self.trigger_stacks,
            touchpads: self.touchpads,
            motion: self.motion,
            extra_axes: self.extra_axes,
            custom_modules: self.custom_modules,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: InputTopology) -> Self {
        Self {
            auxiliary_buttons: value.auxiliary_buttons,
            face_button_clusters: value.face_button_clusters,
            dpads: value.dpads,
            sticks: value.sticks,
            trigger_stacks: value.trigger_stacks,
            touchpads: value.touchpads,
            motion: value.motion,
            extra_axes: value.extra_axes,
            custom_modules: value.custom_modules,
        }
    }
}

/// Construction record for `ControllerSurface`; outside the application contract.
#[derive(Clone, Copy)]
pub struct ControllerSurfaceSpec {
    pub target: RealizationTarget,
    pub validation_status: RealizationValidationStatus,
    pub digital_controls: &'static [DigitalControlSurface],
    pub axes: &'static [AbsoluteAxisSurface],
    pub outputs: &'static [OutputSurface],
    pub restrictions: &'static [TargetRestriction],
    pub input_topology: &'static InputTopology,
}
impl ControllerSurfaceSpec {
    #[must_use]
    pub const fn build(self) -> ControllerSurface {
        ControllerSurface {
            target: self.target,
            validation_status: self.validation_status,
            digital_controls: self.digital_controls,
            axes: self.axes,
            outputs: self.outputs,
            restrictions: self.restrictions,
            input_topology: self.input_topology,
        }
    }
    #[must_use]
    pub const fn from_descriptor(value: ControllerSurface) -> Self {
        Self {
            target: value.target,
            validation_status: value.validation_status,
            digital_controls: value.digital_controls,
            axes: value.axes,
            outputs: value.outputs,
            restrictions: value.restrictions,
            input_topology: value.input_topology,
        }
    }
}
