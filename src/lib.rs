#![forbid(unsafe_code)]

//! Core contracts for future compiled virtual-controller packages.
//!
//! Controller packages add typed creation APIs after they implement and
//! validate their own independent realization manifests.

pub use gr_controller_contract::{
    CommitError, ControlError, ControlUpdate, DpadDirection, FaceButton, ManifestError,
    ModeAwareControllerDriver, PreparedRealization, RealizationControllerDefinition,
    RealizationManifest, RealizationManifestEntry, Stick, StickPosition, Trigger,
    prepare_deployment_realization, prepare_hardware_validation_realization, prepare_realization,
};
pub use gr_controller_runtime::{FrameSink, ModeControllerRuntime};
pub use gr_realization_api::{
    ControllerId, DeploymentTarget, EventReadiness, HardwareValidationTarget, LinuxTarget,
    NativeControllerRealization, NativeProviderFactory, NativeProviderSession,
    NativeRealizationError, ProviderCapabilities, ProviderDiagnostics, ProviderError,
    ProviderFrame, ProviderOpenRequest, ProviderOpenValidationError, ProviderRequirements,
    ProviderReverseEvent, ProviderReverseEventSink, ProviderState, RawReverseEvent,
    RealizationError, RealizationMode, RealizationModeSet, RealizationSelection,
    RealizationSessionId, validate_provider,
};
