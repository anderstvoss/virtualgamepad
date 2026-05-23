#![forbid(unsafe_code)]

//! Built-in profile inventory and capability registry for `virtualgamepad`.

use gr_core::{
    CapabilityCategory, FidelityTier, ProductId, ProfileId, SemanticInputFunction,
    SemanticOutputFunction, VendorId,
};
use serde::Serialize;
use std::sync::LazyLock;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ControllerProfile {
    pub profile_id: ProfileId,
    pub display_name: &'static str,
    pub profile_family: ProfileFamily,
    pub identity: ProfileIdentity,
    pub capabilities: ControllerCapabilities,
    pub supported_fidelity: &'static [FidelityTier],
    pub input_contract: ProfileInputContract,
    pub descriptor_templates: &'static [DescriptorTemplate],
    pub reverse_command_support: ReverseCommandSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[non_exhaustive]
pub enum ProfileFamily {
    #[serde(rename = "generic-gamepad")]
    GenericGamepad,
    #[serde(rename = "xbox360")]
    Xbox360,
    #[serde(rename = "dualsense")]
    DualSense,
    #[serde(rename = "steam-controller")]
    SteamController,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ProfileIdentity {
    pub vendor_id: VendorId,
    pub product_id: ProductId,
    pub version: Option<u16>,
    pub transport_hints: &'static [TransportHint],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ControllerCapabilities {
    pub input: &'static [CapabilityItem],
    pub output: &'static [CapabilityItem],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CapabilityItem {
    pub category: CapabilityCategory,
    pub semantic: SemanticRef,
    pub optionality: Optionality,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<ValueRange>,
}

/// A category-typed reference to either a semantic input function or a
/// semantic output function.
///
/// The serde representation uses external tags (`!input face-bottom`,
/// `!output rumble`) so capability dumps stay unambiguous when the input
/// and output namespaces share a semantic word (for example, `rumble`
/// appears in both namespaces with different meaning). The cost is one
/// extra token per line in YAML output; the alternative — a flat
/// kebab-case scalar — would silently collide between input and output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticRef {
    Input(SemanticInputFunction),
    Output(SemanticOutputFunction),
}

impl SemanticRef {
    #[must_use]
    pub const fn is_input(self) -> bool {
        matches!(self, Self::Input(_))
    }

    #[must_use]
    pub const fn is_output(self) -> bool {
        matches!(self, Self::Output(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Optionality {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProfileInputContract {
    pub required_fields: &'static [InputFieldRef],
    pub optional_fields: &'static [InputFieldRef],
    pub ranges: &'static [InputFieldRange],
    pub delta_support: DeltaSupportRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct InputFieldRef(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct InputFieldRange {
    pub field: InputFieldRef,
    pub range: ValueRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum ValueRange {
    I16 { min: i16, max: i16 },
    U16 { min: u16, max: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeltaSupportRule {
    SparsePartial,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportHint {
    Usb,
    Bluetooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DescriptorTemplate {
    pub fidelity: FidelityTier,
    pub descriptor: DescriptorBytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct DescriptorBytes(pub &'static [u8]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ReverseCommandSupport {
    pub supported: &'static [OutputFunctionRef],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum OutputFunctionRef {
    Semantic(SemanticOutputFunction),
}

/// Query handle for the built-in profile + capability registry.
///
/// v1 ships a closed built-in registry per
/// [RUST_IMPLEMENTATION_SPEC.md profile-extension rule](https://github.com/anderstvoss/virtualgamepad/blob/main/docs/spec/implementation/RUST_IMPLEMENTATION_SPEC.md#gr-profiles).
/// A public `register_external` API is intentionally a v2 concern, so
/// the registry is exposed as a zero-sized handle: query methods
/// (`profiles`, `profile_by_str`, `validate_profile_contract`) can grow
/// without changing call sites, and a future v2 can add registration
/// methods without breaking the v1 read-only surface.
///
/// Built-in profile data lives in [`static@BUILTIN_PROFILES`]; obtain
/// the handle via the [`registry`] constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilityRegistry;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistryError {
    #[error("profile `{profile_id}` is missing required field `{field}`")]
    MissingRequiredField {
        profile_id: String,
        field: &'static str,
    },
    #[error("profile `{profile_id}` has a duplicate {slice} capability `{capability}`")]
    DuplicateCapability {
        profile_id: String,
        slice: &'static str,
        capability: String,
    },
    #[error(
        "profile `{profile_id}` declared an {slice} capability with the wrong semantic kind: `{capability}`"
    )]
    WrongSemanticKind {
        profile_id: String,
        slice: &'static str,
        capability: String,
    },
    #[error(
        "profile `{profile_id}` declared output capability `{capability}` without matching reverse support"
    )]
    OutputCapabilityMissingReverseSupport {
        profile_id: String,
        capability: String,
    },
    #[error(
        "profile `{profile_id}` declared reverse support `{capability}` without matching output capability"
    )]
    ReverseSupportMissingOutputCapability {
        profile_id: String,
        capability: String,
    },
}

impl CapabilityRegistry {
    #[must_use]
    pub fn profiles(self) -> &'static [ControllerProfile] {
        BUILTIN_PROFILES.as_slice()
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn profile(self, profile_id: ProfileId) -> Option<&'static ControllerProfile> {
        self.profile_by_str(profile_id.as_ref())
    }

    #[must_use]
    pub fn profile_by_str(self, profile_id: &str) -> Option<&'static ControllerProfile> {
        self.profiles()
            .iter()
            .find(|profile| profile.profile_id.as_ref() == profile_id)
    }

    /// Validate one profile's registry contract and consistency rules.
    ///
    /// # Errors
    ///
    /// Returns a [`RegistryError`] when required data is missing or the
    /// profile's capability declarations are self-inconsistent.
    pub fn validate_profile_contract(
        self,
        profile: &ControllerProfile,
    ) -> Result<(), RegistryError> {
        validate_required_fields(profile)?;
        validate_capability_slice(profile, "input", profile.capabilities.input, true)?;
        validate_capability_slice(profile, "output", profile.capabilities.output, false)?;
        validate_reverse_support(profile)?;
        Ok(())
    }
}

/// Return the closed v1 [`CapabilityRegistry`] singleton.
///
/// This is the only supported entry point for accessing the built-in
/// profile inventory; consumers should not depend on `CapabilityRegistry`
/// having a public constructor, since v2 may replace the unit-struct
/// facade with a richer handle without breaking this call site.
#[must_use]
pub const fn registry() -> CapabilityRegistry {
    CapabilityRegistry
}

static BUILTIN_PROFILES: LazyLock<[ControllerProfile; 4]> = LazyLock::new(|| {
    [
        generic_gamepad_profile(),
        xbox360_profile(),
        dualsense_profile(),
        steam_controller_profile(),
    ]
});

const EMPTY_HINTS: &[TransportHint] = &[];
const USB_ONLY: &[TransportHint] = &[TransportHint::Usb];
const USB_AND_BLUETOOTH: &[TransportHint] = &[TransportHint::Usb, TransportHint::Bluetooth];
const COMPATIBILITY_ONLY: &[FidelityTier] = &[FidelityTier::Compatibility];
const ALL_FIDELITY: &[FidelityTier] = &[
    FidelityTier::Compatibility,
    FidelityTier::IdentityAware,
    FidelityTier::HardwareFaithful,
];
const NO_OUTPUTS: &[OutputFunctionRef] = &[];
const EMPTY_FIELDS: &[InputFieldRef] = &[];
const EMPTY_DESCRIPTOR: DescriptorBytes = DescriptorBytes(&[]);

const DESCRIPTORS_COMPAT_ONLY: &[DescriptorTemplate] = &[DescriptorTemplate {
    fidelity: FidelityTier::Compatibility,
    descriptor: EMPTY_DESCRIPTOR,
}];
const XBOX360_DESCRIPTOR: DescriptorBytes = DescriptorBytes(&[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x05, 0x01, 0x09, 0x3a, 0xa1, 0x02, 0x75, 0x08, 0x95, 0x02,
    0x05, 0x01, 0x09, 0x3f, 0x09, 0x3b, 0x81, 0x01, 0x75, 0x01, 0x15, 0x00, 0x25, 0x01, 0x35, 0x00,
    0x45, 0x01, 0x95, 0x04, 0x05, 0x09, 0x19, 0x0c, 0x29, 0x0f, 0x81, 0x02, 0x75, 0x01, 0x15, 0x00,
    0x25, 0x01, 0x35, 0x00, 0x45, 0x01, 0x95, 0x04, 0x05, 0x09, 0x09, 0x09, 0x09, 0x0a, 0x09, 0x07,
    0x09, 0x08, 0x81, 0x02, 0x75, 0x01, 0x15, 0x00, 0x25, 0x01, 0x35, 0x00, 0x45, 0x01, 0x95, 0x03,
    0x05, 0x09, 0x09, 0x05, 0x09, 0x06, 0x09, 0x0b, 0x81, 0x02, 0x75, 0x01, 0x95, 0x01, 0x81, 0x01,
    0x75, 0x01, 0x15, 0x00, 0x25, 0x01, 0x35, 0x00, 0x45, 0x01, 0x95, 0x04, 0x05, 0x09, 0x19, 0x01,
    0x29, 0x04, 0x81, 0x02, 0x75, 0x08, 0x15, 0x00, 0x26, 0xff, 0x00, 0x35, 0x00, 0x46, 0xff, 0x00,
    0x95, 0x02, 0x05, 0x01, 0x09, 0x32, 0x09, 0x35, 0x81, 0x02, 0x75, 0x10, 0x16, 0x00, 0x80, 0x26,
    0xff, 0x7f, 0x36, 0x00, 0x80, 0x46, 0xff, 0x7f, 0x05, 0x01, 0x09, 0x01, 0xa1, 0x00, 0x95, 0x02,
    0x05, 0x01, 0x09, 0x30, 0x09, 0x31, 0x81, 0x02, 0xc0, 0x05, 0x01, 0x09, 0x01, 0xa1, 0x00, 0x95,
    0x02, 0x05, 0x01, 0x09, 0x33, 0x09, 0x34, 0x81, 0x02, 0xc0, 0xc0, 0xc0,
]);
const DUALSENSE_USB_DESCRIPTOR: DescriptorBytes = DescriptorBytes(&[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x09, 0x30, 0x09, 0x31, 0x09, 0x32, 0x09, 0x35,
    0x09, 0x33, 0x09, 0x34, 0x15, 0x00, 0x26, 0xff, 0x00, 0x75, 0x08, 0x95, 0x06, 0x81, 0x02, 0x06,
    0x00, 0xff, 0x09, 0x20, 0x95, 0x01, 0x81, 0x02, 0x05, 0x01, 0x09, 0x39, 0x15, 0x00, 0x25, 0x07,
    0x35, 0x00, 0x46, 0x3b, 0x01, 0x65, 0x14, 0x75, 0x04, 0x95, 0x01, 0x81, 0x42, 0x65, 0x00, 0x05,
    0x09, 0x19, 0x01, 0x29, 0x0f, 0x15, 0x00, 0x25, 0x01, 0x75, 0x01, 0x95, 0x0f, 0x81, 0x02, 0x06,
    0x00, 0xff, 0x09, 0x21, 0x95, 0x0d, 0x81, 0x02, 0x06, 0x00, 0xff, 0x09, 0x22, 0x15, 0x00, 0x26,
    0xff, 0x00, 0x75, 0x08, 0x95, 0x34, 0x81, 0x02, 0x85, 0x02, 0x09, 0x23, 0x95, 0x2f, 0x91, 0x02,
    0x85, 0x05, 0x09, 0x33, 0x95, 0x28, 0xb1, 0x02, 0x85, 0x08, 0x09, 0x34, 0x95, 0x2f, 0xb1, 0x02,
    0x85, 0x09, 0x09, 0x24, 0x95, 0x13, 0xb1, 0x02, 0x85, 0x0a, 0x09, 0x25, 0x95, 0x1a, 0xb1, 0x02,
    0x85, 0x20, 0x09, 0x26, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x21, 0x09, 0x27, 0x95, 0x04, 0xb1, 0x02,
    0x85, 0x22, 0x09, 0x40, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x80, 0x09, 0x28, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0x81, 0x09, 0x29, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x82, 0x09, 0x2a, 0x95, 0x09, 0xb1, 0x02,
    0x85, 0x83, 0x09, 0x2b, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0x84, 0x09, 0x2c, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0x85, 0x09, 0x2d, 0x95, 0x02, 0xb1, 0x02, 0x85, 0xa0, 0x09, 0x2e, 0x95, 0x01, 0xb1, 0x02,
    0x85, 0xe0, 0x09, 0x2f, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf0, 0x09, 0x30, 0x95, 0x3f, 0xb1, 0x02,
    0x85, 0xf1, 0x09, 0x31, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf2, 0x09, 0x32, 0x95, 0x0f, 0xb1, 0x02,
    0x85, 0xf4, 0x09, 0x35, 0x95, 0x3f, 0xb1, 0x02, 0x85, 0xf5, 0x09, 0x36, 0x95, 0x03, 0xb1, 0x02,
    0xc0,
]);
const STEAM_CONTROLLER_DESCRIPTOR: DescriptorBytes = DescriptorBytes(&[
    0x05, 0x01, 0x09, 0x05, 0xa1, 0x01, 0x85, 0x01, 0x05, 0x09, 0x19, 0x01, 0x29, 0x0e, 0x15, 0x00,
    0x25, 0x01, 0x75, 0x01, 0x95, 0x0e, 0x81, 0x02, 0x75, 0x02, 0x95, 0x01, 0x81, 0x01, 0x16, 0x00,
    0x80, 0x26, 0xff, 0x7f, 0x36, 0x00, 0x80, 0x46, 0xff, 0x7f, 0x75, 0x10, 0x95, 0x06, 0x05, 0x01,
    0x09, 0x30, 0x09, 0x31, 0x09, 0x32, 0x09, 0x35, 0x09, 0x33, 0x09, 0x34, 0x81, 0x02, 0x15, 0x00,
    0x26, 0xff, 0xff, 0x75, 0x10, 0x95, 0x02, 0x05, 0x02, 0x09, 0xc5, 0x09, 0xc4, 0x81, 0x02, 0x85,
    0x02, 0x06, 0x00, 0xff, 0x09, 0x23, 0x75, 0x08, 0x95, 0x08, 0x91, 0x02, 0xc0,
]);
// Each `*_DESCRIPTORS_ALL` const reuses the same descriptor bytes
// across all three fidelity tiers because a device's HID descriptor
// identifies the family and report structure, not the fidelity tier
// the host has selected. Compatibility-tier sessions go through evdev
// and do not consult these bytes, but having them present keeps the
// per-tier registry shape uniform and lets a future tier (e.g. a
// hardware-faithful transport refinement) reuse the same bytes
// without a separate const.
const XBOX360_DESCRIPTORS_ALL: &[DescriptorTemplate] = &[
    DescriptorTemplate {
        fidelity: FidelityTier::Compatibility,
        descriptor: XBOX360_DESCRIPTOR,
    },
    DescriptorTemplate {
        fidelity: FidelityTier::IdentityAware,
        descriptor: XBOX360_DESCRIPTOR,
    },
    DescriptorTemplate {
        fidelity: FidelityTier::HardwareFaithful,
        descriptor: XBOX360_DESCRIPTOR,
    },
];
const DUALSENSE_DESCRIPTORS_ALL: &[DescriptorTemplate] = &[
    DescriptorTemplate {
        fidelity: FidelityTier::Compatibility,
        descriptor: DUALSENSE_USB_DESCRIPTOR,
    },
    DescriptorTemplate {
        fidelity: FidelityTier::IdentityAware,
        descriptor: DUALSENSE_USB_DESCRIPTOR,
    },
    DescriptorTemplate {
        fidelity: FidelityTier::HardwareFaithful,
        descriptor: DUALSENSE_USB_DESCRIPTOR,
    },
];
const STEAM_CONTROLLER_DESCRIPTORS_ALL: &[DescriptorTemplate] = &[
    DescriptorTemplate {
        fidelity: FidelityTier::Compatibility,
        descriptor: STEAM_CONTROLLER_DESCRIPTOR,
    },
    DescriptorTemplate {
        fidelity: FidelityTier::IdentityAware,
        descriptor: STEAM_CONTROLLER_DESCRIPTOR,
    },
    DescriptorTemplate {
        fidelity: FidelityTier::HardwareFaithful,
        descriptor: STEAM_CONTROLLER_DESCRIPTOR,
    },
];
const RANGE_STICK: ValueRange = ValueRange::I16 {
    min: i16::MIN,
    max: i16::MAX,
};
const RANGE_TRIGGER: ValueRange = ValueRange::U16 {
    min: 0,
    max: u16::MAX,
};
const RANGE_TOUCH_X: ValueRange = ValueRange::U16 { min: 0, max: 1920 };
const RANGE_TOUCH_Y: ValueRange = ValueRange::U16 { min: 0, max: 1080 };

const GENERIC_GAMEPAD_INPUT_CAPABILITIES: &[CapabilityItem] = &[
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceBottom,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceRight,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceLeft,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceTop,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::LeftShoulder,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::RightShoulder,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::LeftStickButton,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::RightStickButton,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::MenuPrimary,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::MenuSecondary,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::Guide,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::Dpad,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Stick,
        SemanticInputFunction::LeftStick,
        Optionality::Required,
        Some(RANGE_STICK),
    ),
    cap_input(
        CapabilityCategory::Stick,
        SemanticInputFunction::RightStick,
        Optionality::Required,
        Some(RANGE_STICK),
    ),
    cap_input(
        CapabilityCategory::Trigger,
        SemanticInputFunction::LeftTrigger,
        Optionality::Required,
        Some(RANGE_TRIGGER),
    ),
    cap_input(
        CapabilityCategory::Trigger,
        SemanticInputFunction::RightTrigger,
        Optionality::Required,
        Some(RANGE_TRIGGER),
    ),
];

const XBOX360_INPUT_CAPABILITIES: &[CapabilityItem] = GENERIC_GAMEPAD_INPUT_CAPABILITIES;
const XBOX360_OUTPUT_CAPABILITIES: &[CapabilityItem] = &[
    cap_output(
        CapabilityCategory::Haptic,
        SemanticOutputFunction::Rumble,
        Optionality::Required,
    ),
    cap_output(
        CapabilityCategory::Lighting,
        SemanticOutputFunction::Lighting,
        Optionality::Required,
    ),
    cap_output(
        CapabilityCategory::Lighting,
        SemanticOutputFunction::PlayerIndicators,
        Optionality::Required,
    ),
];
const XBOX360_REVERSE_SUPPORT: &[OutputFunctionRef] = &[
    OutputFunctionRef::Semantic(SemanticOutputFunction::Rumble),
    OutputFunctionRef::Semantic(SemanticOutputFunction::Lighting),
    OutputFunctionRef::Semantic(SemanticOutputFunction::PlayerIndicators),
];

const DUALSENSE_INPUT_CAPABILITIES: &[CapabilityItem] = &[
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceBottom,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceRight,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceLeft,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceTop,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::LeftShoulder,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::RightShoulder,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::LeftStickButton,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::RightStickButton,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::MenuPrimary,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::MenuSecondary,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::Guide,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::TouchClick,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::Dpad,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Stick,
        SemanticInputFunction::LeftStick,
        Optionality::Required,
        Some(RANGE_STICK),
    ),
    cap_input(
        CapabilityCategory::Stick,
        SemanticInputFunction::RightStick,
        Optionality::Required,
        Some(RANGE_STICK),
    ),
    cap_input(
        CapabilityCategory::Trigger,
        SemanticInputFunction::LeftTrigger,
        Optionality::Required,
        Some(RANGE_TRIGGER),
    ),
    cap_input(
        CapabilityCategory::Trigger,
        SemanticInputFunction::RightTrigger,
        Optionality::Required,
        Some(RANGE_TRIGGER),
    ),
    cap_input(
        CapabilityCategory::TouchSurface,
        SemanticInputFunction::TouchSurface,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::MotionSensor,
        SemanticInputFunction::Accelerometer,
        Optionality::Optional,
        None,
    ),
    cap_input(
        CapabilityCategory::MotionSensor,
        SemanticInputFunction::Gyroscope,
        Optionality::Optional,
        None,
    ),
];

const DUALSENSE_OUTPUT_CAPABILITIES: &[CapabilityItem] = &[
    cap_output(
        CapabilityCategory::Haptic,
        SemanticOutputFunction::Rumble,
        Optionality::Required,
    ),
    cap_output(
        CapabilityCategory::Haptic,
        SemanticOutputFunction::Haptics,
        Optionality::Optional,
    ),
    cap_output(
        CapabilityCategory::Lighting,
        SemanticOutputFunction::Lighting,
        Optionality::Required,
    ),
    cap_output(
        CapabilityCategory::Lighting,
        SemanticOutputFunction::PlayerIndicators,
        Optionality::Required,
    ),
    cap_output(
        CapabilityCategory::Trigger,
        SemanticOutputFunction::TriggerEffect,
        Optionality::Required,
    ),
    cap_output(
        CapabilityCategory::Audio,
        SemanticOutputFunction::Audio,
        Optionality::Optional,
    ),
];

const DUALSENSE_REVERSE_SUPPORT: &[OutputFunctionRef] = &[
    OutputFunctionRef::Semantic(SemanticOutputFunction::Rumble),
    OutputFunctionRef::Semantic(SemanticOutputFunction::Haptics),
    OutputFunctionRef::Semantic(SemanticOutputFunction::Lighting),
    OutputFunctionRef::Semantic(SemanticOutputFunction::PlayerIndicators),
    OutputFunctionRef::Semantic(SemanticOutputFunction::TriggerEffect),
    OutputFunctionRef::Semantic(SemanticOutputFunction::Audio),
];

const STEAM_CONTROLLER_INPUT_CAPABILITIES: &[CapabilityItem] = &[
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceBottom,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceRight,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceLeft,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::FaceTop,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::LeftShoulder,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::RightShoulder,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::LeftStickButton,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::PaddleLeft,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Button,
        SemanticInputFunction::PaddleRight,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::MenuPrimary,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::MenuSecondary,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::System,
        SemanticInputFunction::Guide,
        Optionality::Required,
        None,
    ),
    cap_input(
        CapabilityCategory::Stick,
        SemanticInputFunction::LeftStick,
        Optionality::Required,
        Some(RANGE_STICK),
    ),
    cap_input(
        CapabilityCategory::TouchSurface,
        SemanticInputFunction::TouchSurface,
        Optionality::Required,
        Some(RANGE_STICK),
    ),
    cap_input(
        CapabilityCategory::Trigger,
        SemanticInputFunction::LeftTrigger,
        Optionality::Required,
        Some(RANGE_TRIGGER),
    ),
    cap_input(
        CapabilityCategory::Trigger,
        SemanticInputFunction::RightTrigger,
        Optionality::Required,
        Some(RANGE_TRIGGER),
    ),
];

const STEAM_CONTROLLER_OUTPUT_CAPABILITIES: &[CapabilityItem] = &[
    cap_output(
        CapabilityCategory::Haptic,
        SemanticOutputFunction::Rumble,
        Optionality::Required,
    ),
    cap_output(
        CapabilityCategory::Lighting,
        SemanticOutputFunction::Lighting,
        Optionality::Optional,
    ),
];

const STEAM_CONTROLLER_REVERSE_SUPPORT: &[OutputFunctionRef] = &[
    OutputFunctionRef::Semantic(SemanticOutputFunction::Rumble),
    OutputFunctionRef::Semantic(SemanticOutputFunction::Lighting),
];

const GENERIC_GAMEPAD_REQUIRED_FIELDS: &[InputFieldRef] = &[
    InputFieldRef("buttons.south"),
    InputFieldRef("buttons.east"),
    InputFieldRef("buttons.west"),
    InputFieldRef("buttons.north"),
    InputFieldRef("buttons.left_shoulder"),
    InputFieldRef("buttons.right_shoulder"),
    InputFieldRef("buttons.left_stick_button"),
    InputFieldRef("buttons.right_stick_button"),
    InputFieldRef("buttons.menu_primary"),
    InputFieldRef("buttons.menu_secondary"),
    InputFieldRef("buttons.guide"),
    InputFieldRef("dpad.up"),
    InputFieldRef("dpad.down"),
    InputFieldRef("dpad.left"),
    InputFieldRef("dpad.right"),
    InputFieldRef("sticks.left_x"),
    InputFieldRef("sticks.left_y"),
    InputFieldRef("sticks.right_x"),
    InputFieldRef("sticks.right_y"),
    InputFieldRef("triggers.left_trigger"),
    InputFieldRef("triggers.right_trigger"),
];

const GENERIC_GAMEPAD_RANGES: &[InputFieldRange] = &[
    InputFieldRange {
        field: InputFieldRef("sticks.left_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.left_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.left_trigger"),
        range: RANGE_TRIGGER,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.right_trigger"),
        range: RANGE_TRIGGER,
    },
];

const XBOX360_REQUIRED_FIELDS: &[InputFieldRef] = &[
    InputFieldRef("buttons.face.a"),
    InputFieldRef("buttons.face.b"),
    InputFieldRef("buttons.face.x"),
    InputFieldRef("buttons.face.y"),
    InputFieldRef("buttons.shoulders.lb"),
    InputFieldRef("buttons.shoulders.rb"),
    InputFieldRef("buttons.stick_clicks.ls"),
    InputFieldRef("buttons.stick_clicks.rs"),
    InputFieldRef("buttons.system.start"),
    InputFieldRef("buttons.system.back"),
    InputFieldRef("buttons.system.guide"),
    InputFieldRef("dpad.up"),
    InputFieldRef("dpad.down"),
    InputFieldRef("dpad.left"),
    InputFieldRef("dpad.right"),
    InputFieldRef("sticks.left_x"),
    InputFieldRef("sticks.left_y"),
    InputFieldRef("sticks.right_x"),
    InputFieldRef("sticks.right_y"),
    InputFieldRef("triggers.lt"),
    InputFieldRef("triggers.rt"),
];

const XBOX360_RANGES: &[InputFieldRange] = &[
    InputFieldRange {
        field: InputFieldRef("sticks.left_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.left_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.lt"),
        range: RANGE_TRIGGER,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.rt"),
        range: RANGE_TRIGGER,
    },
];

const DUALSENSE_REQUIRED_FIELDS: &[InputFieldRef] = &[
    InputFieldRef("buttons.face.cross"),
    InputFieldRef("buttons.face.circle"),
    InputFieldRef("buttons.face.square"),
    InputFieldRef("buttons.face.triangle"),
    InputFieldRef("buttons.shoulders.l1"),
    InputFieldRef("buttons.shoulders.r1"),
    InputFieldRef("buttons.stick_clicks.l3"),
    InputFieldRef("buttons.stick_clicks.r3"),
    InputFieldRef("buttons.system.create"),
    InputFieldRef("buttons.system.options"),
    InputFieldRef("buttons.system.ps"),
    InputFieldRef("buttons.system.touchpad_click"),
    InputFieldRef("dpad.up"),
    InputFieldRef("dpad.down"),
    InputFieldRef("dpad.left"),
    InputFieldRef("dpad.right"),
    InputFieldRef("sticks.left_x"),
    InputFieldRef("sticks.left_y"),
    InputFieldRef("sticks.right_x"),
    InputFieldRef("sticks.right_y"),
    InputFieldRef("triggers.l2"),
    InputFieldRef("triggers.r2"),
    InputFieldRef("touchpad.contact_1.active"),
    InputFieldRef("touchpad.contact_1.x"),
    InputFieldRef("touchpad.contact_1.y"),
    InputFieldRef("touchpad.contact_2.active"),
    InputFieldRef("touchpad.contact_2.x"),
    InputFieldRef("touchpad.contact_2.y"),
];

const DUALSENSE_RANGES: &[InputFieldRange] = &[
    InputFieldRange {
        field: InputFieldRef("sticks.left_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.left_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.l2"),
        range: RANGE_TRIGGER,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.r2"),
        range: RANGE_TRIGGER,
    },
    InputFieldRange {
        field: InputFieldRef("touchpad.contact_1.x"),
        range: RANGE_TOUCH_X,
    },
    InputFieldRange {
        field: InputFieldRef("touchpad.contact_1.y"),
        range: RANGE_TOUCH_Y,
    },
    InputFieldRange {
        field: InputFieldRef("touchpad.contact_2.x"),
        range: RANGE_TOUCH_X,
    },
    InputFieldRange {
        field: InputFieldRef("touchpad.contact_2.y"),
        range: RANGE_TOUCH_Y,
    },
];

const STEAM_CONTROLLER_REQUIRED_FIELDS: &[InputFieldRef] = &[
    InputFieldRef("buttons.a"),
    InputFieldRef("buttons.b"),
    InputFieldRef("buttons.x"),
    InputFieldRef("buttons.y"),
    InputFieldRef("buttons.left_grip"),
    InputFieldRef("buttons.right_grip"),
    InputFieldRef("buttons.lb"),
    InputFieldRef("buttons.rb"),
    InputFieldRef("buttons.menu_primary"),
    InputFieldRef("buttons.menu_secondary"),
    InputFieldRef("buttons.steam"),
    InputFieldRef("buttons.left_pad_click"),
    InputFieldRef("buttons.right_pad_click"),
    InputFieldRef("buttons.left_stick_click"),
    InputFieldRef("sticks.left_pad_x"),
    InputFieldRef("sticks.left_pad_y"),
    InputFieldRef("sticks.right_pad_x"),
    InputFieldRef("sticks.right_pad_y"),
    InputFieldRef("sticks.left_stick_x"),
    InputFieldRef("sticks.left_stick_y"),
    InputFieldRef("triggers.lt"),
    InputFieldRef("triggers.rt"),
];

const STEAM_CONTROLLER_RANGES: &[InputFieldRange] = &[
    InputFieldRange {
        field: InputFieldRef("sticks.left_pad_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.left_pad_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_pad_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.right_pad_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.left_stick_x"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("sticks.left_stick_y"),
        range: RANGE_STICK,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.lt"),
        range: RANGE_TRIGGER,
    },
    InputFieldRange {
        field: InputFieldRef("triggers.rt"),
        range: RANGE_TRIGGER,
    },
];

fn generic_gamepad_profile() -> ControllerProfile {
    ControllerProfile {
        profile_id: ProfileId::from("generic-gamepad"),
        display_name: "Generic gamepad",
        profile_family: ProfileFamily::GenericGamepad,
        identity: ProfileIdentity {
            vendor_id: VendorId::new(1),
            product_id: ProductId::new(1),
            version: None,
            transport_hints: EMPTY_HINTS,
        },
        capabilities: ControllerCapabilities {
            input: GENERIC_GAMEPAD_INPUT_CAPABILITIES,
            output: &[],
        },
        supported_fidelity: COMPATIBILITY_ONLY,
        input_contract: ProfileInputContract {
            required_fields: GENERIC_GAMEPAD_REQUIRED_FIELDS,
            optional_fields: EMPTY_FIELDS,
            ranges: GENERIC_GAMEPAD_RANGES,
            delta_support: DeltaSupportRule::SparsePartial,
        },
        descriptor_templates: DESCRIPTORS_COMPAT_ONLY,
        reverse_command_support: ReverseCommandSupport {
            supported: NO_OUTPUTS,
        },
    }
}

fn xbox360_profile() -> ControllerProfile {
    ControllerProfile {
        profile_id: ProfileId::from("xbox360"),
        display_name: "Xbox 360",
        profile_family: ProfileFamily::Xbox360,
        identity: ProfileIdentity {
            vendor_id: VendorId::new(0x045e),
            product_id: ProductId::new(0x028e),
            version: None,
            transport_hints: USB_ONLY,
        },
        capabilities: ControllerCapabilities {
            input: XBOX360_INPUT_CAPABILITIES,
            output: XBOX360_OUTPUT_CAPABILITIES,
        },
        supported_fidelity: ALL_FIDELITY,
        input_contract: ProfileInputContract {
            required_fields: XBOX360_REQUIRED_FIELDS,
            optional_fields: EMPTY_FIELDS,
            ranges: XBOX360_RANGES,
            delta_support: DeltaSupportRule::SparsePartial,
        },
        descriptor_templates: XBOX360_DESCRIPTORS_ALL,
        reverse_command_support: ReverseCommandSupport {
            supported: XBOX360_REVERSE_SUPPORT,
        },
    }
}

fn dualsense_profile() -> ControllerProfile {
    ControllerProfile {
        profile_id: ProfileId::from("dualsense"),
        display_name: "DualSense",
        profile_family: ProfileFamily::DualSense,
        identity: ProfileIdentity {
            vendor_id: VendorId::new(0x054c),
            product_id: ProductId::new(0x0ce6),
            version: Some(0x0100),
            transport_hints: USB_AND_BLUETOOTH,
        },
        capabilities: ControllerCapabilities {
            input: DUALSENSE_INPUT_CAPABILITIES,
            output: DUALSENSE_OUTPUT_CAPABILITIES,
        },
        supported_fidelity: ALL_FIDELITY,
        input_contract: ProfileInputContract {
            required_fields: DUALSENSE_REQUIRED_FIELDS,
            optional_fields: EMPTY_FIELDS,
            ranges: DUALSENSE_RANGES,
            delta_support: DeltaSupportRule::SparsePartial,
        },
        descriptor_templates: DUALSENSE_DESCRIPTORS_ALL,
        reverse_command_support: ReverseCommandSupport {
            supported: DUALSENSE_REVERSE_SUPPORT,
        },
    }
}

fn steam_controller_profile() -> ControllerProfile {
    ControllerProfile {
        profile_id: ProfileId::from("steam-controller"),
        display_name: "Steam Controller",
        profile_family: ProfileFamily::SteamController,
        identity: ProfileIdentity {
            vendor_id: VendorId::new(0x28de),
            product_id: ProductId::new(0x1102),
            version: None,
            transport_hints: USB_AND_BLUETOOTH,
        },
        capabilities: ControllerCapabilities {
            input: STEAM_CONTROLLER_INPUT_CAPABILITIES,
            output: STEAM_CONTROLLER_OUTPUT_CAPABILITIES,
        },
        supported_fidelity: ALL_FIDELITY,
        input_contract: ProfileInputContract {
            required_fields: STEAM_CONTROLLER_REQUIRED_FIELDS,
            optional_fields: EMPTY_FIELDS,
            ranges: STEAM_CONTROLLER_RANGES,
            delta_support: DeltaSupportRule::SparsePartial,
        },
        descriptor_templates: STEAM_CONTROLLER_DESCRIPTORS_ALL,
        reverse_command_support: ReverseCommandSupport {
            supported: STEAM_CONTROLLER_REVERSE_SUPPORT,
        },
    }
}

const fn cap_input(
    category: CapabilityCategory,
    semantic: SemanticInputFunction,
    optionality: Optionality,
    range: Option<ValueRange>,
) -> CapabilityItem {
    CapabilityItem {
        category,
        semantic: SemanticRef::Input(semantic),
        optionality,
        range,
    }
}

const fn cap_output(
    category: CapabilityCategory,
    semantic: SemanticOutputFunction,
    optionality: Optionality,
) -> CapabilityItem {
    CapabilityItem {
        category,
        semantic: SemanticRef::Output(semantic),
        optionality,
        range: None,
    }
}

fn validate_required_fields(profile: &ControllerProfile) -> Result<(), RegistryError> {
    let profile_id = profile.profile_id.to_string();
    if profile.profile_id.as_ref().is_empty() {
        return Err(RegistryError::MissingRequiredField {
            profile_id,
            field: "profile_id",
        });
    }
    if profile.display_name.is_empty() {
        return Err(RegistryError::MissingRequiredField {
            profile_id,
            field: "display_name",
        });
    }
    if profile.supported_fidelity.is_empty() {
        return Err(RegistryError::MissingRequiredField {
            profile_id,
            field: "supported_fidelity",
        });
    }
    if profile.input_contract.required_fields.is_empty() {
        return Err(RegistryError::MissingRequiredField {
            profile_id,
            field: "input_contract.required_fields",
        });
    }
    if profile.capabilities.input.is_empty() {
        return Err(RegistryError::MissingRequiredField {
            profile_id,
            field: "capabilities.input",
        });
    }
    if profile.identity.vendor_id.get() == 0 {
        return Err(RegistryError::MissingRequiredField {
            profile_id,
            field: "identity.vendor_id",
        });
    }
    if profile.identity.product_id.get() == 0 {
        return Err(RegistryError::MissingRequiredField {
            profile_id,
            field: "identity.product_id",
        });
    }
    Ok(())
}

fn validate_capability_slice(
    profile: &ControllerProfile,
    slice: &'static str,
    capabilities: &[CapabilityItem],
    expect_input: bool,
) -> Result<(), RegistryError> {
    for (index, capability) in capabilities.iter().enumerate() {
        let semantic_ok = if expect_input {
            capability.semantic.is_input()
        } else {
            capability.semantic.is_output()
        };
        if !semantic_ok {
            return Err(RegistryError::WrongSemanticKind {
                profile_id: profile.profile_id.to_string(),
                slice,
                capability: capability_key(*capability),
            });
        }

        let duplicate = capabilities.iter().take(index).any(|existing| {
            existing.category == capability.category && existing.semantic == capability.semantic
        });
        if duplicate {
            return Err(RegistryError::DuplicateCapability {
                profile_id: profile.profile_id.to_string(),
                slice,
                capability: capability_key(*capability),
            });
        }
    }
    Ok(())
}

fn validate_reverse_support(profile: &ControllerProfile) -> Result<(), RegistryError> {
    for capability in profile.capabilities.output {
        let SemanticRef::Output(output) = capability.semantic else {
            continue;
        };
        let supported = profile
            .reverse_command_support
            .supported
            .contains(&OutputFunctionRef::Semantic(output));
        if !supported {
            return Err(RegistryError::OutputCapabilityMissingReverseSupport {
                profile_id: profile.profile_id.to_string(),
                capability: output.to_string(),
            });
        }
    }

    for function in profile.reverse_command_support.supported {
        let OutputFunctionRef::Semantic(output) = function;
        let declared = profile
            .capabilities
            .output
            .iter()
            .any(|capability| capability.semantic == SemanticRef::Output(*output));
        if !declared {
            return Err(RegistryError::ReverseSupportMissingOutputCapability {
                profile_id: profile.profile_id.to_string(),
                capability: output.to_string(),
            });
        }
    }

    Ok(())
}

fn capability_key(capability: CapabilityItem) -> String {
    match capability.semantic {
        SemanticRef::Input(input) => format!("{}:{}", capability.category, input),
        SemanticRef::Output(output) => format!("{}:{}", capability.category, output),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use rstest::rstest;

    #[test]
    fn smoke() {}

    #[test]
    fn built_in_registry_order_is_stable() {
        let ids = registry()
            .profiles()
            .iter()
            .map(|profile| profile.profile_id.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "generic-gamepad",
                "xbox360",
                "dualsense",
                "steam-controller"
            ]
        );
    }

    #[test]
    fn built_in_ids_and_display_names_match_spec() {
        let profiles = registry().profiles();
        assert_eq!(profiles[0].display_name, "Generic gamepad");
        assert_eq!(profiles[1].display_name, "Xbox 360");
        assert_eq!(profiles[2].display_name, "DualSense");
        assert_eq!(profiles[3].display_name, "Steam Controller");
    }

    #[test]
    fn profile_family_serialization_uses_spec_names() {
        assert_eq!(
            serde_yaml::to_string(&ProfileFamily::GenericGamepad).expect("yaml"),
            "generic-gamepad\n"
        );
        assert_eq!(
            serde_yaml::to_string(&ProfileFamily::Xbox360).expect("yaml"),
            "xbox360\n"
        );
        assert_eq!(
            serde_yaml::to_string(&ProfileFamily::DualSense).expect("yaml"),
            "dualsense\n"
        );
        assert_eq!(
            serde_yaml::to_string(&ProfileFamily::SteamController).expect("yaml"),
            "steam-controller\n"
        );
    }

    #[test]
    fn lookup_works_by_profile_id_and_str() {
        let registry = registry();
        let by_str = registry
            .profile_by_str("dualsense")
            .expect("dualsense profile");
        let by_id = registry
            .profile(ProfileId::from("dualsense"))
            .expect("dualsense profile");
        assert_eq!(by_str.profile_id, by_id.profile_id);
    }

    #[test]
    fn duplicate_profile_ids_are_impossible_in_builtins() {
        let mut ids = registry()
            .profiles()
            .iter()
            .map(|profile| profile.profile_id.as_ref())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), registry().profiles().len());
    }

    #[test]
    fn input_capabilities_use_only_input_semantics() {
        for profile in registry().profiles() {
            assert!(
                profile
                    .capabilities
                    .input
                    .iter()
                    .all(|capability| capability.semantic.is_input())
            );
        }
    }

    #[test]
    fn output_capabilities_use_only_output_semantics() {
        for profile in registry().profiles() {
            assert!(
                profile
                    .capabilities
                    .output
                    .iter()
                    .all(|capability| capability.semantic.is_output())
            );
        }
    }

    #[test]
    fn every_declared_output_has_matching_reverse_support() {
        for profile in registry().profiles() {
            registry()
                .validate_profile_contract(profile)
                .expect("built-in profile validates");
        }
    }

    #[test]
    fn xbox360_declares_led_outputs() {
        let profile = registry()
            .profile_by_str("xbox360")
            .expect("xbox360 profile exists");

        assert!(
            profile.capabilities.output.iter().any(|capability| {
                capability.semantic == SemanticRef::Output(SemanticOutputFunction::Lighting)
            }),
            "xbox360 should declare lighting output"
        );
        assert!(
            profile.capabilities.output.iter().any(|capability| {
                capability.semantic == SemanticRef::Output(SemanticOutputFunction::PlayerIndicators)
            }),
            "xbox360 should declare player indicator output"
        );
    }

    #[test]
    fn non_dualsense_profiles_do_not_declare_dualsense_specific_outputs() {
        let forbidden = [
            SemanticOutputFunction::TriggerEffect,
            SemanticOutputFunction::Audio,
            SemanticOutputFunction::Haptics,
        ];
        for profile in registry().profiles() {
            if profile.profile_id.as_ref() == "dualsense" {
                continue;
            }
            for capability in profile.capabilities.output {
                let SemanticRef::Output(output) = capability.semantic else {
                    continue;
                };
                assert!(
                    !forbidden.contains(&output),
                    "{} declared dualsense-specific output `{output}`",
                    profile.profile_id
                );
            }
        }
    }

    #[test]
    fn every_built_in_profile_has_analog_ranges() {
        for profile in registry().profiles() {
            assert!(
                !profile.input_contract.ranges.is_empty(),
                "{} should declare analog ranges",
                profile.profile_id
            );
        }
    }

    #[test]
    fn identity_aware_profiles_have_non_empty_descriptors() {
        for profile_id in ["xbox360", "dualsense", "steam-controller"] {
            let profile = registry()
                .profile_by_str(profile_id)
                .expect("profile exists");
            let descriptor = profile
                .descriptor_templates
                .iter()
                .find(|template| template.fidelity == FidelityTier::IdentityAware)
                .expect("identity-aware descriptor");
            assert!(
                !descriptor.descriptor.0.is_empty(),
                "{profile_id} identity-aware descriptor should be non-empty"
            );
        }
    }

    #[rstest]
    #[case::display_name("display_name", |p: &mut ControllerProfile| p.display_name = "")]
    #[case::supported_fidelity(
        "supported_fidelity",
        |p: &mut ControllerProfile| p.supported_fidelity = &[]
    )]
    #[case::input_contract_required_fields(
        "input_contract.required_fields",
        |p: &mut ControllerProfile| p.input_contract.required_fields = &[]
    )]
    #[case::capabilities_input(
        "capabilities.input",
        |p: &mut ControllerProfile| p.capabilities.input = &[]
    )]
    #[case::identity_vendor_id(
        "identity.vendor_id",
        |p: &mut ControllerProfile| p.identity.vendor_id = VendorId::new(0)
    )]
    #[case::identity_product_id(
        "identity.product_id",
        |p: &mut ControllerProfile| p.identity.product_id = ProductId::new(0)
    )]
    fn invalid_profiles_fail_with_field_specific_errors(
        #[case] expected_field: &'static str,
        #[case] mutate: fn(&mut ControllerProfile),
    ) {
        let mut profile = generic_gamepad_profile();
        profile.profile_id = ProfileId::from("invalid-test");
        mutate(&mut profile);

        let error = registry()
            .validate_profile_contract(&profile)
            .expect_err("invalid profile should fail");

        match error {
            RegistryError::MissingRequiredField { field, .. } => {
                assert_eq!(field, expected_field);
            }
            other => panic!("expected MissingRequiredField, got {other:?}"),
        }
    }

    #[test]
    fn empty_profile_id_fails_validation() {
        let mut profile = generic_gamepad_profile();
        profile.profile_id = ProfileId::from("");

        let error = registry()
            .validate_profile_contract(&profile)
            .expect_err("empty profile_id should fail");
        assert!(matches!(
            error,
            RegistryError::MissingRequiredField {
                field: "profile_id",
                ..
            }
        ));
    }

    #[test]
    fn validator_catches_duplicate_capability() {
        let dup = CapabilityItem {
            category: CapabilityCategory::Button,
            semantic: SemanticRef::Input(SemanticInputFunction::FaceBottom),
            optionality: Optionality::Required,
            range: None,
        };
        let mut profile = generic_gamepad_profile();
        profile.profile_id = ProfileId::from("dup-test");
        profile.capabilities = ControllerCapabilities {
            input: Box::leak(Box::new([dup, dup])),
            output: &[],
        };
        let error = registry()
            .validate_profile_contract(&profile)
            .expect_err("duplicate input capability");
        assert!(matches!(
            error,
            RegistryError::DuplicateCapability { slice: "input", .. }
        ));
    }

    #[test]
    fn validator_catches_wrong_semantic_kind_in_input_slice() {
        let bad = CapabilityItem {
            category: CapabilityCategory::Haptic,
            semantic: SemanticRef::Output(SemanticOutputFunction::Rumble),
            optionality: Optionality::Required,
            range: None,
        };
        let mut profile = generic_gamepad_profile();
        profile.profile_id = ProfileId::from("wrong-kind-test");
        profile.capabilities = ControllerCapabilities {
            input: Box::leak(Box::new([bad])),
            output: &[],
        };
        let error = registry()
            .validate_profile_contract(&profile)
            .expect_err("wrong semantic kind in input slice");
        assert!(matches!(
            error,
            RegistryError::WrongSemanticKind { slice: "input", .. }
        ));
    }

    #[test]
    fn validator_catches_output_capability_without_reverse_support() {
        let output = CapabilityItem {
            category: CapabilityCategory::Haptic,
            semantic: SemanticRef::Output(SemanticOutputFunction::Rumble),
            optionality: Optionality::Required,
            range: None,
        };
        let mut profile = generic_gamepad_profile();
        profile.profile_id = ProfileId::from("missing-reverse-test");
        profile.capabilities = ControllerCapabilities {
            input: profile.capabilities.input,
            output: Box::leak(Box::new([output])),
        };
        let error = registry()
            .validate_profile_contract(&profile)
            .expect_err("output without reverse support");
        assert!(matches!(
            error,
            RegistryError::OutputCapabilityMissingReverseSupport { .. }
        ));
    }

    #[test]
    fn validator_catches_reverse_support_without_output_capability() {
        let mut profile = generic_gamepad_profile();
        profile.profile_id = ProfileId::from("orphan-reverse-test");
        profile.reverse_command_support = ReverseCommandSupport {
            supported: Box::leak(Box::new([OutputFunctionRef::Semantic(
                SemanticOutputFunction::Rumble,
            )])),
        };
        let error = registry()
            .validate_profile_contract(&profile)
            .expect_err("orphan reverse-support entry");
        assert!(matches!(
            error,
            RegistryError::ReverseSupportMissingOutputCapability { .. }
        ));
    }

    #[test]
    fn registry_snapshots_are_human_readable() {
        assert_snapshot!(
            "built-in-profile-ids",
            serde_yaml::to_string(
                &registry()
                    .profiles()
                    .iter()
                    .map(|profile| profile.profile_id.as_ref())
                    .collect::<Vec<_>>()
            )
            .expect("yaml")
        );

        for profile in registry().profiles() {
            assert_snapshot!(
                format!("profile-{}", profile.profile_id),
                serde_yaml::to_string(profile).expect("yaml")
            );
        }
    }
}
