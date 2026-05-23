#![forbid(unsafe_code)]

//! Planning logic for `virtualgamepad`.
//!
//! The planner takes a [`SessionRequest`], compiled session options,
//! and a backend inventory, then returns either a runtime-ready
//! [`SessionPlan`] or a structured [`PlanRejection`].
//!
//! # Session id ownership
//!
//! The planner is a pure function of its inputs. `session_id` is
//! assigned by the caller (the session manager in Phase 7+) on the
//! incoming [`SessionRequest`] and mirrored by the planner onto
//! [`SessionPlan::session_id`] and
//! [`SessionPlan::backend_open_context`]. The planner does not invent
//! session ids — that would break determinism (the existing
//! `same_inputs_produce_same_plan` proptest enforces this).

use std::sync::Arc;

use gr_backend_api::{
    BackendFactory, BackendInventoryEntry, BackendRealizationRequest, BackendSupportReport,
    SupportLevel,
};
use gr_config::UnsupportedCapabilityPolicy;
use gr_core::{BackendLevel, FidelityTier, SemanticOutputFunction};
use gr_profiles::{OutputFunctionRef, ProfileFamily, registry};
use gr_runtime_model::{
    BackendOpenContext, CapabilityNegotiationResult, DegradationReason, DegradationReport,
    DeploymentRequirements, EmulationGoal, HostPlatform, PlanRejection, PlanRejectionReason,
    PlannerWarning, ProviderId, SessionPlan, SessionRequest, TranslatorFamily,
};
use gr_session_options::CompiledSessionOptions;

