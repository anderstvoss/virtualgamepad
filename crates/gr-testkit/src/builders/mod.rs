//! Minimal profile-input builders for test authors.

use gr_core::{
    Dpad, DpadDelta, DualSenseDelta, DualSenseInput, GenericGamepadDelta, GenericGamepadInput,
    SteamControllerDelta, SteamControllerInput, Xbox360Delta, Xbox360Input,
};

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
///
/// Phase 2 deliverable. Returns a placeholder `AdHocProfile` value
/// until the canonical `ControllerProfile` struct lands in `gr-profiles`,
/// at which point this builder will be renamed `ControllerProfileBuilder`
/// per [`TESTING_TOOLING_SPEC.md`](../../../../docs/spec/implementation/TESTING_TOOLING_SPEC.md).
#[must_use]
pub fn ad_hoc_profile(id: &str) -> AdHocProfileBuilder {
    AdHocProfileBuilder {
        id: id.to_string(),
        missing_required_fields: Vec::new(),
    }
}

/// Placeholder profile value returned by [`AdHocProfileBuilder::build`].
///
/// Holds only the data needed to exercise the ad-hoc-rejection path of
/// the registry; will gain real `ControllerProfile` fields in Phase 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdHocProfile {
    pub id: String,
    pub missing_required_fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AdHocProfileBuilder {
    id: String,
    missing_required_fields: Vec<String>,
}

impl AdHocProfileBuilder {
    /// Declare that a required profile field is intentionally missing,
    /// so a Phase-2 registry-loading test can assert the registry rejects
    /// the profile.
    #[must_use]
    pub fn missing_required_field(mut self, field: &'static str) -> Self {
        self.missing_required_fields.push(field.to_string());
        self
    }

    #[must_use]
    pub fn build(self) -> AdHocProfile {
        AdHocProfile {
            id: self.id,
            missing_required_fields: self.missing_required_fields,
        }
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

        assert_eq!(profile.id, "invalid-test");
        assert_eq!(
            profile.missing_required_fields,
            vec!["display_name".to_string(), "identity".to_string()]
        );
    }
}
