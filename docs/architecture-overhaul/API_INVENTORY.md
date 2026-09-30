# Working root API inventory

Baseline: PR #121 (`f8a10d0`) plus the current refinement working tree.
Generated from Linux all-feature rustdoc. Supporting crates and
`experimental` are outside the alpha application compatibility promise.
This is the working inventory; regenerate after GUI feedback before freeze.

## Review decisions

| Group | Decision / rationale | Availability / future stress |
| --- | --- | --- |
| Native controller state, controls and output | Keep: package-owned native semantics | Four current families; future families retain independent state |
| Creation, identity, service and lifecycle | Keep; cleanup errors supplement typed initiating cause | All platforms compile; unsupported providers reject; no fallback/manager |
| Topology and surfaces | Change: private containers plus accessors; remove pixel sizing | Read-only UI introspection; construction SPI is not root |
| Realization/component metadata | Change: exact primary realization and typed kinds | Explicit associated audio and composite USB; no role parsing |
| PCM read, format, ownership and errors | Change: root-owned read, typed argument/ownership errors; remove matching | API types always visible; backends require Linux opt-in features |
| Application audio health and pacing | Keep/change: opaque diagnostics and optional consumption progress | Borrowed direction-wide access; future groups remain additive |
| Graph timing and bridge telemetry | Move to experimental instrumentation | Unstable qualification tooling; not end-to-end latency |
| Old session/factory SPI | Remove unused traits; retain distinct manifest requirements | No ordinary-root construction contract |

Public fields appear in declarations below. Inherent public methods follow
each item. Re-exported types are expanded by rustdoc; their original owner
remains the supporting controller/contract crate, not a new plugin contract.
All listed ordinary-root items are retained with the group rationale above;
the change/remove migration is in [migration notes](../ALPHA_API_MIGRATION.md).

## Exact current public declarations

### `AudioAccess`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum AudioAccess { Samples, NativeClient, }
```

### `AudioChannel`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum AudioChannel { AudibleLeft, AudibleRight, Speaker, Microphone, MicrophoneLeft, MicrophoneRight, HapticLeft, HapticRight, }
```

### `AudioEndpointSelector`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
#[non_exhaustive]pub enum AudioEndpointSelector { PipeWireNode { name: String, }, AlsaPcm { card_id: String, device: u8, subdevice: u8, }, }
pub fn alsa_pcm(&self) -> Option<(&str, u8, u8)>
pub fn pipewire_node(&self) -> Option<&str>
```

### `AudioError`

Owner/source: `../src/gr_audio_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum AudioError { Unavailable, InvalidRequirement, InvalidSampleBuffer, OwnershipMismatch, Unsupported, IncompatibleTopology, AccessDenied, Closed, Backend { reason: String, }, }
```

### `AudioExposure`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum AudioExposure { Disabled, Emulated, }
```

### `CommitError`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum CommitError { Closed, Backend { reason: String, }, }
```

### `ComponentKind`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ComponentKind { Input, Audio, }
```

### `ControlError`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum ControlError { UnsupportedControl { control: &'static str, }, ValueOutOfRange { control: &'static str, value: u32, maximum: u32, }, UnavailableInRealization { selected_target: RealizationId, available_in: RealizationTargetSet, }, Closed, }
```

### `ControllerError`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ControllerError { MissingDeviceNode { target: RealizationId, path: String, }, AccessDenied { target: RealizationId, path: String, }, UnsupportedPlatform { target: RealizationId, }, InvalidRequest { reason: String, }, Open { reason: String, }, Write { reason: String, }, Read { reason: String, }, Unsupported { reason: String, }, Cleanup { cause: Box<ControllerError>, cleanup: Vec<String>, }, Closed, WouldBlock, }
```

### `ControllerStatus`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ControllerStatus { NotOpen, Open, Closed, Failed, }
```

### `DigitalControlUpdate`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DigitalControlUpdate { FaceButton { button: FaceButton, pressed: bool, }, Dpad { direction: DpadDirection, pressed: bool, }, }
```

### `DpadDirection`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DpadDirection { Up, Down, Left, Right, }
```

### `DpadHoldBehavior`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DpadHoldBehavior { AdjacentPair, IndependentButtons, }
```