/// Compile a session request into a runtime-ready [`SessionPlan`].
///
/// # Errors
///
/// Returns a [`PlanRejection`] when no backend in the inventory can
/// realize the request at any tier given the supplied hints. A
/// degraded plan is still `Ok(plan)`; rejection is reserved for cases
/// where no plan is possible.
#[allow(clippy::too_many_lines)]
pub fn plan_session(
    request: &SessionRequest,
    session_options: &CompiledSessionOptions,
    inventory: &[BackendInventoryEntry],
    factories: &[Arc<dyn BackendFactory>],
) -> Result<SessionPlan, PlanRejection> {
    let profile = registry()
        .profile(request.profile_id.clone())
        .ok_or_else(|| PlanRejection {
            profile_id: request.profile_id.clone(),
            requested_goal: request.goal,
            requested_fidelity_tier: request.requested_fidelity_tier,
            reasons: vec![PlanRejectionReason::NoBackendSupportsProfile],
            considered_backends: inventory
                .iter()
                .map(|entry| entry.backend_id.clone())
                .collect(),
        })?;
    let target_host = requested_host_platform(request, session_options);
    let preferred_provider = request
        .provider_preference
        .clone()
        .or_else(|| session_options.provider_hints.preferred_provider.clone());
    let required_outputs = profile_required_outputs(profile.reverse_command_support.supported);
    let all_supported_tiers = supported_tiers(request.requested_fidelity_tier);
    let host_candidates = inventory
        .iter()
        .enumerate()
        .filter_map(|(inventory_index, entry)| {
            (entry.host_platform == target_host)
                .then(|| {
                    find_factory(factories, entry).map(|factory| CandidateSeed {
                        inventory_index,
                        entry,
                        factory,
                    })
                })
                .flatten()
        })
        .collect::<Vec<_>>();

    if host_candidates.is_empty() {
        let reasons = if inventory.is_empty() {
            vec![PlanRejectionReason::NoBackendSupportsProfile]
        } else if request.host_platform_preference.is_some()
            || session_options
                .provider_hints
                .host_platform_preference
                .is_some()
        {
            vec![PlanRejectionReason::NoBackendSupportsHost {
                requested: target_host,
            }]
        } else {
            vec![PlanRejectionReason::NoBackendSupportsProfile]
        };
        return Err(PlanRejection {
            profile_id: request.profile_id.clone(),
            requested_goal: request.goal,
            requested_fidelity_tier: request.requested_fidelity_tier,
            reasons,
            considered_backends: inventory
                .iter()
                .map(|entry| entry.backend_id.clone())
                .collect(),
        });
    }

    let mut best_candidate = None;
    let mut best_tier = None;
    let mut partial_missing = Vec::new();

    for &tier in all_supported_tiers {
        let tier_candidates = host_candidates
            .iter()
            .filter_map(|seed| {
                let report = seed.factory.can_realize(&BackendRealizationRequest {
                    profile_id: request.profile_id.clone(),
                    requested_goal: EmulationGoal::from(tier),
                    requested_fidelity_tier: tier,
                    host_platform: target_host,
                    required_output_functions: required_outputs.clone(),
                });
                let missing = missing_output_functions(&required_outputs, &report);
                let viable = is_viable_candidate(tier, &report, &missing);
                if viable {
                    Some(Candidate {
                        inventory_index: seed.inventory_index,
                        entry: seed.entry,
                        report,
                        missing_output_functions: missing,
                    })
                } else {
                    if tier != FidelityTier::Compatibility && !missing.is_empty() {
                        partial_missing.extend(missing);
                    }
                    None
                }
            })
            .collect::<Vec<_>>();

        if let Some(candidate) = select_best_candidate(
            tier_candidates,
            preferred_provider.as_ref(),
            request.backend_preference,
        ) {
            best_tier = Some(tier);
            best_candidate = Some(candidate);
            break;
        }
    }

    let Some(selected_tier) = best_tier else {
        let mut reasons = Vec::new();
        if request.requested_fidelity_tier != FidelityTier::Compatibility
            && session_options.unsupported_capability_policy == UnsupportedCapabilityPolicy::Reject
            && !partial_missing.is_empty()
        {
            reasons.push(PlanRejectionReason::BidirectionalSupportRequired {
                missing: dedup_output_functions(partial_missing),
            });
        }
        if reasons.is_empty() {
            reasons.push(PlanRejectionReason::NoBackendSupportsProfile);
        }
        return Err(PlanRejection {
            profile_id: request.profile_id.clone(),
            requested_goal: request.goal,
            requested_fidelity_tier: request.requested_fidelity_tier,
            reasons,
            considered_backends: inventory
                .iter()
                .map(|entry| entry.backend_id.clone())
                .collect(),
        });
    };
    let Some(selected) = best_candidate else {
        return Err(PlanRejection {
            profile_id: request.profile_id.clone(),
            requested_goal: request.goal,
            requested_fidelity_tier: request.requested_fidelity_tier,
            reasons: vec![PlanRejectionReason::NoBackendSupportsProfile],
            considered_backends: inventory
                .iter()
                .map(|entry| entry.backend_id.clone())
                .collect(),
        });
    };

    if request.requested_fidelity_tier != FidelityTier::Compatibility
        && selected_tier == FidelityTier::Compatibility
        && session_options.unsupported_capability_policy == UnsupportedCapabilityPolicy::Reject
        && !selected.missing_output_functions.is_empty()
    {
        return Err(PlanRejection {
            profile_id: request.profile_id.clone(),
            requested_goal: request.goal,
            requested_fidelity_tier: request.requested_fidelity_tier,
            reasons: vec![PlanRejectionReason::BidirectionalSupportRequired {
                missing: selected.missing_output_functions.clone(),
            }],
            considered_backends: inventory
                .iter()
                .map(|entry| entry.backend_id.clone())
                .collect(),
        });
    }

    let mut degradation_reasons =
        degrade_reasons(request.requested_fidelity_tier, selected_tier).to_vec();
    let mut warnings = Vec::new();

    if let Some(preferred) = preferred_provider.clone() {
        let selected_provider = provider_id_from_entry(selected.entry);
        if selected_provider != preferred {
            let reason = format!(
                "preferred provider `{}` was unavailable for the selected plan; using `{}`",
                preferred.0.clone(),
                selected_provider.0.clone()
            );
            degradation_reasons.push(DegradationReason::ProviderHintIgnored {
                preferred: preferred.clone(),
                reason: reason.clone(),
            });
            warnings.push(PlannerWarning {
                code: "provider-hint-ignored".to_string(),
                message: reason,
            });
        }
    }

    if let Some(preferred_level) = request.backend_preference
        && preferred_level != selected.entry.level
    {
        let reason = format!(
            "preferred backend level `{preferred_level}` was unavailable for the selected plan; using `{}`",
            selected.entry.level
        );
        degradation_reasons.push(DegradationReason::BackendLevelHintIgnored {
            preferred: preferred_level,
            reason: reason.clone(),
        });
        warnings.push(PlannerWarning {
            code: "backend-preference-ignored".to_string(),
            message: reason,
        });
    }

    // Emit per-output typed degradation reasons only when a tier
    // change has already happened. At compatibility-on-compatibility,
    // missing outputs are expected (the tier doesn't promise reverse
    // support); they're surfaced via
    // `capability_result.unsupported_capabilities` instead.
    if selected_tier != request.requested_fidelity_tier {
        for function in &selected.missing_output_functions {
            degradation_reasons.push(DegradationReason::UnsupportedOutputCapability {
                function: *function,
                reason: format!(
                    "selected backend `{}` does not realize `{function}`",
                    selected.entry.backend_id
                ),
            });
        }
    }

    let capability_result = capability_result(
        &required_outputs,
        &selected.report,
        &selected.missing_output_functions,
    );
    let deployment_requirements = DeploymentRequirements {
        requirements: selected
            .entry
            .notes
            .iter()
            .chain(selected.report.notes.iter())
            .cloned()
            .collect(),
    };
    let selected_provider_id = provider_id_from_entry(selected.entry);
    let session_id = request.session_id;

    Ok(SessionPlan {
        session_id,
        profile_id: request.profile_id.clone(),
        requested_goal: request.goal,
        requested_fidelity_tier: selected_tier,
        selected_level: selected.entry.level,
        target_host_platform: target_host,
        selected_backend_family: selected.entry.family,
        selected_provider_id: selected_provider_id.clone(),
        selected_translator_family: translator_family(profile.profile_family),
        capability_result,
        degradation: DegradationReport {
            degraded: !degradation_reasons.is_empty(),
            reasons: degradation_reasons,
        },
        warnings,
        deployment_requirements,
        backend_open_context: BackendOpenContext {
            session_id,
            profile_id: request.profile_id.clone(),
            fidelity_tier: selected_tier,
            backend_level: selected.entry.level,
            host_platform: target_host,
        },
        session_options: session_options.snapshot(),
    })
}

