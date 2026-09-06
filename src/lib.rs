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
    TargetRestriction, prepare_realization,
};
pub use gr_controller_runtime::{ControllerRuntime, FrameSink};
pub use gr_curated_controllers::{
    BatteryLevel, BatteryState, CreationOptions, DualSenseAxis, DualSenseControl,
    DualSenseController, DualSenseFeature, DualSenseHidOutput, DualSenseOutputEvent,
    DualSenseState, DualSenseSurface, DualSenseTouchContact, DualSenseTrigger, DualShock4Axis,
    DualShock4Control, DualShock4Controller, DualShock4HidOutput, DualShock4MotionSample,
    DualShock4OutputEvent, DualShock4State, DualShock4Surface, DualShock4TouchContact,
    DualShock4TouchSlot, DualShock4Trigger, MotionSample, SwitchProAxis, SwitchProControl,
    SwitchProController, SwitchProMotionSample, SwitchProOutputEvent, SwitchProState,
    SwitchProSurface, TouchSlot, Xbox360Axis, Xbox360Control, Xbox360Controller,
    Xbox360OutputEvent, Xbox360State, Xbox360Surface, Xbox360Trigger, create_dualsense,
    create_dualshock4, create_switch_pro, create_xbox360,
};
pub use gr_realization_api::{
    ControllerId, EventReadiness, NativeControllerRealization, NativeHidReportKey,
    NativeProviderFactory, NativeProviderSession, NativeRealizationError, ProviderCapabilities,
    ProviderDiagnostics, ProviderError, ProviderFrame, ProviderOpenRequest,
    ProviderOpenValidationError, ProviderPreflightError, ProviderRequirements,
    ProviderReverseEvent, ProviderReverseEventSink, ProviderState, RawReverseEvent,
    RealizationError, RealizationId, RealizationSelection, RealizationSessionId, RealizationTarget,
    RealizationTargetSet, validate_provider,
};