### `DpadPresentation`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DpadPresentation { IndependentButtons, SnappingAxis, }
```

### `DualSenseAudioPath`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
#[non_exhaustive]pub enum DualSenseAudioPath { HeadphonesStereo, HeadphonesDualMono, HeadphonesLeftSpeakerRight, SpeakerRightOnly, }
```

### `DualSenseControl`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub enum DualSenseControl { Show 13 variants Cross, Circle, Square, Triangle, L1, R1, Create, Options, PlayStation, TouchpadClick, MicrophoneMute, LeftStickPress, RightStickPress, }
```

### `DualSenseFeature`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub enum DualSenseFeature { Touch, Motion, Lightbar, AdaptiveTriggers, Audio, }
```

### `DualSenseHidOutput`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
#[non_exhaustive]pub enum DualSenseHidOutput { UsbOutput {Show 16 fields raw: Vec<u8>, valid_flag0: u8, valid_flag1: u8, valid_flag2: u8, right_motor: Option<u8>, left_motor: Option<u8>, right_trigger_effect: [u8; 11], left_trigger_effect: [u8; 11], mute_button_led: Option<bool>, microphone_muted: Option<bool>, audio_path: Option<DualSenseAudioPath>, speaker_volume: Option<u8>, microphone_volume: Option<u8>, speaker_preamp: Option<u8>, player_leds: Option<u8>, lightbar_rgb: Option<[u8; 3]>, }, Unknown { report_id: Option<u8>, raw: Vec<u8>, }, }
```

### `DualSenseOutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum DualSenseOutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput(DualSenseHidOutput), }
```

### `DualShock4Control`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub enum DualShock4Control { Cross, Circle, Square, Triangle, L1, R1, Share, Options, PlayStation, TouchpadClick, LeftStickPress, RightStickPress, }
```

### `DualShock4HidOutput`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
#[non_exhaustive]pub enum DualShock4HidOutput { UsbOutput { raw: Vec<u8>, right_motor: u8, left_motor: u8, lightbar_rgb: Option<[u8; 3]>, }, Unknown { report_id: Option<u8>, raw: Vec<u8>, }, }
```

### `DualShock4OutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum DualShock4OutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput(DualShock4HidOutput), }
```

### `DualShock4TouchSlot`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub enum DualShock4TouchSlot { First, Second, }
```

### `ExtraAxisInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum ExtraAxisInput { OneDimensional { id: InputControlId, title: &'static str, range: InputAxisRange, }, TwoDimensional { id: InputControlId, title: &'static str, x: InputAxisRange, y: InputAxisRange, }, }
```

### `FaceButton`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum FaceButton { North, South, East, West, }
```

### `ForceFeedbackEffect`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub enum ForceFeedbackEffect { Rumble(RumbleEffect), Unsupported { id: i16, kind: u16, }, }
```

### `ForceFeedbackEvent`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub enum ForceFeedbackEvent { Uploaded { request_id: u32, effect: ForceFeedbackEffect, status: i32, }, Erased { request_id: u32, effect_id: u32, status: i32, }, Playback { effect: RumbleEffect, repetitions: u32, }, }
```

### `HostLifecycle`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum HostLifecycle { Started, Stopped, Opened, Closed, }
```

### `InputTopologyError`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum InputTopologyError { EmptyIdentifier, DuplicateIdentifier(InputControlId), DuplicatePlacement(InputControlId), InvalidPlacement(InputControlId), InvalidAxisRange(InputControlId), InvalidTouchpad(InputControlId), InvalidScale(InputControlId), }
```

### `RealizationValidationStatus`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum RealizationValidationStatus { ResearchBacked, HostValidated, PhysicallyValidated, }
```

### `SampleDirection`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum SampleDirection { HostToController, ControllerToHost, }
```

### `ServiceReadiness`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ServiceReadiness<'a> { Poll, Descriptor(ReadinessDescriptor<'a>), }
```

### `SwitchProControl`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub enum SwitchProControl { Show 14 variants B, A, Y, X, L, R, Zl, Zr, Minus, Plus, Home, Capture, LeftStickPress, RightStickPress, }
```

### `SwitchProOutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum SwitchProOutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), Output { report_id: Option<u8>, bytes: Vec<u8>, }, }
pub fn rumble(&self) -> Option<SwitchProRumble>
```

### `TouchSlot`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub enum TouchSlot { First, Second, }
```

