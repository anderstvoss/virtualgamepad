//! 14a declarative family registration. No driver, provider or constructor.
use crate::registry::{
    ControllerFamily, FamilyRealization, FamilySpec, PermissionClass, RealizationTier,
};
use gr_controller_contract::{
    ControllerSurface, InputTopology, RealizationControllerDefinition, RealizationManifest,
    RealizationManifestEntry, RealizationValidationStatus, TargetRestriction,
    construction::ControllerSurfaceSpec,
};
use gr_realization_api::{ControllerId, ProviderRequirements, RealizationTarget};

pub(crate) struct SteamController2026Definition;
const BLOCKED: TargetRestriction = TargetRestriction {
    feature: "14a planned target only",
    reason: "Blocked at 14c/step 16: no provider, driver or constructor; kernel/SDL acceptance requires the 7.3+ host",
};
const fn surface(target: RealizationTarget) -> ControllerSurface {
    ControllerSurfaceSpec {
        target,
        validation_status: RealizationValidationStatus::ResearchBacked,
        digital_controls: &[],
        axes: &[],
        outputs: &[],
        output_kinds: &[],
        restrictions: &[BLOCKED],
        input_topology: &super::topology::TOPOLOGY,
    }
    .build()
}
static UINPUT: ControllerSurface = surface(RealizationTarget::LINUX_UINPUT);
static WIRED: ControllerSurface = surface(RealizationTarget::LINUX_UHID_USB);
impl RealizationControllerDefinition for SteamController2026Definition {
    fn controller_id(&self) -> ControllerId {
        gr_controller_wire::registry::STEAM_CONTROLLER_2026.id
    }
    fn realization_manifest(&self) -> RealizationManifest {
        // Declares planned targets under 14a, not a provider open request.
        static ENTRIES: &[RealizationManifestEntry] = &[
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UINPUT,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
            RealizationManifestEntry {
                target: RealizationTarget::LINUX_UHID_USB,
                provider_requirements: ProviderRequirements {
                    requires_reverse_output: true,
                },
                audio_sidecar: None,
            },
        ];
        RealizationManifest::new(ENTRIES)
    }
}
impl ControllerFamily for SteamController2026Definition {
    const SPEC: &'static FamilySpec = &gr_controller_wire::registry::STEAM_CONTROLLER_2026;
    fn topology() -> &'static InputTopology {
        &super::topology::TOPOLOGY
    }
    fn realizations() -> &'static [FamilyRealization] {
        static TARGETS: &[FamilyRealization] = &[
            FamilyRealization::new(
                &UINPUT,
                RealizationTier::PlayableReduced,
                PermissionClass::Uinput,
            ),
            FamilyRealization::new(&WIRED, RealizationTier::PlayableFull, PermissionClass::Uhid),
        ];
        TARGETS
    }
    fn manifest() -> RealizationManifest {
        Self.realization_manifest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_family_declares_only_blocked_planned_targets_without_host_profiles() {
        let id = SteamController2026Definition.controller_id();
        let family = crate::registry::STAGED_FAMILIES
            .iter()
            .find(|f| f.id == id)
            .unwrap();
        assert!(!crate::registry::FAMILIES.iter().any(|f| f.id == id));
        assert!(gr_controller_wire::registry::find(id).is_none());
        assert_eq!((family.manifest)().entries().len(), 2);
        assert!(family.broker_tag.is_none() && family.spec.dummy_hcd_tag.is_none());
        assert!((family.usb_profile)().is_none());
        assert!(gr_controller_wire::registry::worker_id("steam-controller-2026").is_none());
        let targets = (family.realizations)();
        assert_eq!(
            targets
                .iter()
                .map(|t| t.surface.target())
                .collect::<Vec<_>>(),
            [
                RealizationTarget::LINUX_UINPUT,
                RealizationTarget::LINUX_UHID_USB
            ]
        );
        for target in targets {
            assert_eq!(
                target.surface.validation_status(),
                RealizationValidationStatus::ResearchBacked
            );
            assert!(
                target
                    .surface
                    .restrictions()
                    .iter()
                    .any(|r| r.reason.contains("no provider, driver or constructor"))
            );
            assert!(
                target.surface.outputs().is_empty() && target.surface.output_kinds().is_empty()
            );
            assert_eq!(
                target.surface.input_topology(),
                &super::super::topology::TOPOLOGY
            );
        }
    }
}
