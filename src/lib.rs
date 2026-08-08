#![forbid(unsafe_code)]

//! Core contracts for future compiled virtual-controller packages.
//!
//! Controller packages add typed creation APIs after they implement and
//! validate their own independent realization manifests.

pub use gr_audio_contract::{
    AudioBackendFactory, AudioDirection, AudioError, AudioFormat, AudioSession,
    AudioSidecarRequirement, AudioStreamRequirement, ChannelLayout, ClockRequirement, RouteIntent,
};
pub use gr_controller_contract::{
    CommitError, ControlError, ControlUpdate, DpadDirection, FaceButton, ManifestError,
    PreparedRealization, RealizationControllerDefinition, RealizationManifest,
    RealizationManifestEntry, Stick, StickPosition, TargetAwareControllerDriver, Trigger,
    prepare_deployment_realization, prepare_realization, prepare_transport_validation_realization,
};
pub use gr_controller_runtime::{ControllerRuntime, FrameSink};
pub use gr_realization_api::{
    ControllerId, DeploymentTarget, EventReadiness, NativeControllerRealization,
    NativeProviderFactory, NativeProviderSession, NativeRealizationError, ProviderCapabilities,
    ProviderDiagnostics, ProviderError, ProviderFrame, ProviderOpenRequest,
    ProviderOpenValidationError, ProviderPreflightError, ProviderRequirements,
    ProviderReverseEvent, ProviderReverseEventSink, ProviderState, RawReverseEvent,
    RealizationError, RealizationSelection, RealizationSessionId, RealizationTarget,
    RealizationTargetSet, TransportValidationTarget, validate_provider,
};
