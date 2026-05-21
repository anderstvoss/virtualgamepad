#![forbid(unsafe_code)]

//! Core domain primitives for `virtualgamepad`.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

macro_rules! string_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

macro_rules! numeric_newtype {
    ($name:ident, $inner:ty) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $inner);

        impl $name {
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}

string_newtype!(ProfileId);
string_newtype!(BackendId);

numeric_newtype!(SessionId, u64);
numeric_newtype!(VendorId, u16);
numeric_newtype!(ProductId, u16);
numeric_newtype!(SequenceId, u64);
numeric_newtype!(Timestamp, u64);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error("unknown {kind} name `{value}`")]
    UnknownHumanName { kind: &'static str, value: String },
    #[error("profile id `{profile_id}` does not match payload variant `{payload_variant}`")]
    ProfilePayloadMismatch {
        profile_id: ProfileId,
        payload_variant: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FidelityTier {
    Compatibility,
    IdentityAware,
    HardwareFaithful,
}

impl FidelityTier {
    pub const ALL: [Self; 3] = [
        Self::Compatibility,
        Self::IdentityAware,
        Self::HardwareFaithful,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::IdentityAware => "identity-aware",
            Self::HardwareFaithful => "hardware-faithful",
        }
    }
}

impl fmt::Display for FidelityTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FidelityTier {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "compatibility" => Ok(Self::Compatibility),
            "identity-aware" => Ok(Self::IdentityAware),
            "hardware-faithful" => Ok(Self::HardwareFaithful),
            _ => Err(CoreError::UnknownHumanName {
                kind: "fidelity tier",
                value: value.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendLevel {
    Evdev,
    Hid,
    Transport,
}

impl BackendLevel {
    pub const ALL: [Self; 3] = [Self::Evdev, Self::Hid, Self::Transport];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Evdev => "evdev",
            Self::Hid => "hid",
            Self::Transport => "transport",
        }
    }
}

impl fmt::Display for BackendLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendFamily {
    LinuxUinput,
    LinuxUhid,
    LinuxTransportUsb,
    LinuxTransportBluetooth,
    WindowsHid,
    MacosHid,
}

impl BackendFamily {
    pub const ALL: [Self; 6] = [
        Self::LinuxUinput,
        Self::LinuxUhid,
        Self::LinuxTransportUsb,
        Self::LinuxTransportBluetooth,
        Self::WindowsHid,
        Self::MacosHid,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LinuxUinput => "linux-uinput",
            Self::LinuxUhid => "linux-uhid",
            Self::LinuxTransportUsb => "linux-transport-usb",
            Self::LinuxTransportBluetooth => "linux-transport-bluetooth",
            Self::WindowsHid => "windows-hid",
            Self::MacosHid => "macos-hid",
        }
    }
}

impl fmt::Display for BackendFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticInputFunction {
    Dpad,
    FaceBottom,
    FaceRight,
    FaceLeft,
    FaceTop,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    LeftStick,
    RightStick,
    LeftStickButton,
    RightStickButton,
    MenuPrimary,
    MenuSecondary,
    Guide,
    TouchSurface,
    TouchClick,
    Accelerometer,
    Gyroscope,
    PaddleLeft,
    PaddleRight,
}

impl SemanticInputFunction {
    pub const ALL: [Self; 22] = [
        Self::Dpad,
        Self::FaceBottom,
        Self::FaceRight,
        Self::FaceLeft,
        Self::FaceTop,
        Self::LeftShoulder,
        Self::RightShoulder,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::LeftStick,
        Self::RightStick,
        Self::LeftStickButton,
        Self::RightStickButton,
        Self::MenuPrimary,
        Self::MenuSecondary,
        Self::Guide,
        Self::TouchSurface,
        Self::TouchClick,
        Self::Accelerometer,
        Self::Gyroscope,
        Self::PaddleLeft,
        Self::PaddleRight,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dpad => "dpad",
            Self::FaceBottom => "face-bottom",
            Self::FaceRight => "face-right",
            Self::FaceLeft => "face-left",
            Self::FaceTop => "face-top",
            Self::LeftShoulder => "left-shoulder",
            Self::RightShoulder => "right-shoulder",
            Self::LeftTrigger => "left-trigger",
            Self::RightTrigger => "right-trigger",
            Self::LeftStick => "left-stick",
            Self::RightStick => "right-stick",
            Self::LeftStickButton => "left-stick-button",
            Self::RightStickButton => "right-stick-button",
            Self::MenuPrimary => "menu-primary",
            Self::MenuSecondary => "menu-secondary",
            Self::Guide => "guide",
            Self::TouchSurface => "touch-surface",
            Self::TouchClick => "touch-click",
            Self::Accelerometer => "accelerometer",
            Self::Gyroscope => "gyroscope",
            Self::PaddleLeft => "paddle-left",
            Self::PaddleRight => "paddle-right",
        }
    }
}

