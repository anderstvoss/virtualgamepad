#![forbid(unsafe_code)]

//! Curated virtual-controller API and provider-neutral core contracts.

pub use gr_audio_contract::{
    AudioBackendFactory, AudioDirection, AudioError, AudioFormat, AudioSession,
    AudioSidecarRequirement, AudioStreamRequirement, ChannelLayout, ClockRequirement, RouteIntent,
};
pub use gr_controller_contract::{
    AbsoluteAxisSurface, CommitError, ControlError, ControllerSurface, ControllerSurfaceInfo,
    DigitalControlSurface, DigitalControlUpdate, DpadDirection, FaceButton, ManifestError,
    OutputSurface, PreparedRealization, RealizationControllerDefinition, RealizationManifest,
    RealizationManifestEntry, RealizationValidationStatus, TargetAwareControllerDriver,
    TargetRestriction, prepare_deployment_realization, prepare_realization,
    prepare_transport_validation_realization,
};
pub use gr_controller_runtime::{ControllerRuntime, FrameSink};
pub use gr_curated_controllers::{
    CreationOptions, DualSenseAxis, DualSenseControl, DualSenseController, DualSenseFeature,
    DualSenseHidOutput, DualSenseOutputEvent, DualSenseState, DualSenseSurface,
    DualSenseTouchContact, DualSenseTrigger, GenericGamepadAxis, GenericGamepadControl,
    GenericGamepadController, GenericGamepadOutputEvent, GenericGamepadState,
    GenericGamepadSurface, GenericGamepadTrigger, MotionSample, TouchSlot, Xbox360Axis,
    Xbox360Control, Xbox360Controller, Xbox360OutputEvent, Xbox360State, Xbox360Surface,
    Xbox360Trigger, create_dualsense, create_generic_gamepad, create_xbox360,
};
pub use gr_realization_api::{
    ControllerId, DeploymentTarget, EventReadiness, NativeControllerRealization,
    NativeHidReportKey, NativeProviderFactory, NativeProviderSession, NativeRealizationError,
    NativeUsbCompositeEndpoint, NativeUsbCompositeRealization, ProviderCapabilities,
    ProviderDiagnostics, ProviderError, ProviderFrame, ProviderOpenRequest,
    ProviderOpenValidationError, ProviderPreflightError, ProviderRequirements,
    ProviderReverseEvent, ProviderReverseEventSink, ProviderState, RawReverseEvent,
    RealizationError, RealizationSelection, RealizationSessionId, RealizationTarget,
    RealizationTargetSet, TransportValidationTarget, validate_provider,
};
