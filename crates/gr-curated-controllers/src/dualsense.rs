//! `DualSense` controller with native input, touch, motion, and output types.

use crate::{BatteryLevel, BatteryState, CreationOptions, common};
use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, FaceButton, OutputSurface,
    RealizationControllerDefinition, RealizationManifest, RealizationManifestEntry,
    RealizationValidationStatus, TargetAwareControllerDriver, TargetRestriction,
};
use gr_controller_runtime::ControllerRuntime;
use gr_realization_api::{
    ControllerId, DeploymentTarget, EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization,
    NativeDeviceIdentity, NativeEvdevRealization, NativeHidRealization, NativeHidReportKey,
    NativeUsbCompositeRealization, NativeUsbEndpointDirection, ProviderError, ProviderFrame,
    ProviderRequirements, RawReverseEvent, RealizationSelection, RealizationSessionId,
    RealizationTarget,
};
use std::collections::BTreeMap;

/// `DualSense` stick-axis value (`0..=255`, neutral `128`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseAxis(u8);
impl DualSenseAxis {
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
    #[must_use]
    pub const fn neutral() -> Self {
        Self(128)
    }
}
/// `DualSense` trigger value (`0..=255`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseTrigger(u8);
impl DualSenseTrigger {
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
    #[must_use]
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }
}
/// One native `DualSense` touch contact. Coordinates use the touch-surface report domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseTouchContact {
    id: u8,
    x: u16,
    y: u16,
}
impl DualSenseTouchContact {
    pub fn new(id: u8, x: u16, y: u16) -> Result<Self, ControlError> {
        if x > 1919 {
            return Err(ControlError::ValueOutOfRange {
                control: "dualsense touch x",
                value: u32::from(x),
                maximum: 1919,
            });
        }
        if y > 941 {
            return Err(ControlError::ValueOutOfRange {
                control: "dualsense touch y",
                value: u32::from(y),
                maximum: 941,
            });
        }
        Ok(Self { id, x, y })
    }
    #[must_use]
    pub const fn id(self) -> u8 {
        self.id
    }
    #[must_use]
    pub const fn x(self) -> u16 {
        self.x
    }
    #[must_use]
    pub const fn y(self) -> u16 {
        self.y
    }
}
/// A controller-native physical touch slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchSlot {
    First,
    Second,
}
impl TouchSlot {
    const fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}
/// Raw `DualSense` IMU sample. The evdev target currently does not claim a
/// faithful IMU presentation, so updates return `UnavailableInRealization`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionSample {
    pub accelerometer: [i16; 3],
    pub gyroscope: [i16; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualSenseFeature {
    Touch,
    Motion,
    Lightbar,
    AdaptiveTriggers,
    Audio,
}

/// Endpoints expected from the operator-provisioned `DualSense` USB gadget.
///
/// Addresses include the USB direction bit. The HID pair is used by this
/// crate; the audio pair and control endpoint are validated as part of the
/// composite facility and remain available to an application audio bridge.
pub const DUALSENSE_USB_HID_INPUT_ENDPOINT: u8 = 0x81;
pub const DUALSENSE_USB_HID_OUTPUT_ENDPOINT: u8 = 0x01;
pub const DUALSENSE_USB_AUDIO_CAPTURE_ENDPOINT: u8 = 0x82;
pub const DUALSENSE_USB_AUDIO_PLAYBACK_ENDPOINT: u8 = 0x02;
pub const DUALSENSE_USB_AUDIO_CONTROL_ENDPOINT: u8 = 0x03;

/// Explicit physical-PC USB creation options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualSenseUsbOptions {
    pub session: RealizationSessionId,
    pub composite: NativeUsbCompositeRealization,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualSenseControl {
    Cross,
    Circle,
    Square,
    Triangle,
    L1,
    R1,
    Create,
    Options,
    PlayStation,
    TouchpadClick,
    MicrophoneMute,
    LeftStickPress,
    RightStickPress,
}

/// Complete semantic state for a `DualSense` controller. Fields remain private
/// so every mutation passes through the runtime's cloned-candidate validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualSenseState {
    face: [bool; 4],
    dpad: [bool; 4],
    left: (DualSenseAxis, DualSenseAxis),
    right: (DualSenseAxis, DualSenseAxis),
    triggers: (DualSenseTrigger, DualSenseTrigger),
    buttons: [bool; 9],
    touches: [Option<DualSenseTouchContact>; 2],
    motion: MotionSample,
    sensor_timestamp: u32,
    battery: BatteryState,
}
impl Default for DualSenseState {
    fn default() -> Self {
        Self {
            face: [false; 4],
            dpad: [false; 4],
            left: (DualSenseAxis::neutral(), DualSenseAxis::neutral()),
            right: (DualSenseAxis::neutral(), DualSenseAxis::neutral()),
            triggers: (DualSenseTrigger(0), DualSenseTrigger(0)),
            buttons: [false; 9],
            touches: [None, None],
            motion: MotionSample {
                accelerometer: [0; 3],
                gyroscope: [0; 3],
            },
            sensor_timestamp: 0,
            battery: BatteryState::default(),
        }
    }
}
impl DualSenseState {
    #[must_use]
    pub const fn left_stick(&self) -> (DualSenseAxis, DualSenseAxis) {
        self.left
    }
    #[must_use]
    pub const fn right_stick(&self) -> (DualSenseAxis, DualSenseAxis) {
        self.right
    }
    #[must_use]
    pub const fn triggers(&self) -> (DualSenseTrigger, DualSenseTrigger) {
        self.triggers
    }
    #[must_use]
    pub const fn touch(&self, slot: TouchSlot) -> Option<DualSenseTouchContact> {
        self.touches[slot.index()]
    }
    #[must_use]
    pub const fn motion(&self) -> MotionSample {
        self.motion
    }
    #[must_use]
    pub const fn battery(&self) -> BatteryState {
        self.battery
    }
    #[must_use]
    pub const fn face_pressed(&self, button: FaceButton) -> bool {
        self.face[common::face_index(button)]
    }
    fn set_native(&mut self, control: DualSenseControl, pressed: bool) {
        match control {
            DualSenseControl::Cross => self.face[0] = pressed,
            DualSenseControl::Circle => self.face[1] = pressed,
            DualSenseControl::Square => self.face[2] = pressed,
            DualSenseControl::Triangle => self.face[3] = pressed,
            DualSenseControl::L1 => self.buttons[0] = pressed,
            DualSenseControl::R1 => self.buttons[1] = pressed,
            DualSenseControl::Create => self.buttons[2] = pressed,
            DualSenseControl::Options => self.buttons[3] = pressed,
            DualSenseControl::PlayStation => self.buttons[4] = pressed,
            DualSenseControl::TouchpadClick => self.buttons[5] = pressed,
            DualSenseControl::MicrophoneMute => self.buttons[8] = pressed,
            DualSenseControl::LeftStickPress => self.buttons[6] = pressed,
            DualSenseControl::RightStickPress => self.buttons[7] = pressed,
        }
    }
}

