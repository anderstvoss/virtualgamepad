//! Wii Remote native state. Source-only scope; no physical fidelity claim.
//!
//! Evidence: `wii-kernel-report-modes`, `wii-kernel-ir-mode-policy`,
//! `wii-kernel-extension-init`, `wii-kernel-motionplus-pass-through`.

use gr_controller_contract::{ControlError, InputKind};

/// Revision boundaries are deliberately explicit. In particular, an early Plus
/// is not inferred from the original Remote's product name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WiiRemoteRevision {
    Original,
    EarlyIntegratedPlus,
    Tr,
}

impl WiiRemoteRevision {
    /// Whether the pinned consumer ledger settles this revision's identity.
    #[must_use]
    pub const fn has_identity_evidence(self) -> bool {
        // wii-original-remote-consumer-revision, wii-tr-consumer-revision,
        // wii-early-plus-revision-gap. This is NOT a physical revision oracle.
        !matches!(self, Self::EarlyIntegratedPlus)
    }
}

/// Core button order follows wii-kernel-evdev-base-keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WiiRemoteButton {
    Left,
    Right,
    Up,
    Down,
    Plus,
    Minus,
    One,
    Two,
    A,
    B,
    Home,
}
impl WiiRemoteButton {
    const fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Up => 2,
            Self::Down => 3,
            Self::Plus => 4,
            Self::Minus => 5,
            Self::One => 6,
            Self::Two => 7,
            Self::A => 8,
            Self::B => 9,
            Self::Home => 10,
        }
    }
}

/// Visible IR point in the source-backed camera coordinate domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WiiIrPoint {
    x: u16,
    y: u16,
}
impl WiiIrPoint {
    pub fn new(x: u16, y: u16) -> Result<Self, ControlError> {
        // wii-kernel-evdev-ir: native coordinates represented by the consumer.
        if x > 1023 || y > 767 {
            return Err(ControlError::UnsupportedControl {
                control: "IR point outside the source-backed coordinate domain",
            });
        }
        Ok(Self { x, y })
    }
    #[must_use]
    pub const fn coordinates(self) -> (u16, u16) {
        (self.x, self.y)
    }
}

/// Nunchuk values remain native; no cross-family normalized gamepad state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WiiNunchukState {
    pub stick: [u8; 2],
    pub accelerometer: [u16; 3],
    pub c: bool,
    pub z: bool,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WiiClassicState {
    pub left: [u8; 2],
    pub right: [u8; 2],
    pub triggers: [u8; 2],
    pub buttons: [bool; 15],
}

/// One physical downstream accessory, distinct from a `MotionPlus` bridge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WiiExtension {
    #[default]
    None,
    Nunchuk(WiiNunchukState),
    Classic(WiiClassicState),
    ClassicPro(WiiClassicState),
}

/// The bridge and its downstream accessory are independent topology facts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WiiMotionPlusState {
    pub attached: bool,
    pub gyroscope: [u16; 3],
    pub slow: [bool; 3],
}

