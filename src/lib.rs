#![forbid(unsafe_code)]

//! Core contracts for future compiled virtual-controller packages.
//!
//! This pre-1.0 migration intentionally exposes no production controller
//! constructors. Controller packages will add typed creation APIs after they
//! implement and validate their own independent realization manifests.

pub use gr_controller_contract::{
    CommitError, ControlError, ControlUpdate, DpadDirection, FaceButton, ManifestError,
    ModeAwareControllerDriver, RealizationControllerDefinition, RealizationManifest,
    RealizationManifestEntry, Stick, StickPosition, Trigger, select_realization,
};
pub use gr_controller_runtime::{FrameSink, ModeControllerRuntime};
pub use gr_realization_api::{
    ControllerId, LinuxTarget, ProviderCapabilities, ProviderRequirements, RealizationError,
    RealizationMode, RealizationModeSet, RealizationSelection, validate_provider,
};

/// The active product has no compiled controller packages during the core
/// migration.
#[must_use]
pub const fn has_curated_controllers() -> bool {
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn core_only_product_does_not_advertise_controller_constructors() {
        assert!(!super::has_curated_controllers());
    }
}