fn advance_sensor_timestamp(state: &mut DualSenseState) {
    // DualSense reports timestamps in 0.33 µs units. A monotonically
    // advancing synthetic timestamp keeps the Linux sensor input stream
    // meaningful when motion is driven from the demo.
    state.sensor_timestamp = state.sensor_timestamp.wrapping_add(3_000);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualSenseSurface {
    common: ControllerSurface,
}
impl ControllerSurfaceInfo for DualSenseSurface {
    fn common_surface(&self) -> &ControllerSurface {
        &self.common
    }
}
impl DualSenseSurface {
    #[must_use]
    pub const fn common(&self) -> &ControllerSurface {
        &self.common
    }
}
static DIGITAL: [DigitalControlSurface; 14] = [
    DigitalControlSurface {
        control: "cross",
        event_code: 304,
    },
    DigitalControlSurface {
        control: "circle",
        event_code: 305,
    },
    DigitalControlSurface {
        control: "square",
        event_code: 307,
    },
    DigitalControlSurface {
        control: "triangle",
        event_code: 308,
    },
    DigitalControlSurface {
        control: "l1",
        event_code: 310,
    },
    DigitalControlSurface {
        control: "r1",
        event_code: 311,
    },
    DigitalControlSurface {
        control: "create",
        event_code: 314,
    },
    DigitalControlSurface {
        control: "options",
        event_code: 315,
    },
    DigitalControlSurface {
        control: "playstation",
        event_code: 316,
    },
    DigitalControlSurface {
        control: "touchpad-click",
        event_code: 317,
    },
    DigitalControlSurface {
        control: "left-stick-press",
        event_code: 318,
    },
    DigitalControlSurface {
        control: "right-stick-press",
        event_code: 319,
    },
    DigitalControlSurface {
        control: "touch-active",
        event_code: 330,
    },
    DigitalControlSurface {
        control: "touch-finger",
        event_code: 325,
    },
];
static AXES: [AbsoluteAxisSurface; 12] = [
    AbsoluteAxisSurface {
        control: "left-stick-x",
        event_code: 0,
        minimum: 0,
        maximum: 255,
        neutral: 128,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "left-stick-y",
        event_code: 1,
        minimum: 0,
        maximum: 255,
        neutral: 128,
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
        minimum: 0,
        maximum: 255,
        neutral: 128,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "right-stick-y",
        event_code: 4,
        minimum: 0,
        maximum: 255,
        neutral: 128,
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
    AbsoluteAxisSurface {
        control: "touch-slot",
        event_code: 47,
        minimum: 0,
        maximum: 1,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "touch-x",
        event_code: 53,
        minimum: 0,
        maximum: 1919,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "touch-y",
        event_code: 54,
        minimum: 0,
        maximum: 941,
        neutral: 0,
        flat: 0,
    },
    AbsoluteAxisSurface {
        control: "touch-tracking-id",
        event_code: 57,
        minimum: 0,
        maximum: 255,
        neutral: -1,
        flat: 0,
    },
];
static OUTPUTS: [OutputSurface; 1] = [OutputSurface {
    name: "conventional-rumble",
    event_type: 21,
    event_code: 80,
}];
static RESTRICTIONS: [TargetRestriction; 4] = [
    TargetRestriction {
        feature: "motion",
        reason: "evdev has no evidenced faithful DualSense IMU presentation",
    },
    TargetRestriction {
        feature: "RGB lightbar",
        reason: "generic evdev LEDs cannot faithfully express DualSense lightbar reports",
    },
    TargetRestriction {
        feature: "adaptive triggers",
        reason: "generic force-feedback effects cannot faithfully express adaptive-trigger reports",
    },
    TargetRestriction {
        feature: "audio",
        reason: "controller reports do not create audio streams; an audio sidecar is required",
    },
];
static HID_RESTRICTIONS: [TargetRestriction; 2] = [
    TargetRestriction {
        feature: "controller audio",
        reason: "UHID creates no USB-audio interface; an audio sidecar is required",
    },
    TargetRestriction {
        feature: "physical-device fidelity",
        reason: "the USB HID report contract is research-backed until reference-device comparison",
    },
];
static SURFACE: DualSenseSurface = DualSenseSurface {
    common: ControllerSurface {
        target: RealizationTarget::Evdev,
        validation_status: RealizationValidationStatus::HostValidated,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &RESTRICTIONS,
    },
};
static HID_SURFACE: DualSenseSurface = DualSenseSurface {
    common: ControllerSurface {
        target: RealizationTarget::Hid,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &HID_RESTRICTIONS,
    },
};
static USB_RESTRICTIONS: [TargetRestriction; 2] = [
    TargetRestriction {
        feature: "controller audio routing",
        reason: "the pre-provisioned audio endpoints require a caller-provided backend-neutral audio bridge",
    },
    TargetRestriction {
        feature: "physical-device fidelity",
        reason: "the USB composite protocol is research-backed until external PC acceptance testing",
    },
];
static USB_SURFACE: DualSenseSurface = DualSenseSurface {
    common: ControllerSurface {
        target: RealizationTarget::UsbTransportValidation,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        restrictions: &USB_RESTRICTIONS,
    },
};

pub struct DualSenseDefinition;
impl RealizationControllerDefinition for DualSenseDefinition {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("virtualgamepad.dualsense")
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static ENTRIES: [RealizationManifestEntry; 3] = [
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
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::UsbTransportValidation,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(&ENTRIES)
    }
}
impl TargetAwareControllerDriver for DualSenseDefinition {
    type State = DualSenseState;
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        DualSenseState::default()
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
            RealizationTarget::Evdev
                | RealizationTarget::Hid
                | RealizationTarget::UsbTransportValidation
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
            return Ok(dualsense_hid_input_report(state));
        }
        if selection.target == RealizationTarget::UsbTransportValidation {
            let ProviderFrame::HidInput {
                report_id: Some(report_id),
                mut bytes,
            } = dualsense_hid_input_report(state)
            else {
                unreachable!("DualSense USB input report is numbered")
            };
            bytes.insert(0, report_id);
            return Ok(ProviderFrame::Transport {
                endpoint: DUALSENSE_USB_HID_INPUT_ENDPOINT,
                bytes,
            });
        }
        let mut events = Vec::new();
        for (code, pressed) in [304, 305, 307, 308].into_iter().zip(state.face) {
            events.push(EvdevEvent {
                event_type: common::EV_KEY,
                code,
                value: i32::from(pressed),
            });
        }
        for (code, pressed) in [310, 311, 314, 315, 316, 317, 318, 319]
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
        ]);
        encode_touches(&mut events, state.touches);
        events.push(EvdevEvent {
            event_type: common::EV_SYN,
            code: common::SYN_REPORT,
            value: 0,
        });
        Ok(ProviderFrame::Evdev(events))
    }
}