### `TouchpadActuation`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum TouchpadActuation { None, Button(AuxiliaryButtonInput), Deflection { id: InputControlId, range: InputAxisRange, }, }
```

### `TriggerInputKind`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum TriggerInputKind { Button { id: InputControlId, }, Axis { id: InputControlId, range: InputAxisRange, }, }
```

### `Xbox360Control`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub enum Xbox360Control { A, B, X, Y, LeftShoulder, RightShoulder, Back, Start, Guide, LeftStickPress, RightStickPress, }
```

### `Xbox360OutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum Xbox360OutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput { report_id: Option<u8>, bytes: Vec<u8>, }, }
```

### `create_dualsense`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_dualsense( options: CreationOptions, ) -> Result<DualSenseController, ControllerError>
```

### `create_dualsense_with_identity`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_dualsense_with_identity( options: CreationOptions, identity: DualSenseIdentity, ) -> Result<DualSenseController, ControllerError>
```

### `create_dualshock4`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_dualshock4( options: CreationOptions, ) -> Result<DualShock4Controller, ControllerError>
```

### `create_dualshock4_with_identity`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_dualshock4_with_identity( options: CreationOptions, identity: DualShock4Identity, ) -> Result<DualShock4Controller, ControllerError>
```

### `create_switch_pro`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_switch_pro( options: CreationOptions, ) -> Result<SwitchProController, ControllerError>
```

### `create_xbox360`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_xbox360( options: CreationOptions, ) -> Result<Xbox360Controller, ControllerError>
```

### `AbsoluteAxisSurface`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct AbsoluteAxisSurface { pub control: &'static str, pub event_code: u16, pub minimum: i32, pub maximum: i32, pub neutral: i32, pub flat: i32, }
```

### `AudioDiagnostics`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
pub struct AudioDiagnostics { /* private fields */ }
pub const fn is_closed(&self) -> bool
pub const fn failed(&self) -> bool
pub const fn underrun_frames(&self) -> u64
pub const fn dropped_playback_frames(&self) -> u64
pub const fn last_error(&self) -> Option<&AudioError>
```

### `AudioEndpoint`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
pub struct AudioEndpoint { /* private fields */ }
pub const fn group(&self) -> &'static str
pub fn clock_domain(&self) -> &str
pub const fn host(&self) -> &AudioEndpointSelector
pub const fn caller(&self) -> Option<&AudioEndpointSelector>
pub const fn format(&self) -> &PcmFormat
pub const fn direction(&self) -> SampleDirection
pub const fn access(&self) -> AudioAccess
```

### `AudioOptions`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
pub struct AudioOptions { /* private fields */ }
pub const fn new(exposure: AudioExposure) -> AudioOptions
pub const fn with_playback_access(self, access: AudioAccess) -> AudioOptions
pub const fn with_microphone_access(self, access: AudioAccess) -> AudioOptions
pub const fn exposure(self) -> AudioExposure
pub const fn playback_access(self) -> AudioAccess
pub const fn microphone_access(self) -> AudioAccess
```

### `AudioRead`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
#[non_exhaustive]pub struct AudioRead { pub frames: usize, pub first_frame: u64, pub discontinuity: bool, }
```

### `AuxiliaryButtonInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct AuxiliaryButtonInput { pub id: InputControlId, pub label: &'static str, }
```

### `BatteryLevel`

Owner/source: `../src/gr_curated_controllers/lib.rs.html`

```rust
pub struct BatteryLevel(/* private fields */);
pub const fn percent(self) -> u8
pub fn new(percent: u8) -> Result<BatteryLevel, ControlError>
```

### `BatteryState`

Owner/source: `../src/gr_curated_controllers/lib.rs.html`

```rust
pub struct BatteryState { /* private fields */ }
pub const fn is_exposed(self) -> bool
pub const fn level(self) -> BatteryLevel
```

### `ClusterPlacement`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct ClusterPlacement { pub column: i8, pub row: i8, }
pub const NORTH: ClusterPlacement
pub const SOUTH: ClusterPlacement
pub const EAST: ClusterPlacement
pub const WEST: ClusterPlacement
```

