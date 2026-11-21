//! Switch Pro Controller HID implementation for Steam/Linux hosts.
//!
//! The report mode gate and `0x21` replies follow `OpenPuck`'s USB personality;
//! console pairing and persistent SPI calibration intentionally remain out of scope.

use crate::{CreationOptions, common};
use gr_controller_contract::{
    AbsoluteAxisSurface, AuxiliaryButtonInput, ClusterPlacement, CommitError, ControlError,
    ControllerSurface, ControllerSurfaceInfo, DigitalControlSurface, DigitalControlUpdate,
    DpadCluster, DpadHoldBehavior, DpadPresentation, FaceButton, FaceButtonCluster,
    FaceButtonInput, InputAxisRange, InputControlId, InputScale, InputTopology, MotionInput,
    MotorRole, OutputKind, OutputSurface, RealizationControllerDefinition, RealizationManifest,
    RealizationManifestEntry, RealizationValidationStatus, StickInput, TargetAwareControllerDriver,
    TargetRestriction, TriggerInput, TriggerInputKind, TriggerStack,
};
mod protocol;

use gr_controller_wire::SWITCH_PRO_USB_DESCRIPTOR;
use gr_realization_api::{
    ControllerId, EvdevEvent, NativeAbsoluteAxis, NativeControllerRealization,
    NativeDeviceIdentity, NativeDummyHcdRealization, NativeEvdevRealization, NativeHidRealization,
    ProviderError, ProviderFrame, ProviderRequirements, RawReverseEvent, RealizationSelection,
    RealizationSessionId, RealizationTarget, RealizationTargetSet,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProAxis(i16);
impl SwitchProAxis {
    #[must_use]
    pub const fn new(raw: i16) -> Self {
        Self(raw)
    }
    #[must_use]
    pub const fn raw(self) -> i16 {
        self.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProMotionSample {
    pub accelerometer: [i16; 3],
    pub gyroscope: [i16; 3],
}
/// Source-supported Switch Pro battery level; no voltage thresholds are inferred.
/// Research: `swpro-l0-device-battery-connection-bits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchProBatteryLevel {
    Empty,
    Critical,
    Low,
    Medium,
    Full,
}
impl Default for SwitchProBatteryLevel {
    fn default() -> Self {
        Self::Full
    }
}

/// Native battery report status. Pro connection nibble 0 means unpowered;
/// bit zero indicates host/USB power (other controller types are out of scope).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SwitchProBattery {
    pub level: SwitchProBatteryLevel,
    pub charging: bool,
    pub usb_powered: bool,
}
impl SwitchProBattery {
    const fn wire_byte(self) -> u8 {
        let level = match self.level {
            SwitchProBatteryLevel::Empty => 0,
            SwitchProBatteryLevel::Critical => 2,
            SwitchProBatteryLevel::Low => 4,
            SwitchProBatteryLevel::Medium => 6,
            SwitchProBatteryLevel::Full => 8,
        };
        ((level | (self.charging as u8)) << 4) | (self.usb_powered as u8)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchProControl {
    B,
    A,
    Y,
    X,

    L,
    R,
    Zl,
    Zr,
    Minus,
    Plus,
    Home,
    Capture,
    LeftStickPress,
    RightStickPress,
}
/// Caller-owned native input levels, including motion measurements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchProInputs {
    face: [bool; 4],
    dpad: [bool; 4],
    left: (SwitchProAxis, SwitchProAxis),
    right: (SwitchProAxis, SwitchProAxis),
    buttons: [bool; 10],
    motion: SwitchProMotionSample,
}
impl Default for SwitchProInputs {
    fn default() -> Self {
        Self {
            face: [false; 4],
            dpad: [false; 4],
            left: (SwitchProAxis(0), SwitchProAxis(0)),
            right: (SwitchProAxis(0), SwitchProAxis(0)),
            buttons: [false; 10],
            motion: SwitchProMotionSample {
                accelerometer: [0; 3],
                gyroscope: [0; 3],
            },
        }
    }
}
const fn motion_targets() -> RealizationTargetSet {
    RealizationTargetSet::new(&[
        RealizationTarget::LINUX_UHID_USB,
        RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
    ])
}

impl SwitchProInputs {
    #[must_use]
    pub const fn left_stick(&self) -> (SwitchProAxis, SwitchProAxis) {
        self.left
    }
    #[must_use]
    pub const fn right_stick(&self) -> (SwitchProAxis, SwitchProAxis) {
        self.right
    }
    #[must_use]
    pub const fn motion(&self) -> SwitchProMotionSample {
        self.motion
    }
    /// Current accepted value of a controller-native button.
    #[must_use]
    pub const fn native_pressed(&self, control: SwitchProControl) -> bool {
        match control {
            SwitchProControl::B => self.face[0],
            SwitchProControl::A => self.face[1],
            SwitchProControl::Y => self.face[2],
            SwitchProControl::X => self.face[3],
            SwitchProControl::L => self.buttons[0],
            SwitchProControl::R => self.buttons[1],
            SwitchProControl::Zl => self.buttons[2],
            SwitchProControl::Zr => self.buttons[3],
            SwitchProControl::Minus => self.buttons[4],
            SwitchProControl::Plus => self.buttons[5],
            SwitchProControl::Home => self.buttons[6],
            SwitchProControl::Capture => self.buttons[7],
            SwitchProControl::LeftStickPress => self.buttons[8],
            SwitchProControl::RightStickPress => self.buttons[9],
        }
    }
    /// Current accepted D-pad direction, before realization-specific encoding.
    #[must_use]
    pub const fn dpad_pressed(&self, direction: gr_controller_contract::DpadDirection) -> bool {
        self.dpad[common::dpad_index(direction)]
    }
    /// Current accepted spatial face button, independent of printed labels.
    #[must_use]
    pub const fn face_pressed(&self, button: gr_controller_contract::FaceButton) -> bool {
        self.face[common::face_index(button)]
    }
    /// Set both sticks in the family's native units.
    pub fn set_sticks(
        &mut self,
        left: (SwitchProAxis, SwitchProAxis),
        right: (SwitchProAxis, SwitchProAxis),
    ) {
        self.left = left;
        self.right = right;
    }
    /// Set the motion sample in native units.
    pub fn set_motion(&mut self, motion: SwitchProMotionSample) {
        self.motion = motion;
    }
    /// Set a face button or D-pad direction by spatial position.
    pub fn set_digital(&mut self, update: DigitalControlUpdate) {
        match update {
            DigitalControlUpdate::FaceButton { button, pressed } => {
                self.face[common::face_index(button)] = pressed;
            }
            DigitalControlUpdate::Dpad { direction, pressed } => {
                self.dpad[common::dpad_index(direction)] = pressed;
            }
        }
    }
    /// Set the level of a controller-native button.
    pub fn set_native(&mut self, c: SwitchProControl, p: bool) {
        match c {
            SwitchProControl::B => self.face[0] = p,
            SwitchProControl::A => self.face[1] = p,
            SwitchProControl::Y => self.face[2] = p,
            SwitchProControl::X => self.face[3] = p,
            SwitchProControl::L => self.buttons[0] = p,
            SwitchProControl::R => self.buttons[1] = p,
            SwitchProControl::Zl => self.buttons[2] = p,
            SwitchProControl::Zr => self.buttons[3] = p,
            SwitchProControl::Minus => self.buttons[4] = p,
            SwitchProControl::Plus => self.buttons[5] = p,
            SwitchProControl::Home => self.buttons[6] = p,
            SwitchProControl::Capture => self.buttons[7] = p,
            SwitchProControl::LeftStickPress => self.buttons[8] = p,
            SwitchProControl::RightStickPress => self.buttons[9] = p,
        }
    }
}
/// Accepted motion clock and live host-selected stream state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SwitchProStatus {
    timer: u8,
    stream_enabled: bool,
    battery: SwitchProBattery,
    imu_gating: bool,
    imu_enabled: bool,
}
impl SwitchProStatus {
    /// Accepted motion clock; the transport has a separate report counter.
    #[must_use]
    pub const fn timer(self) -> u8 {
        self.timer
    }
    #[must_use]
    pub const fn stream_enabled(self) -> bool {
        self.stream_enabled
    }
    /// Current caller-selected battery and Pro power status.
    #[must_use]
    pub const fn battery(self) -> SwitchProBattery {
        self.battery
    }
    /// Whether source-only IMU gating was selected at creation.
    #[must_use]
    pub const fn imu_gating(self) -> bool {
        self.imu_gating
    }
    /// Latest accepted 0x40 IMU enable state.
    #[must_use]
    pub const fn imu_enabled(self) -> bool {
        self.imu_enabled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProSurface {
    common: ControllerSurface,
}
impl ControllerSurfaceInfo for SwitchProSurface {
    fn common_surface(&self) -> &ControllerSurface {
        &self.common
    }
}
impl SwitchProSurface {
    #[must_use]
    pub const fn common(&self) -> &ControllerSurface {
        &self.common
    }
}
static DIGITAL: [DigitalControlSurface; 14] = [
    DigitalControlSurface {
        control: "b",
        event_code: 304,
    },
    DigitalControlSurface {
        control: "a",
        event_code: 305,
    },
    DigitalControlSurface {
        control: "y",
        event_code: 308,
    },
    DigitalControlSurface {
        control: "x",
        event_code: 307,
    },
    DigitalControlSurface {
        control: "l",
        event_code: 310,
    },
    DigitalControlSurface {
        control: "r",
        event_code: 311,
    },
    DigitalControlSurface {
        control: "zl",
        event_code: 312,
    },
    DigitalControlSurface {
        control: "zr",
        event_code: 313,
    },
    DigitalControlSurface {
        control: "minus",
        event_code: 314,
    },
    DigitalControlSurface {
        control: "plus",
        event_code: 315,
    },
    DigitalControlSurface {
        control: "home",
        event_code: 316,
    },
    DigitalControlSurface {
        control: "capture",
        event_code: 309,
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
static AXES: [AbsoluteAxisSurface; 6] = [
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
static NATIVE_OUTPUT_KINDS: &[OutputKind] = &[
    OutputKind::Rumble {
        motors: &[MotorRole::Left, MotorRole::Right],
    },
    OutputKind::PlayerIndicator { count: 4 },
];
static EVDEV_OUTPUT_KINDS: &[OutputKind] = &[OutputKind::Rumble {
    motors: &[MotorRole::Strong, MotorRole::Weak],
}];
static OUTPUTS: [OutputSurface; 1] = [OutputSurface {
    name: "switch-hd-rumble",
    event_type: 0,
    event_code: 0x10,
}];
static RESTRICTIONS: [TargetRestriction; 1] = [TargetRestriction {
    feature: "console pairing",
    reason: "Steam/Linux mode deliberately omits Switch-console pairing and SPI calibration persistence",
}];
static EVDEV_RESTRICTIONS: [TargetRestriction; 5] = [
    common::FEEDBACK_RESTRICTION,
    TargetRestriction {
        feature: "HD rumble",
        reason: "evdev FF_RUMBLE represents two magnitudes, not Switch frequency/amplitude encoding",
    },
    TargetRestriction {
        feature: "motion",
        reason: "evdev has no faithful Switch Pro IMU presentation",
    },
    TargetRestriction {
        feature: "host-visible controller address",
        reason: "uinput has neither a unique-ID setter nor a USB controller-info channel",
    },
    RESTRICTIONS[0],
];
const STICK_AXIS: InputAxisRange = InputAxisRange {
    minimum: i16::MIN as i32,
    maximum: i16::MAX as i32,
    neutral: 0,
};
const MOTION_AXIS: InputAxisRange = InputAxisRange {
    minimum: i16::MIN as i32,
    maximum: i16::MAX as i32,
    neutral: 0,
};
static INPUT_AUXILIARY: [AuxiliaryButtonInput; 4] = [
    AuxiliaryButtonInput {
        id: InputControlId::new("minus"),
        label: "Minus",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("plus"),
        label: "Plus",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("home"),
        label: "Home",
    },
    AuxiliaryButtonInput {
        id: InputControlId::new("capture"),
        label: "Capture",
    },
];
static INPUT_FACE_BUTTONS: [FaceButtonInput; 4] = [
    FaceButtonInput {
        button: FaceButton::South,
        label: "B",
        placement: ClusterPlacement::SOUTH,
    },
    FaceButtonInput {
        button: FaceButton::East,
        label: "A",
        placement: ClusterPlacement::EAST,
    },
    FaceButtonInput {
        button: FaceButton::West,
        label: "Y",
        placement: ClusterPlacement::WEST,
    },
    FaceButtonInput {
        button: FaceButton::North,
        label: "X",
        placement: ClusterPlacement::NORTH,
    },
];
static INPUT_FACE_CLUSTERS: [FaceButtonCluster; 1] = [
    gr_controller_contract::construction::FaceButtonClusterSpec {
        id: InputControlId::new("face"),
        title: "Face buttons",

        buttons: &INPUT_FACE_BUTTONS,
    }
    .build(),
];
static INPUT_DPADS: [DpadCluster; 1] = [gr_controller_contract::construction::DpadClusterSpec {
    id: InputControlId::new("dpad"),
    title: "D-pad",
    presentation: DpadPresentation::IndependentButtons,
    hold_behavior: DpadHoldBehavior::AdjacentPair,
}
.build()];
static INPUT_STICKS: [StickInput; 2] = [
    gr_controller_contract::construction::StickInputSpec {
        id: InputControlId::new("left-stick"),
        title: "Left stick",
        x: STICK_AXIS,
        y: STICK_AXIS,
        press: Some(AuxiliaryButtonInput {
            id: InputControlId::new("left-stick-press"),
            label: "Stick press",
        }),
        capacitive: None,
    }
    .build(),
    gr_controller_contract::construction::StickInputSpec {
        id: InputControlId::new("right-stick"),
        title: "Right stick",
        x: STICK_AXIS,
        y: STICK_AXIS,
        press: Some(AuxiliaryButtonInput {
            id: InputControlId::new("right-stick-press"),
            label: "Stick press",
        }),
        capacitive: None,
    }
    .build(),
];
static LEFT_TRIGGER_CONTROLS: [TriggerInput; 2] = [
    TriggerInput {
        label: "L",
        kind: TriggerInputKind::Button {
            id: InputControlId::new("l"),
        },
    },
    TriggerInput {
        label: "ZL",
        kind: TriggerInputKind::Button {
            id: InputControlId::new("zl"),
        },
    },
];
static RIGHT_TRIGGER_CONTROLS: [TriggerInput; 2] = [
    TriggerInput {
        label: "R",
        kind: TriggerInputKind::Button {
            id: InputControlId::new("r"),
        },
    },
    TriggerInput {
        label: "ZR",
        kind: TriggerInputKind::Button {
            id: InputControlId::new("zr"),
        },
    },
];
static INPUT_TRIGGER_STACKS: [TriggerStack; 2] = [
    gr_controller_contract::construction::TriggerStackSpec {
        id: InputControlId::new("left-trigger-stack"),
        title: "Left trigger stack",
        controls: &LEFT_TRIGGER_CONTROLS,
    }
    .build(),
    gr_controller_contract::construction::TriggerStackSpec {
        id: InputControlId::new("right-trigger-stack"),
        title: "Right trigger stack",
        controls: &RIGHT_TRIGGER_CONTROLS,
    }
    .build(),
];
static INPUT_MOTION: [MotionInput; 1] = [gr_controller_contract::construction::MotionInputSpec {
    id: InputControlId::new("motion"),
    title: "Motion",
    range: MOTION_AXIS,
    gyroscope_scale: [InputScale::IDENTITY; 3],
    accelerometer_scale: [InputScale::IDENTITY; 3],
}
.build()];
static INPUT_TOPOLOGY: InputTopology = gr_controller_contract::construction::InputTopologySpec {
    auxiliary_buttons: &INPUT_AUXILIARY,
    face_button_clusters: &INPUT_FACE_CLUSTERS,
    dpads: &INPUT_DPADS,
    sticks: &INPUT_STICKS,
    trigger_stacks: &INPUT_TRIGGER_STACKS,
    touchpads: &[],
    motion: &[],
    extra_axes: &[],
    custom_modules: &[],
    typed_inputs: &[],
}
.build();
static INPUT_TOPOLOGY_WITH_MOTION: InputTopology =
    gr_controller_contract::construction::InputTopologySpec {
        motion: &INPUT_MOTION,
        ..gr_controller_contract::construction::InputTopologySpec::from_descriptor(INPUT_TOPOLOGY)
    }
    .build();
static EVDEV_SURFACE: SwitchProSurface = SwitchProSurface {
    common: gr_controller_contract::construction::ControllerSurfaceSpec {
        target: RealizationTarget::LINUX_UINPUT,
        validation_status: RealizationValidationStatus::HostValidated,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &common::CONVENTIONAL_RUMBLE,
        output_kinds: EVDEV_OUTPUT_KINDS,
        restrictions: &EVDEV_RESTRICTIONS,
        input_topology: &INPUT_TOPOLOGY,
    }
    .build(),
};
static HID_SURFACE: SwitchProSurface = SwitchProSurface {
    common: gr_controller_contract::construction::ControllerSurfaceSpec {
        target: RealizationTarget::LINUX_UHID_USB,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        output_kinds: NATIVE_OUTPUT_KINDS,
        restrictions: &RESTRICTIONS,
        input_topology: &INPUT_TOPOLOGY_WITH_MOTION,
    }
    .build(),
};
static RETAINED_DUMMY: [TargetRestriction; 2] =
    [RESTRICTIONS[0], crate::retained::DUMMY_RESTRICTION];
static USB_SURFACE: SwitchProSurface = SwitchProSurface {
    common: gr_controller_contract::construction::ControllerSurfaceSpec {
        target: RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &DIGITAL,
        axes: &AXES,
        outputs: &OUTPUTS,
        output_kinds: NATIVE_OUTPUT_KINDS,
        restrictions: &RETAINED_DUMMY,
        input_topology: &INPUT_TOPOLOGY_WITH_MOTION,
    }
    .build(),
};
pub struct SwitchProDefinition;
impl RealizationControllerDefinition for SwitchProDefinition {
    fn controller_id(&self) -> ControllerId {
        ControllerId::new("virtualgamepad.switch-pro")
    }
    fn realization_manifest(&self) -> RealizationManifest {
        static E: [RealizationManifestEntry; 3] = [
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UINPUT,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UHID_USB,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(&E)
    }
}
impl TargetAwareControllerDriver for SwitchProDefinition {
    type State = (SwitchProInputs, SwitchProStatus);
    type Frame = ProviderFrame;
    fn neutral_state(&self) -> Self::State {
        <(SwitchProInputs, SwitchProStatus)>::default()
    }
    fn apply_digital(
        &self,
        s: &mut Self::State,
        u: DigitalControlUpdate,
    ) -> Result<(), ControlError> {
        s.0.set_digital(u);
        Ok(())
    }
    fn validate_state(
        &self,
        sel: RealizationSelection,
        state: &Self::State,
    ) -> Result<(), ControlError> {
        if !matches!(
            sel.target,
            RealizationTarget::LINUX_UINPUT
                | RealizationTarget::LINUX_UHID_USB
                | RealizationTarget::LINUX_DUMMY_HCD_USB_HID
        ) {
            return Err(common::unavailable(sel.target));
        }
        if sel.target == RealizationTarget::LINUX_UINPUT {
            common::validate_hat(state.0.dpad)?;
        }
        if state.0.motion != SwitchProInputs::default().motion {
            common::require_input_target(sel.target, motion_targets())?;
        }
        Ok(())
    }
    fn encode(
        &self,
        sel: RealizationSelection,
        s: &Self::State,
    ) -> Result<ProviderFrame, ControlError> {
        if sel.target == RealizationTarget::LINUX_UINPUT {
            return Ok(switch_evdev_frame(s));
        }
        let ProviderFrame::HidInput { report_id, bytes } = switch_frame(s) else {
            unreachable!()
        };
        if sel.target == RealizationTarget::LINUX_UHID_USB {
            Ok(ProviderFrame::HidInput { report_id, bytes })
        } else {
            let Some(report_id) = report_id else {
                unreachable!("Switch USB reports are numbered")
            };
            let mut wire = Vec::with_capacity(bytes.len() + 1);
            wire.push(report_id);
            wire.extend_from_slice(&bytes);
            Ok(ProviderFrame::DummyHcdInput(wire))
        }
    }
}
fn switch_evdev_frame(state: &(SwitchProInputs, SwitchProStatus)) -> ProviderFrame {
    let mut events = Vec::with_capacity(22);
    for (code, pressed) in [304, 305, 308, 307].into_iter().zip(state.0.face) {
        events.push(EvdevEvent {
            event_type: common::EV_KEY,
            code,
            value: i32::from(pressed),
        });
    }
    for (code, pressed) in [310, 311, 312, 313, 314, 315, 316, 309, 317, 318]
        .into_iter()
        .zip(state.0.buttons)
    {
        events.push(EvdevEvent {
            event_type: common::EV_KEY,
            code,
            value: i32::from(pressed),
        });
    }
    for (code, value) in [
        (0, i32::from(state.0.left.0.raw())),
        (1, i32::from(state.0.left.1.raw())),
        (3, i32::from(state.0.right.0.raw())),
        (4, i32::from(state.0.right.1.raw())),
        (16, i32::from(state.0.dpad[3]) - i32::from(state.0.dpad[2])),
        (17, i32::from(state.0.dpad[1]) - i32::from(state.0.dpad[0])),
    ] {
        events.push(EvdevEvent {
            event_type: common::EV_ABS,
            code,
            value,
        });
    }
    events.push(EvdevEvent {
        event_type: common::EV_SYN,
        code: common::SYN_REPORT,
        value: 0,
    });
    ProviderFrame::Evdev(events)
}
fn stick(v: i16) -> u16 {
    // The Switch wire format is twelve-bit (0..=4095), not a truncated
    // sixteen-bit Linux axis.  Truncating the latter made center land at zero
    // and produced erratic stick positions.
    u16::try_from(((i32::from(v) + 32_768) * 4095 + 32_767) / 65_535)
        .expect("scaled Switch stick domain")
}
#[allow(clippy::cast_possible_truncation)] // Packing intentionally takes low eight/four bits.
fn pack(out: &mut [u8], x: i16, y: i16) {
    let x = stick(x);
    let y = stick(y);
    out[0] = x as u8;
    out[1] = ((x >> 8) as u8 & 0x0f) | ((y as u8 & 0x0f) << 4);
    out[2] = (y >> 4) as u8;
}
fn switch_frame(s: &(SwitchProInputs, SwitchProStatus)) -> ProviderFrame {
    let mut b = vec![0; 63];
    b[0] = s.1.timer;
    b[1] = s.1.battery.wire_byte();
    // `FaceButton` follows the host gamepad layout, so its South/East controls
    // map to Nintendo B/A respectively (the printed labels are reversed).
    b[2] = u8::from(s.0.face[2])
        | (u8::from(s.0.face[3]) << 1)
        | (u8::from(s.0.face[0]) << 2)
        | (u8::from(s.0.face[1]) << 3)
        | (u8::from(s.0.buttons[1]) << 6)
        | (u8::from(s.0.buttons[3]) << 7);
    b[3] = u8::from(s.0.buttons[4])
        | (u8::from(s.0.buttons[5]) << 1)
        | (u8::from(s.0.buttons[9]) << 2)
        | (u8::from(s.0.buttons[8]) << 3)
        | (u8::from(s.0.buttons[6]) << 4)
        | (u8::from(s.0.buttons[7]) << 5);
    b[4] = u8::from(s.0.dpad[1])
        | (u8::from(s.0.dpad[0]) << 1)
        | (u8::from(s.0.dpad[3]) << 2)
        | (u8::from(s.0.dpad[2]) << 3)
        | (u8::from(s.0.buttons[0]) << 6)
        | (u8::from(s.0.buttons[2]) << 7);
    // hid-nintendo negates the packed Switch Y axes before publishing Linux
    // `ABS_Y`/`ABS_RY`. The public API follows that Linux convention (positive
    // means down), so compensate only at the Switch wire boundary.
    pack(
        &mut b[5..8],
        s.0.left.0.raw(),
        s.0.left.1.raw().saturating_neg(),
    );
    pack(
        &mut b[8..11],
        s.0.right.0.raw(),
        s.0.right.1.raw().saturating_neg(),
    );
    b[11] = 9;
    // Optional source-only compatibility gate; default keeps the prior raw
    // samples even before the host's 0x40 command.
    if s.1.imu_gating && !s.1.imu_enabled {
        return ProviderFrame::HidInput {
            report_id: Some(0x30),
            bytes: b,
        };
    }
    let ax = s.0.motion.accelerometer[1] / 4;
    let ay = s.0.motion.accelerometer[0].wrapping_neg() / 4;
    let az = s.0.motion.accelerometer[2] / 4;
    let gyro = [
        s.0.motion.gyroscope[1],
        s.0.motion.gyroscope[0].wrapping_neg(),
        s.0.motion.gyroscope[2],
    ];
    for sample in 0..3 {
        let o = 12 + sample * 12;
        for (i, v) in [ax, ay, az, gyro[0], gyro[1], gyro[2]]
            .into_iter()
            .enumerate()
        {
            b[o + i * 2..o + i * 2 + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    ProviderFrame::HidInput {
        report_id: Some(0x30),
        bytes: b,
    }
}

fn switch_input_prefix(state: &(SwitchProInputs, SwitchProStatus)) -> [u8; 12] {
    let ProviderFrame::HidInput { bytes, .. } = switch_frame(state) else {
        unreachable!("Switch frame is always HID input")
    };
    bytes[..12].try_into().expect("Switch input prefix length")
}

fn switch_usb_reply(command: u8, identity: SwitchProIdentity) -> ProviderFrame {
    let mut bytes = vec![0; 63];
    bytes[0] = command;
    if command == 1 {
        bytes[2] = 3; // Pro Controller
        // audio-sdl-switch-pro (SDL 535d80b, BReadDeviceInfo, lines 633–652).
        // swpro-l0-consumer-sdl-usb-status-address-order records the reversal.
        let mut address = identity.bytes();
        address.reverse();
        bytes[3..9].copy_from_slice(&address);
    }
    ProviderFrame::HidInput {
        report_id: Some(0x81),
        bytes,
    }
}

fn switch_spi_byte(address: u32) -> u8 {
    const IMU_CAL: [u8; 24] = [
        0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0x40, 0, 0x40, 0, 0, 0, 0, 0, 0, 0x3b, 0x34, 0x3b, 0x34,
        0x3b, 0x34,
    ];
    // OpenPuck's neutral 12-bit calibration, packed as six values per stick.
    const STICK_CAL: [u8; 18] = [
        0x08, 0x87, 0x70, 0x00, 0x08, 0x80, 0x08, 0x87, 0x70, 0x00, 0x08, 0x80, 0x08, 0x87, 0x70,
        0x08, 0x87, 0x70,
    ];
    match address {
        0x6020..=0x6037 => IMU_CAL[(address - 0x6020) as usize],
        0x603d..=0x604e => STICK_CAL[(address - 0x603d) as usize],
        // This includes colors (0x6050..0x605b): no source-backed retail set is recorded.
        _ => 0xff,
    }
}

fn switch_subcommand_reply(
    state: &(SwitchProInputs, SwitchProStatus),
    subcommand: u8,
    arguments: &[u8],
    identity: SwitchProIdentity,
) -> (ProviderFrame, bool) {
    let mut bytes = vec![0; 63];
    bytes[..12].copy_from_slice(&switch_input_prefix(state));
    bytes[13] = subcommand;
    let mut enable_stream = false;
    match subcommand {
        0x02 => {
            bytes[12] = 0x82;
            bytes[14..18].copy_from_slice(&[3, 0x48, 3, 2]);
            // swpro-l0-device-info02-fields, swpro-l0-consumer-sdl-mac-serial-guid,
            // swpro-l0-consumer-linux-mac-uniq: device-info uses display order.
            bytes[18..24].copy_from_slice(&identity.bytes());
            bytes[24] = 1;
            bytes[25] = 0; // Colors are not available in SPI.
        }
        0x10 if arguments.len() >= 5 => {
            bytes[12] = 0x90;
            bytes[14..19].copy_from_slice(&arguments[..5]);
            let address =
                u32::from_le_bytes(arguments[..4].try_into().expect("four-byte SPI address"));
            for (index, byte) in bytes[19..]
                .iter_mut()
                .take(usize::from(arguments[4]))
                .enumerate()
            {
                *byte = switch_spi_byte(
                    address.wrapping_add(u32::try_from(index).expect("SPI reply index fits u32")),
                );
            }
        }
        0x03 if arguments.first() == Some(&0x30) => {
            bytes[12] = 0x80;
            enable_stream = true;
        }
        0x04 => {
            bytes[12] = 0x83;
            bytes[14..20].copy_from_slice(&[0, 0xcc, 0, 0xee, 0, 0xff]);
        }
        0x21 => bytes[12] = 0xa0,
        _ => bytes[12] = 0x80,
    }
    (
        ProviderFrame::HidInput {
            report_id: Some(0x21),
            bytes,
        },
        enable_stream,
    )
}
fn hid(identity: SwitchProIdentity) -> NativeControllerRealization {
    NativeControllerRealization::Uhid(NativeHidRealization {
        target: RealizationTarget::LINUX_UHID_USB,
        bus_type: 3,
        device_name: "Pro Controller".into(),
        // `common::create` appends the realization session exactly once.
        physical_path: "virtualgamepad/uhid/switch-pro".into(),
        unique_id: common::address_label(identity.bytes()),
        identity: NativeDeviceIdentity {
            vendor_id: 0x057e,
            product_id: 0x2009,
            // OpenPuck and physical USB Pro Controllers advertise 2.20.
            version: 0x0220,
        },
        descriptor: SWITCH_PRO_USB_DESCRIPTOR.to_vec(),
        numbered_input_reports: true,
        numbered_output_reports: true,
        numbered_feature_reports: false,
    })
}
fn evdev_realization() -> NativeControllerRealization {
    NativeControllerRealization::Evdev(NativeEvdevRealization {
        physical_path: None,
        device_name: "Pro Controller".into(),
        identity: NativeDeviceIdentity {
            vendor_id: 0x057e,
            product_id: 0x2009,
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
/// Caller-restorable Switch Pro address in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProIdentity([u8; 6]);

impl SwitchProIdentity {
    pub fn from_bytes(address: [u8; 6]) -> Result<Self, ProviderError> {
        common::validate_address(address, "Switch Pro").map(Self)
    }

    pub fn generate() -> Result<Self, ProviderError> {
        common::creation_identity().map(Self)
    }

    #[must_use]
    pub const fn bytes(self) -> [u8; 6] {
        self.0
    }
}

pub struct SwitchProController(
    common::ControllerSession<SwitchProDefinition>,
    SwitchProIdentity,
);
impl SwitchProController {
    #[must_use]
    pub const fn identity(&self) -> SwitchProIdentity {
        self.1
    }

    /// Latest accepted host values and revision, retained after close. USB
    /// workers are unavailable: their lossy event channel has no recovery query.
    pub fn host_outputs(&self) -> Result<crate::retained::SwitchProOutputState, ControlError> {
        self.0.retained_outputs()
    }

    #[must_use]
    pub fn stream_enabled(&self) -> bool {
        self.0
            .optional_protocol()
            .map_or(self.0.state().1.stream_enabled, |p| p.stream_enabled)
    }
    #[must_use]
    pub fn battery(&self) -> SwitchProBattery {
        self.0.state().1.battery
    }
    /// Live caller battery metadata; updates the subsequent 0x30 and 0x21 prefixes.
    pub fn set_battery(&mut self, battery: SwitchProBattery) -> Result<(), ControlError> {
        self.0.update_state(|state| {
            state.1.battery = battery;
            Ok(())
        })
    }
    #[must_use]
    pub fn motion_report_counter(&self) -> u8 {
        self.0
            .optional_protocol()
            .map_or(self.0.state().1.timer, |p| p.timer)
    }

    /// Whether the HID readiness descriptor should also be watched for writability.
    #[must_use]
    pub fn wants_write(&self) -> bool {
        self.0.wants_write()
    }

    /// Service on this readiness source and at `next_service_in`, including idle state.
    #[must_use]
    pub fn readiness(&self) -> Option<gr_hid::Readiness> {
        self.0.readiness()
    }
    #[must_use]
    pub fn next_service_in(&self) -> Option<std::time::Duration> {
        self.0.next_service_in()
    }
    /// Count of bounded optional output notifications evicted by slow consumption.
    #[must_use]
    pub fn dropped_output_events(&self) -> u64 {
        self.0.dropped_observations()
    }

    #[must_use]
    pub const fn inputs(&self) -> &SwitchProInputs {
        &self.0.state().0
    }
    /// Accepted motion clock and live host-selected stream state.
    #[must_use]
    pub fn status(&self) -> SwitchProStatus {
        SwitchProStatus {
            stream_enabled: self.stream_enabled(),
            imu_enabled: self.0.optional_protocol().map_or(self.0.state().1.imu_enabled, |p| p.imu_enabled),
            ..self.0.state().1
        }
    }
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.0.is_dirty()
    }
    #[must_use]
    pub fn surface(&self) -> &'static SwitchProSurface {
        match self.0.selection().target {
            RealizationTarget::LINUX_UINPUT => &EVDEV_SURFACE,
            RealizationTarget::LINUX_UHID_USB => &HID_SURFACE,
            _ => &USB_SURFACE,
        }
    }
    /// Edit an isolated native input candidate as one transaction.
    /// Only the final candidate is validated. Rejection preserves accepted input,
    /// clocks, dirty work and queued reports. An unchanged edit adds no dirty work.
    /// Keep external side effects outside the closure. Call `commit()` to deliver.
    ///
    /// # Errors
    /// Returns the closure's error, a validation error, or `ControlError::Closed`.
    pub fn edit_inputs(
        &mut self,
        edit: impl FnOnce(&mut SwitchProInputs) -> Result<(), ControlError>,
    ) -> Result<(), ControlError> {
        self.0.edit_inputs(|state| edit(&mut state.0))
    }
    /// Replace the complete native input state as one validated edit.
    /// Call `commit()` to deliver it. Protocol counters are session-owned.
    ///
    /// # Errors
    /// Returns a control error if the session is closed or the snapshot cannot
    /// be represented by the selected realization.
    pub fn set_inputs(&mut self, state: SwitchProInputs) -> Result<(), ControlError> {
        self.edit_inputs(|current| {
            *current = state;
            Ok(())
        })
    }
    pub fn set_digital(&mut self, u: DigitalControlUpdate) -> Result<(), ControlError> {
        self.0.apply_digital(u)
    }
    pub fn set_native(&mut self, c: SwitchProControl, p: bool) -> Result<(), ControlError> {
        self.edit_inputs(|s| {
            s.set_native(c, p);
            Ok(())
        })
    }
    pub fn set_left_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), ControlError> {
        self.edit_inputs(|state| {
            state.left = (x, y);
            Ok(())
        })
    }
    pub fn set_right_stick(
        &mut self,
        x: SwitchProAxis,
        y: SwitchProAxis,
    ) -> Result<(), ControlError> {
        self.edit_inputs(|state| {
            state.right = (x, y);
            Ok(())
        })
    }
    pub fn set_motion(&mut self, m: SwitchProMotionSample) -> Result<(), ControlError> {
        let target = self.0.selection().target;
        self.edit_inputs(|state| {
            common::require_input_target(target, motion_targets())?;
            state.set_motion(m);
            Ok(())
        })
    }
    /// Release inputs as one accepted edit; call `commit()` to deliver it.
    /// Preserves identity, battery metadata and host outputs. Changed inputs advance clocks.
    pub fn neutralize(&mut self) -> Result<(), ControlError> {
        self.0.neutralize()
    }
    /// Requested component labels and the cached creation-time host observation.
    #[must_use]
    pub fn association(&self) -> &crate::ControllerAssociation {
        self.0.association()
    }

    /// Current transport and retained cleanup diagnostics.
    pub fn provider_diagnostics(&mut self) -> gr_realization_api::ProviderDiagnostics {
        self.0.diagnostics()
    }
    pub fn commit(&mut self) -> Result<(), CommitError> {
        self.0.commit()
    }
    pub fn refresh_motion(&mut self) -> Result<(), CommitError> {
        if self.0.selection().target == RealizationTarget::LINUX_UHID_USB {
            return self.0.commit();
        }
        if self.0.state().1.stream_enabled {
            self.0
                .update_state(|s| {
                    s.1.timer = s.1.timer.wrapping_add(1);
                    Ok(())
                })
                .map_err(|e| CommitError::Backend {
                    reason: e.to_string(),
                })?;
            self.0.commit()
        } else {
            Ok(())
        }
    }
    pub fn close(&mut self) {
        self.0.close();
    }
    /// Compatibility alias for [`Self::service`]; this performs required protocol work.
    pub fn poll_output(
        &mut self,
        callback: &mut dyn FnMut(SwitchProOutputEvent),
    ) -> Result<(), ProviderError> {
        self.service(callback)
    }

    /// Service protocol I/O, including while input state is unchanged.
    ///
    /// Call on [`Self::readiness`] and at [`Self::next_service_in`], watching
    /// writability when [`Self::wants_write`] is true. Recompute interest after
    /// each call. `commit` does not replace idle servicing.
    /// Required curated HID/evdev replies are processed before optional output
    /// callbacks. Callbacks must return promptly to permit the next service cycle.
    /// See the crate-level scheduling contract. No thread or executor is started.
    pub fn service(
        &mut self,
        callback: &mut dyn FnMut(SwitchProOutputEvent),
    ) -> Result<(), ProviderError> {
        if self.0.selection().target == RealizationTarget::LINUX_UHID_USB {
            return self
                .0
                .drain(&mut |event| callback(SwitchProOutputEvent::from(event)));
        }
        let mut events = Vec::new();
        self.0
            .with_sink(|sink| sink.drain(&mut |event| events.push(event)))?;
        let mut enable = false;
        let mut requested_imu = None;
        let state = self.0.state().clone();
        let mut replies = Vec::new();
        for event in events {
            match &event {
                RawReverseEvent::HidOutput {
                    report_id: Some(0x80),
                    bytes,
                } if !bytes.is_empty() => replies.push(switch_usb_reply(bytes[0], self.1)),
                RawReverseEvent::HidOutput {
                    report_id: Some(1),
                    bytes,
                } if bytes.len() >= 10 => {
                    let (reply, stream) =
                        switch_subcommand_reply(&state, bytes[9], &bytes[10..], self.1);
                    replies.push(reply);
                    enable |= stream;
                    if bytes[9] == 0x40 {
                        requested_imu = bytes.get(10).map(|value| *value != 0);
                    }
                }
                RawReverseEvent::HidSetReportRequest { request_id, .. } => {
                    replies.push(ProviderFrame::HidSetReportReply {
                        request_id: *request_id,
                        status: 0,
                    });
                }
                _ => {}
            }
            callback(SwitchProOutputEvent::from(event));
        }
        let target = self.0.selection().target;
        self.0.with_sink(|sink| {
            for reply in replies {
                sink.reply(dummy_hcd_reply(target, reply)?)?;
            }
            Ok::<(), ProviderError>(())
        })?;
        if enable || requested_imu.is_some() {
            self.0
                .update_state(|s| {
                    if enable {
                        s.1.stream_enabled = true;
                    }
                    if let Some(enabled) = requested_imu {
                        s.1.imu_enabled = enabled;
                    }
                    Ok(())
                })
                .map_err(|e| ProviderError::Open {
                    reason: e.to_string(),
                })?;
        }
        Ok(())
    }
}

fn dummy_hcd_reply(
    target: RealizationTarget,
    frame: ProviderFrame,
) -> Result<ProviderFrame, ProviderError> {
    if target != RealizationTarget::LINUX_DUMMY_HCD_USB_HID {
        return Ok(frame);
    }
    let ProviderFrame::HidInput {
        report_id: Some(report_id),
        bytes,
    } = frame
    else {
        return Err(ProviderError::Unsupported {
            reason: "Switch dummy_hcd can reply only with numbered USB input reports".into(),
        });
    };
    let mut wire = Vec::with_capacity(bytes.len() + 1);
    wire.push(report_id);
    wire.extend_from_slice(&bytes);
    Ok(ProviderFrame::DummyHcdInput(wire))
}
/// Original motor words from one Switch host output; not physical actuator measurements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProRumble {
    pub packet_counter: u8,
    pub left: [u8; 4],
    pub right: [u8; 4],
}

/// A band's source-derived motor intent. Missing values require earlier motor state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProRumbleBand {
    frequency_step: Option<i16>,
    amplitude_step: Option<i16>,
    center_hz: u16,
}

impl SwitchProRumbleBand {
    /// Frequency in Hz, or None when the previous frequency is unknown.
    #[must_use]
    pub fn frequency_hz(&self) -> Option<f32> {
        self.frequency_step
            .map(|step| f32::from(self.center_hz) * (f32::from(step) / 32.0).exp2())
    }

    /// Dimensionless `MissionControl` compatibility ratio (absolute index 127 = 1).
    /// This is not acceleration, displacement, force, or a safe-drive guarantee.
    /// None means an earlier amplitude is required.
    #[must_use]
    pub fn amplitude_ratio(&self) -> Option<f32> {
        self.amplitude_step.map(|step| {
            if step < -254 {
                0.0
            } else {
                (f32::from(step) / 32.0).exp2()
            }
        })
    }

    fn unknown(center_hz: u16) -> Self {
        Self {
            frequency_step: None,
            amplitude_step: None,
            center_hz,
        }
    }

    fn set_frequency(&mut self, index: i16) {
        self.frequency_step = Some(index - 64);
    }

    fn set_amplitude(&mut self, index: i16) {
        self.amplitude_step = Some(match index {
            0 => -256,
            1..=15 => 8 * index - 248,
            16..=31 => 2 * index - 158,
            _ => index - 127,
        });
    }

    fn relative(&mut self, command: i16) {
        match command {
            0 => {
                self.frequency_step = Some(0);
                self.amplitude_step = Some(-256);
            }
            1..=11 => self.amplitude_step = Some(-16 * (command - 1)),
            12..=16 => self.frequency_step = Some(6 * (command - 14)),
            17..=31 => {
                let index = command - 17;
                let amplitude_delta = match index / 3 {
                    0 => 4,
                    1 => 1,
                    2 => 0,
                    3 => -1,
                    _ => -4,
                };
                let frequency_delta = 1 - index % 3;
                self.amplitude_step = self
                    .amplitude_step
                    .map(|step| (step + amplitude_delta).clamp(-256, 0));
                self.frequency_step = self
                    .frequency_step
                    .map(|step| (step + frequency_delta).clamp(-64, 64));
            }
            _ => unreachable!("relative command is a five-bit field"),
        }
    }
}

/// One ordered sample of a single motor's high- and low-band intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProRumbleSample {
    high: SwitchProRumbleBand,
    low: SwitchProRumbleBand,
}

impl SwitchProRumbleSample {
    #[must_use]
    pub const fn high(&self) -> SwitchProRumbleBand {
        self.high
    }

    #[must_use]
    pub const fn low(&self) -> SwitchProRumbleBand {
        self.low
    }

    fn unknown() -> Self {
        Self {
            high: SwitchProRumbleBand::unknown(320),
            low: SwitchProRumbleBand::unknown(160),
        }
    }

    fn relative_pair(&mut self, high: i16, low: i16) {
        self.high.relative(high);
        self.low.relative(low);
    }
}

/// Bounded source-supported sample sequences, or an unpinned encoding.
/// A recognized relative sample may still contain unknown individual values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SwitchProRumbleSamples {
    NoSamples,
    One([SwitchProRumbleSample; 1]),
    Two([SwitchProRumbleSample; 2]),
    Three([SwitchProRumbleSample; 3]),
    Unknown,
}

impl SwitchProRumbleSamples {
    /// None for an unpinned encoding; an empty slice for a no-sample word.
    #[must_use]
    pub fn as_slice(&self) -> Option<&[SwitchProRumbleSample]> {
        match self {
            Self::NoSamples => Some(&[]),
            Self::One(samples) => Some(samples),
            Self::Two(samples) => Some(samples),
            Self::Three(samples) => Some(samples),
            Self::Unknown => None,
        }
    }
}

/// One motor word, including its unchanged bytes even when decoding is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProRumbleWord {
    pub raw: [u8; 4],
    pub samples: SwitchProRumbleSamples,
}

/// The two independently decoded motors of a host output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProRumbleDecoded {
    pub left: SwitchProRumbleWord,
    pub right: SwitchProRumbleWord,
}

impl SwitchProRumble {
    /// Decode without assuming any previous motor intent.
    #[must_use]
    pub fn decode(&self) -> SwitchProRumbleDecoded {
        self.decode_with_previous(None, None)
    }

    /// Decode with explicit preceding samples for each side, if available.
    /// A no-sample word leaves previous state unchanged; an unknown word must
    /// invalidate it. Never reuse earlier samples across a gap or reset.
    #[must_use]
    pub fn decode_with_previous(
        &self,
        left: Option<SwitchProRumbleSample>,
        right: Option<SwitchProRumbleSample>,
    ) -> SwitchProRumbleDecoded {
        SwitchProRumbleDecoded {
            left: decode_switch_pro_rumble_word_with_previous(self.left, left),
            right: decode_switch_pro_rumble_word_with_previous(self.right, right),
        }
    }
}

/// Decode one native word with no assumed preceding state.
#[must_use]
pub fn decode_switch_pro_rumble_word(raw: [u8; 4]) -> SwitchProRumbleWord {
    decode_switch_pro_rumble_word_with_previous(raw, None)
}

/// Decode one word using only the supplied previous sample and native bytes.
/// Model: swpro-l0-rumble-absolute-layout, swpro-l0-rumble-sample-layouts,
/// swpro-l0-rumble-logarithmic-units, swpro-l0-rumble-relative-commands.
/// Source evidence only; no physical actuator response is asserted.
#[must_use]
pub fn decode_switch_pro_rumble_word_with_previous(
    raw: [u8; 4],
    previous: Option<SwitchProRumbleSample>,
) -> SwitchProRumbleWord {
    let word = u32::from_le_bytes(raw);
    let field = |shift: u32, mask: u32| i16::from(((word >> shift) & mask).to_le_bytes()[0]);
    let mut sample = previous.unwrap_or_else(SwitchProRumbleSample::unknown);

    let samples = match word >> 30 {
        0 if word == 0 => SwitchProRumbleSamples::NoSamples,
        1 if word.trailing_zeros() >= 20 => {
            sample.relative_pair(field(20, 31), field(25, 31));
            SwitchProRumbleSamples::One([sample])
        }
        1 if word.trailing_zeros() >= 2 => {
            sample.high.set_frequency(field(2, 127));
            sample.high.set_amplitude(field(9, 127));
            sample.low.set_frequency(field(16, 127));
            sample.low.set_amplitude(field(23, 127));
            SwitchProRumbleSamples::One([sample])
        }
        1 if word & 2 != 0 => {
            let selected = if word & 1 != 0 {
                &mut sample.high
            } else {
                &mut sample.low
            };
            if word & 4 != 0 {
                selected.set_frequency(field(23, 127));
            } else {
                selected.set_amplitude(field(23, 127));
            }
            let first = sample;
            sample.relative_pair(field(13, 31), field(18, 31));
            let second = sample;
            sample.relative_pair(field(3, 31), field(8, 31));
            SwitchProRumbleSamples::Three([first, second, sample])
        }
        2 if word.trailing_zeros() >= 10 => {
            sample.relative_pair(field(20, 31), field(25, 31));
            let first = sample;
            sample.relative_pair(field(10, 31), field(15, 31));
            SwitchProRumbleSamples::Two([first, sample])
        }
        2 => {
            let (absolute, relative) = if word & 1 != 0 {
                (&mut sample.high, &mut sample.low)
            } else {
                (&mut sample.low, &mut sample.high)
            };
            absolute.set_frequency(field(1, 127));
            absolute.set_amplitude(field(23, 127));
            relative.relative(field(18, 31));
            let first = sample;
            sample.relative_pair(field(8, 31), field(13, 31));
            SwitchProRumbleSamples::Two([first, sample])
        }
        3 => {
            sample.relative_pair(field(20, 31), field(25, 31));
            let first = sample;
            sample.relative_pair(field(10, 31), field(15, 31));
            let second = sample;
            sample.relative_pair(field(0, 31), field(5, 31));
            SwitchProRumbleSamples::Three([first, second, sample])
        }
        // Nonzero type-zero data and a missing documented mixed-format bit
        // cannot be established as valid from the pinned sources.
        _ => SwitchProRumbleSamples::Unknown,
    };
    SwitchProRumbleWord { raw, samples }
}

#[cfg(test)]
mod rumble_decode_tests {
    use super::{
        SwitchProOutputEvent, SwitchProRumbleSample, SwitchProRumbleSamples,
        decode_switch_pro_rumble_word, decode_switch_pro_rumble_word_with_previous,
    };

    fn absolute(hf: u8, ha: u8, lf: u8, la: u8) -> [u8; 4] {
        [
            hf << 2,
            (hf >> 6) | (ha << 1),
            lf | ((la & 1) << 7),
            0x40 | (la >> 1),
        ]
    }

    fn sample(word: [u8; 4]) -> SwitchProRumbleSample {
        let decoded = decode_switch_pro_rumble_word(word);
        assert_eq!(decoded.raw, word);
        let SwitchProRumbleSamples::One([sample]) = decoded.samples else {
            panic!("expected one sample");
        };
        sample
    }

    fn close(actual: Option<f32>, expected: f32) {
        let value = actual.expect("expected known value");
        assert!((value - expected).abs() <= 0.000_01 * expected.abs().max(1.0));
    }

    #[test]
    fn absolute_indices_and_neutral() {
        // swpro-l0-rumble-absolute-layout, swpro-l0-rumble-logarithmic-units.
        // Use the other band's centered frequency to avoid the overlapping
        // compact-format selector when sweeping each absolute field.
        let neutral = sample([0x00, 0x01, 0x40, 0x40]);
        close(neutral.high().frequency_hz(), 320.0);
        close(neutral.low().frequency_hz(), 160.0);
        close(neutral.high().amplitude_ratio(), 0.0);
        close(neutral.low().amplitude_ratio(), 0.0);
        for index in 0_u8..=127 {
            let factor = (f32::from(index) / 32.0 - 2.0).exp2();
            let a = sample(absolute(index, 64, 64, 64));
            let b = sample(absolute(64, 64, index, 64));
            close(a.high().frequency_hz(), 320.0 * factor);
            close(b.low().frequency_hz(), 160.0 * factor);
            let ratio = match index {
                0 => 0.0,
                1..=15 => 0.0625 * ((f32::from(index) - 15.0) / 4.0).exp2(),
                16..=31 => 0.125 * ((f32::from(index) - 31.0) / 16.0).exp2(),
                _ => ((f32::from(index) - 127.0) / 32.0).exp2(),
            };
            let c = sample(absolute(64, index, 64, 64));
            let d = sample(absolute(64, 64, 64, index));
            close(c.high().amplitude_ratio(), ratio);
            close(d.low().amplitude_ratio(), ratio);
        }
    }

    #[test]
    fn relative_commands_and_partial_state() {
        // swpro-l0-rumble-relative-commands, swpro-l0-rumble-logarithmic-units.
        let baseline = sample(absolute(64, 64, 64, 64));
        for command in 0_u32..=31 {
            let raw = ((1_u32 << 30) | (command << 20) | (command << 25)).to_le_bytes();
            let decoded = decode_switch_pro_rumble_word_with_previous(raw, Some(baseline));
            let [value] = decoded.samples.as_slice().unwrap() else {
                panic!("expected one sample");
            };
            let (amp, frequency) = match command {
                0 => (-256, 0),
                1..=11 => (-16 * (i16::try_from(command).unwrap() - 1), 0),
                12..=16 => (-63, 6 * (i16::try_from(command).unwrap() - 14)),
                _ => {
                    let i = usize::try_from(command - 17).unwrap();
                    (
                        -63 + [4, 1, 0, -1, -4][i / 3],
                        1 - i16::try_from(i % 3).unwrap(),
                    )
                }
            };
            let ratio = if amp < -254 {
                0.0
            } else {
                (f32::from(amp) / 32.0).exp2()
            };
            close(value.high().amplitude_ratio(), ratio);
            close(value.low().amplitude_ratio(), ratio);
            close(
                value.high().frequency_hz(),
                320.0 * (f32::from(frequency) / 32.0).exp2(),
            );
            close(
                value.low().frequency_hz(),
                160.0 * (f32::from(frequency) / 32.0).exp2(),
            );
        }
        let raw = ((1_u32 << 30) | (1 << 20) | (14 << 25)).to_le_bytes();
        let partial = sample(raw);
        close(partial.high().amplitude_ratio(), 1.0);
        assert!(partial.high().frequency_hz().is_none());
        close(partial.low().frequency_hz(), 160.0);
        assert!(partial.low().amplitude_ratio().is_none());
    }

    #[test]
    fn relative_multi_sample_forms_and_clamps() {
        // swpro-l0-rumble-sample-layouts, swpro-l0-rumble-relative-commands.
        let two = ((2_u32 << 30) | (1 << 10) | (14 << 15)).to_le_bytes();
        let result = decode_switch_pro_rumble_word(two);
        let [first, second] = result.samples.as_slice().unwrap() else {
            panic!("expected two samples");
        };
        close(first.high().amplitude_ratio(), 0.0);
        close(second.high().amplitude_ratio(), 1.0);
        close(second.low().frequency_hz(), 160.0);
        close(second.low().amplitude_ratio(), 0.0);

        let three = ((3_u32 << 30) | (17 << 10) | (31 << 15) | 0x17 | (2 << 5)).to_le_bytes();
        let result = decode_switch_pro_rumble_word(three);
        let [first, second, third] = result.samples.as_slice().unwrap() else {
            panic!("expected three samples");
        };
        close(first.high().frequency_hz(), 320.0);
        close(
            second.high().frequency_hz(),
            320.0 * (1.0_f32 / 32.0).exp2(),
        );
        close(third.high().frequency_hz(), 320.0 * (2.0_f32 / 32.0).exp2());
        close(second.high().amplitude_ratio(), (-252.0_f32 / 32.0).exp2());
        close(second.low().amplitude_ratio(), 0.0);
        close(third.low().amplitude_ratio(), 0.5_f32.sqrt());

        let max = sample(absolute(127, 127, 127, 127));
        let up =
            ((3_u32 << 30) | (17 << 25) | (17 << 20) | (17 << 15) | (17 << 10) | (17 << 5) | 0x11)
                .to_le_bytes();
        let result = decode_switch_pro_rumble_word_with_previous(up, Some(max));
        for value in result.samples.as_slice().unwrap() {
            close(value.high().frequency_hz(), 1280.0);
            close(value.low().frequency_hz(), 640.0);
            close(value.high().amplitude_ratio(), 1.0);
            close(value.low().amplitude_ratio(), 1.0);
        }
    }

    #[test]
    fn mixed_forms_both_bands_and_fields() {
        // swpro-l0-rumble-sample-layouts, swpro-l0-rumble-relative-commands.
        for selected_high in 0_u32..=1 {
            let word =
                ((2_u32 << 30) | (127 << 23) | (24 << 13) | (24 << 8) | (64 << 1) | selected_high)
                    .to_le_bytes();
            let result = decode_switch_pro_rumble_word(word);
            assert_eq!(result.raw, word);
            let [first, second] = result.samples.as_slice().unwrap() else {
                panic!("expected two mixed samples");
            };
            assert_eq!(first, second);
            let (selected, other) = if selected_high == 1 {
                (first.high(), first.low())
            } else {
                (first.low(), first.high())
            };
            close(selected.amplitude_ratio(), 1.0);
            close(other.amplitude_ratio(), 0.0);
            for select_frequency in 0_u32..=1 {
                let word = ((1_u32 << 30)
                    | (64 << 23)
                    | (24 << 8)
                    | (24 << 3)
                    | (select_frequency << 2)
                    | 2
                    | selected_high)
                    .to_le_bytes();
                let result = decode_switch_pro_rumble_word(word);
                assert_eq!(result.raw, word);
                let [first, second, third] = result.samples.as_slice().unwrap() else {
                    panic!("expected three mixed samples");
                };
                let band = if selected_high == 1 {
                    first.high()
                } else {
                    first.low()
                };
                if select_frequency == 1 {
                    assert!(band.amplitude_ratio().is_none());
                    close(
                        band.frequency_hz(),
                        if selected_high == 1 { 320.0 } else { 160.0 },
                    );
                } else {
                    assert!(band.frequency_hz().is_none());
                    close(band.amplitude_ratio(), (-63.0_f32 / 32.0).exp2());
                }
                close(second.high().amplitude_ratio(), 0.0);
                close(second.low().frequency_hz(), 160.0);
                assert_eq!(second, third);
            }
        }
    }

    #[test]
    fn no_sample_unknown_and_raw_event_agreement() {
        // swpro-l0-rumble-sample-layouts, swpro-l0-linux-rumble-framing.
        let zero = decode_switch_pro_rumble_word([0; 4]);
        assert_eq!(zero.samples, SwitchProRumbleSamples::NoSamples);
        assert_eq!(zero.samples.as_slice(), Some(&[][..]));
        let unknown = [1, 0, 0, 0x40];
        let decoded = decode_switch_pro_rumble_word(unknown);
        assert_eq!(decoded.raw, unknown);
        assert_eq!(decoded.samples, SwitchProRumbleSamples::Unknown);
        assert!(decoded.samples.as_slice().is_none());
        assert_eq!(
            decode_switch_pro_rumble_word([1, 0, 0, 0]).samples,
            SwitchProRumbleSamples::Unknown
        );
        let left = absolute(64, 30, 64, 31);
        let right = unknown;
        for report_id in [1, 0x10] {
            let mut bytes = vec![7];
            bytes.extend_from_slice(&left);
            bytes.extend_from_slice(&right);
            if report_id == 1 {
                bytes.push(0x48);
            }
            let raw = SwitchProOutputEvent::Output {
                report_id: Some(report_id),
                bytes,
            }
            .rumble()
            .unwrap();
            assert_eq!(raw.packet_counter, 7);
            assert_eq!(raw.left, left);
            assert_eq!(raw.right, right);
            let decoded = raw.decode();
            assert_eq!(decoded.left.raw, left);
            assert_eq!(decoded.right.raw, right);
            assert_eq!(decoded.left, decode_switch_pro_rumble_word(left));
            assert_eq!(decoded.right, decode_switch_pro_rumble_word(right));
        }
    }
}

/// Source-only HOME light request via subcommand 0x38.
/// `mini_cycle` retains all five request bytes; only Linux's brightness
/// nibble is interpreted. Not a player indicator.
/// Claim: `swpro-cgap-home-led-consumer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchProHomeLight {
    pub brightness: u8,
    pub mini_cycle: [u8; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchProOutputEvent {
    ForceFeedback(gr_realization_api::ForceFeedbackEvent),
    /// Accepted HOME light command, distinct from the four player indicators.
    HomeLight(SwitchProHomeLight),
    ProviderEvent(Vec<EvdevEvent>),
    HidLifecycle(gr_hid::Lifecycle),
    Output {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    HostRequest {
        request_id: u32,
        report_id: u8,
        report_type: u8,
    },
    Other,
}
impl SwitchProOutputEvent {
    /// Extract exact encoded motor words from accepted rumble/subcommand output.
    /// Decode source-supported words with `SwitchProRumble::decode`; physical response is unknown.
    #[must_use]
    pub fn rumble(&self) -> Option<SwitchProRumble> {
        let Self::Output {
            report_id: Some(id @ (1 | 0x10)),
            bytes,
        } = self
        else {
            return None;
        };
        if bytes.len() > 63 || bytes.len() < if *id == 1 { 10 } else { 9 } {
            return None;
        }
        Some(SwitchProRumble {
            packet_counter: bytes[0],
            left: bytes[1..5].try_into().ok()?,
            right: bytes[5..9].try_into().ok()?,
        })
    }
}

impl From<RawReverseEvent> for SwitchProOutputEvent {
    fn from(e: RawReverseEvent) -> Self {
        match e {
            RawReverseEvent::ForceFeedback(event) => Self::ForceFeedback(event),
            RawReverseEvent::Evdev(events) => Self::ProviderEvent(events),
            RawReverseEvent::HidLifecycle(event) => Self::HidLifecycle(event),
            RawReverseEvent::HidOutput {
                report_id: Some(1),
                bytes,
            } if bytes.len() >= 15 && bytes[9] == 0x38 => {
                let mini_cycle: [u8; 5] = bytes[10..15]
                    .try_into()
                    .expect("validated HOME mini-cycle payload");
                Self::HomeLight(SwitchProHomeLight {
                    brightness: mini_cycle[1] >> 4,
                    mini_cycle,
                })
            }
            RawReverseEvent::HidOutput { report_id, bytes } => Self::Output { report_id, bytes },
            RawReverseEvent::HidGetReportRequest {
                request_id,
                report_id,
                report_type,
            }
            | RawReverseEvent::HidSetReportRequest {
                request_id,
                report_id,
                report_type,
                ..
            } => Self::HostRequest {
                request_id,
                report_id,
                report_type,
            },
            _ => Self::Other,
        }
    }
}
/// Family options for a new `SwitchPro` session.
///
/// IMU enable behavior is source-backed by research claim
/// `swpro-l0-consumer-imu-rumble-ack-gates`; retail timing remains unmeasured.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct SwitchProOptions {
    identity: Option<SwitchProIdentity>,
    battery: SwitchProBattery,
    imu_gating: bool,
}

impl SwitchProOptions {
    /// Restore a caller-owned controller identity. USB/UHID presents this address to the host. Evdev retains it in the
    /// handle for caller storage without presenting it to the host.
    #[must_use]
    pub const fn with_identity(mut self, identity: SwitchProIdentity) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Set the initial battery report status (defaults to full, not charging,
    /// Pro connection nibble zero, preserving report byte 0x80).
    #[must_use]
    pub const fn with_battery(mut self, battery: SwitchProBattery) -> Self {
        self.battery = battery;
        self
    }

    /// Initial battery status for the new session.
    #[must_use]
    pub const fn battery(self) -> SwitchProBattery {
        self.battery
    }

    /// Opt into gating the IMU sample bytes until subcommand 0x40 enables them.
    /// Defaults to false to preserve existing emissions.
    /// Source-only: `swpro-l0-consumer-imu-rumble-ack-gates`.
    #[must_use]
    pub const fn with_imu_gating(mut self, enabled: bool) -> Self {
        self.imu_gating = enabled;
        self
    }

    #[must_use]
    pub const fn imu_gating(self) -> bool {
        self.imu_gating
    }

    /// An explicitly supplied identity, if any.
    #[must_use]
    pub const fn identity(self) -> Option<SwitchProIdentity> {
        self.identity
    }
}

/// Create an exact realization with fresh session state.
pub fn create_switch_pro(
    options: CreationOptions,
    family: SwitchProOptions,
) -> Result<SwitchProController, ProviderError> {
    create_switch_pro_configured(options, family, SwitchProIdentity::generate)
}

fn create_switch_pro_configured(
    o: CreationOptions,
    family: SwitchProOptions,
    generate_identity: impl FnOnce() -> Result<SwitchProIdentity, ProviderError>,
) -> Result<SwitchProController, ProviderError> {
    let build_realization: fn(SwitchProIdentity) -> NativeControllerRealization = match o.target {
        RealizationTarget::LINUX_UINPUT => |_| evdev_realization(),
        RealizationTarget::LINUX_UHID_USB => hid,
        RealizationTarget::LINUX_DUMMY_HCD_USB_HID => |_| {
            NativeControllerRealization::DummyHcd(NativeDummyHcdRealization {
                controller: gr_controller_wire::registry::ids::SWITCH_PRO,
            })
        },
        _ => {
            return Err(ProviderError::Unsupported {
                reason: "unknown deployment target".into(),
            });
        }
    };
    common::require_linux_platform(o.target)?;
    let identity = family.identity().map_or_else(generate_identity, Ok)?;
    let realization = build_realization(identity);
    let is_dummy_hcd = o.target == RealizationTarget::LINUX_DUMMY_HCD_USB_HID;
    let mut controller = if o.target == RealizationTarget::LINUX_UHID_USB {
        common::create_restored(SwitchProDefinition, realization, o, Some(identity.bytes()))?
    } else {
        common::create(SwitchProDefinition, realization, o)?
    };
    controller.update_state(|state| {
        state.1.battery = family.battery();
        state.1.imu_gating = family.imu_gating();
        Ok(())
    }).map_err(|error| ProviderError::Open { reason: error.to_string() })?;
    if is_dummy_hcd {
        // A physical USB attachment should expose a neutral 0x30 report as
        // soon as it appears; host-side controller-info traffic then has a
        // live interrupt endpoint before the first GUI interaction.
        controller.commit().map_err(|error| ProviderError::Open {
            reason: error.to_string(),
        })?;
    }
    Ok(SwitchProController(controller, identity))
}
/// Workspace test seam. Never opens a real provider or accepts runtime profiles.
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn test_controller(
    session: Box<dyn gr_realization_api::NativeProviderSession>,
) -> Result<SwitchProController, ProviderError> {
    let identity = SwitchProIdentity::from_bytes(common::test_address(1))?;
    common::injected(SwitchProDefinition, session)
        .map(|session| SwitchProController(session, identity))
}

impl crate::registry::ControllerFamily for SwitchProDefinition {
    const SPEC: &'static crate::registry::FamilySpec = &gr_controller_wire::registry::SWITCH_PRO;
    const OUTPUT_KINDS: &'static [OutputKind] = NATIVE_OUTPUT_KINDS;
    const REFUSED_OPERATIONS: &'static [crate::registry::FamilyOperation] = &[
        crate::registry::refusal(EVDEV_RESTRICTIONS[0], &[RealizationTarget::LINUX_UINPUT]),
        crate::registry::refusal(EVDEV_RESTRICTIONS[1], &[RealizationTarget::LINUX_UINPUT]),
        crate::registry::refusal(EVDEV_RESTRICTIONS[2], &[RealizationTarget::LINUX_UINPUT]),
        crate::registry::refusal(EVDEV_RESTRICTIONS[3], &[RealizationTarget::LINUX_UINPUT]),
        crate::registry::refusal(
            RESTRICTIONS[0],
            &[
                RealizationTarget::LINUX_UINPUT,
                RealizationTarget::LINUX_UHID_USB,
                RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
            ],
        ),
        crate::registry::refusal(
            RETAINED_DUMMY[1],
            &[RealizationTarget::LINUX_DUMMY_HCD_USB_HID],
        ),
    ];
    fn topology() -> &'static InputTopology {
        &INPUT_TOPOLOGY_WITH_MOTION
    }
    fn manifest() -> RealizationManifest {
        Self.realization_manifest()
    }
    fn realizations() -> &'static [crate::registry::FamilyRealization] {
        static REALIZATIONS: &[crate::registry::FamilyRealization] = &[
            crate::registry::FamilyRealization::new(
                &EVDEV_SURFACE.common,
                crate::registry::RealizationTier::PlayableReduced,
                crate::registry::PermissionClass::Uinput,
            ),
            crate::registry::FamilyRealization::new(
                &HID_SURFACE.common,
                crate::registry::RealizationTier::PlayableFull,
                crate::registry::PermissionClass::Uhid,
            ),
            crate::registry::FamilyRealization::new(
                &USB_SURFACE.common,
                crate::registry::RealizationTier::FaithfulUsb,
                crate::registry::PermissionClass::RootBroker,
            ),
        ];
        REALIZATIONS
    }
}

#[cfg(test)]
impl crate::common::session_tests::InputFixture for (SwitchProInputs, SwitchProStatus) {
    type Inputs = SwitchProInputs;
    fn test_inputs(&self) -> &Self::Inputs {
        &self.0
    }
    fn test_inputs_mut(&mut self) -> &mut Self::Inputs {
        &mut self.0
    }
    fn test_into_inputs(self) -> Self::Inputs {
        self.0
    }
}
#[cfg(test)]
impl crate::common::session_tests::ControllerFixture for SwitchProController {
    type State = (SwitchProInputs, SwitchProStatus);
    fn test_state(&self) -> &Self::State {
        self.0.state()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::session_tests::{ControllerFixture as _, InputFixture as _};

    #[test]
    fn input_values_are_independent_of_controller_clocks_and_stream_state() {
        use common::session_tests::input_validation_session;
        let neutral = SwitchProInputs::default();
        assert_eq!(neutral, neutral.clone());
        assert_eq!(
            neutral.left_stick(),
            (SwitchProAxis::new(0), SwitchProAxis::new(0))
        );
        assert_eq!(
            neutral.motion(),
            SwitchProMotionSample {
                accelerometer: [0; 3],
                gyroscope: [0; 3]
            }
        );
        for target in [
            RealizationTarget::LINUX_UINPUT,
            RealizationTarget::LINUX_UHID_USB,
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
        ] {
            let session =
                input_validation_session(SwitchProDefinition, target, [true, true, false]);
            let mut controller = SwitchProController(session, fixture_identity());
            controller.service(&mut |_| {}).unwrap();
            let status = controller.status();
            let outputs = controller.host_outputs();
            let mut inputs = neutral.clone();
            inputs.set_native(SwitchProControl::B, true);
            inputs.set_sticks(
                (SwitchProAxis::new(-123), SwitchProAxis::new(456)),
                (SwitchProAxis::new(789), SwitchProAxis::new(-321)),
            );
            let motion = target != RealizationTarget::LINUX_UINPUT;
            if motion {
                inputs.set_motion(SwitchProMotionSample {
                    accelerometer: [1, 2, 3],
                    gyroscope: [4, 5, 6],
                });
            }
            controller.set_inputs(inputs.clone()).unwrap();
            assert_eq!(controller.inputs(), &inputs);
            assert_eq!(
                controller.status().timer(),
                status.timer().wrapping_add(u8::from(motion))
            );
            assert_eq!(
                controller.status().stream_enabled(),
                status.stream_enabled()
            );
            controller.commit().unwrap();
            let accepted_status = controller.status();
            controller.edit_inputs(|_| Ok(())).unwrap();
            controller.set_inputs(inputs.clone()).unwrap();
            assert!(!controller.is_dirty());
            assert_eq!(controller.status(), accepted_status);
            let error = ControlError::UnsupportedControl {
                control: "synthetic late input failure",
            };
            assert_eq!(
                controller.edit_inputs(|candidate| {
                    candidate.set_native(SwitchProControl::A, true);
                    Err(error.clone())
                }),
                Err(error)
            );
            assert_eq!(controller.inputs(), &inputs);
            assert_eq!(controller.status(), accepted_status);
            assert!(!controller.is_dirty());
            controller.neutralize().unwrap();
            assert_eq!(controller.inputs(), &neutral);
            assert_eq!(
                controller.status().timer(),
                status.timer().wrapping_add(u8::from(motion) * 2)
            );
            assert_eq!(
                controller.status().stream_enabled(),
                status.stream_enabled()
            );
            assert_eq!(controller.host_outputs(), outputs);
            let final_status = controller.status();
            controller.close();
            controller.close();
            assert_eq!(controller.status(), final_status);
            assert_eq!(
                controller.set_inputs(neutral.clone()),
                Err(ControlError::Closed)
            );
        }
    }

    #[test]
    fn native_input_edits_are_transactional_and_unchanged_edits_stay_clean() {
        use common::session_tests::{check_input_transactions, snapshot_session};
        for target in [
            RealizationTarget::LINUX_UINPUT,
            RealizationTarget::LINUX_UHID_USB,
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
        ] {
            let (session, io) = snapshot_session(SwitchProDefinition, target, [true, true, false]);
            let mut controller = SwitchProController(session, fixture_identity());
            controller.service(&mut |_| {}).unwrap();
            controller
                .edit_inputs(|state| {
                    for (direction, pressed) in [
                        (gr_controller_contract::DpadDirection::Up, true),
                        (gr_controller_contract::DpadDirection::Down, true),
                        (gr_controller_contract::DpadDirection::Up, false),
                    ] {
                        state.set_digital(DigitalControlUpdate::Dpad { direction, pressed });
                    }
                    Ok(())
                })
                .unwrap();
            assert!(
                controller
                    .inputs()
                    .dpad_pressed(gr_controller_contract::DpadDirection::Down)
            );
            controller.neutralize().unwrap();
            check_input_transactions!(
                controller,
                |state: &mut SwitchProInputs| {
                    state.set_native(SwitchProControl::B, true);
                    state.set_digital(DigitalControlUpdate::Dpad {
                        direction: gr_controller_contract::DpadDirection::Up,
                        pressed: true,
                    });
                    Ok(())
                },
                |state: &mut SwitchProInputs| {
                    state.set_digital(DigitalControlUpdate::Dpad {
                        direction: gr_controller_contract::DpadDirection::Down,
                        pressed: true,
                    });
                    Err(ControlError::UnsupportedControl {
                        control: "synthetic late failure on independent-bit USB",
                    })
                },
                io.lock().unwrap().sent.len()
            );
        }
    }

    #[test]
    fn field_edits_and_snapshots_share_validation_and_transitions() {
        use common::session_tests::assert_input_rejected;
        use common::session_tests::{check_opposing_dpad, input_validation_session};
        for target in [
            RealizationTarget::LINUX_UINPUT,
            RealizationTarget::LINUX_UHID_USB,
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
        ] {
            let is_uinput = target == RealizationTarget::LINUX_UINPUT;
            let session =
                input_validation_session(SwitchProDefinition, target, [true, true, false]);
            let mut controller = SwitchProController(session, fixture_identity());
            controller.service(&mut |_| {}).unwrap();
            check_opposing_dpad!(controller, is_uinput);
            controller.neutralize().unwrap();
            controller.commit().unwrap();
            let unavailable = ControlError::UnavailableInRealization {
                selected_target: target,
                available_in: motion_targets(),
            };

            let motion = SwitchProMotionSample {
                accelerometer: [10, 20, 30],
                gyroscope: [40, 50, 60],
            };
            let mut candidate = controller.test_state().clone();
            candidate.0.set_motion(motion);
            if is_uinput {
                assert_input_rejected!(
                    controller,
                    candidate,
                    controller.set_motion(motion),
                    unavailable.clone()
                );
                assert_eq!(
                    controller.set_motion(controller.test_state().0.motion()),
                    Err(unavailable)
                );
                controller
                    .set_inputs(controller.test_state().clone().test_into_inputs())
                    .unwrap();
            } else {
                let timer = controller.test_state().1.timer;
                controller.set_motion(motion).unwrap();
                let changed = controller.test_state().clone();
                assert_eq!(changed.1.timer, timer.wrapping_add(1));
                controller.set_motion(motion).unwrap();
                controller.set_inputs(candidate.test_into_inputs()).unwrap();
                controller
                    .edit_inputs(|state| {
                        state.set_motion(motion);
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(controller.test_state(), &changed);
                controller.neutralize().unwrap();
                assert_eq!(controller.test_state().1.timer, timer.wrapping_add(2));
            }
            controller.close();
            controller.close();
            let accepted = controller.test_state().clone();
            assert_eq!(controller.neutralize(), Err(ControlError::Closed));
            assert_eq!(controller.test_state(), &accepted);
        }
    }

    #[test]
    fn whole_state_through_each_report_encoder() {
        use common::session_tests::{check_snapshot_session, snapshot_session};
        for target in [
            RealizationTarget::LINUX_UINPUT,
            RealizationTarget::LINUX_UHID_USB,
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
        ] {
            let (session, io) = snapshot_session(SwitchProDefinition, target, [true, true, false]);
            let mut controller = SwitchProController(session, fixture_identity());
            controller
                .0
                .update_state(|current| {
                    current.1.timer = 17;
                    Ok(())
                })
                .unwrap();
            if target != RealizationTarget::LINUX_UINPUT {
                let mut bytes = vec![0; 63];
                bytes[9] = 3;
                bytes[10] = 0x30;
                io.lock()
                    .unwrap()
                    .events
                    .push_back(RawReverseEvent::HidOutput {
                        report_id: Some(1),
                        bytes,
                    });
                controller.service(&mut |_| {}).unwrap();
                controller.service(&mut |_| {}).unwrap();
                assert!(controller.stream_enabled());
                assert!(controller.status().stream_enabled());
            }
            let identity = controller.identity();
            let axis = SwitchProAxis::new;
            let mut snapshot = (
                SwitchProInputs {
                    face: [true; 4],
                    dpad: [true, false, false, true],
                    left: (axis(i16::MIN), axis(i16::MAX)),
                    right: (axis(i16::MAX), axis(i16::MIN)),
                    buttons: [true; 10],
                    ..Default::default()
                },
                SwitchProStatus::default(),
            );
            let motion = SwitchProMotionSample {
                accelerometer: [i16::MIN, 24, i16::MAX],
                gyroscope: [50, -60, 70],
            };
            if target != RealizationTarget::LINUX_UINPUT {
                snapshot.0.set_motion(motion);
            }
            // Switch's USB report exposes opposing directions as separate bits.
            if target != RealizationTarget::LINUX_UINPUT {
                snapshot.0.dpad[1] = true;
            }
            let current = controller.test_state();
            let motion_changed = snapshot.0.motion != current.0.motion;
            let expected = (
                (snapshot.clone()).0,
                SwitchProStatus {
                    timer: current.1.timer.wrapping_add(u8::from(motion_changed)),
                    ..current.1
                },
            );
            let mut bad = snapshot.clone();
            bad.0.face[0] = false;
            bad.0.dpad[1] = true;
            let invalid = (target == RealizationTarget::LINUX_UINPUT).then_some((
                bad,
                ControlError::UnsupportedControl {
                    control: "opposing D-pad directions on a hat",
                },
            ));
            check_snapshot_session!(
                controller,
                io,
                snapshot,
                expected,
                invalid,
                SwitchProDefinition
                    .encode(controller.0.selection(), controller.test_state())
                    .unwrap(),
                &[(0, 255)]
            );
            assert_eq!(controller.identity(), identity);
            assert_eq!(
                controller.status().stream_enabled(),
                controller.stream_enabled()
            );
            assert_eq!(
                controller.stream_enabled(),
                target != RealizationTarget::LINUX_UINPUT
            );
        }
    }

    pub(super) fn fixture_identity() -> SwitchProIdentity {
        SwitchProIdentity::from_bytes(common::test_address(1)).unwrap()
    }

    #[test]
    fn undeclared_target_is_rejected_before_resolving_identity() {
        let options = CreationOptions {
            target: RealizationTarget::LINUX_UHID_BLUETOOTH,
            session: RealizationSessionId(7),
        };
        assert!(matches!(
            create_switch_pro_configured(options, SwitchProOptions::default(), || panic!(
                "invalid target requested entropy"
            )),
            Err(ProviderError::Unsupported { .. })
        ));
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn unsupported_platform_is_rejected_before_resolving_identity() {
        for target in [
            RealizationTarget::LINUX_UINPUT,
            RealizationTarget::LINUX_UHID_USB,
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
        ] {
            let options = CreationOptions {
                target,
                session: RealizationSessionId(7),
            };
            assert!(matches!(
                create_switch_pro(options, SwitchProOptions::default()),
                Err(ProviderError::Preflight(gr_realization_api::ProviderPreflightError::UnsupportedPlatform {
                    target: rejected
                })) if rejected == target
            ));
        }
    }

    #[test]
    fn switch_topology_tracks_target_motion_support() {
        for topology in [&INPUT_TOPOLOGY, &INPUT_TOPOLOGY_WITH_MOTION] {
            assert_eq!(topology.validate(), Ok(()));
            assert_eq!(topology.trigger_stacks().len(), 2);
        }
        assert!(EVDEV_SURFACE.common.input_topology().motion().is_empty());
        assert_eq!(HID_SURFACE.common.input_topology().motion().len(), 1);
        assert_eq!(USB_SURFACE.common.input_topology().motion().len(), 1);
    }

    #[test]
    fn rumble_words_skip_counter_and_keep_motor_order() {
        for id in [1, 0x10] {
            let event = SwitchProOutputEvent::Output {
                report_id: Some(id),
                bytes: vec![15, 0, 1, 64, 64, 11, 12, 13, 14, 3],
            };
            assert_eq!(
                event.rumble(),
                Some(SwitchProRumble {
                    packet_counter: 15,
                    left: [0, 1, 64, 64],
                    right: [11, 12, 13, 14]
                })
            );
        }
        for length in 0..9 {
            assert!(
                SwitchProOutputEvent::Output {
                    report_id: Some(0x10),
                    bytes: vec![0; length]
                }
                .rumble()
                .is_none()
            );
        }
    }

    #[test]
    fn neutralization_releases_native_inputs_and_preserves_metadata() {
        let mut expected = (
            SwitchProInputs::default(),
            SwitchProStatus {
                timer: 77,
                stream_enabled: true,
                ..Default::default()
            },
        );
        let mut state = expected.clone();
        state.0.face.fill(true);
        state.0.dpad.fill(true);
        state.0.buttons.fill(true);
        state.0.left = (SwitchProAxis(100), SwitchProAxis(200));
        state.0.right = (SwitchProAxis(200), SwitchProAxis(100));
        state.0.motion.accelerometer = [100; 3];
        state.0.motion.gyroscope = [200; 3];
        expected.1.timer += 1;
        let previous = state.clone();
        <SwitchProDefinition as common::HidDriver>::neutralize_state(&mut state);
        <SwitchProDefinition as common::HidDriver>::transition_inputs(&previous, &mut state);
        assert_eq!(state, expected);
        let previous = state.clone();
        <SwitchProDefinition as common::HidDriver>::neutralize_state(&mut state);
        <SwitchProDefinition as common::HidDriver>::transition_inputs(&previous, &mut state);
        assert_eq!(state, expected);
    }

    #[test]
    fn evdev_auxiliary_buttons_do_not_alias_stick_presses() {
        let selection = RealizationSelection {
            controller: SwitchProDefinition.controller_id(),
            target: RealizationTarget::LINUX_UINPUT,
        };
        for (control, code) in [
            (SwitchProControl::Capture, 309),
            (SwitchProControl::LeftStickPress, 317),
            (SwitchProControl::RightStickPress, 318),
            (SwitchProControl::Home, 316),
        ] {
            let mut state = <(SwitchProInputs, SwitchProStatus)>::default();
            for pressed in [true, false] {
                state.0.set_native(control, pressed);
                let ProviderFrame::Evdev(events) =
                    SwitchProDefinition.encode(selection, &state).unwrap()
                else {
                    panic!("evdev")
                };
                let active: Vec<_> = events
                    .iter()
                    .filter(|event| event.event_type == common::EV_KEY && event.value != 0)
                    .map(|event| event.code)
                    .collect();
                assert_eq!(active, if pressed { vec![code] } else { vec![] });
            }
        }
    }

    #[test]
    fn evdev_rumble_surface_matches_capabilities_and_explicit_trigger_limit() {
        let NativeControllerRealization::Evdev(spec) = evdev_realization() else {
            panic!("evdev")
        };
        assert!(spec.event_codes.contains(&common::EV_FF));
        assert_eq!(spec.force_feedback_codes, [0x50]);
        assert_eq!(EVDEV_SURFACE.common.outputs().len(), 1);
        assert_eq!(
            (
                EVDEV_SURFACE.common.outputs()[0].event_type,
                EVDEV_SURFACE.common.outputs()[0].event_code
            ),
            (21, 0x50)
        );
        assert!(
            EVDEV_SURFACE
                .common
                .restrictions()
                .iter()
                .any(|restriction| restriction.feature == "automatic force-feedback trigger")
        );
    }

    #[test]
    fn battery_status_encodes_native_level_charging_and_pro_power() {
        let mut state = <(SwitchProInputs, SwitchProStatus)>::default();
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[1], 0x80, "legacy default byte is unchanged");
        assert_eq!(state.1.battery(), SwitchProBattery::default());
        for (level, prefix) in [
            (SwitchProBatteryLevel::Empty, 0x00),
            (SwitchProBatteryLevel::Critical, 0x20),
            (SwitchProBatteryLevel::Low, 0x40),
            (SwitchProBatteryLevel::Medium, 0x60),
            (SwitchProBatteryLevel::Full, 0x80),
        ] {
            for charging in [false, true] {
                for usb_powered in [false, true] {
                    let battery = SwitchProBattery { level, charging, usb_powered };
                    state.1.battery = battery;
                    let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
                        unreachable!()
                    };
                    let expected = prefix | (u8::from(charging) << 4) | u8::from(usb_powered);
                    assert_eq!(bytes[1], expected);
                    let (reply, enabled) =
                        switch_subcommand_reply(&state, 0x38, &[1, 0x50, 0x55, 0x11, 0x11], fixture_identity());
                    assert!(!enabled);
                    let ProviderFrame::HidInput { bytes, .. } = reply else {
                        unreachable!()
                    };
                    assert_eq!(bytes[1], expected, "0x21 prefix uses current battery");
                }
            }
        }
        let selected = SwitchProBattery {
            level: SwitchProBatteryLevel::Critical,
            charging: true,
            usb_powered: true,
        };
        let options = SwitchProOptions::default().with_battery(selected);
        assert_eq!(options.battery(), selected);
        assert!(!options.imu_gating());
        assert!(options.with_imu_gating(true).imu_gating());
    }

    #[test]
    fn home_light_keeps_generic_ack_and_typed_minicycle_separate_from_players() {
        let state = <(SwitchProInputs, SwitchProStatus)>::default();
        let payload = [0x01, 0xb0, 0xbb, 0x11, 0x11];
        let (reply, enabled) =
            switch_subcommand_reply(&state, 0x38, &payload, fixture_identity());
        assert!(!enabled);
        let mut expected = vec![0; 63];
        expected[..12].copy_from_slice(&switch_input_prefix(&state));
        expected[12] = 0x80;
        expected[13] = 0x38;
        assert_eq!(reply, ProviderFrame::HidInput { report_id: Some(0x21), bytes: expected });
        let mut raw = vec![0; 63];
        raw[9] = 0x38;
        raw[10..15].copy_from_slice(&payload);
        assert_eq!(
            SwitchProOutputEvent::from(RawReverseEvent::HidOutput {
                report_id: Some(1),
                bytes: raw,
            }),
            SwitchProOutputEvent::HomeLight(SwitchProHomeLight {
                brightness: 11,
                mini_cycle: payload,
            })
        );
        let mut truncated = vec![0; 14];
        truncated[9] = 0x38;
        assert!(matches!(
            SwitchProOutputEvent::from(RawReverseEvent::HidOutput {
                report_id: Some(1),
                bytes: truncated,
            }),
            SwitchProOutputEvent::Output { .. }
        ));
        assert_eq!(NATIVE_OUTPUT_KINDS[1], OutputKind::PlayerIndicator { count: 4 });
    }

    #[test]
    fn native_service_retains_imu_enable_status_for_opted_in_frames() {
        use common::session_tests::snapshot_session;
        let (session, io) = snapshot_session(
            SwitchProDefinition,
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
            [true, true, false],
        );
        let mut controller = SwitchProController(session, fixture_identity());
        controller.0.update_state(|state| {
            state.1.imu_gating = true;
            state.0.motion = SwitchProMotionSample {
                accelerometer: [400, -800, 1200],
                gyroscope: [1, -2, 3],
            };
            Ok(())
        }).unwrap();
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(controller.test_state()) else {
            unreachable!()
        };
        assert!(bytes[12..48].iter().all(|byte| *byte == 0));
        let mut enable = vec![0; 63];
        enable[9] = 0x40;
        enable[10] = 1;
        io.lock().unwrap().events.push_back(RawReverseEvent::HidOutput {
            report_id: Some(1),
            bytes: enable,
        });
        controller.service(&mut |_| {}).unwrap();
        assert!(controller.status().imu_enabled());
        assert!(controller.status().imu_gating());
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(controller.test_state()) else {
            unreachable!()
        };
        assert!(bytes[12..48].iter().any(|byte| *byte != 0));
    }

    #[test]
    fn openpuck_motion_triplicates_and_maps_axes() {
        let s = (
            SwitchProInputs {
                motion: SwitchProMotionSample {
                    accelerometer: [400, -800, 1200],
                    gyroscope: [1, -2, 3],
                },
                ..Default::default()
            },
            SwitchProStatus::default(),
        );
        let ProviderFrame::HidInput { report_id, bytes } = switch_frame(&s) else {
            unreachable!()
        };
        assert_eq!(report_id, Some(0x30));
        assert_eq!(
            &bytes[12..24],
            &[56, 255, 156, 255, 44, 1, 254, 255, 255, 255, 3, 0]
        );
        assert_eq!(&bytes[12..24], &bytes[24..36]);
    }
    #[test]
    fn usb_frame_is_numbered() {
        let f = SwitchProDefinition
            .encode(
                RealizationSelection {
                    controller: SwitchProDefinition.controller_id(),
                    target: RealizationTarget::LINUX_UHID_USB,
                },
                &<(SwitchProInputs, SwitchProStatus)>::default(),
            )
            .unwrap();
        assert!(matches!(f,ProviderFrame::HidInput{report_id:Some(0x30),bytes}if bytes.len()==63));
    }

    #[test]
    fn dummy_hcd_frame_preserves_the_numbered_openpuck_usb_report() {
        let frame = SwitchProDefinition
            .encode(
                RealizationSelection {
                    controller: SwitchProDefinition.controller_id(),
                    target: RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                },
                &<(SwitchProInputs, SwitchProStatus)>::default(),
            )
            .expect("DummyHcd Switch frame");
        assert!(
            matches!(frame, ProviderFrame::DummyHcdInput(bytes) if bytes.len() == 64 && bytes[0] == 0x30)
        );
        assert!(
            SwitchProDefinition
                .realization_manifest()
                .entries()
                .iter()
                .any(|entry| entry.target == RealizationTarget::LINUX_DUMMY_HCD_USB_HID)
        );
    }

    #[test]
    fn dummy_hcd_replies_retain_the_usb_report_id() {
        let reply = dummy_hcd_reply(
            RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
            switch_usb_reply(2, fixture_identity()),
        )
        .expect("DummyHcd reply");
        assert!(
            matches!(reply, ProviderFrame::DummyHcdInput(bytes) if bytes.len() == 64 && bytes[0] == 0x81 && bytes[1] == 2)
        );
    }

    #[test]
    #[ignore = "requires the installed root-owned DummyHcd broker"]
    fn dummy_hcd_broker_opens_the_switch_usb_profile() {
        let mut controller = create_switch_pro(
            CreationOptions {
                target: RealizationTarget::LINUX_DUMMY_HCD_USB_HID,
                session: RealizationSessionId(0x5357_4954),
            },
            SwitchProOptions::default(),
        )
        .expect("open Switch Pro through the privileged broker");
        controller.close();
    }

    #[test]
    fn evdev_realization_preserves_hid_controls_without_motion_claims() {
        let frame = SwitchProDefinition
            .encode(
                RealizationSelection {
                    controller: SwitchProDefinition.controller_id(),
                    target: RealizationTarget::LINUX_UINPUT,
                },
                &<(SwitchProInputs, SwitchProStatus)>::default(),
            )
            .expect("evdev frame");
        assert!(
            matches!(frame, ProviderFrame::Evdev(events) if events.last().is_some_and(|event| event.event_type == common::EV_SYN))
        );
        assert!(
            EVDEV_RESTRICTIONS
                .iter()
                .any(|restriction| restriction.feature == "motion")
        );
        let NativeControllerRealization::Evdev(realization) = evdev_realization() else {
            panic!("evdev realization")
        };
        assert_eq!(realization.identity.product_id, 0x2009);
        assert!(
            EVDEV_RESTRICTIONS
                .iter()
                .any(|restriction| restriction.feature == "HD rumble")
        );
        assert_eq!(realization.absolute_axes.len(), AXES.len());
    }

    #[test]
    fn trigger_controls_reach_hid_and_evdev_presentations() {
        let mut state = <(SwitchProInputs, SwitchProStatus)>::default();
        state.0.set_native(SwitchProControl::Zl, true);
        state.0.set_native(SwitchProControl::Zr, true);
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x80, 0x80);
        assert_eq!(bytes[4] & 0x80, 0x80);
        let ProviderFrame::Evdev(events) = switch_evdev_frame(&state) else {
            unreachable!()
        };
        assert!(
            events
                .iter()
                .any(|event| event.code == 312 && event.value == 1)
        );
        assert!(
            events
                .iter()
                .any(|event| event.code == 313 && event.value == 1)
        );
    }

    #[test]
    fn host_face_positions_use_the_nintendo_a_b_x_y_wire_bits() {
        let state = (
            SwitchProInputs {
                face: [true, true, true, true],
                ..Default::default()
            },
            SwitchProStatus::default(),
        );
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x0f, 0x0f);
        let state = (
            SwitchProInputs {
                face: [true, false, false, false],
                ..Default::default()
            },
            SwitchProStatus::default(),
        );
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x0f, 0b0100);
        let state = (
            SwitchProInputs {
                face: [false, true, false, false],
                ..Default::default()
            },
            SwitchProStatus::default(),
        );
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        assert_eq!(bytes[2] & 0x0f, 0b1000);
    }

    #[test]
    fn switch_stick_packing_has_a_centered_twelve_bit_domain() {
        let mut packed = [0; 3];
        pack(&mut packed, 0, 0);
        assert_eq!(packed, [0, 0x08, 0x80]);
        pack(&mut packed, i16::MIN, i16::MAX);
        assert_eq!(packed, [0, 0xf0, 0xff]);
    }

    #[test]
    fn switch_hid_wire_y_compensates_for_hid_nintendo_inversion() {
        let state = (
            SwitchProInputs {
                left: (SwitchProAxis::new(0), SwitchProAxis::new(i16::MAX)),
                right: (SwitchProAxis::new(0), SwitchProAxis::new(i16::MIN)),
                ..Default::default()
            },
            SwitchProStatus::default(),
        );
        let ProviderFrame::HidInput { bytes, .. } = switch_frame(&state) else {
            unreachable!()
        };
        let unpack_y = |packed: &[u8]| u16::from(packed[1] >> 4) | (u16::from(packed[2]) << 4);
        assert!(
            unpack_y(&bytes[5..8]) <= 1,
            "positive public Y must become low Switch-wire Y so Linux reports down"
        );
        assert!(
            unpack_y(&bytes[8..11]) >= 4094,
            "negative public Y must become high Switch-wire Y so Linux reports up"
        );
    }

    #[test]
    fn hid_identity_matches_openpuck_usb_personality() {
        let identity = fixture_identity();
        let realization = hid(identity);
        let association = crate::ControllerAssociation::requested(&realization);
        assert_eq!(
            association.requested_unique_id,
            Some(common::address_label(identity.bytes()))
        );
        let NativeControllerRealization::Uhid(realization) = realization else {
            unreachable!()
        };
        assert_eq!(realization.device_name, "Pro Controller");
        assert_eq!(realization.identity.vendor_id, 0x057e);
        assert_eq!(realization.identity.product_id, 0x2009);
        assert!(
            EVDEV_RESTRICTIONS
                .iter()
                .any(|restriction| restriction.feature == "HD rumble")
        );
        assert_eq!(realization.identity.version, 0x0220);
    }

    fn assert_spi_reply(address: u32, expected: &[u8]) {
        let state = <(SwitchProInputs, SwitchProStatus)>::default();
        let mut arguments = address.to_le_bytes().to_vec();
        arguments.push(u8::try_from(expected.len()).unwrap());
        let (reply, enabled) =
            switch_subcommand_reply(&state, 0x10, &arguments, fixture_identity());
        assert!(!enabled);
        let mut bytes = vec![0; 63];
        bytes[..12].copy_from_slice(&switch_input_prefix(&state));
        bytes[12] = 0x90;
        bytes[13] = 0x10;
        bytes[14..19].copy_from_slice(&arguments);
        bytes[19..19 + expected.len()].copy_from_slice(expected);
        assert_eq!(
            reply,
            ProviderFrame::HidInput {
                report_id: Some(0x21),
                bytes,
            },
            "SPI read at {address:#x}"
        );
    }

    #[test]
    fn spi_reads_return_imu_and_factory_stick_calibration() {
        assert_spi_reply(
            0x6020,
            &[
                0, 0, 0, 0, 0, 0, 0, 0x40, 0, 0x40, 0, 0x40, 0, 0, 0, 0, 0, 0, 0x3b, 0x34, 0x3b,
                0x34, 0x3b, 0x34,
            ],
        );
        assert_spi_reply(
            0x603d,
            &[
                0x08, 0x87, 0x70, 0x00, 0x08, 0x80, 0x08, 0x87, 0x70, 0x00, 0x08, 0x80, 0x08, 0x87,
                0x70, 0x08, 0x87, 0x70,
            ],
        );
    }

    #[test]
    fn spi_reads_respect_range_boundaries_and_erased_gaps() {
        for (address, expected) in [
            (0x601f, 0xff),
            (0x6020, 0),
            (0x6037, 0x34),
            (0x6038, 0xff),
            (0x603c, 0xff),
            (0x603d, 0x08),
            (0x604e, 0x70),
            (0x604f, 0xff),
            (0x6050, 0xff),
            (0x605a, 0xff),
            (0x605b, 0xff),
        ] {
            assert_spi_reply(address, &[expected]);
        }
        assert_spi_reply(0x6038, &[0xff; 5]);
        assert_spi_reply(0x6050, &[0xff; 11]);
        assert_spi_reply(0x7000, &[0xff; 16]);
        assert_spi_reply(
            0x6036,
            &[0x3b, 0x34, 0xff, 0xff, 0xff, 0xff, 0xff, 0x08, 0x87],
        );
        assert_spi_reply(0x604d, &[0x87, 0x70, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn device_info_does_not_advertise_erased_spi_colors() {
        assert_spi_reply(0x6050, &[0xff; 11]);
        let state = <(SwitchProInputs, SwitchProStatus)>::default();
        let identity = fixture_identity();
        let (reply, enabled) = switch_subcommand_reply(&state, 0x02, &[], identity);
        assert!(!enabled);
        let mut bytes = vec![0; 63];
        bytes[..12].copy_from_slice(&switch_input_prefix(&state));
        bytes[12..18].copy_from_slice(&[0x82, 0x02, 3, 0x48, 3, 2]);
        bytes[18..24].copy_from_slice(&identity.bytes());
        bytes[24] = 1;
        assert_eq!(
            reply,
            ProviderFrame::HidInput {
                report_id: Some(0x21),
                bytes,
            }
        );
    }

    #[test]
    fn steam_style_subcommands_receive_full_replies_and_enable_motion() {
        let state = <(SwitchProInputs, SwitchProStatus)>::default();
        let (device_info, enabled) = switch_subcommand_reply(&state, 0x02, &[], fixture_identity());
        assert!(!enabled);
        assert!(
            matches!(device_info, ProviderFrame::HidInput { report_id: Some(0x21), bytes } if bytes.len() == 63 && bytes[12] == 0x82 && bytes[13] == 0x02)
        );
        let (mode, enabled) = switch_subcommand_reply(&state, 0x03, &[0x30], fixture_identity());
        assert!(enabled);
        assert!(
            matches!(mode, ProviderFrame::HidInput { report_id: Some(0x21), bytes } if bytes[12] == 0x80 && bytes[13] == 0x03)
        );
        assert!(
            matches!(switch_usb_reply(2, fixture_identity()), ProviderFrame::HidInput { report_id: Some(0x81), bytes } if bytes.len() == 63 && bytes[0] == 2)
        );
        let (calibration, enabled) =
            switch_subcommand_reply(&state, 0x10, &[0x20, 0x60, 0, 0, 6], fixture_identity());
        assert!(!enabled);
        assert!(
            matches!(calibration, ProviderFrame::HidInput { report_id: Some(0x21), bytes } if bytes[12] == 0x90 && bytes[14..25] == [0x20, 0x60, 0, 0, 6, 0, 0, 0, 0, 0, 0])
        );
    }
}