/// Application-owned semantic inputs. Host-owned LEDs and rumble are separate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiiRemoteState {
    revision: WiiRemoteRevision,
    buttons: [bool; 11],
    accelerometer: [u16; 3],
    ir: [Option<WiiIrPoint>; 4],
    extension: WiiExtension,
    motion_plus: WiiMotionPlusState,
    battery: u8,
}
impl Default for WiiRemoteState {
    fn default() -> Self {
        Self::new(WiiRemoteRevision::Original)
    }
}
impl WiiRemoteState {
    #[must_use]
    pub const fn new(revision: WiiRemoteRevision) -> Self {
        Self {
            revision,
            buttons: [false; 11],
            // No invented factory calibration or one-g default. These are
            // uncalibrated raw inputs; calibrated defaults need their own record.
            accelerometer: [0; 3],
            ir: [None; 4],
            extension: WiiExtension::None,
            motion_plus: WiiMotionPlusState {
                attached: matches!(
                    revision,
                    WiiRemoteRevision::Tr | WiiRemoteRevision::EarlyIntegratedPlus
                ),
                gyroscope: [0; 3],
                slow: [false; 3],
            },
            battery: 0,
        }
    }
    #[must_use]
    pub const fn revision(&self) -> WiiRemoteRevision {
        self.revision
    }
    #[must_use]
    pub const fn pressed(&self, button: WiiRemoteButton) -> bool {
        self.buttons[button.index()]
    }
    pub fn set_button(&mut self, button: WiiRemoteButton, pressed: bool) {
        self.buttons[button.index()] = pressed;
    }
    #[must_use]
    pub const fn accelerometer(&self) -> [u16; 3] {
        self.accelerometer
    }
    pub fn set_accelerometer(&mut self, values: [u16; 3]) -> Result<(), ControlError> {
        if values.iter().any(|value| *value > 1023) {
            return Err(ControlError::UnsupportedControl {
                control: "Wii accelerometer requires ten-bit raw values",
            });
        }
        self.accelerometer = values;
        Ok(())
    }
    #[must_use]
    pub const fn ir(&self) -> &[Option<WiiIrPoint>; 4] {
        &self.ir
    }
    pub fn set_ir(&mut self, points: [Option<WiiIrPoint>; 4]) {
        self.ir = points;
    }
    #[must_use]
    pub const fn extension(&self) -> &WiiExtension {
        &self.extension
    }
    #[must_use]
    pub const fn motion_plus(&self) -> &WiiMotionPlusState {
        &self.motion_plus
    }
    #[must_use]
    pub const fn battery_raw(&self) -> u8 {
        self.battery
    }
    pub fn set_battery_raw(&mut self, battery: u8) {
        self.battery = battery;
    }
}

/// Retained host intent is never reconstructed from optional observation events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WiiRemoteOutputs {
    pub revision: u64,
    pub leds: [bool; 4],
    pub rumble: bool,
}

/// IR is a point set on the native surface, including invisible slots.
#[must_use]
pub const fn ir_kind() -> InputKind {
    use gr_controller_contract::InputAxisRange;
    InputKind::PointSet {
        points: 4,
        x: InputAxisRange {
            minimum: 0,
            maximum: 1023,
            neutral: 0,
        },
        y: InputAxisRange {
            minimum: 0,
            maximum: 767,
            neutral: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revisions_remain_distinct_and_unknown_identity_is_not_promoted() {
        assert!(WiiRemoteRevision::Original.has_identity_evidence());
        assert!(WiiRemoteRevision::Tr.has_identity_evidence());
        assert!(!WiiRemoteRevision::EarlyIntegratedPlus.has_identity_evidence());
        assert!(!WiiRemoteState::default().motion_plus().attached);
        assert!(
            WiiRemoteState::new(WiiRemoteRevision::Tr)
                .motion_plus()
                .attached
        );
        assert_ne!(
            WiiRemoteState::new(WiiRemoteRevision::EarlyIntegratedPlus),
            WiiRemoteState::new(WiiRemoteRevision::Tr)
        );
    }
    #[test]
    fn ir_is_a_typed_point_set_with_independent_invisible_slots() {
        assert!(matches!(ir_kind(), InputKind::PointSet { points: 4, .. }));
        for x in [0, 1, 511, 1023] {
            for y in [0, 1, 383, 767] {
                assert_eq!(WiiIrPoint::new(x, y).unwrap().coordinates(), (x, y));
            }
        }
        assert!(WiiIrPoint::new(1024, 0).is_err());
        assert!(WiiIrPoint::new(0, 768).is_err());
        let mut state = WiiRemoteState::default();
        let point = WiiIrPoint::new(1023, 767).unwrap();
        state.set_ir([None, Some(point), None, Some(point)]);
        assert_eq!(state.ir(), &[None, Some(point), None, Some(point)]);
    }
    #[test]
    fn rejected_sensor_edit_preserves_state_and_buttons_are_independent() {
        let mut state = WiiRemoteState::default();
        state.set_accelerometer([0, 511, 1023]).unwrap();
        let before = state.clone();
        assert!(state.set_accelerometer([0, 1024, 0]).is_err());
        assert_eq!(state, before);
        state.set_button(WiiRemoteButton::A, true);
        assert!(state.pressed(WiiRemoteButton::A));
        assert!(!state.pressed(WiiRemoteButton::B));
    }
}

mod definition;
pub use definition::{BLUETOOTH, TOPOLOGY, UINPUT, WiiRemoteDefinition};

mod controller;
pub use controller::{WiiRemoteController, create_wii_remote};

mod codec;