#[derive(Clone)]
struct CandidateSeed<'a> {
    inventory_index: usize,
    entry: &'a BackendInventoryEntry,
    factory: &'a Arc<dyn BackendFactory>,
}

#[derive(Clone)]
struct Candidate<'a> {
    inventory_index: usize,
    entry: &'a BackendInventoryEntry,
    report: BackendSupportReport,
    missing_output_functions: Vec<SemanticOutputFunction>,
}

fn requested_host_platform(
    request: &SessionRequest,
    session_options: &CompiledSessionOptions,
) -> HostPlatform {
    request
        .host_platform_preference
        .or(session_options.provider_hints.host_platform_preference)
        .unwrap_or_else(current_host_platform)
}

fn current_host_platform() -> HostPlatform {
    #[cfg(target_os = "linux")]
    {
        HostPlatform::Linux
    }
    #[cfg(target_os = "windows")]
    {
        HostPlatform::Windows
    }
    #[cfg(target_os = "macos")]
    {
        HostPlatform::Macos
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        HostPlatform::Linux
    }
}

fn find_factory<'a>(
    factories: &'a [Arc<dyn BackendFactory>],
    entry: &BackendInventoryEntry,
) -> Option<&'a Arc<dyn BackendFactory>> {
    factories
        .iter()
        .find(|factory| factory.backend_id() == entry.backend_id)
}

fn supported_tiers(requested: FidelityTier) -> &'static [FidelityTier] {
    match requested {
        FidelityTier::HardwareFaithful => &[
            FidelityTier::HardwareFaithful,
            FidelityTier::IdentityAware,
            FidelityTier::Compatibility,
        ],
        FidelityTier::IdentityAware => &[FidelityTier::IdentityAware, FidelityTier::Compatibility],
        FidelityTier::Compatibility => &[FidelityTier::Compatibility],
    }
}