### `ComponentAssociation`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
pub struct ComponentAssociation { /* private fields */ }
pub const fn kind(&self) -> ComponentKind
pub const fn role(&self) -> &'static str
pub const fn surface(&self) -> Option<&'static ControllerSurface>
pub fn audio_endpoint(&self) -> Option<&AudioEndpoint>
pub fn requested_physical_path(&self) -> Option<&str>
pub fn requested_unique_id(&self) -> Option<&str>
pub fn observed_host_path(&self) -> Option<&str>
```

### `ControllerAssociation`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
pub struct ControllerAssociation { /* private fields */ }
pub const fn controller(&self) -> ControllerId
pub const fn realization(&self) -> RealizationId
pub const fn creation(&self) -> u64
pub fn components(&self) -> &[ComponentAssociation]
```

### `ControllerAudio`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
pub struct ControllerAudio { /* private fields */ }
pub fn diagnostics(&self) -> AudioDiagnostics
pub fn endpoints(&self) -> &[AudioEndpoint]
pub const fn limitation(&self) -> &'static str
pub fn read_playback( &mut self, dest: &mut [i16], ) -> Result<AudioRead, AudioError>
pub fn write_microphone(&mut self, samples: &[i16]) -> Result<usize, AudioError>
pub fn flush_playback(&mut self) -> Result<(), AudioError>
pub fn flush_microphone(&mut self) -> Result<(), AudioError>
pub fn is_closed(&self) -> bool
pub fn underrun_frames(&self) -> u64
pub fn dropped_playback_frames(&self) -> u64
pub fn microphone_consumed_frames(&mut self) -> Result<Option<u64>, AudioError>
pub fn dropped_microphone_frames(&mut self) -> Result<Option<u64>, AudioError>
pub fn last_error(&self) -> Option<&AudioError>
```

### `ControllerDiagnostics`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
pub struct ControllerDiagnostics { /* private fields */ }
pub const fn status(&self) -> ControllerStatus
pub const fn frames_sent(&self) -> u64
pub const fn reverse_events_drained(&self) -> u64
pub const fn write_failures(&self) -> u64
pub const fn lifecycle_events(&self) -> u64
pub const fn dropped_output_events(&self) -> u64
pub fn last_error(&self) -> Option<&str>
```

