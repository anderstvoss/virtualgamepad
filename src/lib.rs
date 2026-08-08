#![forbid(unsafe_code)]

//! Core contracts for future compiled virtual-controller packages.
//!
//! Controller packages add typed creation APIs after they implement and
//! validate their own independent realization manifests.

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
