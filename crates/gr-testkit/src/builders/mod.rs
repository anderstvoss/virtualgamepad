//! Minimal profile-input builders for test authors.

use gr_core::{
    BackendLevel, Dpad, DpadDelta, DualSenseDelta, DualSenseInput, FidelityTier,
    GenericGamepadDelta, GenericGamepadInput, ProfileId, SessionId, SteamControllerDelta,
    SteamControllerInput, Xbox360Delta, Xbox360Input,
};
use gr_profiles::{ControllerProfile, registry};
use gr_runtime_model::{
    EmulationGoal, HostPlatform, ProviderId, SessionHostMetadata, SessionRequest,
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

/// Fluent builder for [`SessionRequest`] values used by planner +
/// session tests. Mirrors the existing input builders' style. Default
/// values are a dualsense identity-aware request with `session_id: 1`
/// and no hints.
#[must_use]
pub fn session_request(profile_id: impl Into<ProfileId>) -> SessionRequestBuilder {
    SessionRequestBuilder {
        session_id: SessionId::new(1),
        profile_id: profile_id.into(),
        goal: EmulationGoal::IdentityAware,
        requested_fidelity_tier: FidelityTier::IdentityAware,
        host_platform_preference: None,
        backend_preference: None,
        provider_preference: None,
        host_metadata: SessionHostMetadata::default(),
    }
}

#[derive(Debug, Clone)]
pub struct SessionRequestBuilder {
    session_id: SessionId,
    profile_id: ProfileId,
    goal: EmulationGoal,
    requested_fidelity_tier: FidelityTier,
    host_platform_preference: Option<HostPlatform>,
    backend_preference: Option<BackendLevel>,
    provider_preference: Option<ProviderId>,
    host_metadata: SessionHostMetadata,
}

impl SessionRequestBuilder {
    #[must_use]
    pub fn session_id(mut self, id: impl Into<SessionId>) -> Self {
        self.session_id = id.into();
        self
    }

    #[must_use]
    pub fn goal(mut self, goal: EmulationGoal) -> Self {
        self.goal = goal;
        self
    }

    #[must_use]
    pub fn requested_fidelity_tier(mut self, tier: FidelityTier) -> Self {
        self.requested_fidelity_tier = tier;
        self
    }

    #[must_use]
    pub fn host_platform(mut self, host: HostPlatform) -> Self {
        self.host_platform_preference = Some(host);
        self
    }

    #[must_use]
    pub fn backend_preference(mut self, level: BackendLevel) -> Self {
        self.backend_preference = Some(level);
        self
    }

    #[must_use]
    pub fn provider_preference(mut self, provider: impl Into<ProviderId>) -> Self {
        self.provider_preference = Some(provider.into());
        self
    }

    #[must_use]
    pub fn host_metadata(mut self, metadata: SessionHostMetadata) -> Self {
        self.host_metadata = metadata;
        self
    }

    #[must_use]
    pub fn build(self) -> SessionRequest {
        SessionRequest {
            session_id: self.session_id,
            profile_id: self.profile_id,
            goal: self.goal,
            requested_fidelity_tier: self.requested_fidelity_tier,
            host_platform_preference: self.host_platform_preference,
            backend_preference: self.backend_preference,
            provider_preference: self.provider_preference,
            host_metadata: self.host_metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ad_hoc_profile, session_request};
    use gr_core::{BackendLevel, FidelityTier, SessionId};
    use gr_runtime_model::{EmulationGoal, HostPlatform};

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

    #[test]
    fn session_request_defaults_are_identity_aware_dualsense() {
        let request = session_request("dualsense").build();
        assert_eq!(request.session_id, SessionId::new(1));
        assert_eq!(request.profile_id.as_ref(), "dualsense");
        assert_eq!(request.goal, EmulationGoal::IdentityAware);
        assert_eq!(request.requested_fidelity_tier, FidelityTier::IdentityAware);
        assert!(request.host_platform_preference.is_none());
        assert!(request.backend_preference.is_none());
        assert!(request.provider_preference.is_none());
    }

    #[test]
    fn session_request_builder_threads_all_hints() {
        let request = session_request("xbox360")
            .session_id(SessionId::new(42))
            .goal(EmulationGoal::Compatibility)
            .requested_fidelity_tier(FidelityTier::Compatibility)
            .host_platform(HostPlatform::Linux)
            .backend_preference(BackendLevel::Evdev)
            .provider_preference("linux-uinput")
            .build();
        assert_eq!(request.session_id, SessionId::new(42));
        assert_eq!(request.goal, EmulationGoal::Compatibility);
        assert_eq!(request.requested_fidelity_tier, FidelityTier::Compatibility);
        assert_eq!(request.host_platform_preference, Some(HostPlatform::Linux));
        assert_eq!(request.backend_preference, Some(BackendLevel::Evdev));
        assert_eq!(
            request.provider_preference.as_ref().map(|p| p.0.as_str()),
            Some("linux-uinput")
        );
    }
}
