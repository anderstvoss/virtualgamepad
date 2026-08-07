#![forbid(unsafe_code)]

//! Compiled implementations for the curated controller set.
//!
//! The public state layer deliberately has no provider dependency. Providers
//! receive a complete typed state only at commit time, keeping updates cheap,
//! deterministic, and straightforward to test.

use gr_backend_api::{BackendFrame, EvdevEvent};
use gr_controller_contract::{
    ControlError, ControlUpdate, ControllerDefinition, ControllerDriver, ControllerKind,
    DpadDirection, FaceButton, RealizationRequirements, Stick, StickPosition, Trigger,
};
use gr_core::{
    DualSenseInput, GenericGamepadInput, ProfileInputPayload, SteamControllerInput, TwinStickAxes,
    Xbox360Input,
};

pub use gr_core::{DualSenseMotion, DualSenseTouchContact, MotionAxes};

/// Reverse output from a generic compatibility gamepad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericGamepadOutputEvent {
    Rumble { strong: u16, weak: u16 },
    RawEvdevEvents { events: Vec<EvdevEvent> },
}

/// Reverse output from an Xbox 360 controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Xbox360OutputEvent {
    Rumble { strong: u16, weak: u16 },
    RawEvdevEvents { events: Vec<EvdevEvent> },
}

/// Reverse output from a `DualSense` controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DualSenseOutputEvent {
    Rumble {
        strong: u16,
        weak: u16,
    },
    Lighting {
        red: Option<u8>,
        green: Option<u8>,
        blue: Option<u8>,
        player_index: Option<u8>,
    },
    TriggerEffect {
        mode: String,
    },
    Audio {
        action: String,
        target: Option<String>,
    },
    FeatureRequest {
        request: String,
    },
    RawHidReport {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
    RawTransportPacket {
        endpoint_id: u8,
        bytes: Vec<u8>,
    },
}

/// Reverse output from a Steam Controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamControllerOutputEvent {
    Rumble {
        strong: u16,
        weak: u16,
    },
    Lighting {
        red: Option<u8>,
        green: Option<u8>,
        blue: Option<u8>,
        player_index: Option<u8>,
    },
    FeatureRequest {
        request: String,
    },
    RawHidReport {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
}

/// Closed tagged reverse-output wrapper for heterogeneous collections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CuratedControllerOutputEvent {
    GenericGamepad(GenericGamepadOutputEvent),
    Xbox360(Xbox360OutputEvent),
    DualSense(DualSenseOutputEvent),
    SteamController(SteamControllerOutputEvent),
}

/// Static definition for the generic compatibility controller.
pub struct GenericGamepadDefinition;
/// Static definition for the Xbox 360 controller.
pub struct Xbox360Definition;
/// Static definition for the `DualSense` controller.
pub struct DualSenseDefinition;
/// Static definition for the Steam Controller.
pub struct SteamControllerDefinition;

macro_rules! definition {
    ($type:ident, $kind:expr, $requirements:expr) => {
        impl ControllerDefinition for $type {
            fn kind(&self) -> ControllerKind {
                $kind
            }

            fn requirements(&self) -> RealizationRequirements {
                $requirements
            }
        }
    };
}

definition!(
    GenericGamepadDefinition,
    ControllerKind::GenericGamepad,
    RealizationRequirements {
        requires_identity: false,
        requires_transport: false,
        requires_reverse_output: true,
    }
);
definition!(
    Xbox360Definition,
    ControllerKind::Xbox360,
    RealizationRequirements {
        requires_identity: false,
        requires_transport: false,
        requires_reverse_output: true,
    }
);
definition!(
    DualSenseDefinition,
    ControllerKind::DualSense,
    RealizationRequirements {
        requires_identity: true,
        requires_transport: false,
        requires_reverse_output: true,
    }
);
definition!(
    SteamControllerDefinition,
    ControllerKind::SteamController,
    RealizationRequirements {
        requires_identity: true,
        requires_transport: false,
        requires_reverse_output: true,
    }
);

static GENERIC_GAMEPAD_DEFINITION: GenericGamepadDefinition = GenericGamepadDefinition;
static XBOX360_DEFINITION: Xbox360Definition = Xbox360Definition;
static DUALSENSE_DEFINITION: DualSenseDefinition = DualSenseDefinition;
static STEAM_CONTROLLER_DEFINITION: SteamControllerDefinition = SteamControllerDefinition;

