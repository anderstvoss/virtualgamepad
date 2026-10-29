//! Private S3 groundwork. No registry entry, descriptor, provider or public API
//! is advertised until the descriptor and API gates on #159 are cleared.
//! All served behaviour is compat, grounded in consumer-source records rather
//! than physical readback. See `docs/STEAM_CONTROLLER_2026.md`.

mod codec;

// steam-c6-input-buttons: native button bit order, excluding capacitive bits.
#[derive(Debug, Clone, Copy)]
enum Button {
    A,
    B,
    X,
    Y,
    Qam,
    RightStick,
    Menu,
    R4,
    R5,
    RightShoulder,
    Down,
    Right,
    Left,
    Up,
    View,
    LeftStick,
    Steam,
    L4,
    L5,
    LeftShoulder,
    RightTrigger,
    LeftTrigger,
}
impl Button {
    const fn bit(self) -> u32 {
        match self {
            Self::A => 0,
            Self::B => 1,
            Self::X => 2,
            Self::Y => 3,
            Self::Qam => 4,
            Self::RightStick => 5,
            Self::Menu => 6,
            Self::R4 => 7,
            Self::R5 => 8,
            Self::RightShoulder => 9,
            Self::Down => 10,
            Self::Right => 11,
            Self::Left => 12,
            Self::Up => 13,
            Self::View => 14,
            Self::LeftStick => 15,
            Self::Steam => 16,
            Self::L4 => 17,
            Self::L5 => 18,
            Self::LeftShoulder => 19,
            Self::RightTrigger => 23,
            Self::LeftTrigger => 27,
        }
    }
}

// steam-c6-input-scaling: keep raw native s16 coordinates/u16 pressure, with no
// SDL transforms, invented calibration or clipping to consumer normalized units.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Pad {
    position: [i16; 2],
    pressure: u16,
    touch: bool,
    click: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Motion {
    timestamp_us: u32,
    accelerometer: [i16; 3],
    gyroscope: [i16; 3],
    quaternion: [i16; 4],
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct State {
    buttons: u32,
    sticks: [[i16; 2]; 2],
    triggers: [i16; 2],
    pads: [Pad; 2],
    stick_touch: [bool; 2],
    grip_touch: [bool; 2],
    motion: Motion,
}
impl State {
    fn set_button(&mut self, button: Button, pressed: bool) {
        let bit = 1 << button.bit();
        self.buttons = (self.buttons & !bit) | if pressed { bit } else { 0 };
    }
    fn button_bits(&self) -> u32 {
        // steam-c6-input-buttons: touch/click bits are native state, independent
        // of position/pressure. Reserved bits remain zero.
        let mut bits = self.buttons & 0x088f_ffff;
        for (pressed, bit) in [
            (self.stick_touch[1], 20),
            (self.pads[1].touch, 21),
            (self.pads[1].click, 22),
            (self.stick_touch[0], 24),
            (self.pads[0].touch, 25),
            (self.pads[0].click, 26),
            (self.grip_touch[1], 28),
            (self.grip_touch[0], 29),
        ] {
            bits |= u32::from(pressed) << bit;
        }
        bits
    }
}

// steam-c6-battery-43: optional caller-supplied raw status; no manufactured
// voltage, charge state or report cadence is emitted at creation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Battery {
    state: u8,
    level: u8,
    voltage: u16,
    system_voltage: u16,
    input_voltage: u16,
    current: u16,
    input_current: u16,
    temperature: u16,
}
