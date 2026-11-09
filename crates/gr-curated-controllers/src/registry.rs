//! Typed family metadata; no application-facing root exports.
#[cfg(test)]
use crate::common::HidDriver;
use gr_controller_contract::{
    ControllerSurface, InputTopology, OutputKind, RealizationControllerDefinition,
    RealizationManifest,
};
pub use gr_controller_wire::registry::{FamilySpec, UsbProfileSpec, worker_id};
use gr_realization_api::{ControllerId, RealizationTarget};

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
/// Whether an option is a consumer compatibility shim or an opt-in feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CreationOptionKind {
    Compat,
    Feature,
}

/// A family-specific creation toggle. Names are stable within a family.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct CreationToggle {
    pub name: &'static str,
    pub label: &'static str,
    pub default: bool,
    pub kind: CreationOptionKind,
    pub targets: &'static [RealizationTarget],
    pub fidelity: &'static str,
}

/// A family capability or limitation scoped to specific realizations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FamilyOperation {
    pub name: &'static str,
    pub targets: &'static [RealizationTarget],
    pub detail: &'static str,
}

use gr_controller_contract::TargetRestriction;

pub(crate) const AUDIO_SUBSYSTEMS: &[FamilyOperation] = &[
    FamilyOperation {
        name: "pipewire_audio",
        targets: &[RealizationTarget::LINUX_UHID_USB],
        detail: "The application crate can create associated PipeWire endpoints with explicit emulated audio.",
    },
    FamilyOperation {
        name: "usbip_audio",
        targets: &[RealizationTarget::LINUX_USBIP_USB_AUDIO],
        detail: "The application crate can create an opt-in functional USB/IP composite with emulated audio.",
    },
];

pub(crate) const fn refusal(
    restriction: TargetRestriction,
    targets: &'static [RealizationTarget],
) -> FamilyOperation {
    FamilyOperation {
        name: restriction.feature,
        targets,
        detail: restriction.reason,
    }
}

// Metadata registration must not require a provider implementation before its gate.
pub(crate) trait ControllerFamily: RealizationControllerDefinition {
    const SPEC: &'static FamilySpec;
    const OUTPUT_KINDS: &'static [OutputKind] = &[];
    const SUBSYSTEMS: &'static [FamilyOperation] = &[];
    const PERSISTENT_RESOURCES: &'static [FamilyOperation] = &[];
    const REFUSED_OPERATIONS: &'static [FamilyOperation] = &[];
    const CREATION_TOGGLES: &'static [CreationToggle] = &[];
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
    pub subsystems: &'static [FamilyOperation],
    pub persistent_resources: &'static [FamilyOperation],
    pub refused_operations: &'static [FamilyOperation],
    pub creation_toggles: &'static [CreationToggle],
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
        subsystems: F::SUBSYSTEMS,
        persistent_resources: F::PERSISTENT_RESOURCES,
        refused_operations: F::REFUSED_OPERATIONS,
        creation_toggles: F::CREATION_TOGGLES,
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
#[cfg(test)]
mod test_family;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiled_metadata_agrees_with_controller_definitions() {
        use gr_controller_contract::RealizationControllerDefinition;
        macro_rules! definition {
            (SteamController2026Definition, $module:ident) => {
                crate::$module::SteamController2026Definition::default()
            };
            ($name:ident, $module:ident) => {
                crate::$module::$name
            };
        }
        macro_rules! check_definitions {
            ($(($module:ident, $definition:ident, $spec:ident, [$($variant:ident)?]),)*) => {
                $(assert_eq!(definition!($definition, $module).controller_id(), gr_controller_wire::registry::$spec.id);)*
            };
        }
        gr_controller_wire::compiled_families!(check_definitions);
    }