/// Return the single compiled definition for a curated controller kind.
#[must_use]
pub fn definition_for(kind: ControllerKind) -> &'static dyn ControllerDefinition {
    match kind {
        ControllerKind::GenericGamepad => &GENERIC_GAMEPAD_DEFINITION,
        ControllerKind::Xbox360 => &XBOX360_DEFINITION,
        ControllerKind::DualSense => &DUALSENSE_DEFINITION,
        ControllerKind::SteamController => &STEAM_CONTROLLER_DEFINITION,
    }
}

/// Prepared compiled driver for one curated controller kind.
#[derive(Debug, Clone, Copy)]
pub struct CompiledControllerDriver {
    kind: ControllerKind,
}

impl CompiledControllerDriver {
    #[must_use]
    pub const fn new(kind: ControllerKind) -> Self {
        Self { kind }
    }
}

impl ControllerDefinition for CompiledControllerDriver {
    fn kind(&self) -> ControllerKind {
        self.kind
    }

    fn requirements(&self) -> RealizationRequirements {
        definition_for(self.kind).requirements()
    }
}

impl ControllerDriver for CompiledControllerDriver {
    type State = ControllerState;
    type Frame = PreparedControllerFrame;

    fn neutral_state(&self) -> Self::State {
        ControllerState::neutral(self.kind)
    }

    fn apply_normalized(
        &self,
        state: &mut Self::State,
        update: ControlUpdate,
    ) -> Result<(), ControlError> {
        if state.kind() != self.kind {
            return Err(ControlError::UnsupportedControl {
                controller: self.kind,
                control: "controller state from a different driver",
            });
        }
        state.apply(update)
    }

    fn encode(&self, state: &Self::State) -> Result<Self::Frame, ControlError> {
        if state.kind() != self.kind {
            return Err(ControlError::UnsupportedControl {
                controller: self.kind,
                control: "controller state from a different driver",
            });
        }
        Ok(PreparedControllerFrame::from(state))
    }
}

/// A complete, immutable native frame prepared by a compiled controller.
///
/// Providers receive this boundary instead of mutable state.  Each variant is
/// deliberately controller-specific, so adding a curated controller extends
/// this enum and its provider realization once, without profile identifiers or
/// runtime translation selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparedControllerFrame {
    GenericGamepad(GenericGamepadInput),
    Xbox360(Xbox360Input),
    DualSense(DualSenseInput),
    SteamController(SteamControllerInput),
}

impl PreparedControllerFrame {
    #[must_use]
    pub const fn kind(&self) -> ControllerKind {
        match self {
            Self::GenericGamepad(_) => ControllerKind::GenericGamepad,
            Self::Xbox360(_) => ControllerKind::Xbox360,
            Self::DualSense(_) => ControllerKind::DualSense,
            Self::SteamController(_) => ControllerKind::SteamController,
        }
    }

    /// Convert only at the legacy provider boundary during the migration.
    #[must_use]
    pub fn legacy_payload(self) -> ProfileInputPayload {
        match self {
            Self::GenericGamepad(state) => ProfileInputPayload::GenericGamepad(state),
            Self::Xbox360(state) => ProfileInputPayload::Xbox360(state),
            Self::DualSense(state) => ProfileInputPayload::DualSense(state),
            Self::SteamController(state) => ProfileInputPayload::SteamController(state),
        }
    }

    /// Encode this immutable native frame for its explicitly selected Linux
    /// target. This is the profile-free forward provider boundary.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::UnsupportedControl`] if the target cannot
    /// transport this controller frame.
    pub fn encode_for(
        self,
        target: gr_controller_contract::LinuxTarget,
    ) -> Result<BackendFrame, ControlError> {
        match (self, target) {
            (Self::GenericGamepad(state), gr_controller_contract::LinuxTarget::Uinput) => {
                Ok(BackendFrame::EvdevEvents {
                    events: generic_evdev_events(&state),
                })
            }
            (Self::Xbox360(state), gr_controller_contract::LinuxTarget::Uinput) => {
                Ok(BackendFrame::EvdevEvents {
                    events: xbox_evdev_events(&state),
                })
            }
            (Self::DualSense(state), gr_controller_contract::LinuxTarget::Uhid) => {
                Ok(BackendFrame::HidInputReport {
                    report_id: Some(0x01),
                    bytes: dualsense_hid_report(&state),
                })
            }
            (Self::DualSense(state), gr_controller_contract::LinuxTarget::UsbTransport) => {
                Ok(BackendFrame::TransportPacket {
                    endpoint_id: 0x01,
                    bytes: dualsense_hid_report(&state),
                })
            }
            (frame, target) => Err(ControlError::UnsupportedControl {
                controller: frame.kind(),
                control: match target {
                    gr_controller_contract::LinuxTarget::Uinput => "uinput frame encoding",
                    gr_controller_contract::LinuxTarget::Uhid => "UHID frame encoding",
                    gr_controller_contract::LinuxTarget::UsbTransport => {
                        "USB transport frame encoding"
                    }
                },
            }),
        }
    }
}

