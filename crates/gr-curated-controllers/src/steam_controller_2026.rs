//! Steam Controller 2026 native supporting state (S3 steps 14a/14b).
//!
//! Staged registry metadata describes planned targets, blocked at 14c/step 16. There is
//! no provider or controller creation path. Descriptor bytes/shapes come from
//! fresh physical capture; field semantics remain compat consumer interpretations.
//! See `docs/STEAM_CONTROLLER_2026.md`.
//!
//! ```
//! use gr_curated_controllers::steam_controller_2026::{
//!     SteamController2026Button, SteamController2026State,
//!     SteamMotionSample, SteamTrackpadState,
//! };
//! let mut state = SteamController2026State::default();
//! state.set_button(SteamController2026Button::Menu, true);
//! state.pads[0] = SteamTrackpadState {
//!     position: [-100, 200], pressure: 123, touch: true, click: false,
//! };
//! state.motion = SteamMotionSample { gyroscope: [10, -20, 30], ..Default::default() };
//! assert!(state.pressed(SteamController2026Button::Menu));
//! state.neutralize();
//! assert_eq!(state, SteamController2026State::default());
//! ```

// Protocol/provider integration remains gated at 14c/step 16.
#[allow(dead_code)]
mod codec;
mod definition;
#[allow(dead_code)] // Exact compiled capture is reserved for the gated 14c provider.
mod descriptor;
mod topology;
pub(crate) use definition::SteamController2026Definition;

/// Staged family metadata; not advertised by the active registry. No host profile.
pub use gr_controller_wire::registry::STEAM_CONTROLLER_2026;
/// Stable compiled family ID, without a constructor or realization fallback.
pub use gr_controller_wire::registry::ids::STEAM_CONTROLLER_2026 as STEAM_CONTROLLER_2026_ID;

/// Native digital controls; capacitive touch and pad clicks are separate state.
/// Record: `steam-c6-input-buttons` (consumer order, not the historical encoder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SteamController2026Button {
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
impl SteamController2026Button {
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
/// One trackpad in native units. Record: `steam-c6-input-scaling`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SteamTrackpadState {
    /// Native signed X/Y, before consumer scaling or Y inversion.
    pub position: [i16; 2],
    /// Native unsigned pressure; no physical maximum/calibration is inferred.
    pub pressure: u16,
    /// Capacitive contact, independent of coordinates and pressure.
    pub touch: bool,
    /// Digital pad click, independent of contact and pressure.
    pub click: bool,
}
/// Native IMU sample. Records: `steam-c6-input-42`, `steam-c6-input-scaling`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SteamMotionSample {
    /// Native 0x42/0x45 timestamp interpreted as microseconds by SDL.
    pub timestamp_us: u32,
    /// Raw accelerometer X/Y/Z; no factory calibration or one-g default.
    pub accelerometer: [i16; 3],
    /// Raw gyroscope X/Y/Z, before consumer axis reordering/scaling.
    pub gyroscope: [i16; 3],
    /// Raw quaternion W/X/Y/Z tail in 0x42; zero is not an orientation claim.
    pub quaternion: [i16; 4],
}
/// Application-owned native inputs; default is raw zero with no calibration claim.
/// Records: `steam-c6-input-buttons`, `steam-c6-input-scaling`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SteamController2026State {
    buttons: u32,
    /// Left/right sticks, each native signed X/Y.
    pub sticks: [[i16; 2]; 2],
    /// Left/right analog triggers as signed raw consumer fields.
    pub triggers: [i16; 2],
    /// Left/right trackpads, with independent touch, click and pressure.
    pub pads: [SteamTrackpadState; 2],
    /// Left/right capacitive stick contact.
    pub stick_touch: [bool; 2],
    /// Left/right capacitive grip contact, separate from L4/L5/R4/R5 presses.
    pub grip_touch: [bool; 2],
    /// Native motion and timestamp, without consumer transforms.
    pub motion: SteamMotionSample,
}
impl SteamController2026State {
    /// Native control schema. This does not advertise a constructible realization.
    #[must_use]
    pub const fn input_topology(&self) -> &'static gr_controller_contract::InputTopology {
        &topology::TOPOLOGY
    }
    /// Clear every native input, touch flag, pressure and timestamp to raw zero.
    pub fn neutralize(&mut self) {
        *self = Self::default();
    }
    /// Whether a native digital control is pressed.
    #[must_use]
    pub const fn pressed(&self, button: SteamController2026Button) -> bool {
        self.buttons & (1 << button.bit()) != 0
    }
    /// Set one digital control without changing touch, pressure or analog state.
    pub fn set_button(&mut self, button: SteamController2026Button, pressed: bool) {
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
#[allow(dead_code)] // Optional private encoder; no battery API or creation path yet.
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