    #[test]
    fn wii_and_steam_registry_describe_existing_capabilities_and_option_defaults() {
        use crate::{
            steam_controller_2026::SteamController2026Options, wii_remote::WiiRemoteOptions,
        };
        use gr_controller_wire::registry::ids;
        let find = |id| FAMILIES.iter().find(|family| family.id == id).unwrap();
        let wii = find(ids::WII_REMOTE);
        let wii_defaults = WiiRemoteOptions::default();
        let steam = find(ids::STEAM_CONTROLLER_2026);
        let steam_defaults = SteamController2026Options::default();
        for (entry, defaults) in [
            (
                wii,
                &[
                    (
                        "motionplus_passthrough_id_mirror",
                        wii_defaults.motionplus_passthrough_id_mirror(),
                    ),
                    (
                        "motionplus_passthrough_calibration_copy",
                        wii_defaults.motionplus_passthrough_calibration_copy(),
                    ),
                    (
                        "motionplus_activation_reattach_status",
                        wii_defaults.motionplus_activation_reattach_status(),
                    ),
                    (
                        "motionplus_inactive_id_external_style",
                        wii_defaults.motionplus_inactive_id_external_style(),
                    ),
                ][..],
            ),
            (
                steam,
                &[
                    ("steam_client_compat", steam_defaults.steam_client_compat()),
                    ("lizard_mode", steam_defaults.lizard_mode()),
                    ("haptic_streaming", steam_defaults.haptic_streaming()),
                ][..],
            ),
        ] {
            assert_eq!(
                entry
                    .creation_toggles
                    .iter()
                    .map(|option| (option.name, option.default))
                    .collect::<Vec<_>>(),
                defaults
            );
        }
        assert_eq!(wii.persistent_resources.len(), 1);
        assert_eq!(wii.persistent_resources[0].name, "eeprom");
        assert_eq!(
            wii.persistent_resources[0].targets,
            &[RealizationTarget::LINUX_UHID_BLUETOOTH]
        );
        assert!(
            wii.refused_operations
                .iter()
                .any(|operation| operation.name == "speaker_control"
                    && operation.targets == [RealizationTarget::LINUX_UINPUT])
        );
        assert!(steam.persistent_resources.is_empty());
        for name in ["haptic_streaming", "lizard_mode"] {
            assert!(
                steam
                    .subsystems
                    .iter()
                    .any(|operation| operation.name == name
                        && operation.targets == [RealizationTarget::LINUX_UHID_USB])
            );
            assert!(
                steam
                    .refused_operations
                    .iter()
                    .any(|operation| operation.name == name
                        && operation.targets == [RealizationTarget::LINUX_UINPUT])
            );
        }
    }

    #[test]
    fn older_family_options_only_restore_an_explicit_typed_identity() {
        use crate::{
            DualSenseIdentity, DualSenseOptions, DualShock4Identity, DualShock4Options,
            SwitchProIdentity, SwitchProOptions,
        };
        let address = crate::common::test_address(7);
        macro_rules! roundtrip {
            ($identity:ident, $options:ident) => {
                let identity = $identity::from_bytes(address).unwrap();
                let defaults = $options::default();
                assert_eq!(defaults.identity(), None);
                let restored = defaults.with_identity(identity);
                assert_eq!(restored.identity().unwrap().bytes(), address);
                assert_eq!(defaults.identity(), None);
                let mut multicast = address;
                multicast[0] |= 1;
                assert!($identity::from_bytes(multicast).is_err());
            };
        }
        roundtrip!(DualSenseIdentity, DualSenseOptions);
        roundtrip!(DualShock4Identity, DualShock4Options);
        roundtrip!(SwitchProIdentity, SwitchProOptions);
    }

    #[test]
    fn older_family_metadata_covers_each_surface_restriction_and_existing_audio() {
        use gr_controller_wire::registry::ids;
        for (id, audio) in [
            (ids::DUALSENSE, true),
            (ids::DUALSHOCK4, true),
            (ids::SWITCH_PRO, false),
            (ids::XBOX360, true),
        ] {
            let family = FAMILIES.iter().find(|family| family.id == id).unwrap();
            let expected: Vec<_> = (family.realizations)()
                .iter()
                .flat_map(|realization| {
                    realization
                        .surface
                        .restrictions()
                        .iter()
                        .map(|restriction| (realization.surface.target(), restriction.reason))
                })
                .collect();
            let registered: Vec<_> = family
                .refused_operations
                .iter()
                .flat_map(|operation| {
                    operation
                        .targets
                        .iter()
                        .map(|&target| (target, operation.detail))
                })
                .collect();
            assert_eq!(registered.len(), expected.len());
            for entry in expected {
                assert_eq!(
                    registered
                        .iter()
                        .filter(|&&registered| registered == entry)
                        .count(),
                    1
                );
            }
            assert_eq!(
                family.subsystems,
                if audio { AUDIO_SUBSYSTEMS } else { &[] }
            );
            assert!(family.creation_toggles.is_empty());
            assert!(family.persistent_resources.is_empty());
        }
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
            let manifest = (family.manifest)();
            let is_declared = |target: &RealizationTarget| {
                manifest
                    .entries()
                    .iter()
                    .any(|entry| entry.target == *target)
            };
            for operations in [
                family.subsystems,
                family.persistent_resources,
                family.refused_operations,
            ] {
                let mut names = std::collections::HashSet::new();
                for operation in operations {
                    assert!(names.insert(operation.name));
                    assert!(!operation.detail.is_empty());
                    assert!(
                        !operation.targets.is_empty() && operation.targets.iter().all(is_declared)
                    );
                }
            }
            let mut names = std::collections::HashSet::new();
            for toggle in family.creation_toggles {
                assert!(names.insert(toggle.name));
                assert_eq!(toggle.default, toggle.kind == CreationOptionKind::Compat);
                assert!(!toggle.label.is_empty() && !toggle.fidelity.is_empty());
                assert!(!toggle.targets.is_empty() && toggle.targets.iter().all(is_declared));
            }
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
            creation_toggles: &[],
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
