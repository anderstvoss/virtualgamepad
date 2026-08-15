//! Generic gamepad with an intentionally generic, but still native, state.

use crate::{CreationOptions, common};
use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, DpadDirection, FaceButton, OutputSurface,
    RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
    RealizationValidationStatus, TargetAwareControllerDriver, TargetRestriction,
};
use gr_controller_runtime::ControllerRuntime;
use gr_realization_api::{
    ControllerId, DeploymentTarget, EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization,
    NativeDeviceIdentity, NativeEvdevRealization, ProviderError, ProviderFrame,
    ProviderRequirements, RawReverseEvent, RealizationSelection, RealizationTarget,
};

const FACE_CODES: [u16; 4] = [304, 305, 307, 308];
const DPAD_X: u16 = 16;
const DPAD_Y: u16 = 17;

/// Native signed stick-axis value for this generic gamepad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericGamepadAxis(i16);
impl GenericGamepadAxis {
    #[must_use]
    pub const fn raw(self) -> i16 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: i16) -> Self {
        Self(raw)
    }
}

/// Native trigger value for this generic gamepad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericGamepadTrigger(u8);
impl GenericGamepadTrigger {
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
}

/// Controller-native generic-gamepad controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericGamepadControl {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    Select,
    Start,
    Guide,
    LeftStickPress,
    RightStickPress,
}

/// Complete semantic state for the Generic Gamepad package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericGamepadState {
    face: [bool; 4],
    dpad: [bool; 4],
    left: (GenericGamepadAxis, GenericGamepadAxis),
    right: (GenericGamepadAxis, GenericGamepadAxis),
    triggers: (GenericGamepadTrigger, GenericGamepadTrigger),
    buttons: [bool; 7],
}
impl Default for GenericGamepadState {
    fn default() -> Self {
        Self {
            face: [false; 4],
            dpad: [false; 4],
            left: (GenericGamepadAxis(0), GenericGamepadAxis(0)),
            right: (GenericGamepadAxis(0), GenericGamepadAxis(0)),
            triggers: (GenericGamepadTrigger(0), GenericGamepadTrigger(0)),
            buttons: [false; 7],
        }
    }
}
impl GenericGamepadState {
    #[must_use]
    pub const fn left_stick(&self) -> (GenericGamepadAxis, GenericGamepadAxis) {
        self.left
    }
    #[must_use]
    pub const fn right_stick(&self) -> (GenericGamepadAxis, GenericGamepadAxis) {
        self.right
    }
    #[must_use]
    pub const fn triggers(&self) -> (GenericGamepadTrigger, GenericGamepadTrigger) {
        self.triggers
    }
    #[must_use]
    pub const fn face_pressed(&self, button: FaceButton) -> bool {
        self.face[common::face_index(button)]
    }
    #[must_use]
    pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool {
        self.dpad[common::dpad_index(direction)]
    }
    fn set_native(&mut self, control: GenericGamepadControl, pressed: bool) {
        match control {
            GenericGamepadControl::South => self.face[0] = pressed,
            GenericGamepadControl::East => self.face[1] = pressed,
            GenericGamepadControl::West => self.face[2] = pressed,
            GenericGamepadControl::North => self.face[3] = pressed,
            GenericGamepadControl::LeftShoulder => self.buttons[0] = pressed,
            GenericGamepadControl::RightShoulder => self.buttons[1] = pressed,
            GenericGamepadControl::Select => self.buttons[2] = pressed,
            GenericGamepadControl::Start => self.buttons[3] = pressed,
            GenericGamepadControl::Guide => self.buttons[4] = pressed,
            GenericGamepadControl::LeftStickPress => self.buttons[5] = pressed,
            GenericGamepadControl::RightStickPress => self.buttons[6] = pressed,
        }
    }
}

/// Generic-gamepad evdev target presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericGamepadSurface {
    common: ControllerSurface,
}
impl ControllerSurfaceInfo for GenericGamepadSurface {
    fn common_surface(&self) -> &ControllerSurface {
        &self.common
    }
}
impl GenericGamepadSurface {
    #[must_use]
    pub const fn common(&self) -> &ControllerSurface {
        &self.common
    }
}

