//! Typed state owned by the compiled curated-controller package.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct GenericGamepadTriggers {
    pub left_trigger: u16,
    pub right_trigger: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
            buttons: GenericGamepadButtons {
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
            },
            dpad: Dpad::neutral(),
            sticks: TwinStickAxes::neutral(),
            triggers: GenericGamepadTriggers {
                left_trigger: 0,
                right_trigger: 0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Xbox360FaceButtons {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Xbox360Shoulders {
    pub lb: bool,
    pub rb: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Xbox360StickClicks {
    pub ls: bool,
    pub rs: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Xbox360SystemButtons {
    pub start: bool,
    pub back: bool,
    pub guide: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Xbox360Buttons {
    pub face: Xbox360FaceButtons,
    pub shoulders: Xbox360Shoulders,
    pub stick_clicks: Xbox360StickClicks,
    pub system: Xbox360SystemButtons,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Xbox360Triggers {
    pub lt: u16,
    pub rt: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
            buttons: Xbox360Buttons {
                face: Xbox360FaceButtons {
                    a: false,
                    b: false,
                    x: false,
                    y: false,
                },
                shoulders: Xbox360Shoulders {
                    lb: false,
                    rb: false,
                },
                stick_clicks: Xbox360StickClicks {
                    ls: false,
                    rs: false,
                },
                system: Xbox360SystemButtons {
                    start: false,
                    back: false,
                    guide: false,
                },
            },
            dpad: Dpad::neutral(),
            sticks: TwinStickAxes::neutral(),
            triggers: Xbox360Triggers { lt: 0, rt: 0 },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct DualSenseFaceButtons {
    pub cross: bool,
    pub circle: bool,
    pub square: bool,
    pub triangle: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DualSenseShoulders {
    pub l1: bool,
    pub r1: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DualSenseStickClicks {
    pub l3: bool,
    pub r3: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct DualSenseSystemButtons {
    pub create: bool,
    pub options: bool,
    pub ps: bool,
    pub touchpad_click: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DualSenseButtons {
    pub face: DualSenseFaceButtons,
    pub shoulders: DualSenseShoulders,
    pub stick_clicks: DualSenseStickClicks,
    pub system: DualSenseSystemButtons,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DualSenseTriggers {
    pub l2: u16,
    pub r2: u16,
}

/// One raw signed three-axis sensor sample in controller report units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct MotionAxes {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

impl MotionAxes {
    #[must_use]
    pub const fn neutral() -> Self {
        Self { x: 0, y: 0, z: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DualSenseMotion {
    pub gyroscope: MotionAxes,
    pub accelerometer: MotionAxes,
}

impl DualSenseMotion {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            gyroscope: MotionAxes::neutral(),
            accelerometer: MotionAxes::neutral(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DualSenseInput {
    pub buttons: DualSenseButtons,
    pub dpad: Dpad,
    pub sticks: TwinStickAxes,
    pub triggers: DualSenseTriggers,
    pub touchpad: DualSenseTouchpad,
    pub motion: DualSenseMotion,
}

impl DualSenseInput {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            buttons: DualSenseButtons {
                face: DualSenseFaceButtons {
                    cross: false,
                    circle: false,
                    square: false,
                    triangle: false,
                },
                shoulders: DualSenseShoulders {
                    l1: false,
                    r1: false,
                },
                stick_clicks: DualSenseStickClicks {
                    l3: false,
                    r3: false,
                },
                system: DualSenseSystemButtons {
                    create: false,
                    options: false,
                    ps: false,
                    touchpad_click: false,
                },
            },
            dpad: Dpad::neutral(),
            sticks: TwinStickAxes::neutral(),
            triggers: DualSenseTriggers { l2: 0, r2: 0 },
            touchpad: DualSenseTouchpad::neutral(),
            motion: DualSenseMotion::neutral(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct SteamControllerSticks {
    pub left_pad_x: i16,
    pub left_pad_y: i16,
    pub right_pad_x: i16,
    pub right_pad_y: i16,
    pub left_stick_x: i16,
    pub left_stick_y: i16,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct SteamControllerTriggers {
    pub lt: u16,
    pub rt: u16,
}
pub type SteamControllerMotion = DualSenseMotion;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct SteamControllerInput {
    pub buttons: SteamControllerButtons,
    pub sticks: SteamControllerSticks,
    pub triggers: SteamControllerTriggers,
    pub motion: SteamControllerMotion,
}

impl SteamControllerInput {
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            buttons: SteamControllerButtons {
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
            },
            sticks: SteamControllerSticks {
                left_pad_x: 0,
                left_pad_y: 0,
                right_pad_x: 0,
                right_pad_y: 0,
                left_stick_x: 0,
                left_stick_y: 0,
            },
            triggers: SteamControllerTriggers { lt: 0, rt: 0 },
            motion: DualSenseMotion::neutral(),
        }
    }
}
