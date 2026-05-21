//! Minimal profile-input builders for test authors.

use gr_core::{Dpad, DualSenseInput, GenericGamepadInput, SteamControllerInput, Xbox360Input};

#[must_use]
pub fn dpad() -> Dpad {
    Dpad::neutral()
}

#[must_use]
pub fn dualsense_input() -> DualSenseInputBuilder {
    DualSenseInputBuilder {
        inner: DualSenseInput::neutral(),
    }
}

#[must_use]
pub fn xbox360_input() -> Xbox360InputBuilder {
    Xbox360InputBuilder {
        inner: Xbox360Input::neutral(),
    }
}

#[must_use]
pub fn steam_controller_input() -> SteamControllerInputBuilder {
    SteamControllerInputBuilder {
        inner: SteamControllerInput::neutral(),
    }
}

#[must_use]
pub fn generic_gamepad_input() -> GenericGamepadInputBuilder {
    GenericGamepadInputBuilder {
        inner: GenericGamepadInput::neutral(),
    }
}

#[derive(Debug, Clone)]
pub struct DualSenseInputBuilder {
    inner: DualSenseInput,
}

impl DualSenseInputBuilder {
    #[must_use]
    pub fn build(self) -> DualSenseInput {
        self.inner
    }
}

#[derive(Debug, Clone)]
pub struct Xbox360InputBuilder {
    inner: Xbox360Input,
}

impl Xbox360InputBuilder {
    #[must_use]
    pub fn build(self) -> Xbox360Input {
        self.inner
    }
}

#[derive(Debug, Clone)]
pub struct SteamControllerInputBuilder {
    inner: SteamControllerInput,
}

impl SteamControllerInputBuilder {
    #[must_use]
    pub fn build(self) -> SteamControllerInput {
        self.inner
    }
}

#[derive(Debug, Clone)]
pub struct GenericGamepadInputBuilder {
    inner: GenericGamepadInput,
}

impl GenericGamepadInputBuilder {
    #[must_use]
    pub fn build(self) -> GenericGamepadInput {
        self.inner
    }
}
