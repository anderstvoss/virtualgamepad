//! Minimal profile-input builders for test authors.

use gr_core::{
    Dpad, DpadDelta, DualSenseDelta, DualSenseInput, GenericGamepadDelta, GenericGamepadInput,
    SteamControllerDelta, SteamControllerInput, Xbox360Delta, Xbox360Input,
};
use gr_profiles::{ControllerProfile, registry};

#[must_use]
pub fn dpad() -> Dpad {
    Dpad::neutral()
}

#[must_use]
pub fn dpad_delta() -> DpadDelta {
    DpadDelta::empty()
}

#[must_use]
pub fn generic_gamepad_delta() -> GenericGamepadDelta {
    GenericGamepadDelta::empty()
}

#[must_use]
pub fn xbox360_delta() -> Xbox360Delta {
    Xbox360Delta::empty()
}

#[must_use]
pub fn dualsense_delta() -> DualSenseDelta {
    DualSenseDelta::empty()
}

#[must_use]
pub fn steam_controller_delta() -> SteamControllerDelta {
    SteamControllerDelta::empty()
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

/// Builder for ad-hoc test profiles.
#[must_use]
pub fn ad_hoc_profile(id: &str) -> ControllerProfileBuilder {
    ControllerProfileBuilder {
        id: id.to_string(),
        missing_required_fields: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct ControllerProfileBuilder {
    id: String,
    missing_required_fields: Vec<String>,
}

impl ControllerProfileBuilder {
    /// Declare that a required profile field is intentionally missing,
    /// so a Phase-2 registry-loading test can assert the registry rejects
    /// the profile.
    #[must_use]
    pub fn missing_required_field(mut self, field: &'static str) -> Self {
        self.missing_required_fields.push(field.to_string());
        self
    }

    /// Build the ad-hoc profile value used by tests.
    ///
    /// # Panics
    ///
    /// Panics if the built-in `generic-gamepad` seed profile is not
    /// available in the closed Phase 2 registry.
    #[must_use]
    pub fn build(self) -> ControllerProfile {
        let mut profile = registry()
            .profile_by_str("generic-gamepad")
            .expect("generic gamepad profile")
            .clone();
        profile.profile_id = self.id.as_str().into();
        profile.display_name = "Ad hoc test profile";

        for field in &self.missing_required_fields {
            match field.as_str() {
                "display_name" => profile.display_name = "",
                "identity" | "identity.vendor_id" => {
                    profile.identity.vendor_id = 0u16.into();
                }
                "identity.product_id" => {
                    profile.identity.product_id = 0u16.into();
                }
                "supported_fidelity" => {
                    profile.supported_fidelity = &[];
                }
                "capabilities.input" => {
                    profile.capabilities.input = &[];
                }
                "input_contract.required_fields" => {
                    profile.input_contract.required_fields = &[];
                }
                _ => {}
            }
        }

        profile
    }
}

#[cfg(test)]
mod tests {
    use super::ad_hoc_profile;

    #[test]
    fn ad_hoc_profile_records_id_and_missing_fields() {
        let profile = ad_hoc_profile("invalid-test")
            .missing_required_field("display_name")
            .missing_required_field("identity")
            .build();

        assert_eq!(profile.profile_id.as_ref(), "invalid-test");
        assert!(profile.display_name.is_empty());
        assert_eq!(profile.identity.vendor_id.get(), 0);
    }
}
