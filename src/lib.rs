#![forbid(unsafe_code)]
#![doc = include_str!("../docs/APPLICATION_API.md")]
#![doc = concat!("\nController-family boundaries:\n\n```compile_fail,E0308\n", include_str!("../tests/ui/dualsense_rejects_xbox_native_control.rs"), "\n```\n\n```compile_fail,E0599\n", include_str!("../tests/ui/xbox_has_no_touch_surface.rs"), "\n```\n")]

//! Standalone, controller-native virtual controllers.
//!
//! Select an exact realization, edit typed state, commit, and keep servicing even
//! when input is idle. No routing, physical capture, host setup, executor, or
//! persistence policy is installed by this library.
//!
//! ```no_run
//! use virtualgamepad::{DualSenseController, DualSenseControl, ControlError};
//! fn press(c: &mut DualSenseController) -> Result<(), ControlError> {
//!     c.set_native(DualSenseControl::Cross, true)
//! }
//! ```
//! ```compile_fail
//! use virtualgamepad::NativeProviderFactory;
//! ```
//! ```compile_fail
//! use virtualgamepad::DualSenseController;
//! fn invalid(c: &mut DualSenseController) { c.reply_get_report(1, 0, vec![]); }
//! ```
//! ```no_run
//! use virtualgamepad::{DualSenseController, DualSenseTouchContact, TouchSlot,
//!     Xbox360Controller, Xbox360Control, ControlError};
//! fn valid(sony: &mut DualSenseController, xbox: &mut Xbox360Controller,
//!          contact: DualSenseTouchContact) -> Result<(), ControlError> {
//!     sony.set_touch(TouchSlot::First, Some(contact))?;
//!     xbox.set_native(Xbox360Control::A, true)
//! }
//! ```
//! ```compile_fail
//! use virtualgamepad::{CreationOptions, RealizationId};
//! let options = CreationOptions { target: RealizationId::LINUX_UINPUT };
//! ```
//! ```compile_fail
//! use virtualgamepad::RealizationId;
//! let ambiguous = RealizationId::Uhid;
//! ```
//! ```compile_fail
//! use virtualgamepad::DualSenseController;
//! fn invalid(c: &mut DualSenseController) { c.poll_output(&mut |_| {}); }
//! ```
//! ```compile_fail
//! use virtualgamepad::DualSenseController;
//! fn invalid(c: &mut DualSenseController) {
//!     let readiness = c.readiness();
//!     c.close();
//!     println!("{readiness:?}");
//! }
//! ```
mod application;
mod controllers;
mod output;
pub use application::*;
pub use controllers::*;
pub use output::*;
/// Unstable implementation/research interfaces, outside the application contract.
#[cfg(feature = "experimental")]
pub mod experimental;
pub use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, DpadDirection, FaceButton, OutputSurface,
    RealizationValidationStatus, TargetRestriction,
};
pub use gr_curated_controllers::{
    BatteryLevel, BatteryState, DualSenseAxis, DualSenseControl, DualSenseFeature,
    DualSenseHidOutput, DualSenseState, DualSenseSurface, DualSenseTouchContact, DualSenseTrigger,
    DualShock4Axis, DualShock4Control, DualShock4HidOutput, DualShock4MotionSample,
    DualShock4State, DualShock4Surface, DualShock4TouchContact, DualShock4TouchSlot,
    DualShock4Trigger, MotionSample, SwitchProAxis, SwitchProControl, SwitchProMotionSample,
    SwitchProRumble, SwitchProState, SwitchProSurface, TouchSlot, Xbox360Axis, Xbox360Control,
    Xbox360State, Xbox360Surface, Xbox360Trigger,
};
pub use gr_realization_api::{
    ControllerId, ForceFeedbackEffect, ForceFeedbackEvent, RealizationId, RealizationTargetSet,
    RumbleEffect,
};