/// USB-format `DualSense` input report (report ID `0x01`). The byte layout is
/// taken from the Linux HID `PlayStation` driver's USB report structure; this
/// project deliberately leaves transport-specific Bluetooth framing out of
/// the UHID target.
fn dualsense_hid_input_report(state: &DualSenseState) -> ProviderFrame {
    let mut bytes = vec![0_u8; 63];
    bytes[0..6].copy_from_slice(&[
        state.left.0.raw(),
        state.left.1.raw(),
        state.right.0.raw(),
        state.right.1.raw(),
        state.triggers.0.raw(),
        state.triggers.1.raw(),
    ]);
    bytes[7] = dualsense_hat(state.dpad)
        | (u8::from(state.face[2]) << 4)
        | (u8::from(state.face[0]) << 5)
        | (u8::from(state.face[1]) << 6)
        | (u8::from(state.face[3]) << 7);
    bytes[8] = u8::from(state.buttons[0])
        | (u8::from(state.buttons[1]) << 1)
        | (u8::from(state.buttons[2]) << 4)
        | (u8::from(state.buttons[3]) << 5)
        | (u8::from(state.buttons[6]) << 6)
        | (u8::from(state.buttons[7]) << 7);
    bytes[9] = u8::from(state.buttons[4])
        | (u8::from(state.buttons[5]) << 1)
        | (u8::from(state.buttons[8]) << 2);
    for (offset, value) in state.motion.gyroscope.into_iter().enumerate() {
        bytes[15 + offset * 2..17 + offset * 2].copy_from_slice(&value.to_le_bytes());
    }
    for (offset, value) in state.motion.accelerometer.into_iter().enumerate() {
        bytes[21 + offset * 2..23 + offset * 2].copy_from_slice(&value.to_le_bytes());
    }
    bytes[27..31].copy_from_slice(&state.sensor_timestamp.to_le_bytes());
    encode_hid_touches(&mut bytes[32..40], state.touches);
    // hid-playstation consumes status[0] at report byte 52. Keeping battery
    // exposure disabled represents an externally powered, fully charged
    // controller; enabling it reports the caller-supplied 10% wire bucket.
    bytes[52] = dualsense_battery_status(state.battery);
    ProviderFrame::HidInput {
        report_id: Some(0x01),
        bytes,
    }
}

fn dualsense_battery_status(battery: BatteryState) -> u8 {
    if !battery.is_exposed() {
        return 0x20; // Fully charged / externally powered.
    }
    // The wire format has ten-percent buckets. Round up so a non-zero caller
    // value never gets presented to the host as an empty battery.
    let level = battery.level().percent();
    level.saturating_add(9).div_euclid(10).min(10)
}

fn dualsense_hat(dpad: [bool; 4]) -> u8 {
    match (dpad[0], dpad[1], dpad[2], dpad[3]) {
        (true, false, false, false) => 0,
        (true, false, false, true) => 1,
        (false, false, false, true) => 2,
        (false, true, false, true) => 3,
        (false, true, false, false) => 4,
        (false, true, true, false) => 5,
        (false, false, true, false) => 6,
        (true, false, true, false) => 7,
        _ => 8,
    }
}

fn encode_hid_touches(bytes: &mut [u8], touches: [Option<DualSenseTouchContact>; 2]) {
    for (slot, contact) in touches.into_iter().enumerate() {
        let offset = slot * 4;
        match contact {
            Some(contact) => {
                let [x_lo, x_hi] = contact.x().to_le_bytes();
                let [y_lo, y_hi] = contact.y().to_le_bytes();
                bytes[offset] = contact.id() & 0x7f;
                bytes[offset + 1] = x_lo;
                bytes[offset + 2] = (x_hi & 0x0f) | ((y_lo & 0x0f) << 4);
                bytes[offset + 3] = (y_hi << 4) | (y_lo >> 4);
            }
            None => bytes[offset] = 0x80,
        }
    }
}