const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;

fn generic_evdev_events(state: &GenericGamepadInput) -> Vec<EvdevEvent> {
    let mut events = Vec::with_capacity(19);
    push_button(&mut events, BTN_SOUTH, state.buttons.south);
    push_button(&mut events, BTN_EAST, state.buttons.east);
    push_button(&mut events, BTN_WEST, state.buttons.west);
    push_button(&mut events, BTN_NORTH, state.buttons.north);
    push_button(&mut events, BTN_TL, state.buttons.left_shoulder);
    push_button(&mut events, BTN_TR, state.buttons.right_shoulder);
    push_button(&mut events, BTN_THUMBL, state.buttons.left_stick_button);
    push_button(&mut events, BTN_THUMBR, state.buttons.right_stick_button);
    push_button(&mut events, BTN_START, state.buttons.menu_primary);
    push_button(&mut events, BTN_SELECT, state.buttons.menu_secondary);
    push_button(&mut events, BTN_MODE, state.buttons.guide);
    push_axis(
        &mut events,
        ABS_HAT0X,
        dpad_axis(state.dpad.left, state.dpad.right),
    );
    push_axis(
        &mut events,
        ABS_HAT0Y,
        dpad_axis(state.dpad.up, state.dpad.down),
    );
    push_axis(&mut events, ABS_X, i32::from(state.sticks.left_x));
    push_axis(&mut events, ABS_Y, i32::from(state.sticks.left_y));
    push_axis(&mut events, ABS_RX, i32::from(state.sticks.right_x));
    push_axis(&mut events, ABS_RY, i32::from(state.sticks.right_y));
    push_axis(&mut events, ABS_Z, i32::from(state.triggers.left_trigger));
    push_axis(&mut events, ABS_RZ, i32::from(state.triggers.right_trigger));
    events
}

fn xbox_evdev_events(state: &Xbox360Input) -> Vec<EvdevEvent> {
    let mut events = Vec::with_capacity(19);
    push_button(&mut events, BTN_SOUTH, state.buttons.face.a);
    push_button(&mut events, BTN_EAST, state.buttons.face.b);
    push_button(&mut events, BTN_WEST, state.buttons.face.x);
    push_button(&mut events, BTN_NORTH, state.buttons.face.y);
    push_button(&mut events, BTN_TL, state.buttons.shoulders.lb);
    push_button(&mut events, BTN_TR, state.buttons.shoulders.rb);
    push_button(&mut events, BTN_THUMBL, state.buttons.stick_clicks.ls);
    push_button(&mut events, BTN_THUMBR, state.buttons.stick_clicks.rs);
    push_button(&mut events, BTN_START, state.buttons.system.start);
    push_button(&mut events, BTN_SELECT, state.buttons.system.back);
    push_button(&mut events, BTN_MODE, state.buttons.system.guide);
    push_axis(
        &mut events,
        ABS_HAT0X,
        dpad_axis(state.dpad.left, state.dpad.right),
    );
    push_axis(
        &mut events,
        ABS_HAT0Y,
        dpad_axis(state.dpad.up, state.dpad.down),
    );
    push_axis(&mut events, ABS_X, i32::from(state.sticks.left_x));
    push_axis(&mut events, ABS_Y, i32::from(state.sticks.left_y));
    push_axis(&mut events, ABS_RX, i32::from(state.sticks.right_x));
    push_axis(&mut events, ABS_RY, i32::from(state.sticks.right_y));
    push_axis(&mut events, ABS_Z, i32::from(state.triggers.lt));
    push_axis(&mut events, ABS_RZ, i32::from(state.triggers.rt));
    events
}