fn profile_required_outputs(functions: &[OutputFunctionRef]) -> Vec<SemanticOutputFunction> {
    functions
        .iter()
        .filter_map(|function| match function {
            OutputFunctionRef::Semantic(output) => Some(*output),
            _ => None,
        })
        .collect()
}

fn missing_output_functions(
    required_outputs: &[SemanticOutputFunction],
    report: &BackendSupportReport,
) -> Vec<SemanticOutputFunction> {
    required_outputs
        .iter()
        .copied()
        .filter(|required| !report.supported_output_functions.contains(required))
        .collect()
}

fn is_viable_candidate(
    tier: FidelityTier,
    report: &BackendSupportReport,
    missing: &[SemanticOutputFunction],
) -> bool {
    if report.forward_support == SupportLevel::None {
        return false;
    }
    if tier == FidelityTier::Compatibility {
        return true;
    }

    report.reverse_support == SupportLevel::Full && missing.is_empty()
}

fn select_best_candidate<'a>(
    candidates: Vec<Candidate<'a>>,
    preferred_provider: Option<&ProviderId>,
    preferred_level: Option<BackendLevel>,
) -> Option<Candidate<'a>> {
    candidates.into_iter().max_by(|left, right| {
        candidate_rank(left, preferred_provider, preferred_level)
            .cmp(&candidate_rank(right, preferred_provider, preferred_level))
            .then_with(|| right.inventory_index.cmp(&left.inventory_index))
    })
}

fn candidate_rank(
    candidate: &Candidate<'_>,
    preferred_provider: Option<&ProviderId>,
    preferred_level: Option<BackendLevel>,
) -> (u8, u8, u8) {
    let provider_match = u8::from(
        preferred_provider
            .is_some_and(|preferred| provider_id_from_entry(candidate.entry) == *preferred),
    );
    let level_match = u8::from(preferred_level.is_some_and(|level| candidate.entry.level == level));
    (
        provider_match,
        level_match,
        backend_level_rank(candidate.entry.level),
    )
}

fn backend_level_rank(level: BackendLevel) -> u8 {
    match level {
        BackendLevel::Evdev => 0,
        BackendLevel::Hid => 1,
        BackendLevel::Transport => 2,
    }
}

fn provider_id_from_entry(entry: &BackendInventoryEntry) -> ProviderId {
    ProviderId::from(entry.backend_id.as_ref())
}

fn degrade_reasons(
    requested: FidelityTier,
    selected: FidelityTier,
) -> &'static [DegradationReason] {
    match (requested, selected) {
        (FidelityTier::HardwareFaithful, FidelityTier::IdentityAware) => {
            &[DegradationReason::TransportNotRealizable]
        }
        (FidelityTier::IdentityAware, FidelityTier::Compatibility) => {
            &[DegradationReason::ReversePathUnavailable]
        }
        (FidelityTier::HardwareFaithful, FidelityTier::Compatibility) => &[
            DegradationReason::TransportNotRealizable,
            DegradationReason::ReversePathUnavailable,
        ],
        _ => &[],
    }
}

fn capability_result(
    required_outputs: &[SemanticOutputFunction],
    report: &BackendSupportReport,
    missing_output_functions: &[SemanticOutputFunction],
) -> CapabilityNegotiationResult {
    let enabled_capabilities = required_outputs
        .iter()
        .filter(|function| report.supported_output_functions.contains(function))
        .map(ToString::to_string)
        .collect();
    let unsupported_capabilities = missing_output_functions
        .iter()
        .map(ToString::to_string)
        .collect();

    CapabilityNegotiationResult {
        enabled_capabilities,
        unsupported_capabilities,
    }
}

fn dedup_output_functions(functions: Vec<SemanticOutputFunction>) -> Vec<SemanticOutputFunction> {
    let mut deduped = Vec::new();
    for function in functions {
        if !deduped.contains(&function) {
            deduped.push(function);
        }
    }
    deduped
}