impl fmt::Display for SemanticInputFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticOutputFunction {
    Rumble,
    Haptics,
    Lighting,
    PlayerIndicators,
    TriggerEffect,
    Audio,
}

impl SemanticOutputFunction {
    pub const ALL: [Self; 6] = [
        Self::Rumble,
        Self::Haptics,
        Self::Lighting,
        Self::PlayerIndicators,
        Self::TriggerEffect,
        Self::Audio,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rumble => "rumble",
            Self::Haptics => "haptics",
            Self::Lighting => "lighting",
            Self::PlayerIndicators => "player-indicators",
            Self::TriggerEffect => "trigger-effect",
            Self::Audio => "audio",
        }
    }
}

impl fmt::Display for SemanticOutputFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityCategory {
    Button,
    Stick,
    Trigger,
    MotionSensor,
    TouchSurface,
    Haptic,
    Lighting,
    Audio,
    System,
}

impl CapabilityCategory {
    pub const ALL: [Self; 9] = [
        Self::Button,
        Self::Stick,
        Self::Trigger,
        Self::MotionSensor,
        Self::TouchSurface,
        Self::Haptic,
        Self::Lighting,
        Self::Audio,
        Self::System,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Stick => "stick",
            Self::Trigger => "trigger",
            Self::MotionSensor => "motion-sensor",
            Self::TouchSurface => "touch-surface",
            Self::Haptic => "haptic",
            Self::Lighting => "lighting",
            Self::Audio => "audio",
            Self::System => "system",
        }
    }
}

impl fmt::Display for CapabilityCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ButtonState {
    Released,
    Pressed,
}