fn dualsense_hid_report(state: &DualSenseInput) -> Vec<u8> {
    let mut bytes = vec![0; 64];
    bytes[0] = axis_u8(state.sticks.left_x);
    bytes[1] = axis_u8(state.sticks.left_y);
    bytes[2] = axis_u8(state.sticks.right_x);
    bytes[3] = axis_u8(state.sticks.right_y);
    bytes[4] = trigger_u8(state.triggers.l2);
    bytes[5] = trigger_u8(state.triggers.r2);
    bytes[7] = dpad_hat(state.dpad)
        | bit(state.buttons.face.square, 4)
        | bit(state.buttons.face.cross, 5)
        | bit(state.buttons.face.circle, 6)
        | bit(state.buttons.face.triangle, 7);
    bytes[8] = bit(state.buttons.shoulders.l1, 0)
        | bit(state.buttons.shoulders.r1, 1)
        | bit(state.buttons.system.create, 4)
        | bit(state.buttons.system.options, 5)
        | bit(state.buttons.stick_clicks.l3, 6)
        | bit(state.buttons.stick_clicks.r3, 7);
    bytes[9] = bit(state.buttons.system.ps, 0) | bit(state.buttons.system.touchpad_click, 1);
    for (offset, value) in [
        (15, state.motion.gyroscope.x),
        (17, state.motion.gyroscope.y),
        (19, state.motion.gyroscope.z),
        (21, state.motion.accelerometer.x),
        (23, state.motion.accelerometer.y),
        (25, state.motion.accelerometer.z),
    ] {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    encode_touch(&mut bytes[32..36], state.touchpad.contact_1, 0);
    encode_touch(&mut bytes[36..40], state.touchpad.contact_2, 1);
    bytes
}

fn push_button(events: &mut Vec<EvdevEvent>, code: u16, pressed: bool) {
    events.push(EvdevEvent {
        event_type: EV_KEY,
        code,
        value: i32::from(pressed),
    });
}
fn push_axis(events: &mut Vec<EvdevEvent>, code: u16, value: i32) {
    events.push(EvdevEvent {
        event_type: EV_ABS,
        code,
        value,
    });
}
fn dpad_axis(negative: bool, positive: bool) -> i32 {
    match (negative, positive) {
        (true, false) => -1,
        (false, true) => 1,
        _ => 0,
    }
}
fn axis_u8(value: i16) -> u8 {
    u8::try_from(((i32::from(value) + 32_768) * 255) / 65_535).expect("scaled axis fits")
}
fn trigger_u8(value: u16) -> u8 {
    (value / 257) as u8
}
fn bit(enabled: bool, position: u8) -> u8 {
    if enabled { 1 << position } else { 0 }
}
fn dpad_hat(dpad: gr_core::Dpad) -> u8 {
    match (dpad.up, dpad.right, dpad.down, dpad.left) {
        (true, false, false, false) => 0,
        (true, true, false, false) => 1,
        (false, true, false, false) => 2,
        (false, true, true, false) => 3,
        (false, false, true, false) => 4,
        (false, false, true, true) => 5,
        (false, false, false, true) => 6,
        (true, false, false, true) => 7,
        _ => 8,
    }
}
fn encode_touch(bytes: &mut [u8], contact: DualSenseTouchContact, counter: u8) {
    let x = contact.x.min(0x0fff);
    let y = contact.y.min(0x0fff);
    let x_high = u8::try_from(x >> 8).expect("12-bit touch coordinate high byte fits");
    let y_low = u8::try_from(y & 0x0f).expect("12-bit touch coordinate low nibble fits");
    let y_high = u8::try_from(y >> 4).expect("12-bit touch coordinate high byte fits");
    bytes[0] = if contact.active {
        counter & 0x7f
    } else {
        0x80 | (counter & 0x7f)
    };
    bytes[1] = u8::try_from(x & 0xff).expect("12-bit touch coordinate low byte fits");
    bytes[2] = (x_high & 0x0f) | (y_low << 4);
    bytes[3] = y_high;
}

impl From<&ControllerState> for PreparedControllerFrame {
    fn from(state: &ControllerState) -> Self {
        match state {
            ControllerState::GenericGamepad(state) => Self::GenericGamepad(state.clone()),
            ControllerState::Xbox360(state) => Self::Xbox360(state.clone()),
            ControllerState::DualSense(state) => Self::DualSense(state.clone()),
            ControllerState::SteamController(state) => Self::SteamController(state.clone()),
        }
    }
}

/// The complete mutable state of one curated controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerState {
    GenericGamepad(GenericGamepadInput),
    Xbox360(Xbox360Input),
    DualSense(DualSenseInput),
    SteamController(SteamControllerInput),
}