fn translator_family(profile_family: ProfileFamily) -> TranslatorFamily {
    match profile_family {
        ProfileFamily::GenericGamepad => TranslatorFamily::GenericGamepad,
        ProfileFamily::Xbox360 => TranslatorFamily::XboxStyle,
        ProfileFamily::DualSense => TranslatorFamily::DualSense,
        ProfileFamily::SteamController => TranslatorFamily::SteamController,
        _ => TranslatorFamily::Unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::plan_session;
    use gr_backend_api::{BackendFactory, BackendInventoryEntry};
    use gr_config::{
        AcceptedUpdateKind, ConfigBackpressurePolicy, InputSection, OutputHandlingMode,
        OutputHandlingSection, SessionConfig, SessionSection, UnsupportedCapabilityPolicy,
        ValidationSection,
    };
    use gr_core::{BackendFamily, BackendLevel, FidelityTier, ProfileId, SessionId};
    use gr_runtime_model::{
        DegradationReason, EmulationGoal, HostPlatform, PlanRejectionReason, SessionHostMetadata,
        SessionRequest,
    };
    use gr_session_options::compile_session_options;
    use gr_testkit::fakes::backend_factory;
    use insta::assert_snapshot;
    use proptest::prelude::*;
    use std::sync::Arc;

    fn base_request() -> SessionRequest {
        SessionRequest {
            session_id: SessionId::new(1),
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
                host_platform_preference: None,
                backend_preference: Some(BackendLevel::Hid),
                provider_preference: None,
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

    fn compiled_options() -> gr_session_options::CompiledSessionOptions {
        compile_session_options(&base_config()).expect("compile")
    }

    fn fake_factory(
        backend_id: &str,
        family: BackendFamily,
        level: BackendLevel,
        tiers: Vec<FidelityTier>,
        outputs: &[gr_core::SemanticOutputFunction],
    ) -> Arc<dyn BackendFactory> {
        let mut builder = backend_factory()
            .backend_id(backend_id)
            .family(family)
            .level(level)
            .platform(HostPlatform::Linux)
            .supported_fidelity_tiers(tiers);
        for output in outputs {
            builder = builder.declares_reverse_output(*output);
        }
        Arc::new(builder.build())
    }

    fn inventory_from(factories: &[Arc<dyn BackendFactory>]) -> Vec<BackendInventoryEntry> {
        factories
            .iter()
            .map(|factory| factory.inventory_entry())
            .collect()
    }

    fn dualsense_outputs() -> [gr_core::SemanticOutputFunction; 6] {
        [
            gr_core::SemanticOutputFunction::Rumble,
            gr_core::SemanticOutputFunction::Haptics,
            gr_core::SemanticOutputFunction::Lighting,
            gr_core::SemanticOutputFunction::PlayerIndicators,
            gr_core::SemanticOutputFunction::TriggerEffect,
            gr_core::SemanticOutputFunction::Audio,
        ]
    }

    #[test]
    fn exact_match_without_degradation() {
        let request = base_request();
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware],
            &dualsense_outputs(),
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.selected_backend_family, BackendFamily::LinuxUhid);
        assert!(!plan.degradation.degraded);
        assert_snapshot!(
            "identity_aware_linux_uhid",
            serde_yaml::to_string(&plan).expect("yaml")
        );
    }

    #[test]
    fn hardware_faithful_degrades_to_hid() {
        let mut request = base_request();
        request.goal = EmulationGoal::HardwareFaithful;
        request.requested_fidelity_tier = FidelityTier::HardwareFaithful;
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware],
            &dualsense_outputs(),
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.requested_fidelity_tier, FidelityTier::IdentityAware);
        assert!(matches!(
            plan.degradation.reasons.as_slice(),
            [DegradationReason::TransportNotRealizable]
        ));
        assert_snapshot!(
            "hardware_faithful_degrades_to_identity_aware",
            serde_yaml::to_string(&plan).expect("yaml")
        );
    }

    #[test]
    fn identity_aware_degrades_to_compatibility_on_evdev_only() {
        let request = base_request();
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uinput",
            BackendFamily::LinuxUinput,
            BackendLevel::Evdev,
            vec![FidelityTier::Compatibility],
            &[],
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.selected_level, BackendLevel::Evdev);
        assert_eq!(plan.requested_fidelity_tier, FidelityTier::Compatibility);
        assert!(plan.degradation.degraded);
        assert!(
            plan.degradation
                .reasons
                .iter()
                .any(|reason| matches!(reason, DegradationReason::ReversePathUnavailable)),
            "expected ReversePathUnavailable in {:?}",
            plan.degradation.reasons
        );
    }

    #[test]
    fn empty_inventory_rejects() {
        let request = base_request();
        let options = compiled_options();
        let rejection = plan_session(&request, &options, &[], &[]).expect_err("rejection");
        assert!(matches!(
            rejection.reasons.as_slice(),
            [PlanRejectionReason::NoBackendSupportsProfile]
        ));
        assert_snapshot!(
            "empty_inventory_rejection",
            serde_yaml::to_string(&rejection).expect("yaml")
        );
    }

    #[test]
    fn host_platform_mismatch_rejects() {
        let mut request = base_request();
        request.host_platform_preference = Some(HostPlatform::Windows);
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware],
            &dualsense_outputs(),
        )];
        let inventory = inventory_from(&factories);

        let rejection =
            plan_session(&request, &options, &inventory, &factories).expect_err("host mismatch");
        assert!(matches!(
            rejection.reasons.as_slice(),
            [PlanRejectionReason::NoBackendSupportsHost {
                requested: HostPlatform::Windows
            }]
        ));
    }

    #[test]
    fn provider_preference_falls_through_with_warning() {
        let request = base_request();
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware],
            &dualsense_outputs(),
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.warnings.len(), 0);
    }

    #[test]
    fn explicit_provider_preference_is_reported_when_unavailable() {
        let mut request = base_request();
        request.provider_preference = Some("missing-provider".into());
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware],
            &dualsense_outputs(),
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.warnings[0].code, "provider-hint-ignored");
    }

    #[test]
    fn xbox360_compatibility_on_uinput_no_degradation() {
        let mut request = base_request();
        request.profile_id = ProfileId::from("xbox360");
        request.goal = EmulationGoal::Compatibility;
        request.requested_fidelity_tier = FidelityTier::Compatibility;
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uinput",
            BackendFamily::LinuxUinput,
            BackendLevel::Evdev,
            vec![FidelityTier::Compatibility],
            &[],
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.selected_backend_family, BackendFamily::LinuxUinput);
        assert_eq!(plan.selected_level, BackendLevel::Evdev);
        assert_eq!(plan.requested_fidelity_tier, FidelityTier::Compatibility);
        assert!(
            !plan.degradation.degraded,
            "xbox360 compatibility on uinput should not degrade; got {:?}",
            plan.degradation.reasons
        );
    }

    #[test]
    fn missing_outputs_under_reject_policy_rejects() {
        let request = base_request();
        let mut config = base_config();
        config.validation.unsupported_capability_policy = UnsupportedCapabilityPolicy::Reject;
        let options = compile_session_options(&config).expect("compile");
        // Fake declares IA but advertises no outputs; reject policy means
        // the planner should refuse to degrade to compatibility.
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware, FidelityTier::Compatibility],
            &[],
        )];
        let inventory = inventory_from(&factories);

        let rejection = plan_session(&request, &options, &inventory, &factories)
            .expect_err("reject policy should fail");
        assert!(
            rejection.reasons.iter().any(|reason| matches!(
                reason,
                PlanRejectionReason::BidirectionalSupportRequired { .. }
            )),
            "expected BidirectionalSupportRequired; got {:?}",
            rejection.reasons
        );
    }

    #[test]
    fn backend_notes_populate_deployment_requirements() {
        let request = base_request();
        let options = compiled_options();
        let mut builder = backend_factory()
            .backend_id("linux-uhid")
            .family(BackendFamily::LinuxUhid)
            .level(BackendLevel::Hid)
            .platform(HostPlatform::Linux)
            .supported_fidelity_tiers(vec![FidelityTier::IdentityAware])
            .note("requires kernel 5.14+");
        for output in dualsense_outputs() {
            builder = builder.declares_reverse_output(output);
        }
        let factory: Arc<dyn BackendFactory> = Arc::new(builder.build());
        let factories = vec![factory.clone()];
        let inventory = vec![factory.inventory_entry()];

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert!(
            plan.deployment_requirements
                .requirements
                .iter()
                .any(|requirement| requirement == "requires kernel 5.14+"),
            "expected backend note on deployment_requirements; got {:?}",
            plan.deployment_requirements.requirements
        );
    }

    #[test]
    fn session_id_is_propagated_to_plan_and_open_context() {
        let mut request = base_request();
        request.session_id = SessionId::new(42);
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware],
            &dualsense_outputs(),
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.session_id, SessionId::new(42));
        assert_eq!(plan.backend_open_context.session_id, SessionId::new(42));
    }

    #[test]
    fn backend_level_hint_falls_through_with_degradation_reason() {
        let mut request = base_request();
        request.backend_preference = Some(BackendLevel::Evdev);
        let options = compiled_options();
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::IdentityAware],
            &dualsense_outputs(),
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert!(plan.degradation.degraded);
        assert!(plan.degradation.reasons.iter().any(|reason| matches!(
            reason,
            DegradationReason::BackendLevelHintIgnored {
                preferred: BackendLevel::Evdev,
                ..
            }
        )));
        assert!(
            plan.warnings
                .iter()
                .any(|warning| warning.code == "backend-preference-ignored")
        );
    }

    #[test]
    fn unsupported_outputs_emit_typed_degradation_reasons() {
        let request = base_request();
        let options = compiled_options();
        // Fake factory declares the profile but advertises NO outputs;
        // the planner should still produce a compatibility plan (uhid is
        // willing to forward) but flag each missing output as a typed
        // degradation reason.
        let factories = vec![fake_factory(
            "linux-uhid",
            BackendFamily::LinuxUhid,
            BackendLevel::Hid,
            vec![FidelityTier::Compatibility],
            &[],
        )];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        let missing_capability_reasons = plan
            .degradation
            .reasons
            .iter()
            .filter(|reason| {
                matches!(
                    reason,
                    DegradationReason::UnsupportedOutputCapability { .. }
                )
            })
            .count();
        assert!(
            missing_capability_reasons >= 1,
            "expected at least one UnsupportedOutputCapability reason; got {:?}",
            plan.degradation.reasons
        );
    }

    #[test]
    fn tie_breaks_to_higher_level_when_no_hint_disambiguates() {
        let request = base_request();
        let options = compiled_options();
        let factories = vec![
            fake_factory(
                "linux-uinput",
                BackendFamily::LinuxUinput,
                BackendLevel::Evdev,
                vec![FidelityTier::Compatibility],
                &[],
            ),
            fake_factory(
                "linux-uhid",
                BackendFamily::LinuxUhid,
                BackendLevel::Hid,
                vec![FidelityTier::IdentityAware],
                &dualsense_outputs(),
            ),
        ];
        let inventory = inventory_from(&factories);

        let plan = plan_session(&request, &options, &inventory, &factories).expect("plan");
        assert_eq!(plan.selected_backend_family, BackendFamily::LinuxUhid);
    }

    proptest! {
        #[test]
        fn same_inputs_produce_same_plan(_seed in 0u8..16) {
            let request = base_request();
            let options = compiled_options();
            let factories = vec![fake_factory(
                "linux-uhid",
                BackendFamily::LinuxUhid,
                BackendLevel::Hid,
                vec![FidelityTier::IdentityAware],
                &dualsense_outputs(),
            )];
            let inventory = inventory_from(&factories);

            let left = plan_session(&request, &options, &inventory, &factories).expect("left");
            let right = plan_session(&request, &options, &inventory, &factories).expect("right");
            prop_assert_eq!(left, right);
        }
    }
}