fn encode_touches(events: &mut Vec<EvdevEvent>, touches: [Option<DualSenseTouchContact>; 2]) {
    let active = touches.iter().any(Option::is_some);
    events.extend([
        EvdevEvent {
            event_type: common::EV_KEY,
            code: 330,
            value: i32::from(active),
        },
        EvdevEvent {
            event_type: common::EV_KEY,
            code: 325,
            value: i32::from(active),
        },
    ]);
    for (slot, contact) in [(0_i32, touches[0]), (1_i32, touches[1])] {
        events.push(EvdevEvent {
            event_type: common::EV_ABS,
            code: 47,
            value: slot,
        });
        match contact {
            Some(contact) => events.extend([
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 57,
                    value: i32::from(contact.id()),
                },
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 53,
                    value: i32::from(contact.x()),
                },
                EvdevEvent {
                    event_type: common::EV_ABS,
                    code: 54,
                    value: i32::from(contact.y()),
                },
            ]),
            None => events.push(EvdevEvent {
                event_type: common::EV_ABS,
                code: 57,
                value: -1,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualSenseOutputEvent {
    ConventionalForceFeedbackUpload {
        request_id: u32,
        effect: Vec<u8>,
    },
    ConventionalForceFeedbackErase {
        request_id: u32,
        effect_id: u32,
    },
    ProviderEvent(Vec<EvdevEvent>),
    HidOutput(DualSenseHidOutput),
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

/// Reverse HID output preserved in its native report form.
///
/// `UsbOutput` exposes the fields whose positions are documented by the Linux
/// HID `PlayStation` driver while retaining the complete raw payload for
/// adaptive-trigger and advanced-haptic effects that SDL does not model with a
/// portable semantic API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualSenseHidOutput {
    UsbOutput {
        raw: Vec<u8>,
        valid_flag0: u8,
        valid_flag1: u8,
        right_motor: u8,
        left_motor: u8,
        right_trigger_effect: [u8; 11],
        left_trigger_effect: [u8; 11],
        /// Present only when the host set the mic-mute LED valid bit.
        mute_button_led: Option<bool>,
        /// Present only when the host set the player-indicator valid bit.
        player_leds: Option<u8>,
        /// Present only when the host set the lightbar valid bit.
        lightbar_rgb: Option<[u8; 3]>,
    },
    Unknown {
        report_id: Option<u8>,
        raw: Vec<u8>,
    },
}

fn decode_dualsense_hid_output(report_id: Option<u8>, raw: Vec<u8>) -> DualSenseHidOutput {
    if report_id == Some(0x02) && raw.len() >= 47 {
        let mut right_trigger_effect = [0_u8; 11];
        right_trigger_effect.copy_from_slice(&raw[10..21]);
        let mut left_trigger_effect = [0_u8; 11];
        left_trigger_effect.copy_from_slice(&raw[21..32]);
        return DualSenseHidOutput::UsbOutput {
            valid_flag0: raw[0],
            valid_flag1: raw[1],
            right_motor: raw[2],
            left_motor: raw[3],
            right_trigger_effect,
            left_trigger_effect,
            mute_button_led: (raw[1] & 0x01 != 0).then_some(raw[8] != 0),
            player_leds: (raw[1] & 0x10 != 0).then_some(raw[43]),
            lightbar_rgb: (raw[1] & 0x04 != 0).then_some([raw[44], raw[45], raw[46]]),
            raw,
        };
    }
    DualSenseHidOutput::Unknown { report_id, raw }
}
pub struct DualSenseController(ControllerRuntime<DualSenseDefinition, common::ProviderSessionSink>);
impl DualSenseController {
    #[must_use]
    pub const fn state(&self) -> &DualSenseState {
        self.0.state()
    }
    #[must_use]
    pub const fn surface(&self) -> &'static DualSenseSurface {
        match self.0.selection().target {
            RealizationTarget::Hid => &HID_SURFACE,
            RealizationTarget::UsbTransportValidation => &USB_SURFACE,
            _ => &SURFACE,
        }
    }
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }
    #[must_use]
    pub fn provider_diagnostics(&mut self) -> gr_realization_api::ProviderDiagnostics {
        self.0.with_sink(|sink| sink.diagnostics())
    }
    pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError> {
        self.0.apply_digital(update)
    }
    pub fn set_native(
        &mut self,
        control: DualSenseControl,
        pressed: bool,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.set_native(control, pressed);
            Ok(())
        })
    }
    pub fn set_left_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.left = (x, y);
            Ok(())
        })
    }
    pub fn set_right_stick(
        &mut self,
        x: DualSenseAxis,
        y: DualSenseAxis,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.right = (x, y);
            Ok(())
        })
    }
    pub fn set_triggers(
        &mut self,
        left: DualSenseTrigger,
        right: DualSenseTrigger,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.triggers = (left, right);
            Ok(())
        })
    }
    pub fn set_touch(
        &mut self,
        slot: TouchSlot,
        contact: Option<DualSenseTouchContact>,
    ) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.touches[slot.index()] = contact;
            Ok(())
        })
    }
    pub fn set_motion(&mut self, motion: MotionSample) -> Result<(), ControlError> {
        if !matches!(
            self.0.selection().target,
            RealizationTarget::Hid | RealizationTarget::UsbTransportValidation
        ) {
            return Err(common::unavailable(self.0.selection().target));
        }
        self.0.update_state(|state| {
            state.motion = motion;
            advance_sensor_timestamp(state);
            Ok(())
        })
    }
    /// Enable or hide the emulated battery without recreating the controller.
    pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.battery.set_exposed(exposed);
            Ok(())
        })
    }
    /// Update the emulated battery percentage without recreating the controller.
    pub fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.battery.set_level(level);
            Ok(())
        })
    }
    pub fn feature_available(&self, feature: DualSenseFeature) -> Result<(), ControlError> {
        match feature {
            DualSenseFeature::Touch => Ok(()),
            DualSenseFeature::Motion
                if matches!(
                    self.0.selection().target,
                    RealizationTarget::Hid | RealizationTarget::UsbTransportValidation
                ) =>
            {
                Ok(())
            }
            DualSenseFeature::Motion
            | DualSenseFeature::Lightbar
            | DualSenseFeature::AdaptiveTriggers
            | DualSenseFeature::Audio => Err(common::unavailable(self.0.selection().target)),
        }
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
        callback: &mut dyn FnMut(DualSenseOutputEvent),
    ) -> Result<(), ProviderError> {
        self.0.with_sink(|sink| {
            sink.drain(&mut |event| {
                let output = match event {
                    RawReverseEvent::ForceFeedbackUpload { request_id, effect } => {
                        DualSenseOutputEvent::ConventionalForceFeedbackUpload { request_id, effect }
                    }
                    RawReverseEvent::ForceFeedbackErase {
                        request_id,
                        effect_id,
                    } => DualSenseOutputEvent::ConventionalForceFeedbackErase {
                        request_id,
                        effect_id,
                    },
                    RawReverseEvent::Evdev(events) => DualSenseOutputEvent::ProviderEvent(events),
                    RawReverseEvent::HidOutput { report_id, bytes } => {
                        DualSenseOutputEvent::HidOutput(decode_dualsense_hid_output(
                            report_id, bytes,
                        ))
                    }
                    RawReverseEvent::HidGetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                    } => DualSenseOutputEvent::HidGetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                    },
                    RawReverseEvent::HidSetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                        bytes,
                    } => DualSenseOutputEvent::HidSetReportRequest {
                        request_id,
                        report_id,
                        report_type,
                        bytes,
                    },
                    RawReverseEvent::Transport {
                        endpoint,
                        mut bytes,
                    } if endpoint == DUALSENSE_USB_HID_OUTPUT_ENDPOINT => {
                        let report_id = bytes.first().copied();
                        if report_id.is_some() {
                            bytes.remove(0);
                        }
                        DualSenseOutputEvent::HidOutput(decode_dualsense_hid_output(
                            report_id, bytes,
                        ))
                    }
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
        device_name: "Virtual DualSense".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x054c,
            product_id: 0x0ce6,
            version: 1,
        },
        event_codes: vec![common::EV_KEY, common::EV_ABS, common::EV_FF],
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
const DUALSENSE_USB_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x32, 0x09, 0x35,
    0x09, 0x33, 0x09, 0x34, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x06, 0x81, 0x02, 0x06,
    0x00, 0xff, 0x09, 0x20, 0x95, 0x01, 0x81, 0x02, 0x05, 0x01, 0x09, 0x39, 0x15, 0x00, 0x25, 0x07,
    0x35, 0x00, 0x46, 0x3b, 0x01, 0x65, 0x14, 0x75, 0x04, 0x95, 0x01, 0x81, 0x42, 0x65, 0x00, 0x05,
    0x09, 0x19, 0x01, 0x29, 0x0f, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x0f, 0x81, 0x02, 0x06,
    0x00, 0xff, 0x09, 0x21, 0x95, 0x0d, 0x81, 0x02, 0x06, 0x00, 0xff, 0x09, 0x22, 0x15, 0x00, 0x26,
    0xff, 0x00, 0x75, 0x08, 0x95, 0x34, 0x81, 0x02, 0x85, 0x02, 0x09, 0x23, 0x95, 0x2f, 0x91, 0x02,
    0x85, 0x05, 0x09, 0x33, 0x95, 0x28, 0xb1, 0x02, 0x85, 0x08, 0x09, 0x34, 0x95, 0x2f, 0xb1, 0x02,
    0x85, 0x09, 0x09, 0x24, 0x95, 0x13, 0xb1, 0x02, 0x85, 0x0a, 0x09, 0x25, 0x95, 0x1a, 0xb1, 0x02,
    0x85, 0x20, 0x09, 0x26, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x21, 0x09, 0x27, 0x95, 0x04, 0xb1, 0x02,
    0x85, 0x22, 0x09, 0x40, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x80, 0x09, 0x28, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0x81, 0x09, 0x29, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x82, 0x09, 0x2a, 0x95, 0x09, 0xb1, 0x02,
    0x85, 0x83, 0x09, 0x2b, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x84, 0x09, 0x2c, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0x85, 0x09, 0x2d, 0x95, 0x02, 0xb1, 0x02, 0x85, 0xa0, 0x09, 0x2e, 0x95, 0x01, 0xb1, 0x02,
    0x85, 0xe0, 0x09, 0x2f, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf0, 0x09, 0x30, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0xf1, 0x09, 0x31, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf2, 0x09, 0x32, 0x95, 0x0f, 0xb1, 0x02,
    0x85, 0xf4, 0x09, 0x35, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf5, 0x09, 0x36, 0x95, 0x03, 0xb1, 0x02,
    0xc0,
];

fn dualsense_feature_responses(
    session: RealizationSessionId,
) -> BTreeMap<NativeHidReportKey, Vec<u8>> {
    // `uhid_report_type`: feature=0, output=1, input=2. This is distinct
    // from the HID class-request values and is the value Linux sends in a
    // `UHID_GET_REPORT` event.
    const FEATURE: u8 = 0;
    let mut calibration = vec![0_u8; 41];
    calibration[0] = 0x05;
    // Consistent synthetic calibration values. `hid-playstation` uses these
    // fields to derive per-axis denominators; zero-filled placeholder values
    // make it take its broken-device fallback and prevent reliable motion
    // handling by host software. No physical controller data is captured.
    for offset in [7, 11, 15] {
        calibration[offset..offset + 2].copy_from_slice(&32_000_i16.to_le_bytes());
    }
    for offset in [9, 13, 17] {
        calibration[offset..offset + 2].copy_from_slice(&(-32_000_i16).to_le_bytes());
    }
    calibration[19..21].copy_from_slice(&2_000_i16.to_le_bytes());
    calibration[21..23].copy_from_slice(&2_000_i16.to_le_bytes());
    for offset in [23, 27, 31] {
        calibration[offset..offset + 2].copy_from_slice(&8_192_i16.to_le_bytes());
    }
    for offset in [25, 29, 33] {
        calibration[offset..offset + 2].copy_from_slice(&(-8_192_i16).to_le_bytes());
    }
    let mut pairing = vec![0_u8; 20];
    pairing[0] = 0x09;
    // `hid-playstation` de-duplicates DualSense connections by this address.
    // Generate an ephemeral locally-administered unicast address so separate
    // virtual sessions never impersonate the same physical controller. It is
    // not read from hardware, persisted, or used outside this process.
    let process = u64::from(std::process::id());
    let process_bytes = process.to_le_bytes();
    let session_bytes = session.0.to_le_bytes();
    pairing[1..7].copy_from_slice(&[
        0x02,
        process_bytes[0],
        process_bytes[1],
        process_bytes[2],
        session_bytes[0],
        session_bytes[1],
    ]);
    let mut firmware = vec![0_u8; 64];
    firmware[0] = 0x20;
    firmware[24] = 1;
    firmware[28] = 1;
    [(0x05, calibration), (0x09, pairing), (0x20, firmware)]
        .into_iter()
        .map(|(report_id, bytes)| {
            (
                NativeHidReportKey {
                    report_id,
                    report_type: FEATURE,
                },
                bytes,
            )
        })
        .collect()
}

fn hid_realization(session: RealizationSessionId) -> NativeControllerRealization {
    // USB HID report structure is based on public research and the Linux
    // DualSense driver; physical comparison remains required for promotion.
    NativeControllerRealization::Hid(NativeHidRealization {
        bus_type: 0x03,
        device_name: "Virtual DualSense".into(),
        physical_path: "virtualgamepad/uhid/dualsense".into(),
        unique_id: "virtualgamepad-dualsense".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x054c,
            product_id: 0x0ce6,
            version: 1,
        },
        descriptor: DUALSENSE_USB_DESCRIPTOR.to_vec(),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: true,
        feature_report_responses: dualsense_feature_responses(session),
    })
}
pub fn create_dualsense(options: CreationOptions) -> Result<DualSenseController, ProviderError> {
    let realization = match options.target {
        DeploymentTarget::Evdev => realization(),
        DeploymentTarget::Hid => hid_realization(options.session),
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    let mut controller = common::create(DualSenseDefinition, realization, options)?;
    // SDL/Steam only waits briefly for the first full USB report while it
    // decides whether a DualSense supports enhanced input (including gyro).
    // Flush neutral state before returning so callers cannot accidentally
    // defer that report until a later UI/event-loop commit.
    controller.commit().map_err(|error| ProviderError::Open {
        reason: format!("send initial DualSense input report: {error}"),
    })?;
    Ok(DualSenseController(controller))
}

/// Create a DualSense-compatible physical USB composite controller.
///
/// The caller must provision the gadget and supply its endpoint files. This
/// function only validates and opens those files, so it is safe to use from a
/// non-privileged process with delegated endpoint access.
pub fn create_dualsense_usb(
    options: DualSenseUsbOptions,
) -> Result<DualSenseController, ProviderError> {
    validate_dualsense_composite(&options.composite)?;
    common::create_usb(
        DualSenseDefinition,
        NativeControllerRealization::UsbComposite(options.composite),
        options.session,
    )
    .map(DualSenseController)
}

fn validate_dualsense_composite(
    composite: &NativeUsbCompositeRealization,
) -> Result<(), ProviderError> {
    const REQUIRED: [(u8, NativeUsbEndpointDirection, u16); 5] = [
        (
            DUALSENSE_USB_HID_INPUT_ENDPOINT,
            NativeUsbEndpointDirection::DeviceToHost,
            64,
        ),
        (
            DUALSENSE_USB_HID_OUTPUT_ENDPOINT,
            NativeUsbEndpointDirection::HostToDevice,
            64,
        ),
        (
            DUALSENSE_USB_AUDIO_CAPTURE_ENDPOINT,
            NativeUsbEndpointDirection::DeviceToHost,
            1,
        ),
        (
            DUALSENSE_USB_AUDIO_PLAYBACK_ENDPOINT,
            NativeUsbEndpointDirection::HostToDevice,
            1,
        ),
        (
            DUALSENSE_USB_AUDIO_CONTROL_ENDPOINT,
            NativeUsbEndpointDirection::HostToDevice,
            1,
        ),
    ];
    for (address, direction, minimum_packet_length) in REQUIRED {
        if !composite.endpoints.iter().any(|endpoint| {
            endpoint.address == address
                && endpoint.direction == direction
                && endpoint.maximum_packet_length >= minimum_packet_length
        }) {
            return Err(ProviderError::Unsupported {
                reason: format!("DualSense USB composite lacks endpoint {address:#04x}"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn composite_endpoint(
        address: u8,
        direction: NativeUsbEndpointDirection,
        maximum_packet_length: u16,
    ) -> gr_realization_api::NativeUsbCompositeEndpoint {
        gr_realization_api::NativeUsbCompositeEndpoint {
            address,
            direction,
            path: format!("/operator-provisioned/{address:02x}"),
            maximum_packet_length,
        }
    }

    fn complete_composite() -> NativeUsbCompositeRealization {
        NativeUsbCompositeRealization {
            device_name: "DualSense composite".into(),
            endpoints: vec![
                composite_endpoint(
                    DUALSENSE_USB_HID_INPUT_ENDPOINT,
                    NativeUsbEndpointDirection::DeviceToHost,
                    64,
                ),
                composite_endpoint(
                    DUALSENSE_USB_HID_OUTPUT_ENDPOINT,
                    NativeUsbEndpointDirection::HostToDevice,
                    64,
                ),
                composite_endpoint(
                    DUALSENSE_USB_AUDIO_CAPTURE_ENDPOINT,
                    NativeUsbEndpointDirection::DeviceToHost,
                    192,
                ),
                composite_endpoint(
                    DUALSENSE_USB_AUDIO_PLAYBACK_ENDPOINT,
                    NativeUsbEndpointDirection::HostToDevice,
                    192,
                ),
                composite_endpoint(
                    DUALSENSE_USB_AUDIO_CONTROL_ENDPOINT,
                    NativeUsbEndpointDirection::HostToDevice,
                    8,
                ),
            ],
        }
    }

    #[test]
    fn physical_usb_requires_the_complete_directed_endpoint_bundle() {
        validate_dualsense_composite(&complete_composite()).expect("complete endpoint bundle");

        let mut missing_output = complete_composite();
        missing_output
            .endpoints
            .retain(|endpoint| endpoint.address != DUALSENSE_USB_HID_OUTPUT_ENDPOINT);
        assert!(validate_dualsense_composite(&missing_output).is_err());

        let mut wrong_direction = complete_composite();
        wrong_direction.endpoints[1].direction = NativeUsbEndpointDirection::DeviceToHost;
        assert!(validate_dualsense_composite(&wrong_direction).is_err());
    }

    #[test]
    fn physical_usb_input_is_numbered_and_sent_to_hid_in() {
        let frame = DualSenseDefinition
            .encode(
                RealizationSelection {
                    controller: DualSenseDefinition.controller_id(),
                    target: RealizationTarget::UsbTransportValidation,
                },
                &DualSenseState::default(),
            )
            .expect("USB input frame");
        assert!(matches!(
            frame,
            ProviderFrame::Transport {
                endpoint: DUALSENSE_USB_HID_INPUT_ENDPOINT,
                bytes,
            } if bytes.len() == 64 && bytes[0] == 0x01
        ));
    }

    #[test]
    fn touch_validation_rejects_without_state_mutation() {
        assert!(DualSenseTouchContact::new(0, 1920, 0).is_err());
        assert!(DualSenseTouchContact::new(0, 0, 942).is_err());
    }
    #[test]
    fn spatial_and_native_faces_are_equivalent() {
        let mut state = DualSenseState::default();
        DualSenseDefinition
            .apply_digital(
                &mut state,
                DigitalControlUpdate::FaceButton {
                    button: FaceButton::South,
                    pressed: true,
                },
            )
            .expect("face");
        assert!(state.face[0]);
        state.set_native(DualSenseControl::Triangle, true);
        assert!(state.face[3]);
    }

    #[test]
    fn evdev_surface_declares_touch_and_rejects_unfaithful_features() {
        assert!(
            SURFACE
                .common
                .axes
                .iter()
                .any(|axis| axis.control == "touch-x")
        );
        assert!(
            RESTRICTIONS
                .iter()
                .any(|restriction| restriction.feature == "adaptive triggers")
        );
    }

    #[test]
    fn hid_surface_is_explicitly_research_backed() {
        assert_eq!(
            HID_SURFACE.common.validation_status,
            RealizationValidationStatus::ResearchBacked
        );
        assert_eq!(HID_SURFACE.common.target, RealizationTarget::Hid);
    }

    #[test]
    fn hid_codec_uses_numbered_usb_report_with_motion_and_touches() {
        let mut state = DualSenseState::default();
        state.set_native(DualSenseControl::Cross, true);
        state.motion = MotionSample {
            gyroscope: [1, -2, 3],
            accelerometer: [-4, 5, -6],
        };
        state.sensor_timestamp = 3_000;
        state.set_native(DualSenseControl::MicrophoneMute, true);
        state.touches[0] = Some(DualSenseTouchContact::new(9, 0x345, 0x2a1).expect("contact"));
        let ProviderFrame::HidInput { report_id, bytes } = dualsense_hid_input_report(&state)
        else {
            panic!("DualSense HID report must be an input frame");
        };
        assert_eq!(report_id, Some(1));
        assert_eq!(bytes.len(), 63);
        assert_eq!(bytes[7], 0x28); // neutral hat plus Cross.
        assert_eq!(&bytes[15..21], &[1, 0, 254, 255, 3, 0]);
        assert_eq!(&bytes[21..27], &[252, 255, 5, 0, 250, 255]);
        assert_eq!(&bytes[27..31], &3_000_u32.to_le_bytes());
        assert_eq!(bytes[9] & 0x04, 0x04);
        assert_eq!(&bytes[32..36], &[9, 0x45, 0x13, 0x2a]);
        assert_eq!(bytes[36], 0x80);
    }

    #[test]
    fn hid_battery_is_hidden_by_default_and_live_when_enabled() {
        let mut state = DualSenseState::default();
        let report = |state: &DualSenseState| {
            let ProviderFrame::HidInput { bytes, .. } = dualsense_hid_input_report(state) else {
                panic!("DualSense HID report must be an input frame");
            };
            bytes
        };
        // Avoid the historical all-zero status, which Linux reports as a
        // nearly empty, discharging battery.
        assert_eq!(report(&state)[52], 0x20);

        state.battery.set_exposed(true);
        state
            .battery
            .set_level(BatteryLevel::new(1).expect("battery level"));
        assert_eq!(report(&state)[52], 1, "non-zero levels round up");

        state
            .battery
            .set_level(BatteryLevel::new(57).expect("battery level"));
        assert_eq!(report(&state)[52], 6);

        state.battery.set_exposed(false);
        assert_eq!(report(&state)[52], 0x20, "live hide restores powered mode");
    }

    #[test]
    fn hid_realization_declares_dualsense_report_ids() {
        let NativeControllerRealization::Hid(realization) =
            hid_realization(RealizationSessionId(1))
        else {
            panic!("DualSense HID realization");
        };
        assert!(realization.numbered_input_reports);
        assert!(realization.numbered_output_reports);
        assert!(realization.numbered_feature_reports);
        assert_eq!(realization.descriptor.len(), 273);
        assert!(
            realization
                .descriptor
                .windows(2)
                .any(|item| item == [0x85, 0x01])
        );
        assert!(
            realization
                .descriptor
                .windows(2)
                .any(|item| item == [0x85, 0x02])
        );
        for report_id in [0x05, 0x09, 0x20] {
            let bytes = realization
                .feature_report_responses
                .get(&NativeHidReportKey {
                    report_id,
                    report_type: 0,
                })
                .expect("DualSense Linux probe reply");
            assert_eq!(bytes[0], report_id);
        }
    }

    #[test]
    fn calibration_feature_has_non_zero_motion_denominators() {
        let calibration = dualsense_feature_responses(RealizationSessionId(1))
            .remove(&NativeHidReportKey {
                report_id: 0x05,
                report_type: 0,
            })
            .expect("DualSense calibration feature");
        let sample = |offset| i16::from_le_bytes([calibration[offset], calibration[offset + 1]]);
        let speed_2x = i32::from(sample(19)) + i32::from(sample(21));
        assert_eq!(speed_2x, 4_000);
        for (plus, minus) in [(7, 9), (11, 13), (15, 17)] {
            assert_eq!(
                i32::from(sample(plus)).abs() + i32::from(sample(minus)).abs(),
                64_000
            );
            // SDL accepts DualSense hardware calibration only when its gyro
            // scale is close to 64 raw counts per degree/second.
            let speed_2x = i16::try_from(speed_2x).expect("synthetic speed fits i16");
            let scale = f32::from(speed_2x) * 1_024.0
                / (f32::from(sample(plus)) - f32::from(sample(minus)));
            assert!((scale - 64.0).abs() < f32::EPSILON);
        }
        for (plus, minus) in [(23, 25), (27, 29), (31, 33)] {
            assert_eq!(i32::from(sample(plus)) - i32::from(sample(minus)), 16_384);
        }
    }

    #[test]
    fn pairing_feature_uses_a_distinct_local_identity_per_session() {
        let pairing = |session| {
            dualsense_feature_responses(RealizationSessionId(session))
                .remove(&NativeHidReportKey {
                    report_id: 0x09,
                    report_type: 0,
                })
                .expect("pairing feature")
        };
        let first = pairing(1);
        let second = pairing(2);
        assert_eq!(first[1] & 0x03, 0x02, "locally administered unicast MAC");
        assert_ne!(&first[1..7], &second[1..7]);
    }

    #[test]
    fn motion_updates_advance_the_sensor_timestamp_and_wrap_safely() {
        let mut state = DualSenseState::default();
        advance_sensor_timestamp(&mut state);
        assert_eq!(state.sensor_timestamp, 3_000);
        state.sensor_timestamp = u32::MAX - 1_000;
        advance_sensor_timestamp(&mut state);
        assert_eq!(state.sensor_timestamp, 1_999);
    }

    #[test]
    fn known_usb_output_exposes_effect_fields_and_preserves_raw_bytes() {
        let mut raw = vec![0_u8; 47];
        raw[0..4].copy_from_slice(&[0x03, 0x15, 0x33, 0x44]);
        raw[10..21].copy_from_slice(&[1; 11]);
        raw[21..32].copy_from_slice(&[2; 11]);
        raw[8] = 1;
        raw[43..47].copy_from_slice(&[0x1f, 0x11, 0x22, 0x33]);
        assert_eq!(
            decode_dualsense_hid_output(Some(2), raw.clone()),
            DualSenseHidOutput::UsbOutput {
                raw,
                valid_flag0: 0x03,
                valid_flag1: 0x15,
                right_motor: 0x33,
                left_motor: 0x44,
                right_trigger_effect: [1; 11],
                left_trigger_effect: [2; 11],
                mute_button_led: Some(true),
                player_leds: Some(0x1f),
                lightbar_rgb: Some([0x11, 0x22, 0x33]),
            }
        );
    }

    #[test]
    fn rumble_only_output_does_not_claim_a_lightbar_or_mute_led_update() {
        let mut raw = vec![0_u8; 47];
        raw[0] = 0x01; // compatible vibration is the only valid field
        raw[2] = 0x55;
        raw[3] = 0x66;
        raw[44..47].copy_from_slice(&[0, 0, 0]);
        assert!(matches!(
            decode_dualsense_hid_output(Some(0x02), raw),
            DualSenseHidOutput::UsbOutput {
                right_motor: 0x55,
                left_motor: 0x66,
                mute_button_led: None,
                player_leds: None,
                lightbar_rgb: None,
                ..
            }
        ));
    }

    proptest! {
        #[test]
        fn touch_constructor_accepts_exact_native_domain(id in any::<u8>(), x in 0_u16..=1919, y in 0_u16..=941) {
            let contact = DualSenseTouchContact::new(id, x, y).expect("in range");
            prop_assert_eq!(contact.x(), x);
            prop_assert_eq!(contact.y(), y);
        }
    }
}
