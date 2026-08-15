#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
//! Compiled, controller-native implementations for the curated public API.
//!
//! Controller modules deliberately share transport helpers, not controller
//! state. Numeric values are native to their controller family.

mod common;
pub mod dualsense;
pub mod dualshock4;
pub mod switch_pro;
pub mod xbox360;

use gr_controller_contract::ControlError;
use gr_realization_api::{RealizationSessionId, RealizationTarget};

/// Options for one exact curated realization target.
#[derive(Debug, Clone, Copy)]
pub struct CreationOptions {
    pub target: RealizationTarget,
    pub session: RealizationSessionId,
}

/// Battery percentage shared by every curated controller family.
///
/// Battery exposure is live state rather than a creation option: callers can
/// model changing between an externally powered controller and a wireless
/// controller without disrupting an active provider session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryLevel(u8);
impl BatteryLevel {
    #[must_use]
    pub const fn percent(self) -> u8 {
        self.0
    }

    pub fn new(percent: u8) -> Result<Self, ControlError> {
        if percent > 100 {
            return Err(ControlError::ValueOutOfRange {
                control: "battery level",
                value: u32::from(percent),
                maximum: 100,
            });
        }
        Ok(Self(percent))
    }
}

/// Semantic battery state shared by all curated controller families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryState {
    exposed: bool,
    level: BatteryLevel,
}
impl Default for BatteryState {
    fn default() -> Self {
        Self {
            exposed: false,
            level: BatteryLevel(100),
        }
    }
}
impl BatteryState {
    #[must_use]
    pub const fn is_exposed(self) -> bool {
        self.exposed
    }

    #[must_use]
    pub const fn level(self) -> BatteryLevel {
        self.level
    }

    pub(crate) fn set_exposed(&mut self, exposed: bool) {
        self.exposed = exposed;
    }

    pub(crate) fn set_level(&mut self, level: BatteryLevel) {
        self.level = level;
    }
}

pub use dualsense::{
    DualSenseAxis, DualSenseControl, DualSenseController, DualSenseFeature, DualSenseHidOutput,
    DualSenseOutputEvent, DualSenseState, DualSenseSurface, DualSenseTouchContact,
    DualSenseTrigger, MotionSample, TouchSlot, create_dualsense,
};
pub use dualshock4::{
    DualShock4Axis, DualShock4Control, DualShock4Controller, DualShock4HidOutput,
    DualShock4MotionSample, DualShock4OutputEvent, DualShock4State, DualShock4Surface,
    DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger, create_dualshock4,
};
pub use switch_pro::{
    SwitchProAxis, SwitchProControl, SwitchProController, SwitchProMotionSample,
    SwitchProOutputEvent, SwitchProState, SwitchProSurface, create_switch_pro,
};
pub use xbox360::{
    Xbox360Axis, Xbox360Control, Xbox360Controller, Xbox360OutputEvent, Xbox360State,
    Xbox360Surface, Xbox360Trigger, create_xbox360,
};

#[cfg(test)]
mod battery_tests {
    use super::*;

    #[test]
    fn battery_level_accepts_the_full_percentage_domain_only() {
        assert_eq!(BatteryLevel::new(0).expect("empty battery").percent(), 0);
        assert_eq!(BatteryLevel::new(100).expect("full battery").percent(), 100);
        assert!(BatteryLevel::new(101).is_err());
    }

    #[test]
    fn battery_state_defaults_to_hidden_and_full() {
        let battery = BatteryState::default();
        assert!(!battery.is_exposed());
        assert_eq!(battery.level().percent(), 100);
    }
}
