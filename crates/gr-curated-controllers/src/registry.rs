//! Typed family metadata; no application-facing root exports.
#[cfg(test)]
use crate::common::HidDriver;
use gr_controller_contract::{
    ControllerSurface, InputTopology, OutputKind, RealizationControllerDefinition,
    RealizationManifest,
};
pub use gr_controller_wire::registry::{FamilySpec, UsbProfileSpec, worker_id};
use gr_realization_api::ControllerId;
#[cfg(test)]
use gr_realization_api::RealizationTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealizationTier {
    PlayableReduced,
    PlayableFull,
    FaithfulUsb,
    FaithfulRadio,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionClass {
    Uinput,
    Uhid,
    RootBroker,
}
#[derive(Debug, Clone, Copy)]
pub struct FamilyRealization {
    pub surface: &'static ControllerSurface,
    pub tier: RealizationTier,
    pub permission: PermissionClass,
}
impl FamilyRealization {
    pub(crate) const fn new(
        surface: &'static ControllerSurface,
        tier: RealizationTier,
        permission: PermissionClass,
    ) -> Self {
        Self {
            surface,
            tier,
            permission,
        }
    }
}
// Metadata registration must not require a provider implementation before its gate.
pub(crate) trait ControllerFamily: RealizationControllerDefinition {
    const SPEC: &'static FamilySpec;
    const OUTPUT_KINDS: &'static [OutputKind] = &[];
    const ID: ControllerId = Self::SPEC.id;
    const BROKER_TAG: Option<u8> = Self::SPEC.broker_tag;
    fn usb_profile() -> Option<UsbProfileSpec> {
        Self::SPEC.usb_profile
    }
    fn topology() -> &'static InputTopology;
    fn realizations() -> &'static [FamilyRealization];
    fn manifest() -> RealizationManifest;
}
#[allow(dead_code)] // Reserved crate-internal contracts for steps 5-9; no runtime allocation.
#[derive(Debug, Clone, Copy)]
pub(crate) enum FamilyKind {
    Controller,
    Receiver {
        hosted_families: &'static [ControllerId],
    },
}
pub struct FamilyEntry {
    pub id: ControllerId,
    pub spec: &'static FamilySpec,
    pub broker_tag: Option<u8>,
    pub usb_profile: fn() -> Option<UsbProfileSpec>,
    pub topology: fn() -> &'static InputTopology,
    pub realizations: fn() -> &'static [FamilyRealization],
    pub manifest: fn() -> RealizationManifest,
    #[allow(dead_code)]
    pub(crate) max_topology: fn() -> &'static InputTopology,
    #[allow(dead_code)]
    pub output_kinds: &'static [OutputKind],
    #[allow(dead_code)]
    pub(crate) subsystems: &'static [&'static str],
    #[allow(dead_code)]
    pub(crate) persistent_resources: &'static [&'static str],
    #[allow(dead_code)]
    pub(crate) refused_operations: &'static [&'static str],
    #[allow(dead_code)]
    pub(crate) kind: FamilyKind,
}
pub(crate) const fn family_entry<F: ControllerFamily>() -> FamilyEntry {
    FamilyEntry {
        id: F::ID,
        spec: F::SPEC,
        broker_tag: F::BROKER_TAG,
        usb_profile: F::usb_profile,
        topology: F::topology,
        realizations: F::realizations,
        manifest: F::manifest,
        max_topology: F::topology,
        output_kinds: F::OUTPUT_KINDS,
        subsystems: &[],
        persistent_resources: &[],
        refused_operations: &[],
        kind: FamilyKind::Controller,
    }
}
macro_rules! typed_registry {
    ($(($module:ident, $definition:ident, $spec:ident, [$($variant:ident)?]),)*) => {
        pub static FAMILIES: &[FamilyEntry] = &[
            $(family_entry::<crate::$module::$definition>(),)*
            #[cfg(test)] family_entry::<test_family::Fifth>(),
        ];
    };
}
gr_controller_wire::compiled_families!(typed_registry);
// 14a may register metadata without an advertised driver/provider. Promotion to
// FAMILIES is a separate 14c change after step 16; existing constructor adapters
// remain an invariant of every publicly advertised family.
#[allow(dead_code)]
pub(crate) static STAGED_FAMILIES: &[FamilyEntry] = &[family_entry::<
    crate::steam_controller_2026::SteamController2026Definition,
>()];
#[cfg(test)]
mod test_family;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_metadata_agrees_with_controller_definitions() {
        use gr_controller_contract::RealizationControllerDefinition;
        macro_rules! check_definitions {
            ($(($module:ident, $definition:ident, $spec:ident, [$($variant:ident)?]),)*) => {
                $(assert_eq!(crate::$module::$definition.controller_id(), gr_controller_wire::registry::$spec.id);)*
            };
        }
        gr_controller_wire::compiled_families!(check_definitions);
    }

    #[test]
    fn unique_ids_tags_and_consistent_surface_manifest_pairs() {
        let mut ids = std::collections::HashSet::new();
        let mut tags = std::collections::HashSet::new();
        for family in FAMILIES {
            assert!(ids.insert(family.id));
            assert_eq!(family.spec.id, family.id);
            assert_eq!(family.spec.broker_tag, family.broker_tag);
            if let Some(tag) = family.broker_tag {
                assert!(tags.insert(tag));
            }
            (family.topology)().validate().unwrap();
            assert_eq!((family.max_topology)(), (family.topology)());
            assert!(
                family.subsystems.is_empty()
                    && family.persistent_resources.is_empty()
                    && family.refused_operations.is_empty()
            );
            assert!(matches!(family.kind, FamilyKind::Controller));
            let manifest = (family.manifest)();
            let surfaces = (family.realizations)();
            assert_eq!(manifest.entries().len(), surfaces.len());
            for entry in manifest.entries() {
                let matches: Vec<_> = surfaces
                    .iter()
                    .filter(|s| s.surface.target() == entry.target)
                    .collect();
                assert_eq!(matches.len(), 1);
                let realization = matches[0];
                let (tier, permission) = match entry.target {
                    RealizationTarget::LINUX_UINPUT => {
                        (RealizationTier::PlayableReduced, PermissionClass::Uinput)
                    }
                    RealizationTarget::LINUX_UHID_USB | RealizationTarget::LINUX_UHID_BLUETOOTH => {
                        (RealizationTier::PlayableFull, PermissionClass::Uhid)
                    }
                    RealizationTarget::LINUX_DUMMY_HCD_USB_HID
                    | RealizationTarget::LINUX_USBIP_USB_AUDIO => {
                        (RealizationTier::FaithfulUsb, PermissionClass::RootBroker)
                    }
                    _ => panic!("unknown tier"),
                };
                assert_eq!(
                    (realization.tier, realization.permission),
                    (tier, permission)
                );
            }
        }
        assert_eq!(tags.len(), 3);
    }
}

