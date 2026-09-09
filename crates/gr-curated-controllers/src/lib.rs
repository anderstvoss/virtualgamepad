#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
//! Compiled, controller-native implementations for the curated public API.
//!
//! Controller modules deliberately share transport helpers, not controller
//! state. Numeric values are native to their controller family.
//!
//! # Scheduling contract
//!
//! A controller is an active protocol endpoint. Call its `service` method even
//! while semantic input is unchanged; `commit` only submits edited input and is
//! not a substitute for idle servicing. `poll_output` is a compatibility alias.
//!
//! After creation and each service/commit, recompute `readiness`, `wants_write`
//! and `next_service_in`. For descriptor readiness, watch reads plus writes only
//! when requested. Also arm a monotonic timer for the returned relative duration:
//! zero means service now, and `None` means no timer, not necessarily no read
//! interest. `Readiness::Poll` requires bounded polling; native sessions currently
//! supply a four-millisecond fallback. Closed sessions advertise no interest.
//!
//! One owner may service many controllers fairly without threads or an async
//! runtime. Keep native edit transactions and output callbacks short. Required
//! curated HID/evdev work precedes optional observers within a service call, but
//! a blocked callback or editor still prevents the *next* call. An embedding
//! with slow producers should send bounded native commands to its service owner,
//! explicitly handle queue saturation and preserve press/release ordering. Do not
//! hold a controller mutex while drawing a UI or doing unrelated work.
//! Optional observations may be dropped; consult `dropped_output_events`.
//! The legacy compiled gadget path retains its separately documented Gate G limits.

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

/// Creation metadata for the current single-component curated profiles.
///
/// Requested physical paths distinguish creations even with restored identity
/// or reused application IDs. A cached host path is an observation at creation,
/// not proof that a node still exists or belongs to this session. Verify ancestry
/// and current identity before using host paths; never associate by display name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ControllerAssociation {
    pub requested_physical_path: Option<String>,
    pub requested_unique_id: Option<String>,
    pub observed_host_path: Option<String>,
}
impl ControllerAssociation {
    /// Controller-owned role; compound profiles have their own distinct roles.
    #[must_use]
    pub const fn role(&self) -> &'static str {
        "primary"
    }
    pub(crate) fn requested(realization: &gr_realization_api::NativeControllerRealization) -> Self {
        use gr_realization_api::NativeControllerRealization;
        match realization {
            NativeControllerRealization::Uhid(spec) => Self {
                requested_physical_path: Some(spec.physical_path.clone()),
                requested_unique_id: Some(spec.unique_id.clone()),
                observed_host_path: None,
            },
            NativeControllerRealization::Evdev(spec) => Self {
                requested_physical_path: spec.physical_path.clone(),
                ..Self::default()
            },
            NativeControllerRealization::DummyHcd(_) => Self::default(),
        }
    }
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
    DualSenseIdentity, DualSenseOutputEvent, DualSenseState, DualSenseSurface,
    DualSenseTouchContact, DualSenseTrigger, MotionSample, TouchSlot, create_dualsense,
    create_dualsense_with_identity,
};
pub use dualshock4::{
    DualShock4Axis, DualShock4Control, DualShock4Controller, DualShock4HidOutput,
    DualShock4Identity, DualShock4MotionSample, DualShock4OutputEvent, DualShock4State,
    DualShock4Surface, DualShock4TouchContact, DualShock4TouchSlot, DualShock4Trigger,
    create_dualshock4, create_dualshock4_with_identity,
};
pub use switch_pro::{
    SwitchProAxis, SwitchProControl, SwitchProController, SwitchProMotionSample,
    SwitchProOutputEvent, SwitchProRumble, SwitchProState, SwitchProSurface, create_switch_pro,
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