static GENERIC_DIGITAL: [DigitalControlSurface; 11] = [
    DigitalControlSurface {
        control: "face-south",
        event_code: 304,
    },
    DigitalControlSurface {
        control: "face-east",
        event_code: 305,
    },
    DigitalControlSurface {
        control: "face-west",
        event_code: 307,
    },
    DigitalControlSurface {
        control: "face-north",
        event_code: 308,
    },
    DigitalControlSurface {
        control: "left-shoulder",
        event_code: 310,
    },
    DigitalControlSurface {
        control: "right-shoulder",
        event_code: 311,
    },
    DigitalControlSurface {
        control: "select",
        event_code: 314,
    },
    DigitalControlSurface {
        control: "start",
        event_code: 315,
    },
    DigitalControlSurface {
        control: "guide",
        event_code: 316,
    },
    DigitalControlSurface {
        control: "left-stick-press",
        event_code: 317,
    },
    DigitalControlSurface {
        control: "right-stick-press",
        event_code: 318,
    },
];
static GENERIC_AXES: [AbsoluteAxisSurface; 8] = [
    AbsoluteAxisSurface {
        control: "left-stick-x",
        event_code: 0,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "left-stick-y",
        event_code: 1,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "left-trigger",
        event_code: 2,
        minimum: 0,
        maximum: 255,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-stick-x",
        event_code: 3,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-stick-y",
        event_code: 4,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-trigger",
        event_code: 5,
        minimum: 0,
        maximum: 255,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "dpad-x",
        event_code: DPAD_X,
        minimum: -1,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "dpad-y",
        event_code: DPAD_Y,
        minimum: -1,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
];
static GENERIC_OUTPUTS: [OutputSurface; 1] = [OutputSurface {
    name: "conventional-rumble",
    event_type: 21,
    event_code: 80,
}];
static GENERIC_HID_OUTPUTS: [OutputSurface; 0] = [];
static GENERIC_RESTRICTIONS: [TargetRestriction; 1] = [TargetRestriction {
    feature: "controller-specific sensors",
    reason: "the generic gamepad package declares no sensor surface",
}];
static GENERIC_SURFACE: GenericGamepadSurface = GenericGamepadSurface {
    common: ControllerSurface {
        target: RealizationTarget::Evdev,
        validation_status: RealizationValidationStatus::HostValidated,
        digital_controls: &GENERIC_DIGITAL,
        axes: &GENERIC_AXES,
        outputs: &GENERIC_OUTPUTS,
        restrictions: &GENERIC_RESTRICTIONS,
    },
};
static GENERIC_HID_SURFACE: GenericGamepadSurface = GenericGamepadSurface {
    common: ControllerSurface {
        target: RealizationTarget::Hid,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &GENERIC_DIGITAL,
        axes: &GENERIC_AXES,
        outputs: &GENERIC_HID_OUTPUTS,
        restrictions: &GENERIC_RESTRICTIONS,
    },
};

pub struct GenericGamepadDefinition;
impl RealizationControllerDefinition for GenericGamepadDefinition {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("virtualgamepad.generic")
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static ENTRIES: [RealizationManifestEntry; 2] = [
            RealizationManifestEntry {
                target: RealizationTarget::Evdev,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::Hid,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: false,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(&ENTRIES)
    }
}
impl TargetAwareControllerDriver for GenericGamepadDefinition {
    type State = GenericGamepadState;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        GenericGamepadState::default()
    }
    fn apply_digital(
        &self,
        state: &mut Self::State,
        update: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        match update {
            DigitalControlUpdate::FaceButton { button, pressed } => {
                state.face[common::face_index(button)] = pressed;
            }
            DigitalControlUpdate::Dpad { direction, pressed } => {
                state.dpad[common::dpad_index(direction)] = pressed;
            }
        }
        Ok(())
    }
    fn validate_state(
        &self,
        selection: RealizationSelection,
        _: &Self::State,
    ) -> Result<(), ControlError> {
        if matches!(
            selection.target,
            RealizationTarget::Evdev | RealizationTarget::Hid
        ) {
            Ok(())
        } else {
            Err(common::unavailable(selection.target))
        }
    }
    fn encode(
        &self,
        selection: RealizationSelection,
        state: &Self::State,
    ) -> Result<Self::Frame, ControlError> {
        if selection.target == RealizationTarget::Hid {
            let byte = |value: i16| u8::try_from((i32::from(value) + 32_768) >> 8).unwrap_or(0);
            return Ok(common::hid_gamepad_frame(
                state.face,
                state.dpad,
                &state.buttons,
                [
                    byte(state.left.0.raw()),
                    byte(state.left.1.raw()),
                    byte(state.right.0.raw()),
                    byte(state.right.1.raw()),
                    state.triggers.0.raw(),
                    state.triggers.1.raw(),
                ],
            ));
        }
        let mut events = Vec::new();
        for (code, pressed) in FACE_CODES.into_iter().zip(state.face) {
            events.push(EvdevEvent {
                event_type: common::EV_KEY,
                code,
                value: i32::from(pressed),
            });
        }
        for (code, pressed) in [310, 311, 314, 315, 316, 317, 318]
            .into_iter()
            .zip(state.buttons)
        {
            events.push(EvdevEvent {
                event_type: common::EV_KEY,
                code,
                value: i32::from(pressed),
            });
        }
        events.extend([
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 0,
                value: i32::from(state.left.0.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 1,
                value: i32::from(state.left.1.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 2,
                value: i32::from(state.triggers.0.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 3,
                value: i32::from(state.right.0.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 4,
                value: i32::from(state.right.1.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 5,
                value: i32::from(state.triggers.1.raw()),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: DPAD_X,
                value: i32::from(state.dpad[3]) - i32::from(state.dpad[2]),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: DPAD_Y,
                value: i32::from(state.dpad[1]) - i32::from(state.dpad[0]),
            },
            EvdevEvent {
                event_type: common::EV_SYN,
                code: common::SYN_REPORT,
                value: 0,
            },
        ]);
        Ok(ProviderFrame::Evdev(events))
    }
}

/// Reverse output specific to the Generic Gamepad package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericGamepadOutputEvent {
    ForceFeedbackUpload {
        request_id: u32,
        effect: Vec<u8>,
    },
    ForceFeedbackErase {
        request_id: u32,
        effect_id: u32,
    },
    ProviderEvent(Vec<EvdevEvent>),
    HidOutput {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    HidGetReportRequest {
        request_id: u32,
        report_id: u8,
        report_type: u8,
    },
    HidSetReportRequest {
        request_id: u32,
        report_id: u8,
        report_type: u8,
        bytes: Vec<u8>,
    },
}

pub struct GenericGamepadController(
    ControllerRuntime<GenericGamepadDefinition, common::ProviderSessionSink>,
);
impl GenericGamepadController {
    #[must_use]
    pub const fn state(&self) -> &GenericGamepadState {
        self.0.state()
    }
    #[must_use]
    pub const fn surface(&self) -> &'static GenericGamepadSurface {
        match self.0.selection().target {
            RealizationTarget::Hid => &GENERIC_HID_SURFACE,
            _ => &GENERIC_SURFACE,
        }
    }
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }
    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.0.apply_digital(update)
    }
    pub fn set_native(
        &mut self,
        control: GenericGamepadControl,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.set_native(control, pressed);
            Ok(())
        })
    }
    pub fn set_left_stick(
        &mut self,
        x: GenericGamepadAxis,
        y: GenericGamepadAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.left = (x, y);
            Ok(())
        })
    }
    pub fn set_right_stick(
        &mut self,
        x: GenericGamepadAxis,
        y: GenericGamepadAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.right = (x, y);
            Ok(())
        })
    }
    pub fn set_triggers(
        &mut self,
        left: GenericGamepadTrigger,
        right: GenericGamepadTrigger,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.triggers = (left, right);
            Ok(())
        })
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.0.commit()
    }
    pub fn close(&mut self) {
        self.0.with_sink(common::ProviderSessionSink::close);
        self.0.close();
    }
    pub fn poll_output(
        &mut self,
        callback: &mut dyn FnMut(GenericGamepadOutputEvent),
    ) -> Result<(), ProviderError> {
        self.0.with_sink(|sink| {
            sink.drain(&mut |event| {
                let output = match event {
                    RawReverseEvent::ForceFeedbackUpload { request_id, effect } => {
                        GenericGamepadOutputEvent::ForceFeedbackUpload { request_id, effect }
                    }
                    RawReverseEvent::ForceFeedbackErase {
                        request_id,
                        effect_id,
                    } => GenericGamepadOutputEvent::ForceFeedbackErase {
                        request_id,
                        effect_id,
                    },
                    RawReverseEvent::Evdev(events) => {
                        GenericGamepadOutputEvent::ProviderEvent(events)
                    }
                    RawReverseEvent::HidOutput { report_id, bytes } => {
                        GenericGamepadOutputEvent::HidOutput { report_id, bytes }
                    }
                    RawReverseEvent::HidGetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                    } => GenericGamepadOutputEvent::HidGetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                    },
                    RawReverseEvent::HidSetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                        bytes,
                    } => GenericGamepadOutputEvent::HidSetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                        bytes,
                    },
                    RawReverseEvent::Transport { .. } => return,
                };
                callback(output);
            })
        })
    }
    pub fn reply_get_report(
        &mut self,
        request_id: u32,
        status: i16,
        bytes: Vec<u8>,
    ) -> Result<(), ProviderError> {
        self.0.with_sink(|sink| {
            sink.reply(ProviderFrame::HidGetReportReply {
                request_id,
                status,
                bytes,
            })
        })
    }
    pub fn reply_set_report(&mut self, request_id: u32, status: i16) -> Result<(), ProviderError> {
        self.0
            .with_sink(|sink| sink.reply(ProviderFrame::HidSetReportReply { request_id, status }))
    }
    pub fn reply_force_feedback_upload(
        &mut self,
        request_id: u32,
        status: i32,
    ) -> Result<(), ProviderError> {
        self.0.with_sink(|sink| {
            sink.reply(ProviderFrame::ForceFeedbackUploadReply { request_id, status })
        })
    }
    pub fn reply_force_feedback_erase(
        &mut self,
        request_id: u32,
        status: i32,
    ) -> Result<(), ProviderError> {
        self.0.with_sink(|sink| {
            sink.reply(ProviderFrame::ForceFeedbackEraseReply { request_id, status })
        })
    }
}