impl ControllerState {
    #[must_use]
    pub const fn neutral(kind: ControllerKind) -> Self {
        match kind {
            ControllerKind::GenericGamepad => Self::GenericGamepad(GenericGamepadInput::neutral()),
            ControllerKind::Xbox360 => Self::Xbox360(Xbox360Input::neutral()),
            ControllerKind::DualSense => Self::DualSense(DualSenseInput::neutral()),
            ControllerKind::SteamController => {
                Self::SteamController(SteamControllerInput::neutral())
            }
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ControllerKind {
        match self {
            Self::GenericGamepad(_) => ControllerKind::GenericGamepad,
            Self::Xbox360(_) => ControllerKind::Xbox360,
            Self::DualSense(_) => ControllerKind::DualSense,
            Self::SteamController(_) => ControllerKind::SteamController,
        }
    }

    /// Convert the prepared state through the legacy provider seam.
    ///
    /// This conversion is intentionally isolated here: Linux providers still
    /// consume the pre-redesign report pipeline while their contracts migrate.
    #[must_use]
    pub fn legacy_payload(&self) -> ProfileInputPayload {
        match self {
            Self::GenericGamepad(state) => ProfileInputPayload::GenericGamepad(state.clone()),
            Self::Xbox360(state) => ProfileInputPayload::Xbox360(state.clone()),
            Self::DualSense(state) => ProfileInputPayload::DualSense(state.clone()),
            Self::SteamController(state) => ProfileInputPayload::SteamController(state.clone()),
        }
    }

    /// Apply a normalized update without touching a provider.
    ///
    /// Errors leave this state unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] when this controller lacks the normalized
    /// control in `update`.
    pub fn apply(&mut self, update: ControlUpdate) -> Result<(), ControlError> {
        match update {
            ControlUpdate::FaceButton { button, pressed } => self.set_face(button, pressed),
            ControlUpdate::Dpad { direction, pressed } => self.set_dpad(direction, pressed),
            ControlUpdate::Stick { stick, position } => self.set_stick(stick, position),
            ControlUpdate::Trigger { trigger, value } => self.set_trigger(trigger, value),
        }
    }

    /// Apply a controller-native update without touching a provider.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::UnsupportedNativeControl`] when `update` does
    /// not belong to this controller. State remains unchanged on error.
    pub fn apply_native(&mut self, update: NativeControlUpdate) -> Result<(), ControlError> {
        if update.control.kind() != self.kind() {
            return Err(ControlError::UnsupportedNativeControl {
                controller: self.kind(),
                control: update.control.name(),
            });
        }
        self.set_native(update.control, update.pressed)
    }

    /// Set a normalized face button.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] if the controller does not expose face buttons.
    pub fn set_face(&mut self, button: FaceButton, pressed: bool) -> Result<(), ControlError> {
        match self {
            Self::GenericGamepad(state) => match button {
                FaceButton::North => state.buttons.north = pressed,
                FaceButton::South => state.buttons.south = pressed,
                FaceButton::East => state.buttons.east = pressed,
                FaceButton::West => state.buttons.west = pressed,
            },
            Self::Xbox360(state) => match button {
                FaceButton::North => state.buttons.face.y = pressed,
                FaceButton::South => state.buttons.face.a = pressed,
                FaceButton::East => state.buttons.face.b = pressed,
                FaceButton::West => state.buttons.face.x = pressed,
            },
            Self::DualSense(state) => match button {
                FaceButton::North => state.buttons.face.triangle = pressed,
                FaceButton::South => state.buttons.face.cross = pressed,
                FaceButton::East => state.buttons.face.circle = pressed,
                FaceButton::West => state.buttons.face.square = pressed,
            },
            Self::SteamController(state) => match button {
                FaceButton::North => state.buttons.y = pressed,
                FaceButton::South => state.buttons.a = pressed,
                FaceButton::East => state.buttons.b = pressed,
                FaceButton::West => state.buttons.x = pressed,
            },
        }
        Ok(())
    }

    /// Set one normalized D-pad direction.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::UnsupportedControl`] when the controller has
    /// no D-pad.
    pub fn set_dpad(
        &mut self,
        direction: DpadDirection,
        pressed: bool,
    ) -> Result<(), ControlError> {
        let dpad = match self {
            Self::GenericGamepad(state) => &mut state.dpad,
            Self::Xbox360(state) => &mut state.dpad,
            Self::DualSense(state) => &mut state.dpad,
            Self::SteamController(_) => {
                return Err(ControlError::UnsupportedControl {
                    controller: self.kind(),
                    control: "dpad",
                });
            }
        };
        match direction {
            DpadDirection::Up => dpad.up = pressed,
            DpadDirection::Down => dpad.down = pressed,
            DpadDirection::Left => dpad.left = pressed,
            DpadDirection::Right => dpad.right = pressed,
        }
        Ok(())
    }

    /// Set a normalized stick position.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::UnsupportedControl`] when that stick is absent.
    pub fn set_stick(&mut self, stick: Stick, position: StickPosition) -> Result<(), ControlError> {
        match self {
            Self::GenericGamepad(state) => set_twin_stick(&mut state.sticks, stick, position),
            Self::Xbox360(state) => set_twin_stick(&mut state.sticks, stick, position),
            Self::DualSense(state) => set_twin_stick(&mut state.sticks, stick, position),
            Self::SteamController(state) => match stick {
                Stick::Left => {
                    state.sticks.left_stick_x = position.x;
                    state.sticks.left_stick_y = position.y;
                }
                Stick::Right => {
                    return Err(ControlError::UnsupportedControl {
                        controller: self.kind(),
                        control: "right stick",
                    });
                }
            },
        }
        Ok(())
    }

    /// Set a normalized analog trigger.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] when the controller lacks the trigger.
    pub fn set_trigger(&mut self, trigger: Trigger, value: u16) -> Result<(), ControlError> {
        match self {
            Self::GenericGamepad(state) => match trigger {
                Trigger::Left => state.triggers.left_trigger = value,
                Trigger::Right => state.triggers.right_trigger = value,
            },
            Self::Xbox360(state) => match trigger {
                Trigger::Left => state.triggers.lt = value,
                Trigger::Right => state.triggers.rt = value,
            },
            Self::DualSense(state) => match trigger {
                Trigger::Left => state.triggers.l2 = value,
                Trigger::Right => state.triggers.r2 = value,
            },
            Self::SteamController(state) => match trigger {
                Trigger::Left => state.triggers.lt = value,
                Trigger::Right => state.triggers.rt = value,
            },
        }
        Ok(())
    }

    /// Set one `DualSense` touch contact in native touch-surface coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] if this is not a `DualSense`, or `contact` is
    /// not zero or one. Errors leave state unchanged.
    pub fn set_dualsense_touch(
        &mut self,
        contact: usize,
        value: DualSenseTouchContact,
    ) -> Result<(), ControlError> {
        if contact >= 2 {
            return Err(ControlError::InvalidIndex {
                control: "DualSense touch contact",
                index: contact,
                exclusive_maximum: 2,
            });
        }
        let Self::DualSense(state) = self else {
            return Err(ControlError::UnsupportedControl {
                controller: self.kind(),
                control: "DualSense touch surface",
            });
        };
        if contact == 0 {
            state.touchpad.contact_1 = value;
        } else {
            state.touchpad.contact_2 = value;
        }
        Ok(())
    }

    /// Set a `DualSense` raw motion sample.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError::UnsupportedControl`] when the controller does
    /// not expose the `DualSense` motion surface.
    pub fn set_dualsense_motion(&mut self, value: DualSenseMotion) -> Result<(), ControlError> {
        let Self::DualSense(state) = self else {
            return Err(ControlError::UnsupportedControl {
                controller: self.kind(),
                control: "DualSense motion",
            });
        };
        state.motion = value;
        Ok(())
    }

    /// Set one `Steam Controller` trackpad position in native units.
    ///
    /// # Errors
    ///
    /// Returns [`ControlError`] if this is not a `Steam Controller`, or if the
    /// pad index is not zero (left) or one (right).
    pub fn set_steam_trackpad(
        &mut self,
        pad: usize,
        position: StickPosition,
    ) -> Result<(), ControlError> {
        if pad >= 2 {
            return Err(ControlError::InvalidIndex {
                control: "Steam Controller trackpad",
                index: pad,
                exclusive_maximum: 2,
            });
        }
        let Self::SteamController(state) = self else {
            return Err(ControlError::UnsupportedControl {
                controller: self.kind(),
                control: "Steam Controller trackpad",
            });
        };
        if pad == 0 {
            state.sticks.left_pad_x = position.x;
            state.sticks.left_pad_y = position.y;
        } else {
            state.sticks.right_pad_x = position.x;
            state.sticks.right_pad_y = position.y;
        }
        Ok(())
    }

    fn set_native(&mut self, control: NativeControl, pressed: bool) -> Result<(), ControlError> {
        let controller = self.kind();
        match (self, control) {
            (Self::GenericGamepad(state), NativeControl::GenericGamepad(control)) => {
                match control {
                    GenericGamepadControl::Guide => state.buttons.guide = pressed,
                }
            }
            (Self::Xbox360(state), NativeControl::Xbox360(control)) => match control {
                XboxControl::A => state.buttons.face.a = pressed,
                XboxControl::B => state.buttons.face.b = pressed,
                XboxControl::X => state.buttons.face.x = pressed,
                XboxControl::Y => state.buttons.face.y = pressed,
                XboxControl::Guide => state.buttons.system.guide = pressed,
                XboxControl::Start => state.buttons.system.start = pressed,
                XboxControl::Back => state.buttons.system.back = pressed,
            },
            (Self::DualSense(state), NativeControl::DualSense(control)) => match control {
                DualSenseControl::Cross => state.buttons.face.cross = pressed,
                DualSenseControl::Circle => state.buttons.face.circle = pressed,
                DualSenseControl::Square => state.buttons.face.square = pressed,
                DualSenseControl::Triangle => state.buttons.face.triangle = pressed,
                DualSenseControl::PlayStation => state.buttons.system.ps = pressed,
                DualSenseControl::Create => state.buttons.system.create = pressed,
                DualSenseControl::Options => state.buttons.system.options = pressed,
                DualSenseControl::TouchpadClick => state.buttons.system.touchpad_click = pressed,
            },
            (Self::SteamController(state), NativeControl::SteamController(control)) => {
                match control {
                    SteamControllerControl::A => state.buttons.a = pressed,
                    SteamControllerControl::B => state.buttons.b = pressed,
                    SteamControllerControl::X => state.buttons.x = pressed,
                    SteamControllerControl::Y => state.buttons.y = pressed,
                    SteamControllerControl::Steam => state.buttons.steam = pressed,
                    SteamControllerControl::LeftGrip => state.buttons.left_grip = pressed,
                    SteamControllerControl::RightGrip => state.buttons.right_grip = pressed,
                }
            }
            (_, control) => {
                return Err(ControlError::UnsupportedNativeControl {
                    controller,
                    control: control.name(),
                });
            }
        }
        Ok(())
    }
}

fn set_twin_stick(sticks: &mut TwinStickAxes, stick: Stick, position: StickPosition) {
    match stick {
        Stick::Left => {
            sticks.left_x = position.x;
            sticks.left_y = position.y;
        }
        Stick::Right => {
            sticks.right_x = position.x;
            sticks.right_y = position.y;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeControlUpdate {
    pub control: NativeControl,
    pub pressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeControl {
    GenericGamepad(GenericGamepadControl),
    Xbox360(XboxControl),
    DualSense(DualSenseControl),
    SteamController(SteamControllerControl),
}

impl NativeControl {
    const fn kind(self) -> ControllerKind {
        match self {
            Self::GenericGamepad(_) => ControllerKind::GenericGamepad,
            Self::Xbox360(_) => ControllerKind::Xbox360,
            Self::DualSense(_) => ControllerKind::DualSense,
            Self::SteamController(_) => ControllerKind::SteamController,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::GenericGamepad(GenericGamepadControl::Guide) => "generic-gamepad.guide",
            Self::Xbox360(XboxControl::A) => "xbox.a",
            Self::Xbox360(XboxControl::B) => "xbox.b",
            Self::Xbox360(XboxControl::X) => "xbox.x",
            Self::Xbox360(XboxControl::Y) => "xbox.y",
            Self::Xbox360(XboxControl::Guide) => "xbox.guide",
            Self::Xbox360(XboxControl::Start) => "xbox.start",
            Self::Xbox360(XboxControl::Back) => "xbox.back",
            Self::DualSense(DualSenseControl::Cross) => "dualsense.cross",
            Self::DualSense(DualSenseControl::Circle) => "dualsense.circle",
            Self::DualSense(DualSenseControl::Square) => "dualsense.square",
            Self::DualSense(DualSenseControl::Triangle) => "dualsense.triangle",
            Self::DualSense(DualSenseControl::PlayStation) => "dualsense.playstation",
            Self::DualSense(DualSenseControl::Create) => "dualsense.create",
            Self::DualSense(DualSenseControl::Options) => "dualsense.options",
            Self::DualSense(DualSenseControl::TouchpadClick) => "dualsense.touchpad-click",
            Self::SteamController(SteamControllerControl::A) => "steam.a",
            Self::SteamController(SteamControllerControl::B) => "steam.b",
            Self::SteamController(SteamControllerControl::X) => "steam.x",
            Self::SteamController(SteamControllerControl::Y) => "steam.y",
            Self::SteamController(SteamControllerControl::Steam) => "steam.steam",
            Self::SteamController(SteamControllerControl::LeftGrip) => "steam.left-grip",
            Self::SteamController(SteamControllerControl::RightGrip) => "steam.right-grip",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericGamepadControl {
    Guide,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XboxControl {
    A,
    B,
    X,
    Y,
    Guide,
    Start,
    Back,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DualSenseControl {
    Cross,
    Circle,
    Square,
    Triangle,
    PlayStation,
    Create,
    Options,
    TouchpadClick,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteamControllerControl {
    A,
    B,
    X,
    Y,
    Steam,
    LeftGrip,
    RightGrip,
}

#[cfg(test)]
mod tests {
    use super::{
        ControllerState, DualSenseControl, NativeControl, NativeControlUpdate,
        PreparedControllerFrame, XboxControl, definition_for,
    };
    use gr_backend_api::BackendFrame;
    use gr_controller_contract::{
        ControlError, ControlUpdate, ControllerKind, FaceButton, LinuxTarget,
    };
    use proptest::prelude::*;

    #[test]
    fn normalized_and_native_face_buttons_share_state() {
        let mut normalized = ControllerState::neutral(ControllerKind::DualSense);
        normalized
            .apply(ControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("normalized update");
        let mut native = ControllerState::neutral(ControllerKind::DualSense);
        native
            .apply_native(NativeControlUpdate {
                control: NativeControl::DualSense(DualSenseControl::Cross),
                pressed: true,
            })
            .expect("native update");
        assert_eq!(normalized, native);
    }

    #[test]
    fn wrong_native_control_preserves_state() {
        let mut state = ControllerState::neutral(ControllerKind::DualSense);
        let original = state.clone();
        let error = state
            .apply_native(NativeControlUpdate {
                control: NativeControl::Xbox360(XboxControl::A),
                pressed: true,
            })
            .expect_err("wrong controller");
        assert!(matches!(
            error,
            ControlError::UnsupportedNativeControl { .. }
        ));
        assert_eq!(state, original);
    }

    #[test]
    fn compiled_definitions_keep_identity_requirements_out_of_runtime_switches() {
        let generic = definition_for(ControllerKind::GenericGamepad).requirements();
        let dualsense = definition_for(ControllerKind::DualSense).requirements();
        assert!(!generic.requires_identity);
        assert!(dualsense.requires_identity);
    }

    #[test]
    fn invalid_touch_contact_keeps_dualsense_state_unchanged() {
        let mut state = ControllerState::neutral(ControllerKind::DualSense);
        let original = state.clone();
        let error = state
            .set_dualsense_touch(2, super::DualSenseTouchContact::neutral())
            .expect_err("only two contacts are available");
        assert!(matches!(error, ControlError::InvalidIndex { .. }));
        assert_eq!(state, original);
    }

    #[test]
    fn native_frame_encoders_select_only_exact_target_shapes() {
        let generic = PreparedControllerFrame::from(&ControllerState::neutral(
            ControllerKind::GenericGamepad,
        ));
        let BackendFrame::EvdevEvents { events } = generic
            .encode_for(LinuxTarget::Uinput)
            .expect("generic uinput frame")
        else {
            panic!("generic controller must encode evdev events");
        };
        assert_eq!(events.len(), 19);

        let dualsense =
            PreparedControllerFrame::from(&ControllerState::neutral(ControllerKind::DualSense));
        let BackendFrame::HidInputReport { report_id, bytes } = dualsense
            .encode_for(LinuxTarget::Uhid)
            .expect("DualSense UHID frame")
        else {
            panic!("DualSense must encode a HID report");
        };
        assert_eq!(report_id, Some(1));
        assert_eq!(bytes.len(), 64);
        assert!(
            PreparedControllerFrame::from(&ControllerState::neutral(
                ControllerKind::SteamController,
            ))
            .encode_for(LinuxTarget::Uhid)
            .is_err()
        );
    }

    proptest! {
        #[test]
        fn every_normalized_dualsense_face_mapping_matches_its_native_label(
            north in any::<bool>(),
            south in any::<bool>(),
            east in any::<bool>(),
            west in any::<bool>(),
        ) {
            let mut normalized = ControllerState::neutral(ControllerKind::DualSense);
            normalized.apply(ControlUpdate::FaceButton { button: FaceButton::North, pressed: north })?;
            normalized.apply(ControlUpdate::FaceButton { button: FaceButton::South, pressed: south })?;
            normalized.apply(ControlUpdate::FaceButton { button: FaceButton::East, pressed: east })?;
            normalized.apply(ControlUpdate::FaceButton { button: FaceButton::West, pressed: west })?;

            let mut native = ControllerState::neutral(ControllerKind::DualSense);
            for (control, pressed) in [
                (DualSenseControl::Triangle, north),
                (DualSenseControl::Cross, south),
                (DualSenseControl::Circle, east),
                (DualSenseControl::Square, west),
            ] {
                native.apply_native(NativeControlUpdate { control: NativeControl::DualSense(control), pressed })?;
            }
            prop_assert_eq!(normalized, native);
        }
    }
}
