//! Wii Remote native state. Source-only scope; no physical fidelity claim.
//!
//! Evidence: `wii-kernel-report-modes`, `wii-kernel-ir-mode-policy`,
//! `wii-kernel-extension-init`, `wii-kernel-motionplus-pass-through`.

use gr_controller_contract::{ControlError, InputKind};

/// Revision boundaries are deliberately explicit. In particular, an early Plus
/// is not inferred from the original Remote's product name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WiiRemoteRevision {
    #[default]
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

/// Creation-time compatibility policy; both bounded shims are initially enabled.
///
/// These reads are consumer compatibility behavior, not physical fidelity.
/// None of these options implements `MotionPlus` challenge data or readiness progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WiiRemoteCreationOptions {
    /// Mirror downstream identity at F6/F8/F9 with fixed compat F7 = 0x02.
    pub motionplus_passthrough_id_mirror: bool,
    /// Expose exact caller-imported downstream calibration at active offset 0x40.
    pub motionplus_passthrough_calibration_copy: bool,
    /// Plus-only compat: use external-style inactive MP identity for Cemu activation.
    /// Default false; Original always retains its external identity.
    pub motionplus_inactive_id_external_style: bool,
}
impl Default for WiiRemoteCreationOptions {
    fn default() -> Self {
        Self {
            motionplus_passthrough_id_mirror: true,
            motionplus_passthrough_calibration_copy: true,
            motionplus_inactive_id_external_style: false,
        }
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiiNunchukState {
    /// Raw X/Y,0..255; source coordinate midpoint128, not factory calibration.
    pub stick: [u8; 2],
    /// Raw X/Y/Z ten-bit samples,0..1023.
    pub accelerometer: [u16; 3],
    pub c: bool,
    pub z: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiiClassicState {
    /// Raw left X/Y,0..63; source midpoint32.
    pub left: [u8; 2],
    /// Raw right X/Y,0..31; source midpoint16.
    pub right: [u8; 2],
    /// Raw left/right trigger values,0..31.
    pub triggers: [u8; 2],
    /// A,B,X,Y,ZL,ZR,Plus,Minus,Home,Left,Right,Up,Down,L,R.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WiiMotionPlusState {
    pub attached: bool,
    /// Raw pitch/roll/yaw,0..16383; source zero-rate center8192.
    pub gyroscope: [u16; 3],
    /// Slow-range flags in the same pitch/roll/yaw order.
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
                gyroscope: [8192; 3],
                slow: [false; 3],
            },
            battery: 0,
        }
    }
    /// Clear inputs while preserving attachment, revision and battery metadata.
    pub fn neutralize(&mut self) {
        self.buttons = [false; 11];
        self.accelerometer = [0; 3];
        self.ir = [None; 4];
        self.extension = match self.extension {
            WiiExtension::None => WiiExtension::None,
            WiiExtension::Nunchuk(_) => WiiExtension::Nunchuk(WiiNunchukState::default()),
            WiiExtension::Classic(_) => WiiExtension::Classic(WiiClassicState::default()),
            WiiExtension::ClassicPro(_) => WiiExtension::ClassicPro(WiiClassicState::default()),
        };
        self.motion_plus.gyroscope = [8192; 3];
        self.motion_plus.slow = [false; 3];
    }
    #[must_use]
    pub fn input_topology(&self) -> &'static gr_controller_contract::InputTopology {
        topology::current(self.extension(), self.motion_plus.attached)
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
    /// Edit a detached native snapshot's attachment without provider I/O.
    ///
    /// Use `WiiRemoteController::attach` to apply a live transition. A shape change
    /// resets `MotionPlus` samples, matching the live attachment transaction.
    pub fn set_attachment(
        &mut self,
        extension: WiiExtension,
        motion_plus: bool,
    ) -> Result<(), ControlError> {
        extensions::validate(&extension)?;
        if std::mem::discriminant(&extension) != std::mem::discriminant(&self.extension)
            || motion_plus != self.motion_plus.attached
        {
            self.motion_plus = WiiMotionPlusState {
                attached: motion_plus,
                gyroscope: [8192; 3],
                slow: [false; 3],
            };
        }
        self.extension = extension;
        Ok(())
    }
    pub fn set_extension_values(&mut self, value: WiiExtension) -> Result<(), ControlError> {
        if std::mem::discriminant(&value) != std::mem::discriminant(&self.extension) {
            return Err(ControlError::UnsupportedControl {
                control: "extension kind requires attachment transition",
            });
        }
        extensions::validate(&value)?;
        self.extension = value;
        Ok(())
    }
    pub fn set_motion_plus_values(
        &mut self,
        gyroscope: [u16; 3],
        slow: [bool; 3],
    ) -> Result<(), ControlError> {
        if !self.motion_plus.attached || gyroscope.iter().any(|v| *v > 16383) {
            return Err(ControlError::UnsupportedControl {
                control: "MotionPlus absent or outside fourteen bits",
            });
        }
        self.motion_plus.gyroscope = gyroscope;
        self.motion_plus.slow = slow;
        Ok(())
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
pub use controller::{
    WiiRemoteController, create_wii_remote, create_wii_remote_with_image,
    create_wii_remote_with_options,
};

mod codec;

mod extensions;
mod memory;
pub use memory::{WiiEepromImage, WiiResourceDisposition};

mod calibration;
pub use calibration::WiiExtensionCalibration;

mod topology;

// Source coordinate midpoints, not factory calibration values.
impl Default for WiiNunchukState {
    fn default() -> Self {
        Self {
            stick: [128; 2],
            accelerometer: [0; 3],
            c: false,
            z: false,
        }
    }
}
impl Default for WiiClassicState {
    fn default() -> Self {
        Self {
            left: [32; 2],
            right: [16; 2],
            triggers: [0; 2],
            buttons: [false; 15],
        }
    }
}
#[cfg(test)]
mod lifecycle_tests;

mod native;
mod realizations;

#[cfg(test)]
mod native_tests;

#[cfg(all(test, target_os = "linux"))]
mod live_tests;

impl Default for WiiMotionPlusState {
    fn default() -> Self {
        // wii-kernel-motionplus-pass-through: nonrotating raw center is8192,
        // independent of physical calibration or fast/slow selection.
        Self {
            attached: false,
            gyroscope: [8192; 3],
            slow: [false; 3],
        }
    }
}
