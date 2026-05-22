#![forbid(unsafe_code)]

//! Planning logic for `virtualgamepad`.
//!
//! The planner takes a [`SessionRequest`], compiled session options,
//! and a backend inventory, then returns either a runtime-ready
//! [`SessionPlan`] or a structured [`PlanRejection`]. The contract is
//! defined in `RUST_IMPLEMENTATION_SPEC.md`; this prep PR pins the
//! signature and Phase 5 fills in the body.

use std::sync::Arc;

use gr_backend_api::{BackendFactory, BackendInventoryEntry};
use gr_runtime_model::{PlanRejection, SessionPlan, SessionRequest};
use gr_session_options::CompiledSessionOptions;

/// Compile a session request into a runtime-ready
/// [`SessionPlan`].
///
/// Phase 5 fills in the body; this prep PR pins the signature so
/// downstream crates (`gr-session`, demo wiring) can be written against
/// a stable contract.
///
/// # Errors
///
/// Returns a [`PlanRejection`] when no backend in the inventory can
/// realize the request at any tier given the supplied hints. A
/// degraded plan is still `Ok(plan)`; rejection is reserved for cases
/// where no plan is possible.
///
/// # Panics
///
/// Stub implementation panics via `unimplemented!()`. Phase 5 replaces
/// the body with the planner logic.
pub fn plan_session(
    _request: &SessionRequest,
    _session_options: &CompiledSessionOptions,
    _inventory: &[BackendInventoryEntry],
    _factories: &[Arc<dyn BackendFactory>],
) -> Result<SessionPlan, PlanRejection> {
    unimplemented!("Phase 5 planner implementation")
}

#[cfg(test)]
mod tests {
    use super::plan_session;
    use gr_backend_api::BackendInventoryEntry;
    use gr_config::{
        AcceptedUpdateKind, ConfigBackpressurePolicy, HostPlatformPreference, InputSection,
        OutputHandlingMode, OutputHandlingSection, SessionConfig, SessionSection,
        UnsupportedCapabilityPolicy, ValidationSection,
    };
    use gr_core::{BackendLevel, FidelityTier, ProfileId};
    use gr_runtime_model::{EmulationGoal, SessionHostMetadata, SessionRequest};
    use gr_session_options::compile_session_options;

    fn base_request() -> SessionRequest {
        SessionRequest {
            profile_id: ProfileId::from("dualsense"),
            goal: EmulationGoal::IdentityAware,
            requested_fidelity_tier: FidelityTier::IdentityAware,
            host_platform_preference: None,
            backend_preference: None,
            provider_preference: None,
            host_metadata: SessionHostMetadata::default(),
        }
    }

    fn base_config() -> SessionConfig {
        SessionConfig {
            session: SessionSection {
                session_id: None,
                profile_id: ProfileId::from("dualsense"),
                fidelity_tier: FidelityTier::IdentityAware,
                host_platform_preference: Some(HostPlatformPreference::Linux),
                backend_preference: Some(BackendLevel::Hid),
                provider_preference: Some("linux-uhid".to_string()),
            },
            input: InputSection {
                accepted_update_kinds: vec![AcceptedUpdateKind::Frame],
                reject_unknown_fields: true,
                reject_out_of_range_values: true,
                coerce_integer_like_values: false,
                allow_missing_optional_fields: true,
                require_monotonic_sequence: false,
            },
            output_handling: OutputHandlingSection {
                mode: OutputHandlingMode::Callback,
                callback_namespace: Some("virtualGamepad".to_string()),
                state_field_prefix: None,
                backpressure_policy: ConfigBackpressurePolicy::DropOldest,
                log_dropped_outputs: true,
                max_queue_depth: Some(8),
                bridge_capabilities: Vec::new(),
            },
            validation: ValidationSection {
                require_supported_profile: true,
                reject_unsupported_fidelity: true,
                reject_unsupported_provider_preference: true,
                reject_unknown_config_fields: false,
                unsupported_capability_policy: UnsupportedCapabilityPolicy::Report,
            },
        }
    }

    #[test]
    #[should_panic(expected = "Phase 5 planner implementation")]
    fn stub_signature_compiles_and_panics() {
        let request = base_request();
        let config = base_config();
        let options = compile_session_options(&config).expect("compile");
        let inventory: Vec<BackendInventoryEntry> = Vec::new();
        let factories = Vec::new();
        let _ = plan_session(&request, &options, &inventory, &factories);
    }
}
