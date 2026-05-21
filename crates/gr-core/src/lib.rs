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
#[allow(clippy::struct_excessive_bools)]
pub struct Dpad {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl Dpad {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DpadDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right: Option<bool>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TwinStickAxes {
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
}

impl TwinStickAxes {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            left_x: 0,
            left_y: 0,
            right_x: 0,
            right_y: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct TwinStickAxesDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_x: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_y: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_x: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_y: Option<i16>,
}

impl TwinStickAxesDelta {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            left_x: None,
            left_y: None,
            right_x: None,
            right_y: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct GenericGamepadButtons {
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub left_shoulder: bool,
    pub right_shoulder: bool,
    pub left_stick_button: bool,
    pub right_stick_button: bool,
    pub menu_primary: bool,
    pub menu_secondary: bool,
    pub guide: bool,
}

impl GenericGamepadButtons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            south: false,
            east: false,
            west: false,
            north: false,
            left_shoulder: false,
            right_shoulder: false,
            left_stick_button: false,
            right_stick_button: false,
            menu_primary: false,
            menu_secondary: false,
            guide: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct GenericGamepadButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub south: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub east: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub west: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub north: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_shoulder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_shoulder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick_button: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_stick_button: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_secondary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guide: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenericGamepadTriggers {
    pub left_trigger: u16,
    pub right_trigger: u16,
}

impl GenericGamepadTriggers {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            left_trigger: 0,
            right_trigger: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct GenericGamepadTriggersDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_trigger: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_trigger: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenericGamepadInput {
    pub buttons: GenericGamepadButtons,
    pub dpad: Dpad,
    pub sticks: TwinStickAxes,
    pub triggers: GenericGamepadTriggers,
}

impl GenericGamepadInput {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            buttons: GenericGamepadButtons::neutral(),
            dpad: Dpad::neutral(),
            sticks: TwinStickAxes::neutral(),
            triggers: GenericGamepadTriggers::neutral(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct GenericGamepadDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<GenericGamepadButtonsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpad: Option<DpadDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticks: Option<TwinStickAxesDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<GenericGamepadTriggersDelta>,
}

impl GenericGamepadDelta {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Xbox360FaceButtons {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
}

impl Xbox360FaceButtons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            a: false,
            b: false,
            x: false,
            y: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360FaceButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Xbox360Shoulders {
    pub lb: bool,
    pub rb: bool,
}

impl Xbox360Shoulders {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            lb: false,
            rb: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360ShouldersDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lb: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Xbox360StickClicks {
    pub ls: bool,
    pub rs: bool,
}

impl Xbox360StickClicks {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            ls: false,
            rs: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360StickClicksDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rs: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Xbox360SystemButtons {
    pub start: bool,
    pub back: bool,
    pub guide: bool,
}

impl Xbox360SystemButtons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            start: false,
            back: false,
            guide: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360SystemButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub back: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guide: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Xbox360Buttons {
    pub face: Xbox360FaceButtons,
    pub shoulders: Xbox360Shoulders,
    pub stick_clicks: Xbox360StickClicks,
    pub system: Xbox360SystemButtons,
}

impl Xbox360Buttons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            face: Xbox360FaceButtons::neutral(),
            shoulders: Xbox360Shoulders::neutral(),
            stick_clicks: Xbox360StickClicks::neutral(),
            system: Xbox360SystemButtons::neutral(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360ButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face: Option<Xbox360FaceButtonsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shoulders: Option<Xbox360ShouldersDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stick_clicks: Option<Xbox360StickClicksDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Xbox360SystemButtonsDelta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Xbox360Triggers {
    pub lt: u16,
    pub rt: u16,
}

impl Xbox360Triggers {
    #[must_use]
    pub const fn neutral() -> Self {
        Self { lt: 0, rt: 0 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360TriggersDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rt: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Xbox360Input {
    pub buttons: Xbox360Buttons,
    pub dpad: Dpad,
    pub sticks: TwinStickAxes,
    pub triggers: Xbox360Triggers,
}

impl Xbox360Input {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            buttons: Xbox360Buttons::neutral(),
            dpad: Dpad::neutral(),
            sticks: TwinStickAxes::neutral(),
            triggers: Xbox360Triggers::neutral(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct Xbox360Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Xbox360ButtonsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpad: Option<DpadDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticks: Option<TwinStickAxesDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<Xbox360TriggersDelta>,
}

impl Xbox360Delta {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct DualSenseFaceButtons {
    pub cross: bool,
    pub circle: bool,
    pub square: bool,
    pub triangle: bool,
}

impl DualSenseFaceButtons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            cross: false,
            circle: false,
            square: false,
            triangle: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseFaceButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cross: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circle: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub square: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triangle: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseShoulders {
    pub l1: bool,
    pub r1: bool,
}

impl DualSenseShoulders {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            l1: false,
            r1: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseShouldersDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r1: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseStickClicks {
    pub l3: bool,
    pub r3: bool,
}

impl DualSenseStickClicks {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            l3: false,
            r3: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseStickClicksDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l3: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r3: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct DualSenseSystemButtons {
    pub create: bool,
    pub options: bool,
    pub ps: bool,
    pub touchpad_click: bool,
}

impl DualSenseSystemButtons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            create: false,
            options: false,
            ps: false,
            touchpad_click: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseSystemButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touchpad_click: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseButtons {
    pub face: DualSenseFaceButtons,
    pub shoulders: DualSenseShoulders,
    pub stick_clicks: DualSenseStickClicks,
    pub system: DualSenseSystemButtons,
}

impl DualSenseButtons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            face: DualSenseFaceButtons::neutral(),
            shoulders: DualSenseShoulders::neutral(),
            stick_clicks: DualSenseStickClicks::neutral(),
            system: DualSenseSystemButtons::neutral(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face: Option<DualSenseFaceButtonsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shoulders: Option<DualSenseShouldersDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stick_clicks: Option<DualSenseStickClicksDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<DualSenseSystemButtonsDelta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseTriggers {
    pub l2: u16,
    pub r2: u16,
}

impl DualSenseTriggers {
    #[must_use]
    pub const fn neutral() -> Self {
        Self { l2: 0, r2: 0 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseTriggersDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l2: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r2: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseTouchContact {
    pub active: bool,
    pub x: u16,
    pub y: u16,
}

impl DualSenseTouchContact {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            active: false,
            x: 0,
            y: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseTouchContactDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseTouchpad {
    pub contact_1: DualSenseTouchContact,
    pub contact_2: DualSenseTouchContact,
}

impl DualSenseTouchpad {
    pub const WIDTH: u16 = 1920;
    pub const HEIGHT: u16 = 1080;

    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            contact_1: DualSenseTouchContact::neutral(),
            contact_2: DualSenseTouchContact::neutral(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseTouchpadDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_1: Option<DualSenseTouchContactDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_2: Option<DualSenseTouchContactDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DualSenseInput {
    pub buttons: DualSenseButtons,
    pub dpad: Dpad,
    pub sticks: TwinStickAxes,
    pub triggers: DualSenseTriggers,
    pub touchpad: DualSenseTouchpad,
}

impl DualSenseInput {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            buttons: DualSenseButtons::neutral(),
            dpad: Dpad::neutral(),
            sticks: TwinStickAxes::neutral(),
            triggers: DualSenseTriggers::neutral(),
            touchpad: DualSenseTouchpad::neutral(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct DualSenseDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<DualSenseButtonsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpad: Option<DpadDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticks: Option<TwinStickAxesDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<DualSenseTriggersDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touchpad: Option<DualSenseTouchpadDelta>,
}

impl DualSenseDelta {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SteamControllerButtons {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub left_grip: bool,
    pub right_grip: bool,
    pub lb: bool,
    pub rb: bool,
    pub menu_primary: bool,
    pub menu_secondary: bool,
    pub steam: bool,
    pub left_pad_click: bool,
    pub right_pad_click: bool,
    pub left_stick_click: bool,
}

impl SteamControllerButtons {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            a: false,
            b: false,
            x: false,
            y: false,
            left_grip: false,
            right_grip: false,
            lb: false,
            rb: false,
            menu_primary: false,
            menu_secondary: false,
            steam: false,
            left_pad_click: false,
            right_pad_click: false,
            left_stick_click: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SteamControllerButtonsDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_grip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_grip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lb: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rb: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_primary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_secondary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steam: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_pad_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_pad_click: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick_click: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SteamControllerSticks {
    pub left_pad_x: i16,
    pub left_pad_y: i16,
    pub right_pad_x: i16,
    pub right_pad_y: i16,
    pub left_stick_x: i16,
    pub left_stick_y: i16,
}

impl SteamControllerSticks {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            left_pad_x: 0,
            left_pad_y: 0,
            right_pad_x: 0,
            right_pad_y: 0,
            left_stick_x: 0,
            left_stick_y: 0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SteamControllerSticksDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_pad_x: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_pad_y: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_pad_x: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_pad_y: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick_x: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_stick_y: Option<i16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SteamControllerTriggers {
    pub lt: u16,
    pub rt: u16,
}

impl SteamControllerTriggers {
    #[must_use]
    pub const fn neutral() -> Self {
        Self { lt: 0, rt: 0 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SteamControllerTriggersDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lt: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rt: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SteamControllerInput {
    pub buttons: SteamControllerButtons,
    pub sticks: SteamControllerSticks,
    pub triggers: SteamControllerTriggers,
}

impl SteamControllerInput {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            buttons: SteamControllerButtons::neutral(),
            sticks: SteamControllerSticks::neutral(),
            triggers: SteamControllerTriggers::neutral(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct SteamControllerDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<SteamControllerButtonsDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticks: Option<SteamControllerSticksDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub triggers: Option<SteamControllerTriggersDelta>,
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

    fn arb_dpad() -> impl Strategy<Value = Dpad> {
        (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
            |(up, down, left, right)| Dpad {
                up,
                down,
                left,
                right,
            },
        )
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
        for tier in FidelityTier::ALL {
            assert_snapshot!(
                format!("fidelity-tier-{}", tier.as_str()),
                serde_yaml::to_string(&tier).expect("yaml")
            );
        }
        for level in BackendLevel::ALL {
            assert_snapshot!(
                format!("backend-level-{}", level.as_str()),
                serde_yaml::to_string(&level).expect("yaml")
            );
        }
        for family in BackendFamily::ALL {
            assert_snapshot!(
                format!("backend-family-{}", family.as_str()),
                serde_yaml::to_string(&family).expect("yaml")
            );
        }
        for category in CapabilityCategory::ALL {
            assert_snapshot!(
                format!("capability-category-{}", category.as_str()),
                serde_yaml::to_string(&category).expect("yaml")
            );
        }
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
    fn dualsense_touchpad_round_trip() {
        let frame = ProfileInputFrame {
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(1),
            sequence: SequenceId::new(2),
            payload: ProfileInputPayload::DualSense(DualSenseInput {
                touchpad: DualSenseTouchpad {
                    contact_1: DualSenseTouchContact {
                        active: true,
                        x: 830,
                        y: 412,
                    },
                    contact_2: DualSenseTouchContact::neutral(),
                },
                ..DualSenseInput::neutral()
            }),
        };
        let yaml = serde_yaml::to_string(&frame).expect("serialize frame");
        assert!(yaml.contains("touchpad:"));
        assert!(yaml.contains("active: true"));
        let decoded: ProfileInputFrame = serde_yaml::from_str(&yaml).expect("decode frame");
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
            left: Some(true),
            right: None,
        });
        payload.triggers = Some(DualSenseTriggersDelta {
            l2: Some(0x42),
            r2: None,
        });
        payload.touchpad = Some(DualSenseTouchpadDelta {
            contact_1: Some(DualSenseTouchContactDelta {
                active: Some(true),
                x: Some(830),
                y: Some(412),
            }),
            contact_2: None,
        });
        let delta = ProfileInputDelta {
            profile_id: ProfileId::from("dualsense"),
            timestamp: Timestamp::new(7),
            sequence: SequenceId::new(11),
            payload: ProfileInputDeltaPayload::DualSense(payload),
        };
        let yaml = serde_yaml::to_string(&delta).expect("serialize sparse delta");
        assert!(yaml.contains("l2: 66"));
        assert!(yaml.contains("left: true"));
        assert!(yaml.contains("x: 830"));
        assert!(!yaml.contains("r2:"));
        assert!(!yaml.contains("cross:"));
        let decoded: ProfileInputDelta = serde_yaml::from_str(&yaml).expect("decode sparse delta");
        let ProfileInputDeltaPayload::DualSense(decoded_payload) = decoded.payload else {
            panic!("expected dualsense delta");
        };
        assert!(decoded_payload.buttons.is_none());
        let triggers = decoded_payload.triggers.expect("trigger change is present");
        assert_eq!(triggers.l2, Some(0x42));
        assert!(triggers.r2.is_none());
        let dpad = decoded_payload.dpad.expect("dpad change is present");
        assert_eq!(dpad.left, Some(true));
        assert!(dpad.up.is_none());
        let touchpad = decoded_payload
            .touchpad
            .expect("touchpad change is present");
        let contact_1 = touchpad.contact_1.expect("first contact changed");
        assert_eq!(contact_1.active, Some(true));
        assert_eq!(contact_1.x, Some(830));
        assert_eq!(contact_1.y, Some(412));
        assert!(touchpad.contact_2.is_none());
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
