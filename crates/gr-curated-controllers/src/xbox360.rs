//! Xbox 360 controller with XInput-native numeric domains.

use crate::{CreationOptions, common};
use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, FaceButton, OutputSurface,
    RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
    RealizationValidationStatus, TargetAwareControllerDriver, TargetRestriction,
};
use gr_controller_runtime::ControllerRuntime;
use gr_realization_api::{
    ControllerId, DeploymentTarget, EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization,
    NativeDeviceIdentity, NativeEvdevRealization, ProviderError, ProviderFrame,
    ProviderRequirements, RawReverseEvent, RealizationSelection, RealizationTarget,
};

/// Xbox 360 signed thumb-stick value (`-32768..=32767`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xbox360Axis(i16);
impl Xbox360Axis {
    #[must_use]
    pub const fn raw(self) -> i16 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: i16) -> Self {
        Self(raw)
    }
}
/// Xbox 360 trigger value (`0..=255`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xbox360Trigger(u8);
impl Xbox360Trigger {
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Xbox360Control {
    A,
    B,
    X,
    Y,
    LeftShoulder,
    RightShoulder,
    Back,
    Start,
    Guide,
    LeftStickPress,
    RightStickPress,
}

/// Complete semantic state for an Xbox 360 controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Xbox360State {
    face: [bool; 4],
    dpad: [bool; 4],
    left: (Xbox360Axis, Xbox360Axis),
    right: (Xbox360Axis, Xbox360Axis),
    triggers: (Xbox360Trigger, Xbox360Trigger),
    buttons: [bool; 7],
}
impl Default for Xbox360State {
    fn default() -> Self {
        Self {
            face: [false; 4],
            dpad: [false; 4],
            left: (Xbox360Axis(0), Xbox360Axis(0)),
            right: (Xbox360Axis(0), Xbox360Axis(0)),
            triggers: (Xbox360Trigger(0), Xbox360Trigger(0)),
            buttons: [false; 7],
        }
    }
}
impl Xbox360State {
    #[must_use]
    pub const fn left_stick(&self) -> (Xbox360Axis, Xbox360Axis) {
        self.left
    }
    #[must_use]
    pub const fn right_stick(&self) -> (Xbox360Axis, Xbox360Axis) {
        self.right
    }
    #[must_use]
    pub const fn triggers(&self) -> (Xbox360Trigger, Xbox360Trigger) {
        self.triggers
    }
    #[must_use]
    pub const fn face_pressed(&self, button: FaceButton) -> bool {
        self.face[common::face_index(button)]
    }
    fn set_native(&mut self, control: Xbox360Control, pressed: bool) {
        match control {
            Xbox360Control::A => self.face[0] = pressed,
            Xbox360Control::B => self.face[1] = pressed,
            Xbox360Control::X => self.face[2] = pressed,
            Xbox360Control::Y => self.face[3] = pressed,
            Xbox360Control::LeftShoulder => self.buttons[0] = pressed,
            Xbox360Control::RightShoulder => self.buttons[1] = pressed,
            Xbox360Control::Back => self.buttons[2] = pressed,
            Xbox360Control::Start => self.buttons[3] = pressed,
            Xbox360Control::Guide => self.buttons[4] = pressed,
            Xbox360Control::LeftStickPress => self.buttons[5] = pressed,
            Xbox360Control::RightStickPress => self.buttons[6] = pressed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Xbox360Surface {
    common: ControllerSurface,
}
impl ControllerSurfaceInfo for Xbox360Surface {
    fn common_surface(&self) -> &ControllerSurface {
        &self.common
    }
}
impl Xbox360Surface {
    #[must_use]
    pub const fn common(&self) -> &ControllerSurface {
        &self.common
    }
}
static DIGITAL: [DigitalControlSurface; 11] = [
    DigitalControlSurface {
        control: "a",
        event_code: 304,
    },
    DigitalControlSurface {
        control: "b",
        event_code: 305,
    },
    DigitalControlSurface {
        control: "x",
        event_code: 307,
    },
    DigitalControlSurface {
        control: "y",
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
        control: "back",
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
static AXES: [AbsoluteAxisSurface; 8] = [
    AbsoluteAxisSurface {
        control: "left-stick-x",
        event_code: 0,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 7849,
    },
    AbsoluteAxisSurface {
        control: "left-stick-y",
        event_code: 1,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 7849,
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
        flat: 8689,
    },
    AbsoluteAxisSurface {
        control: "right-stick-y",
        event_code: 4,
        minimum: -32768,
        maximum: 32767,
        neutral: 0,
        flat: 8689,
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
        event_code: 16,
        minimum: -1,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "dpad-y",
        event_code: 17,
        minimum: -1,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
];
static OUTPUTS: [OutputSurface; 1] = [OutputSurface {
    name: "conventional-rumble",
    event_type: 21,
    event_code: 80,
}];
static HID_OUTPUTS: [OutputSurface; 0] = [];
static RESTRICTIONS: [TargetRestriction; 2] = [
    TargetRestriction {
        feature: "headset-audio",
        reason: "requires a separately declared audio sidecar",
    },
    TargetRestriction {
        feature: "chatpad",
        reason: "requires controller-native accessory transport",
    },
];
static SURFACE: Xbox360Surface = Xbox360Surface {
    common: ControllerSurface {
        target: RealizationTarget::Evdev,
        validation_status: RealizationValidationStatus::HostValidated,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};
static HID_SURFACE: Xbox360Surface = Xbox360Surface {
    common: ControllerSurface {
        target: RealizationTarget::Hid,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &HID_OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};

pub struct Xbox360Definition;
impl RealizationControllerDefinition for Xbox360Definition {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("virtualgamepad.xbox360")
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
impl TargetAwareControllerDriver for Xbox360Definition {
    type State = Xbox360State;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        Xbox360State::default()
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
        for (code, pressed) in [304, 305, 307, 308].into_iter().zip(state.face) {
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
                code: 16,
                value: i32::from(state.dpad[3]) - i32::from(state.dpad[2]),
            },
            EvdevEvent {
                event_type: common::EV_ABS,
                code: 17,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Xbox360OutputEvent {
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
pub struct Xbox360Controller(ControllerRuntime<Xbox360Definition, common::ProviderSessionSink>);
impl Xbox360Controller {
    #[must_use]
    pub const fn state(&self) -> &Xbox360State {
        self.0.state()
    }
    #[must_use]
    pub const fn surface(&self) -> &'static Xbox360Surface {
        match self.0.selection().target {
            RealizationTarget::Hid => &HID_SURFACE,
            _ => &SURFACE,
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
        control: Xbox360Control,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.set_native(control, pressed);
            Ok(())
        })
    }
    pub fn set_left_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.left = (x, y);
            Ok(())
        })
    }
    pub fn set_right_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.right = (x, y);
            Ok(())
        })
    }
    pub fn set_triggers(
        &mut self,
        left: Xbox360Trigger,
        right: Xbox360Trigger,
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
        callback: &mut dyn FnMut(Xbox360OutputEvent),
    ) -> Result<(), ProviderError> {
        self.0.with_sink(|sink| {
            sink.drain(&mut |event| {
                let output = match event {
                    RawReverseEvent::ForceFeedbackUpload { request_id, effect } => {
                        Xbox360OutputEvent::ForceFeedbackUpload { request_id, effect }
                    }
                    RawReverseEvent::ForceFeedbackErase {
                        request_id,
                        effect_id,
                    } => Xbox360OutputEvent::ForceFeedbackErase {
                        request_id,
                        effect_id,
                    },
                    RawReverseEvent::Evdev(events) => Xbox360OutputEvent::ProviderEvent(events),
                    RawReverseEvent::HidOutput { report_id, bytes } => {
                        Xbox360OutputEvent::HidOutput { report_id, bytes }
                    }
                    RawReverseEvent::HidGetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                    } => Xbox360OutputEvent::HidGetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                    },
                    RawReverseEvent::HidSetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                        bytes,
                    } => Xbox360OutputEvent::HidSetReportRequest {
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
        device_name: "Virtual Xbox 360".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x045e,
            product_id: 0x028e,
            version: 1,
        },
        event_codes: vec![common::EV_KEY, common::EV_ABS],
        key_codes: DIGITAL.iter().map(|control| control.event_code).collect(),
        absolute_axes: AXES
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
    // Local HID identity only; physical Xbox/xpad USB behavior requires USB gadget.
    common::hid_realization("Virtual Xbox 360", 0x045e, 0x028e)
}
pub fn create_xbox360(options: CreationOptions) -> Result<Xbox360Controller, ProviderError> {
    let realization = match options.target {
        DeploymentTarget::Evdev => realization(),
        DeploymentTarget::Hid => hid_realization(),
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    common::create(Xbox360Definition, realization, options).map(Xbox360Controller)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_a_and_spatial_south_share_one_physical_button() {
        let mut state = Xbox360State::default();
        Xbox360Definition
            .apply_digital(
                &mut state,
                DigitalControlUpdate::FaceButton {
                    button: FaceButton::South,
                    pressed: true,
                },
            )
            .expect("digital update");
        assert!(state.face_pressed(FaceButton::South));
        state.set_native(Xbox360Control::A, false);
        assert!(!state.face_pressed(FaceButton::South));
    }

    #[test]
    fn surface_keeps_xbox_dead_zones_visible_to_callers() {
        assert_eq!(SURFACE.common.axes[0].flat, 7849);
        assert_eq!(SURFACE.common.axes[3].flat, 8689);
    }

    #[test]
    fn hid_surface_is_explicitly_research_backed() {
        assert_eq!(
            HID_SURFACE.common.validation_status,
            RealizationValidationStatus::ResearchBacked
        );
        assert_eq!(HID_SURFACE.common.target, RealizationTarget::Hid);
        assert!(HID_SURFACE.common.outputs.is_empty());
        assert!(
            !Xbox360Definition.realization_manifest().entries()[1]
                .provider_requirements
                .requires_reverse_output
        );
    }
}