### `ControllerId`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub struct ControllerId(/* private fields */);
pub const fn new(value: &'static str) -> ControllerId
pub const fn as_str(self) -> &'static str
```

### `ControllerSurface`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct ControllerSurface { /* private fields */ }
pub const fn target(&self) -> RealizationId
pub const fn validation_status(&self) -> RealizationValidationStatus
pub const fn digital_controls(&self) -> &'static [DigitalControlSurface]
pub const fn axes(&self) -> &'static [AbsoluteAxisSurface]
pub const fn outputs(&self) -> &'static [OutputSurface]
pub const fn restrictions(&self) -> &'static [TargetRestriction]
pub const fn input_topology(&self) -> &'static InputTopology
```

### `CreationOptions`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
pub struct CreationOptions { /* private fields */ }
pub const fn new(target: RealizationId) -> Self
pub const fn realization(self) -> RealizationId
pub const fn with_audio(self, audio: AudioOptions) -> Self
pub const fn audio(self) -> AudioOptions
```

### `CustomInputModule`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct CustomInputModule { pub id: InputControlId, pub title: &'static str, }
```

### `DigitalControlSurface`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct DigitalControlSurface { pub control: &'static str, pub event_code: u16, }
```

### `DpadCluster`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct DpadCluster { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn presentation(&self) -> DpadPresentation
pub const fn hold_behavior(&self) -> DpadHoldBehavior
```

### `DualSenseAxis`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseAxis(/* private fields */);
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> DualSenseAxis
pub const fn neutral() -> DualSenseAxis
```

### `DualSenseController`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct DualSenseController { /* private fields */ }
pub fn audio(&mut self) -> Option<&mut ControllerAudio>
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &DualSenseState
pub fn surface(&self) -> &'static DualSenseSurface
pub fn is_dirty(&self) -> bool
pub fn set_digital( &mut self, update: DigitalControlUpdate, ) -> Result<(), ControlError>
pub fn set_native( &mut self, control: DualSenseControl, pressed: bool, ) -> Result<(), ControlError>
pub fn set_left_stick( &mut self, x: DualSenseAxis, y: DualSenseAxis, ) -> Result<(), ControlError>
pub fn set_right_stick( &mut self, x: DualSenseAxis, y: DualSenseAxis, ) -> Result<(), ControlError>
pub fn set_triggers( &mut self, left: DualSenseTrigger, right: DualSenseTrigger, ) -> Result<(), ControlError>
pub fn set_touch( &mut self, slot: TouchSlot, contact: Option<DualSenseTouchContact>, ) -> Result<(), ControlError>
pub fn set_motion(&mut self, motion: MotionSample) -> Result<(), ControlError>
pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError>
pub fn set_battery_level( &mut self, level: BatteryLevel, ) -> Result<(), ControlError>
pub fn feature_available( &self, feature: DualSenseFeature, ) -> Result<(), ControlError>
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(DualSenseOutputEvent), ) -> Result<(), ControllerError>
pub const fn identity(&self) -> Option<DualSenseIdentity>
```

### `DualSenseIdentity`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct DualSenseIdentity(/* private fields */);
pub fn generate() -> Result<Self, ControllerError>
pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self>
pub const fn to_bytes(self) -> [u8; 6]
```

### `DualSenseState`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseState { /* private fields */ }
pub const fn left_stick(&self) -> (DualSenseAxis, DualSenseAxis)
pub const fn right_stick(&self) -> (DualSenseAxis, DualSenseAxis)
pub const fn triggers(&self) -> (DualSenseTrigger, DualSenseTrigger)
pub const fn touch(&self, slot: TouchSlot) -> Option<DualSenseTouchContact>
pub const fn motion(&self) -> MotionSample
pub const fn battery(&self) -> BatteryState
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub const fn native_pressed(&self, control: DualSenseControl) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
```

### `DualSenseSurface`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseSurface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

### `DualSenseTouchContact`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseTouchContact { /* private fields */ }
pub fn new( id: u8, x: u16, y: u16, ) -> Result<DualSenseTouchContact, ControlError>
pub const fn id(self) -> u8
pub const fn x(self) -> u16
pub const fn y(self) -> u16
```

### `DualSenseTrigger`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseTrigger(/* private fields */);
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> DualSenseTrigger
```

### `DualShock4Axis`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Axis(/* private fields */);
pub const fn new(raw: u8) -> DualShock4Axis
pub const fn raw(self) -> u8
```

### `DualShock4Controller`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct DualShock4Controller { /* private fields */ }
pub fn audio(&mut self) -> Option<&mut ControllerAudio>
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &DualShock4State
pub fn is_dirty(&self) -> bool
pub fn surface(&self) -> &'static DualShock4Surface
pub fn set_digital( &mut self, u: DigitalControlUpdate, ) -> Result<(), ControlError>
pub fn set_native( &mut self, c: DualShock4Control, p: bool, ) -> Result<(), ControlError>
pub fn set_left_stick( &mut self, x: DualShock4Axis, y: DualShock4Axis, ) -> Result<(), ControlError>
pub fn set_right_stick( &mut self, x: DualShock4Axis, y: DualShock4Axis, ) -> Result<(), ControlError>
pub fn set_triggers( &mut self, l: DualShock4Trigger, r: DualShock4Trigger, ) -> Result<(), ControlError>
pub fn set_motion( &mut self, m: DualShock4MotionSample, ) -> Result<(), ControlError>
pub fn set_touch( &mut self, slot: DualShock4TouchSlot, contact: Option<DualShock4TouchContact>, ) -> Result<(), ControlError>
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(DualShock4OutputEvent), ) -> Result<(), ControllerError>
pub const fn identity(&self) -> Option<DualShock4Identity>
```

### `DualShock4Identity`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct DualShock4Identity(/* private fields */);
pub fn generate() -> Result<Self, ControllerError>
pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self>
pub const fn to_bytes(self) -> [u8; 6]
```

### `DualShock4MotionSample`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4MotionSample { pub accelerometer: [i16; 3], pub gyroscope: [i16; 3], }
```

### `DualShock4State`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4State { /* private fields */ }
pub const fn motion(&self) -> DualShock4MotionSample
pub const fn left_stick(&self) -> (DualShock4Axis, DualShock4Axis)
pub const fn right_stick(&self) -> (DualShock4Axis, DualShock4Axis)
pub const fn triggers(&self) -> (DualShock4Trigger, DualShock4Trigger)
pub const fn touch( &self, slot: DualShock4TouchSlot, ) -> Option<DualShock4TouchContact>
pub const fn native_pressed(&self, control: DualShock4Control) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
pub const fn face_pressed(&self, button: FaceButton) -> bool
```

### `DualShock4Surface`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Surface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

### `DualShock4TouchContact`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4TouchContact { /* private fields */ }
pub fn new( id: u8, x: u16, y: u16, ) -> Result<DualShock4TouchContact, ControlError>
pub const fn id(self) -> u8
pub const fn x(self) -> u16
pub const fn y(self) -> u16
```

### `DualShock4Trigger`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Trigger(/* private fields */);
pub const fn new(raw: u8) -> DualShock4Trigger
pub const fn raw(self) -> u8
```

### `FaceButtonCluster`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct FaceButtonCluster { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn buttons(&self) -> &'static [FaceButtonInput]
```

### `FaceButtonInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct FaceButtonInput { pub button: FaceButton, pub label: &'static str, pub placement: ClusterPlacement, }
```

### `InputAxisRange`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct InputAxisRange { pub minimum: i32, pub maximum: i32, pub neutral: i32, }
pub const fn is_valid(self) -> bool
```

### `InputControlId`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct InputControlId(/* private fields */);
pub const fn new(value: &'static str) -> InputControlId
pub const fn as_str(self) -> &'static str
```

### `InputScale`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct InputScale { pub numerator: i32, pub denominator: u32, }
pub const IDENTITY: InputScale
```

### `InputTopology`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct InputTopology { /* private fields */ }
pub const fn auxiliary_buttons(&self) -> &'static [AuxiliaryButtonInput]
pub const fn face_button_clusters(&self) -> &'static [FaceButtonCluster]
pub const fn dpads(&self) -> &'static [DpadCluster]
pub const fn sticks(&self) -> &'static [StickInput]
pub const fn trigger_stacks(&self) -> &'static [TriggerStack]
pub const fn touchpads(&self) -> &'static [TouchpadInput]
pub const fn motion(&self) -> &'static [MotionInput]
pub const fn extra_axes(&self) -> &'static [ExtraAxisInput]
pub const fn custom_modules(&self) -> &'static [CustomInputModule]
pub const EMPTY: InputTopology
pub fn validate(&self) -> Result<(), InputTopologyError>
```

### `MotionInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct MotionInput { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn range(&self) -> InputAxisRange
pub const fn gyroscope_scale(&self) -> [InputScale; 3]
pub const fn accelerometer_scale(&self) -> [InputScale; 3]
```

### `MotionSample`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct MotionSample { pub accelerometer: [i16; 3], pub gyroscope: [i16; 3], }
```

### `OutputSurface`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct OutputSurface { pub name: &'static str, pub event_type: u16, pub event_code: u16, }
```

### `PcmFormat`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
pub struct PcmFormat { /* private fields */ }
pub fn new( rate: u32, channels: &[AudioChannel], ) -> Result<PcmFormat, AudioError>
pub const fn sample_rate_hz(&self) -> u32
pub fn channels(&self) -> &[AudioChannel]
```

### `ReadinessDescriptor`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
pub struct ReadinessDescriptor<'a> { /* private fields */ }
pub const fn raw_fd(&self) -> i32
```

### `RealizationId`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub struct RealizationId(/* private fields */);
pub const LINUX_UINPUT: RealizationId
pub const LINUX_UHID_BLUETOOTH: RealizationId
pub const LINUX_UHID_USB: RealizationId
pub const LINUX_DUMMY_HCD_USB_HID: RealizationId
pub const LINUX_USBIP_USB_AUDIO: RealizationId
pub const fn new(value: &'static str) -> RealizationId
pub const fn as_str(self) -> &'static str
```

### `RealizationTargetSet`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub struct RealizationTargetSet(/* private fields */);
pub const EMPTY: RealizationTargetSet
pub const fn new(ids: &'static [RealizationId]) -> RealizationTargetSet
pub const fn contains(self, target: RealizationId) -> bool
pub const fn is_empty(self) -> bool
```

### `RumbleEffect`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub struct RumbleEffect { pub id: i16, pub strong: u16, pub weak: u16, pub length_ms: u16, pub delay_ms: u16, pub trigger_button: u16, pub trigger_interval_ms: u16, }
```

### `StickInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct StickInput { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn x(&self) -> InputAxisRange
pub const fn y(&self) -> InputAxisRange
pub const fn press(&self) -> Option<AuxiliaryButtonInput>
pub const fn capacitive(&self) -> Option<AuxiliaryButtonInput>
```

### `SwitchProAxis`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProAxis(/* private fields */);
pub const fn new(raw: i16) -> SwitchProAxis
pub const fn raw(self) -> i16
```

### `SwitchProController`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct SwitchProController { /* private fields */ }
pub fn stream_enabled(&self) -> bool
pub fn motion_report_counter(&self) -> u8
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &SwitchProState
pub fn is_dirty(&self) -> bool
pub fn surface(&self) -> &'static SwitchProSurface
pub fn set_digital( &mut self, u: DigitalControlUpdate, ) -> Result<(), ControlError>
pub fn set_native( &mut self, c: SwitchProControl, p: bool, ) -> Result<(), ControlError>
pub fn set_left_stick( &mut self, x: SwitchProAxis, y: SwitchProAxis, ) -> Result<(), ControlError>
pub fn set_right_stick( &mut self, x: SwitchProAxis, y: SwitchProAxis, ) -> Result<(), ControlError>
pub fn set_motion( &mut self, m: SwitchProMotionSample, ) -> Result<(), ControlError>
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(SwitchProOutputEvent), ) -> Result<(), ControllerError>
```

### `SwitchProMotionSample`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProMotionSample { pub accelerometer: [i16; 3], pub gyroscope: [i16; 3], }
```

### `SwitchProRumble`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProRumble { pub packet_counter: u8, pub left: [u8; 4], pub right: [u8; 4], }
```

### `SwitchProState`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProState { /* private fields */ }
pub const fn left_stick(&self) -> (SwitchProAxis, SwitchProAxis)
pub const fn right_stick(&self) -> (SwitchProAxis, SwitchProAxis)
pub const fn motion(&self) -> SwitchProMotionSample
pub const fn native_pressed(&self, control: SwitchProControl) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
pub const fn face_pressed(&self, button: FaceButton) -> bool
```

### `SwitchProSurface`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProSurface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

### `TargetRestriction`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct TargetRestriction { pub feature: &'static str, pub reason: &'static str, }
```

### `TouchpadInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct TouchpadInput { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn width(&self) -> u32
pub const fn height(&self) -> u32
pub const fn contacts(&self) -> u8
pub const fn actuation(&self) -> TouchpadActuation
```

### `TriggerInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct TriggerInput { pub label: &'static str, pub kind: TriggerInputKind, }
```

### `TriggerStack`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct TriggerStack { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn controls(&self) -> &'static [TriggerInput]
```

### `Xbox360Axis`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Axis(/* private fields */);
pub const fn raw(self) -> i16
pub const fn new(raw: i16) -> Xbox360Axis
```

### `Xbox360Controller`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct Xbox360Controller { /* private fields */ }
pub fn audio(&mut self) -> Option<&mut ControllerAudio>
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &Xbox360State
pub fn surface(&self) -> &'static Xbox360Surface
pub fn is_dirty(&self) -> bool
pub fn set_digital( &mut self, update: DigitalControlUpdate, ) -> Result<(), ControlError>
pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError>
pub fn set_battery_level( &mut self, level: BatteryLevel, ) -> Result<(), ControlError>
pub fn set_native( &mut self, control: Xbox360Control, pressed: bool, ) -> Result<(), ControlError>
pub fn set_left_stick( &mut self, x: Xbox360Axis, y: Xbox360Axis, ) -> Result<(), ControlError>
pub fn set_right_stick( &mut self, x: Xbox360Axis, y: Xbox360Axis, ) -> Result<(), ControlError>
pub fn set_triggers( &mut self, left: Xbox360Trigger, right: Xbox360Trigger, ) -> Result<(), ControlError>
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(Xbox360OutputEvent), ) -> Result<(), ControllerError>
```

### `Xbox360State`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360State { /* private fields */ }
pub const fn left_stick(&self) -> (Xbox360Axis, Xbox360Axis)
pub const fn right_stick(&self) -> (Xbox360Axis, Xbox360Axis)
pub const fn triggers(&self) -> (Xbox360Trigger, Xbox360Trigger)
pub const fn battery(&self) -> BatteryState
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub const fn native_pressed(&self, control: Xbox360Control) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
```

### `Xbox360Surface`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Surface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

### `Xbox360Trigger`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Trigger(/* private fields */);
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> Xbox360Trigger
```

### `ControllerSurfaceInfo`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub trait ControllerSurfaceInfo { // Required method fn common_surface(&self) -> &ControllerSurface; }
```