#[cfg(test)]
mod receiver_metadata_tests {
    use super::*;
    #[test]
    fn receiver_metadata_declares_hosted_families_without_advertising_a_real_device() {
        // Metadata-only synthetic receiver. No production surface, provider or
        // Puck identity is advertised by the lifecycle fixture.
        static SPEC: FamilySpec = FamilySpec {
            id: ControllerId::new("test.receiver"),
            worker_name: "test-receiver",
            broker_tag: None,
            dummy_hcd_tag: None,
            dummy_hcd_report_length: None,
            usb_profile: None,
        };
        let receiver = FamilyEntry {
            id: SPEC.id,
            spec: &SPEC,
            broker_tag: None,
            usb_profile: || None,
            topology: || &InputTopology::EMPTY,
            realizations: || &[],
            manifest: || RealizationManifest::new(&[]),
            max_topology: || &InputTopology::EMPTY,
            output_kinds: &[],
            subsystems: &[],
            persistent_resources: &[],
            refused_operations: &[],
            kind: FamilyKind::Receiver {
                hosted_families: &[test_family::Fifth::ID],
            },
        };
        let FamilyKind::Receiver { hosted_families } = receiver.kind else {
            panic!("receiver kind lost")
        };
        assert_eq!(hosted_families, &[test_family::Fifth::ID]);
        assert!(
            hosted_families
                .iter()
                .all(|id| FAMILIES.iter().any(|family| family.id == *id))
        );
        assert!(FAMILIES.iter().all(|family| family.id != receiver.id));
        assert!((receiver.realizations)().is_empty());
        assert!((receiver.manifest)().entries().is_empty());
    }
}

#[cfg(test)]
mod gadget_tag_tests {
    #[test]
    fn compiled_gadget_tags_are_unique() {
        let mut tags = std::collections::HashSet::new();
        for family in super::FAMILIES {
            if let Some(tag) = family.spec.dummy_hcd_tag {
                assert!(tags.insert(tag), "duplicate gadget tag {tag}");
            }
        }
    }
}

#[cfg(test)]
#[path = "registry/output_kind_tests.rs"]
mod output_kind_tests;