impl ButtonState {
    #[must_use]
    pub const fn released() -> Self {
        Self::Released
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StickPosition {
    pub x: i16,
    pub y: i16,
}

impl StickPosition {
    #[must_use]
    pub const fn neutral() -> Self {
        Self { x: 0, y: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dpad {
    pub up: ButtonState,
    pub down: ButtonState,
    pub left: ButtonState,
    pub right: ButtonState,
}

impl Dpad {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            up: ButtonState::Released,
            down: ButtonState::Released,
            left: ButtonState::Released,
            right: ButtonState::Released,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenericGamepadInput {
    pub south: ButtonState,
    pub east: ButtonState,
    pub west: ButtonState,
    pub north: ButtonState,
    pub dpad: Dpad,
    pub left_shoulder: ButtonState,
    pub right_shoulder: ButtonState,
    pub left_stick_button: ButtonState,
    pub right_stick_button: ButtonState,
    pub menu_primary: ButtonState,
    pub menu_secondary: ButtonState,
    pub guide: ButtonState,
    pub left_stick: StickPosition,
    pub right_stick: StickPosition,
    pub left_trigger: u16,
    pub right_trigger: u16,
}

impl GenericGamepadInput {
    #[must_use]
    pub fn neutral() -> Self {
        Self {
            south: ButtonState::Released,
            east: ButtonState::Released,
            west: ButtonState::Released,
            north: ButtonState::Released,
            dpad: Dpad::neutral(),
            left_shoulder: ButtonState::Released,
            right_shoulder: ButtonState::Released,
            left_stick_button: ButtonState::Released,
            right_stick_button: ButtonState::Released,
            menu_primary: ButtonState::Released,
            menu_secondary: ButtonState::Released,
            guide: ButtonState::Released,
            left_stick: StickPosition::neutral(),
            right_stick: StickPosition::neutral(),
            left_trigger: 0,
            right_trigger: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Xbox360Input {
    pub a: ButtonState,
    pub b: ButtonState,
    pub x: ButtonState,
    pub y: ButtonState,
    pub dpad: Dpad,
    pub left_bumper: ButtonState,
    pub right_bumper: ButtonState,
    pub left_stick_button: ButtonState,
    pub right_stick_button: ButtonState,
    pub start: ButtonState,
    pub back: ButtonState,
    pub guide: ButtonState,
    pub left_stick: StickPosition,
    pub right_stick: StickPosition,
    pub left_trigger: u16,
    pub right_trigger: u16,
}

impl Xbox360Input {
    #[must_use]
    pub fn neutral() -> Self {
        Self {
            a: ButtonState::Released,
            b: ButtonState::Released,
            x: ButtonState::Released,
            y: ButtonState::Released,
            dpad: Dpad::neutral(),
            left_bumper: ButtonState::Released,
            right_bumper: ButtonState::Released,
            left_stick_button: ButtonState::Released,
            right_stick_button: ButtonState::Released,
            start: ButtonState::Released,
            back: ButtonState::Released,
            guide: ButtonState::Released,
            left_stick: StickPosition::neutral(),
            right_stick: StickPosition::neutral(),
            left_trigger: 0,
            right_trigger: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseInput {
    pub cross: ButtonState,
    pub circle: ButtonState,
    pub square: ButtonState,
    pub triangle: ButtonState,
    pub dpad: Dpad,
    pub l1: ButtonState,
    pub r1: ButtonState,
    pub l3: ButtonState,
    pub r3: ButtonState,
    pub create: ButtonState,
    pub options: ButtonState,
    pub ps: ButtonState,
    pub touchpad_click: ButtonState,
    pub left_stick: StickPosition,
    pub right_stick: StickPosition,
    pub l2: u16,
    pub r2: u16,
}

impl DualSenseInput {
    #[must_use]
    pub fn neutral() -> Self {
        Self {
            cross: ButtonState::Released,
            circle: ButtonState::Released,
            square: ButtonState::Released,
            triangle: ButtonState::Released,
            dpad: Dpad::neutral(),
            l1: ButtonState::Released,
            r1: ButtonState::Released,
            l3: ButtonState::Released,
            r3: ButtonState::Released,
            create: ButtonState::Released,
            options: ButtonState::Released,
            ps: ButtonState::Released,
            touchpad_click: ButtonState::Released,
            left_stick: StickPosition::neutral(),
            right_stick: StickPosition::neutral(),
            l2: 0,
            r2: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SteamControllerInput {
    pub a: ButtonState,
    pub b: ButtonState,
    pub x: ButtonState,
    pub y: ButtonState,
    pub left_grip: ButtonState,
    pub right_grip: ButtonState,
    pub left_bumper: ButtonState,
    pub right_bumper: ButtonState,
    pub menu_primary: ButtonState,
    pub menu_secondary: ButtonState,
    pub steam: ButtonState,
    pub left_pad_click: ButtonState,
    pub right_pad_click: ButtonState,
    pub left_stick_click: ButtonState,
    pub left_pad: StickPosition,
    pub right_pad: StickPosition,
    pub left_stick: StickPosition,
    pub left_trigger: u16,
    pub right_trigger: u16,
}

impl SteamControllerInput {
    #[must_use]
    pub fn neutral() -> Self {
        Self {
            a: ButtonState::Released,
            b: ButtonState::Released,
            x: ButtonState::Released,
            y: ButtonState::Released,
            left_grip: ButtonState::Released,
            right_grip: ButtonState::Released,
            left_bumper: ButtonState::Released,
            right_bumper: ButtonState::Released,
            menu_primary: ButtonState::Released,
            menu_secondary: ButtonState::Released,
            steam: ButtonState::Released,
            left_pad_click: ButtonState::Released,
            right_pad_click: ButtonState::Released,
            left_stick_click: ButtonState::Released,
            left_pad: StickPosition::neutral(),
            right_pad: StickPosition::neutral(),
            left_stick: StickPosition::neutral(),
            left_trigger: 0,
            right_trigger: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "profile", content = "fields")]
#[non_exhaustive]
pub enum ProfileInputPayload {
    GenericGamepad(GenericGamepadInput),
    Xbox360(Xbox360Input),
    #[serde(rename = "dualsense")]
    DualSense(DualSenseInput),
    SteamController(SteamControllerInput),
}

impl ProfileInputPayload {
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::GenericGamepad(_) => "generic-gamepad",
            Self::Xbox360(_) => "xbox360",
            Self::DualSense(_) => "dualsense",
            Self::SteamController(_) => "steam-controller",
        }
    }

    #[must_use]
    pub fn neutral_for_profile_id(profile_id: &ProfileId) -> Option<Self> {
        match profile_id.as_ref() {
            "generic-gamepad" => Some(Self::GenericGamepad(GenericGamepadInput::neutral())),
            "xbox360" => Some(Self::Xbox360(Xbox360Input::neutral())),
            "dualsense" => Some(Self::DualSense(DualSenseInput::neutral())),
            "steam-controller" => Some(Self::SteamController(SteamControllerInput::neutral())),
            _ => None,
        }
    }

    /// Validate that the given profile id matches this payload variant.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ProfilePayloadMismatch`] when the supplied
    /// `profile_id` does not correspond to the payload's built-in
    /// profile family.
    pub fn validate_profile_id(&self, profile_id: &ProfileId) -> Result<(), CoreError> {
        let expected = self.variant_name();
        if expected == profile_id.as_ref() {
            Ok(())
        } else {
            Err(CoreError::ProfilePayloadMismatch {
                profile_id: profile_id.clone(),
                payload_variant: expected,
            })
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DpadDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<ButtonState>,
}

impl DpadDelta {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            up: None,
            down: None,
            left: None,
            right: None,
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.up.is_none() && self.down.is_none() && self.left.is_none() && self.right.is_none()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct GenericGamepadDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub south: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub east: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub west: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub north: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpad: Option<DpadDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_shoulder: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_shoulder: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick_button: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_stick_button: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_primary: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_secondary: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guide: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_stick: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_trigger: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_trigger: Option<u16>,
}

impl GenericGamepadDelta {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpad: Option<DpadDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_bumper: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_bumper: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick_button: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_stick_button: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub back: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guide: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_stick: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_trigger: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_trigger: Option<u16>,
}

impl Xbox360Delta {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circle: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub square: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triangle: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpad: Option<DpadDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r1: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l3: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r3: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ps: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touchpad_click: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_stick: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l2: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r2: Option<u16>,
}

impl DualSenseDelta {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SteamControllerDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_grip: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_grip: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_bumper: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_bumper: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_primary: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_secondary: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_pad_click: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_pad_click: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick_click: Option<ButtonState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_pad: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_pad: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick: Option<StickPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_trigger: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_trigger: Option<u16>,
}

impl SteamControllerDelta {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "profile", content = "fields")]
#[non_exhaustive]
pub enum ProfileInputDeltaPayload {
    GenericGamepad(GenericGamepadDelta),
    Xbox360(Xbox360Delta),
    #[serde(rename = "dualsense")]
    DualSense(DualSenseDelta),
    SteamController(SteamControllerDelta),
}

impl ProfileInputDeltaPayload {
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::GenericGamepad(_) => "generic-gamepad",
            Self::Xbox360(_) => "xbox360",
            Self::DualSense(_) => "dualsense",
            Self::SteamController(_) => "steam-controller",
        }
    }

    /// Validate that the given profile id matches this delta variant.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ProfilePayloadMismatch`] when the supplied
    /// `profile_id` does not correspond to the delta's built-in
    /// profile family.
    pub fn validate_profile_id(&self, profile_id: &ProfileId) -> Result<(), CoreError> {
        let expected = self.variant_name();
        if expected == profile_id.as_ref() {
            Ok(())
        } else {
            Err(CoreError::ProfilePayloadMismatch {
                profile_id: profile_id.clone(),
                payload_variant: expected,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileInputFrame {
    pub profile_id: ProfileId,
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    pub payload: ProfileInputPayload,
}

impl ProfileInputFrame {
    /// Validate the frame's profile id against its payload variant.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ProfilePayloadMismatch`] when the payload
    /// variant and `profile_id` disagree.
    pub fn validate(&self) -> Result<(), CoreError> {
        self.payload.validate_profile_id(&self.profile_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileInputDelta {
    pub profile_id: ProfileId,
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    pub payload: ProfileInputDeltaPayload,
}

impl ProfileInputDelta {
    /// Validate the delta's profile id against its payload variant.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::ProfilePayloadMismatch`] when the payload
    /// variant and `profile_id` disagree.
    pub fn validate(&self) -> Result<(), CoreError> {
        self.payload.validate_profile_id(&self.profile_id)
    }
}

#[must_use]
pub fn render_type_catalog() -> String {
    fn lines<T: fmt::Display + Copy>(values: &[T]) -> String {
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    format!(
        concat!(
            "fidelity-tiers\n",
            "{}\n\n",
            "backend-levels\n",
            "{}\n\n",
            "backend-families\n",
            "{}\n\n",
            "capability-categories\n",
            "{}\n"
        ),
        lines(&FidelityTier::ALL),
        lines(&BackendLevel::ALL),
        lines(&BackendFamily::ALL),
        lines(&CapabilityCategory::ALL),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use proptest::prelude::*;

    fn arb_profile_id() -> impl Strategy<Value = ProfileId> {
        prop_oneof![
            Just(ProfileId::from("generic-gamepad")),
            Just(ProfileId::from("xbox360")),
            Just(ProfileId::from("dualsense")),
            Just(ProfileId::from("steam-controller")),
            "[a-z0-9-]{1,16}".prop_map(ProfileId::from),
        ]
    }

    fn arb_backend_id() -> impl Strategy<Value = BackendId> {
        "[a-z0-9-]{1,24}".prop_map(BackendId::from)
    }

    fn arb_fidelity_tier() -> impl Strategy<Value = FidelityTier> {
        prop_oneof![
            Just(FidelityTier::Compatibility),
            Just(FidelityTier::IdentityAware),
            Just(FidelityTier::HardwareFaithful),
        ]
    }

    fn arb_backend_level() -> impl Strategy<Value = BackendLevel> {
        prop_oneof![
            Just(BackendLevel::Evdev),
            Just(BackendLevel::Hid),
            Just(BackendLevel::Transport),
        ]
    }

    fn arb_backend_family() -> impl Strategy<Value = BackendFamily> {
        prop_oneof![
            Just(BackendFamily::LinuxUinput),
            Just(BackendFamily::LinuxUhid),
            Just(BackendFamily::LinuxTransportUsb),
            Just(BackendFamily::LinuxTransportBluetooth),
            Just(BackendFamily::WindowsHid),
            Just(BackendFamily::MacosHid),
        ]
    }

    fn arb_button_state() -> impl Strategy<Value = ButtonState> {
        prop_oneof![Just(ButtonState::Released), Just(ButtonState::Pressed),]
    }

    fn arb_dpad() -> impl Strategy<Value = Dpad> {
        (
            arb_button_state(),
            arb_button_state(),
            arb_button_state(),
            arb_button_state(),
        )
            .prop_map(|(up, down, left, right)| Dpad {
                up,
                down,
                left,
                right,
            })
    }

    fn arb_capability_category() -> impl Strategy<Value = CapabilityCategory> {
        prop_oneof![
            Just(CapabilityCategory::Button),
            Just(CapabilityCategory::Stick),
            Just(CapabilityCategory::Trigger),
            Just(CapabilityCategory::MotionSensor),
            Just(CapabilityCategory::TouchSurface),
            Just(CapabilityCategory::Haptic),
            Just(CapabilityCategory::Lighting),
            Just(CapabilityCategory::Audio),
            Just(CapabilityCategory::System),
        ]
    }

    #[test]
    fn smoke() {}

    #[test]
    fn fidelity_tier_human_names_parse_and_display() {
        for tier in FidelityTier::ALL {
            let parsed = FidelityTier::from_str(tier.as_str()).expect("parse fidelity tier");
            assert_eq!(parsed, tier);
            assert_eq!(parsed.to_string(), tier.as_str());
        }
    }

    #[test]
    fn render_type_catalog_uses_spec_names() {
        let output = render_type_catalog();
        assert!(output.contains("compatibility"));
        assert!(output.contains("identity-aware"));
        assert!(output.contains("hardware-faithful"));
        assert!(output.contains("linux-transport-bluetooth"));
        assert!(output.contains("motion-sensor"));
    }

    #[test]
    fn payload_variant_must_match_profile_id() {
        let frame = ProfileInputFrame {
            profile_id: ProfileId::from("xbox360"),
            timestamp: Timestamp::new(0),
            sequence: SequenceId::new(0),
            payload: ProfileInputPayload::DualSense(DualSenseInput::neutral()),
        };

        let error = frame.validate().expect_err("mismatch should fail");
        assert_eq!(
            error.to_string(),
            "profile id `xbox360` does not match payload variant `dualsense`"
        );
    }

    #[test]
    fn canonical_yaml_snapshots_are_human_readable() {
        assert_snapshot!(
            "fidelity-tier",
            serde_yaml::to_string(&FidelityTier::Compatibility).expect("yaml")
        );
        assert_snapshot!(
            "backend-level",
            serde_yaml::to_string(&BackendLevel::Evdev).expect("yaml")
        );
        assert_snapshot!(
            "backend-family",
            serde_yaml::to_string(&BackendFamily::LinuxTransportBluetooth).expect("yaml")
        );
        assert_snapshot!(
            "capability-category",
            serde_yaml::to_string(&CapabilityCategory::MotionSensor).expect("yaml")
        );
        assert_snapshot!(
            "dualsense-neutral-payload",
            serde_yaml::to_string(&ProfileInputPayload::DualSense(DualSenseInput::neutral()))
                .expect("yaml")
        );
        assert_snapshot!(
            "dualsense-neutral-frame",
            serde_yaml::to_string(&ProfileInputFrame {
                profile_id: ProfileId::from("dualsense"),
                timestamp: Timestamp::new(0),
                sequence: SequenceId::new(0),
                payload: ProfileInputPayload::DualSense(DualSenseInput::neutral()),
            })
            .expect("yaml")
        );
    }

    #[test]
    fn profile_input_frame_yaml_round_trip() {
        let frame = ProfileInputFrame {
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(42),
            sequence: SequenceId::new(7),
            payload: ProfileInputPayload::DualSense(DualSenseInput::neutral()),
        };
        let yaml = serde_yaml::to_string(&frame).expect("serialize frame");
        let decoded: ProfileInputFrame = serde_yaml::from_str(&yaml).expect("decode frame");
        assert_eq!(decoded, frame);
    }

    #[test]
    fn profile_input_frame_json_round_trip() {
        let frame = ProfileInputFrame {
            profile_id: ProfileId::from("xbox360"),
            timestamp: Timestamp::new(3),
            sequence: SequenceId::new(9),
            payload: ProfileInputPayload::Xbox360(Xbox360Input::neutral()),
        };
        let json = serde_json::to_string(&frame).expect("serialize frame");
        let decoded: ProfileInputFrame = serde_json::from_str(&json).expect("decode frame");
        assert_eq!(decoded, frame);
    }

    #[test]
    fn empty_dualsense_delta_yaml_round_trip() {
        let delta = ProfileInputDelta {
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(0),
            sequence: SequenceId::new(0),
            payload: ProfileInputDeltaPayload::DualSense(DualSenseDelta::empty()),
        };
        let yaml = serde_yaml::to_string(&delta).expect("serialize delta");
        let decoded: ProfileInputDelta = serde_yaml::from_str(&yaml).expect("decode delta");
        assert_eq!(decoded, delta);
        delta.validate().expect("delta validates");
    }

    #[test]
    fn sparse_dualsense_delta_only_carries_set_fields() {
        let mut payload = DualSenseDelta::empty();
        payload.dpad = Some(DpadDelta {
            up: None,
            down: None,
            left: Some(ButtonState::Pressed),
            right: None,
        });
        payload.l2 = Some(0x42);
        let delta = ProfileInputDelta {
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(7),
            sequence: SequenceId::new(11),
            payload: ProfileInputDeltaPayload::DualSense(payload),
        };
        let yaml = serde_yaml::to_string(&delta).expect("serialize sparse delta");
        // Sparse: should only mention the fields we actually set.
        assert!(yaml.contains("l2: 66"));
        assert!(yaml.contains("left: pressed"));
        assert!(!yaml.contains("r2:"));
        assert!(!yaml.contains("cross:"));
        let decoded: ProfileInputDelta = serde_yaml::from_str(&yaml).expect("decode sparse delta");
        let ProfileInputDeltaPayload::DualSense(decoded_payload) = decoded.payload else {
            panic!("expected dualsense delta");
        };
        assert!(decoded_payload.r2.is_none());
        assert!(decoded_payload.cross.is_none());
        assert_eq!(decoded_payload.l2, Some(0x42));
        let dpad = decoded_payload.dpad.expect("dpad change is present");
        assert_eq!(dpad.left, Some(ButtonState::Pressed));
        assert!(dpad.up.is_none());
    }

    #[test]
    fn delta_payload_variant_must_match_profile_id() {
        let delta = ProfileInputDelta {
            profile_id: ProfileId::from("xbox360"),
            timestamp: Timestamp::new(0),
            sequence: SequenceId::new(0),
            payload: ProfileInputDeltaPayload::DualSense(DualSenseDelta::empty()),
        };
        let error = delta.validate().expect_err("mismatch should fail");
        assert_eq!(
            error.to_string(),
            "profile id `xbox360` does not match payload variant `dualsense`"
        );
    }

    proptest! {
        #[test]
        fn profile_id_yaml_round_trip(value in arb_profile_id()) {
            let yaml = serde_yaml::to_string(&value)?;
            let decoded: ProfileId = serde_yaml::from_str(&yaml)?;
            prop_assert_eq!(decoded, value);
        }

        #[test]
        fn backend_id_json_round_trip(value in arb_backend_id()) {
            let json = serde_json::to_string(&value)?;
            let decoded: BackendId = serde_json::from_str(&json)?;
            prop_assert_eq!(decoded, value);
        }

        #[test]
        fn fidelity_tier_yaml_round_trip(value in arb_fidelity_tier()) {
            let yaml = serde_yaml::to_string(&value).expect("serialize");
            let decoded: FidelityTier = serde_yaml::from_str(&yaml).expect("deserialize");
            prop_assert_eq!(decoded, value);
            let reparsed = FidelityTier::from_str(value.as_str()).expect("parse");
            prop_assert_eq!(reparsed, value);
        }

        #[test]
        fn backend_level_json_round_trip(value in arb_backend_level()) {
            let json = serde_json::to_string(&value)?;
            let decoded: BackendLevel = serde_json::from_str(&json)?;
            prop_assert_eq!(decoded, value);
        }

        #[test]
        fn backend_family_yaml_round_trip(value in arb_backend_family()) {
            let yaml = serde_yaml::to_string(&value)?;
            let decoded: BackendFamily = serde_yaml::from_str(&yaml)?;
            prop_assert_eq!(decoded, value);
        }

        #[test]
        fn capability_category_json_round_trip(value in arb_capability_category()) {
            let json = serde_json::to_string(&value)?;
            let decoded: CapabilityCategory = serde_json::from_str(&json)?;
            prop_assert_eq!(decoded, value);
        }

        #[test]
        fn dpad_yaml_round_trip(value in arb_dpad()) {
            let yaml = serde_yaml::to_string(&value)?;
            let decoded: Dpad = serde_yaml::from_str(&yaml)?;
            prop_assert_eq!(decoded, value);
        }

        #[test]
        fn dpad_json_round_trip(value in arb_dpad()) {
            let json = serde_json::to_string(&value)?;
            let decoded: Dpad = serde_json::from_str(&json)?;
            prop_assert_eq!(decoded, value);
        }
    }
}
