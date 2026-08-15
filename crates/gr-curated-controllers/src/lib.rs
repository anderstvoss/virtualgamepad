#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]
//! Compiled, controller-native implementations for the curated public API.
//!
//! Controller modules deliberately share transport helpers, not controller
//! state. Numeric values are native to their controller family.

mod common;
pub mod dualsense;
pub mod generic_gamepad;
pub mod xbox360;

use gr_realization_api::{DeploymentTarget, RealizationSessionId};

/// Options for ordinary Linux controller creation.
#[derive(Debug, Clone, Copy)]
pub struct CreationOptions {
    pub target: DeploymentTarget,
    pub session: RealizationSessionId,
}

pub use dualsense::{
    DualSenseAxis, DualSenseControl, DualSenseController, DualSenseFeature, DualSenseHidOutput,
    DualSenseOutputEvent, DualSenseState, DualSenseSurface, DualSenseTouchContact,
    DualSenseTrigger, MotionSample, TouchSlot, create_dualsense,
};
pub use generic_gamepad::{
    GenericGamepadAxis, GenericGamepadControl, GenericGamepadController, GenericGamepadOutputEvent,
    GenericGamepadState, GenericGamepadSurface, GenericGamepadTrigger, create_generic_gamepad,
};
pub use xbox360::{
    Xbox360Axis, Xbox360Control, Xbox360Controller, Xbox360OutputEvent, Xbox360State,
    Xbox360Surface, Xbox360Trigger, create_xbox360,
};

#[cfg(test)]
mod integration_tests {
    use super::*;
    use gr_controller_contract::{DigitalControlUpdate, FaceButton};
    use gr_realization_api::{DeploymentTarget, RealizationSessionId};

    #[test]
    #[ignore = "requires ordinary-user /dev/uinput access"]
    fn all_evdev_controllers_create_commit_changed_state_and_close() {
        let options = |session| CreationOptions {
            target: DeploymentTarget::Evdev,
            session: RealizationSessionId(session),
        };

        let mut generic = create_generic_gamepad(options(101)).expect("generic creation");
        generic
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("generic update");
        generic.commit().expect("generic changed commit");
        generic.close();

        let mut xbox = create_xbox360(options(102)).expect("xbox creation");
        xbox.set_native(Xbox360Control::A, true)
            .expect("xbox update");
        xbox.commit().expect("xbox changed commit");
        xbox.close();

        let mut dualsense = create_dualsense(options(103)).expect("DualSense creation");
        dualsense
            .set_touch(
                TouchSlot::First,
                Some(DualSenseTouchContact::new(0, 960, 470).expect("native contact")),
            )
            .expect("DualSense touch update");
        dualsense.commit().expect("DualSense changed commit");
        dualsense.close();
    }

    #[test]
    #[ignore = "requires pre-provisioned /dev/uhid access"]
    fn all_hid_controllers_create_commit_changed_state_and_close() {
        let options = |session| CreationOptions {
            target: DeploymentTarget::Hid,
            session: RealizationSessionId(session),
        };

        let mut generic = create_generic_gamepad(options(201)).expect("generic creation");
        generic
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("generic update");
        generic.commit().expect("generic changed commit");
        generic.close();

        let mut xbox = create_xbox360(options(202)).expect("xbox creation");
        xbox.set_native(Xbox360Control::A, true)
            .expect("xbox update");
        xbox.commit().expect("xbox changed commit");
        xbox.close();

        let mut dualsense = create_dualsense(options(203)).expect("DualSense creation");
        dualsense
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("DualSense update");
        dualsense.commit().expect("DualSense changed commit");
        dualsense.close();
    }

    #[test]
    #[ignore = "requires pre-provisioned /dev/uhid access"]
    fn multiple_hid_controllers_remain_open_concurrently() {
        let options = |session| CreationOptions {
            target: DeploymentTarget::Hid,
            session: RealizationSessionId(session),
        };
        let mut first = create_generic_gamepad(options(301)).expect("first HID controller");
        let mut second = create_generic_gamepad(options(302)).expect("second HID controller");
        first
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::South,
                pressed: true,
            })
            .expect("first update");
        second
            .set_digital(DigitalControlUpdate::FaceButton {
                button: FaceButton::East,
                pressed: true,
            })
            .expect("second update");
        first.commit().expect("first commit");
        second.commit().expect("second commit");
        first.close();
        second.close();
    }
}