fn realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        device_name: "VirtualGamepad Generic".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x1209,
            product_id: 0x0001,
            version: 1,
        },
        event_codes: vec![common::EV_KEY, common::EV_ABS],
        key_codes: GENERIC_DIGITAL
            .iter()
            .map(|control| control.event_code)
            .collect(),
        absolute_axes: GENERIC_AXES
            .iter()
            .map(|axis| NativeAbsoluteAxis {
                code: axis.event_code,
                minimum: axis.minimum,
                maximum: axis.maximum,
                flat: axis.flat,
            })
            .collect(),
        relative_axes: vec![],
        led_codes: vec![],
        switch_codes: vec![],
        force_feedback_codes: vec![0x50],
    })
}
fn hid_realization() -> NativeControllerRealization {
    // 0x1209:0001 is a provisional development identity, not an allocation claim.
    common::hid_realization("VirtualGamepad Generic", 0x1209, 0x0001)
}
pub fn create_generic_gamepad(
    options: CreationOptions,
) -> Result<GenericGamepadController, ProviderError> {
    let realization = match options.target {
        DeploymentTarget::Evdev => realization(),
        DeploymentTarget::Hid => hid_realization(),
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    common::create(GenericGamepadDefinition, realization, options).map(GenericGamepadController)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digital_mapping_and_native_state_are_independent_of_other_controllers() {
        let mut state = GenericGamepadState::default();
        GenericGamepadDefinition
            .apply_digital(
                &mut state,
                DigitalControlUpdate::FaceButton {
                    button: FaceButton::South,
                    pressed: true,
                },
            )
            .expect("digital update");
        state.set_native(GenericGamepadControl::Guide, true);
        assert!(state.face_pressed(FaceButton::South));
        assert!(matches!(
            GenericGamepadDefinition.encode(
                RealizationSelection {
                    controller: GenericGamepadDefinition.controller_id(),
                    target: RealizationTarget::Evdev
                },
                &state
            ),
            Ok(ProviderFrame::Evdev(_))
        ));
    }

    #[test]
    fn surface_reports_native_evdev_ranges() {
        assert_eq!(GENERIC_SURFACE.common.axes[0].minimum, -32768);
        assert_eq!(GENERIC_SURFACE.common.axes[2].maximum, 255);
    }

    #[test]
    fn hid_surface_and_frame_are_explicitly_research_backed() {
        assert_eq!(
            GENERIC_HID_SURFACE.common.validation_status,
            RealizationValidationStatus::ResearchBacked
        );
        let frame = GenericGamepadDefinition
            .encode(
                RealizationSelection {
                    controller: GenericGamepadDefinition.controller_id(),
                    target: RealizationTarget::Hid,
                },
                &GenericGamepadState::default(),
            )
            .expect("HID frame");
        assert_eq!(
            frame,
            ProviderFrame::HidInput {
                report_id: None,
                bytes: vec![0, 0, 8, 128, 128, 128, 128, 0, 0],
            }
        );
        assert!(GENERIC_HID_SURFACE.common.outputs.is_empty());
        assert!(
            !GenericGamepadDefinition.realization_manifest().entries()[1]
                .provider_requirements
                .requires_reverse_output
        );
    }
}
