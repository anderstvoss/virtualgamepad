# Working root API inventory

Baseline: merged PRs #122–125, #127 and #128 (`ec4c2ff`) plus the post-GUI audit tree.
Generated from Linux all-feature rustdoc. Supporting crates and
`experimental` are outside the alpha application compatibility promise.
This is the post-GUI working inventory; no hard freeze is declared.
Ordinary-root declarations are available with every feature set on all
compiling platforms. Provider creation is separately gated by Linux,
features and host prerequisites; see [the audit](ALPHA_ROOT_API_AUDIT.md).

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

Concrete and auto trait implementations follow each declaration. Compiler
blanket implementations and inherited trait method bodies are omitted.
This rustdoc inventory is an audit aid, not a complete semver checker.

## Exact current public declarations

### `AudioAccess`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum AudioAccess { Samples, NativeClient, }
```

Trait implementations:

```rust
impl Clone for AudioAccess
impl Debug for AudioAccess
impl Default for AudioAccess
impl PartialEq for AudioAccess
impl Copy for AudioAccess
impl Eq for AudioAccess
impl StructuralPartialEq for AudioAccess
impl Freeze for AudioAccess
impl RefUnwindSafe for AudioAccess
impl Send for AudioAccess
impl Sync for AudioAccess
impl Unpin for AudioAccess
impl UnsafeUnpin for AudioAccess
impl UnwindSafe for AudioAccess
```

### `AudioChannel`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum AudioChannel { AudibleLeft, AudibleRight, Speaker, Microphone, MicrophoneLeft, MicrophoneRight, HapticLeft, HapticRight, }
```

Trait implementations:

```rust
impl Clone for AudioChannel
impl Debug for AudioChannel
impl PartialEq for AudioChannel
impl Copy for AudioChannel
impl Eq for AudioChannel
impl StructuralPartialEq for AudioChannel
impl Freeze for AudioChannel
impl RefUnwindSafe for AudioChannel
impl Send for AudioChannel
impl Sync for AudioChannel
impl Unpin for AudioChannel
impl UnsafeUnpin for AudioChannel
impl UnwindSafe for AudioChannel
```

### `AudioEndpointSelector`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
#[non_exhaustive]pub enum AudioEndpointSelector { PipeWireNode { name: String, }, AlsaPcm { card_id: String, device: u8, subdevice: u8, }, }
pub fn alsa_pcm(&self) -> Option<(&str, u8, u8)>
pub fn pipewire_node(&self) -> Option<&str>
```

Trait implementations:

```rust
impl Clone for AudioEndpointSelector
impl Debug for AudioEndpointSelector
impl PartialEq for AudioEndpointSelector
impl Eq for AudioEndpointSelector
impl StructuralPartialEq for AudioEndpointSelector
impl Freeze for AudioEndpointSelector
impl RefUnwindSafe for AudioEndpointSelector
impl Send for AudioEndpointSelector
impl Sync for AudioEndpointSelector
impl Unpin for AudioEndpointSelector
impl UnsafeUnpin for AudioEndpointSelector
impl UnwindSafe for AudioEndpointSelector
```

### `AudioError`

Owner/source: `../src/gr_audio_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum AudioError { Unavailable, InvalidRequirement, InvalidSampleBuffer, OwnershipMismatch, Unsupported, IncompatibleTopology, AccessDenied, Closed, Backend { reason: String, }, }
```

Trait implementations:

```rust
impl Clone for AudioError
impl Debug for AudioError
impl Display for AudioError
impl Error for AudioError
impl PartialEq for AudioError
impl Eq for AudioError
impl StructuralPartialEq for AudioError
impl Freeze for AudioError
impl RefUnwindSafe for AudioError
impl Send for AudioError
impl Sync for AudioError
impl Unpin for AudioError
impl UnsafeUnpin for AudioError
impl UnwindSafe for AudioError
```

### `AudioExposure`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum AudioExposure { Disabled, Emulated, }
```

Trait implementations:

```rust
impl Clone for AudioExposure
impl Debug for AudioExposure
impl Default for AudioExposure
impl PartialEq for AudioExposure
impl Copy for AudioExposure
impl Eq for AudioExposure
impl StructuralPartialEq for AudioExposure
impl Freeze for AudioExposure
impl RefUnwindSafe for AudioExposure
impl Send for AudioExposure
impl Sync for AudioExposure
impl Unpin for AudioExposure
impl UnsafeUnpin for AudioExposure
impl UnwindSafe for AudioExposure
```

### `CommitError`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum CommitError { Closed, Backend { reason: String, }, }
```

Trait implementations:

```rust
impl Clone for CommitError
impl Debug for CommitError
impl Display for CommitError
impl Error for CommitError
impl PartialEq for CommitError
impl Eq for CommitError
impl StructuralPartialEq for CommitError
impl Freeze for CommitError
impl RefUnwindSafe for CommitError
impl Send for CommitError
impl Sync for CommitError
impl Unpin for CommitError
impl UnsafeUnpin for CommitError
impl UnwindSafe for CommitError
```

### `ComponentKind`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ComponentKind { Input, Audio, }
```

Trait implementations:

```rust
impl Clone for ComponentKind
impl Debug for ComponentKind
impl PartialEq for ComponentKind
impl Copy for ComponentKind
impl Eq for ComponentKind
impl StructuralPartialEq for ComponentKind
impl Freeze for ComponentKind
impl RefUnwindSafe for ComponentKind
impl Send for ComponentKind
impl Sync for ComponentKind
impl Unpin for ComponentKind
impl UnsafeUnpin for ComponentKind
impl UnwindSafe for ComponentKind
```

### `ControlError`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum ControlError { UnsupportedControl { control: &'static str, }, ValueOutOfRange { control: &'static str, value: u32, maximum: u32, }, UnavailableInRealization { selected_target: RealizationId, available_in: RealizationTargetSet, }, Closed, }
```

Trait implementations:

```rust
impl Clone for ControlError
impl Debug for ControlError
impl Display for ControlError
impl Error for ControlError
impl PartialEq for ControlError
impl Eq for ControlError
impl StructuralPartialEq for ControlError
impl Freeze for ControlError
impl RefUnwindSafe for ControlError
impl Send for ControlError
impl Sync for ControlError
impl Unpin for ControlError
impl UnsafeUnpin for ControlError
impl UnwindSafe for ControlError
```

### `ControllerError`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ControllerError { MissingDeviceNode { target: RealizationId, path: String, }, AccessDenied { target: RealizationId, path: String, }, UnsupportedPlatform { target: RealizationId, }, InvalidRequest { reason: String, }, Open { reason: String, }, Write { reason: String, }, Read { reason: String, }, Unsupported { reason: String, }, Cleanup { cause: Box<ControllerError>, cleanup: Vec<String>, }, Closed, WouldBlock, }
```

Trait implementations:

```rust
impl Clone for ControllerError
impl Debug for ControllerError
impl Display for ControllerError
impl Error for ControllerError
impl PartialEq for ControllerError
impl Eq for ControllerError
impl StructuralPartialEq for ControllerError
impl Freeze for ControllerError
impl RefUnwindSafe for ControllerError
impl Send for ControllerError
impl Sync for ControllerError
impl Unpin for ControllerError
impl UnsafeUnpin for ControllerError
impl UnwindSafe for ControllerError
```

### `ControllerStatus`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ControllerStatus { NotOpen, Open, Closed, Failed, }
```

Trait implementations:

```rust
impl Clone for ControllerStatus
impl Debug for ControllerStatus
impl PartialEq for ControllerStatus
impl Copy for ControllerStatus
impl Eq for ControllerStatus
impl StructuralPartialEq for ControllerStatus
impl Freeze for ControllerStatus
impl RefUnwindSafe for ControllerStatus
impl Send for ControllerStatus
impl Sync for ControllerStatus
impl Unpin for ControllerStatus
impl UnsafeUnpin for ControllerStatus
impl UnwindSafe for ControllerStatus
```

### `DigitalControlUpdate`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DigitalControlUpdate { FaceButton { button: FaceButton, pressed: bool, }, Dpad { direction: DpadDirection, pressed: bool, }, }
```

Trait implementations:

```rust
impl Clone for DigitalControlUpdate
impl Debug for DigitalControlUpdate
impl PartialEq for DigitalControlUpdate
impl Copy for DigitalControlUpdate
impl Eq for DigitalControlUpdate
impl StructuralPartialEq for DigitalControlUpdate
impl Freeze for DigitalControlUpdate
impl RefUnwindSafe for DigitalControlUpdate
impl Send for DigitalControlUpdate
impl Sync for DigitalControlUpdate
impl Unpin for DigitalControlUpdate
impl UnsafeUnpin for DigitalControlUpdate
impl UnwindSafe for DigitalControlUpdate
```

### `DpadDirection`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DpadDirection { Up, Down, Left, Right, }
```

Trait implementations:

```rust
impl Clone for DpadDirection
impl Debug for DpadDirection
impl Hash for DpadDirection
impl PartialEq for DpadDirection
impl Copy for DpadDirection
impl Eq for DpadDirection
impl StructuralPartialEq for DpadDirection
impl Freeze for DpadDirection
impl RefUnwindSafe for DpadDirection
impl Send for DpadDirection
impl Sync for DpadDirection
impl Unpin for DpadDirection
impl UnsafeUnpin for DpadDirection
impl UnwindSafe for DpadDirection
```

### `DpadHoldBehavior`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DpadHoldBehavior { AdjacentPair, IndependentButtons, }
```

Trait implementations:

```rust
impl Clone for DpadHoldBehavior
impl Debug for DpadHoldBehavior
impl PartialEq for DpadHoldBehavior
impl Copy for DpadHoldBehavior
impl Eq for DpadHoldBehavior
impl StructuralPartialEq for DpadHoldBehavior
impl Freeze for DpadHoldBehavior
impl RefUnwindSafe for DpadHoldBehavior
impl Send for DpadHoldBehavior
impl Sync for DpadHoldBehavior
impl Unpin for DpadHoldBehavior
impl UnsafeUnpin for DpadHoldBehavior
impl UnwindSafe for DpadHoldBehavior
```

### `DpadPresentation`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum DpadPresentation { IndependentButtons, SnappingAxis, }
```

Trait implementations:

```rust
impl Clone for DpadPresentation
impl Debug for DpadPresentation
impl PartialEq for DpadPresentation
impl Copy for DpadPresentation
impl Eq for DpadPresentation
impl StructuralPartialEq for DpadPresentation
impl Freeze for DpadPresentation
impl RefUnwindSafe for DpadPresentation
impl Send for DpadPresentation
impl Sync for DpadPresentation
impl Unpin for DpadPresentation
impl UnsafeUnpin for DpadPresentation
impl UnwindSafe for DpadPresentation
```

### `DualSenseAudioPath`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
#[non_exhaustive]pub enum DualSenseAudioPath { HeadphonesStereo, HeadphonesDualMono, HeadphonesLeftSpeakerRight, SpeakerRightOnly, }
```

Trait implementations:

```rust
impl Clone for DualSenseAudioPath
impl Debug for DualSenseAudioPath
impl PartialEq for DualSenseAudioPath
impl Copy for DualSenseAudioPath
impl Eq for DualSenseAudioPath
impl StructuralPartialEq for DualSenseAudioPath
impl Freeze for DualSenseAudioPath
impl RefUnwindSafe for DualSenseAudioPath
impl Send for DualSenseAudioPath
impl Sync for DualSenseAudioPath
impl Unpin for DualSenseAudioPath
impl UnsafeUnpin for DualSenseAudioPath
impl UnwindSafe for DualSenseAudioPath
```

### `DualSenseControl`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub enum DualSenseControl { Show 13 variants Cross, Circle, Square, Triangle, L1, R1, Create, Options, PlayStation, TouchpadClick, MicrophoneMute, LeftStickPress, RightStickPress, }
```

Trait implementations:

```rust
impl Clone for DualSenseControl
impl Debug for DualSenseControl
impl PartialEq for DualSenseControl
impl Copy for DualSenseControl
impl Eq for DualSenseControl
impl StructuralPartialEq for DualSenseControl
impl Freeze for DualSenseControl
impl RefUnwindSafe for DualSenseControl
impl Send for DualSenseControl
impl Sync for DualSenseControl
impl Unpin for DualSenseControl
impl UnsafeUnpin for DualSenseControl
impl UnwindSafe for DualSenseControl
```

### `DualSenseFeature`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub enum DualSenseFeature { Touch, Motion, Lightbar, AdaptiveTriggers, Audio, }
```

Trait implementations:

```rust
impl Clone for DualSenseFeature
impl Debug for DualSenseFeature
impl PartialEq for DualSenseFeature
impl Copy for DualSenseFeature
impl Eq for DualSenseFeature
impl StructuralPartialEq for DualSenseFeature
impl Freeze for DualSenseFeature
impl RefUnwindSafe for DualSenseFeature
impl Send for DualSenseFeature
impl Sync for DualSenseFeature
impl Unpin for DualSenseFeature
impl UnsafeUnpin for DualSenseFeature
impl UnwindSafe for DualSenseFeature
```

### `DualSenseHidOutput`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
#[non_exhaustive]pub enum DualSenseHidOutput { UsbOutput {Show 16 fields raw: Vec<u8>, valid_flag0: u8, valid_flag1: u8, valid_flag2: u8, right_motor: Option<u8>, left_motor: Option<u8>, right_trigger_effect: [u8; 11], left_trigger_effect: [u8; 11], mute_button_led: Option<bool>, microphone_muted: Option<bool>, audio_path: Option<DualSenseAudioPath>, speaker_volume: Option<u8>, microphone_volume: Option<u8>, speaker_preamp: Option<u8>, player_leds: Option<u8>, lightbar_rgb: Option<[u8; 3]>, }, Unknown { report_id: Option<u8>, raw: Vec<u8>, }, }
```

Trait implementations:

```rust
impl Clone for DualSenseHidOutput
impl Debug for DualSenseHidOutput
impl PartialEq for DualSenseHidOutput
impl Eq for DualSenseHidOutput
impl StructuralPartialEq for DualSenseHidOutput
impl Freeze for DualSenseHidOutput
impl RefUnwindSafe for DualSenseHidOutput
impl Send for DualSenseHidOutput
impl Sync for DualSenseHidOutput
impl Unpin for DualSenseHidOutput
impl UnsafeUnpin for DualSenseHidOutput
impl UnwindSafe for DualSenseHidOutput
```

### `DualSenseOutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum DualSenseOutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput(DualSenseHidOutput), }
```

Trait implementations:

```rust
impl Clone for DualSenseOutputEvent
impl Debug for DualSenseOutputEvent
impl PartialEq for DualSenseOutputEvent
impl Eq for DualSenseOutputEvent
impl StructuralPartialEq for DualSenseOutputEvent
impl Freeze for DualSenseOutputEvent
impl RefUnwindSafe for DualSenseOutputEvent
impl Send for DualSenseOutputEvent
impl Sync for DualSenseOutputEvent
impl Unpin for DualSenseOutputEvent
impl UnsafeUnpin for DualSenseOutputEvent
impl UnwindSafe for DualSenseOutputEvent
```

### `DualShock4Control`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub enum DualShock4Control { Cross, Circle, Square, Triangle, L1, R1, Share, Options, PlayStation, TouchpadClick, LeftStickPress, RightStickPress, }
```

Trait implementations:

```rust
impl Clone for DualShock4Control
impl Debug for DualShock4Control
impl PartialEq for DualShock4Control
impl Copy for DualShock4Control
impl Eq for DualShock4Control
impl StructuralPartialEq for DualShock4Control
impl Freeze for DualShock4Control
impl RefUnwindSafe for DualShock4Control
impl Send for DualShock4Control
impl Sync for DualShock4Control
impl Unpin for DualShock4Control
impl UnsafeUnpin for DualShock4Control
impl UnwindSafe for DualShock4Control
```

### `DualShock4HidOutput`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
#[non_exhaustive]pub enum DualShock4HidOutput { UsbOutput { raw: Vec<u8>, right_motor: u8, left_motor: u8, lightbar_rgb: Option<[u8; 3]>, }, Unknown { report_id: Option<u8>, raw: Vec<u8>, }, }
```

Trait implementations:

```rust
impl Clone for DualShock4HidOutput
impl Debug for DualShock4HidOutput
impl PartialEq for DualShock4HidOutput
impl Eq for DualShock4HidOutput
impl StructuralPartialEq for DualShock4HidOutput
impl Freeze for DualShock4HidOutput
impl RefUnwindSafe for DualShock4HidOutput
impl Send for DualShock4HidOutput
impl Sync for DualShock4HidOutput
impl Unpin for DualShock4HidOutput
impl UnsafeUnpin for DualShock4HidOutput
impl UnwindSafe for DualShock4HidOutput
```

### `DualShock4OutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum DualShock4OutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput(DualShock4HidOutput), }
```

Trait implementations:

```rust
impl Clone for DualShock4OutputEvent
impl Debug for DualShock4OutputEvent
impl PartialEq for DualShock4OutputEvent
impl Eq for DualShock4OutputEvent
impl StructuralPartialEq for DualShock4OutputEvent
impl Freeze for DualShock4OutputEvent
impl RefUnwindSafe for DualShock4OutputEvent
impl Send for DualShock4OutputEvent
impl Sync for DualShock4OutputEvent
impl Unpin for DualShock4OutputEvent
impl UnsafeUnpin for DualShock4OutputEvent
impl UnwindSafe for DualShock4OutputEvent
```

### `DualShock4TouchSlot`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub enum DualShock4TouchSlot { First, Second, }
```

Trait implementations:

```rust
impl Clone for DualShock4TouchSlot
impl Debug for DualShock4TouchSlot
impl PartialEq for DualShock4TouchSlot
impl Copy for DualShock4TouchSlot
impl Eq for DualShock4TouchSlot
impl StructuralPartialEq for DualShock4TouchSlot
impl Freeze for DualShock4TouchSlot
impl RefUnwindSafe for DualShock4TouchSlot
impl Send for DualShock4TouchSlot
impl Sync for DualShock4TouchSlot
impl Unpin for DualShock4TouchSlot
impl UnsafeUnpin for DualShock4TouchSlot
impl UnwindSafe for DualShock4TouchSlot
```

### `ExtraAxisInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum ExtraAxisInput { OneDimensional { id: InputControlId, title: &'static str, range: InputAxisRange, }, TwoDimensional { id: InputControlId, title: &'static str, x: InputAxisRange, y: InputAxisRange, }, }
```

Trait implementations:

```rust
impl Clone for ExtraAxisInput
impl Debug for ExtraAxisInput
impl PartialEq for ExtraAxisInput
impl Copy for ExtraAxisInput
impl Eq for ExtraAxisInput
impl StructuralPartialEq for ExtraAxisInput
impl Freeze for ExtraAxisInput
impl RefUnwindSafe for ExtraAxisInput
impl Send for ExtraAxisInput
impl Sync for ExtraAxisInput
impl Unpin for ExtraAxisInput
impl UnsafeUnpin for ExtraAxisInput
impl UnwindSafe for ExtraAxisInput
```

### `FaceButton`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum FaceButton { North, South, East, West, }
```

Trait implementations:

```rust
impl Clone for FaceButton
impl Debug for FaceButton
impl Hash for FaceButton
impl PartialEq for FaceButton
impl Copy for FaceButton
impl Eq for FaceButton
impl StructuralPartialEq for FaceButton
impl Freeze for FaceButton
impl RefUnwindSafe for FaceButton
impl Send for FaceButton
impl Sync for FaceButton
impl Unpin for FaceButton
impl UnsafeUnpin for FaceButton
impl UnwindSafe for FaceButton
```

### `ForceFeedbackEffect`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub enum ForceFeedbackEffect { Rumble(RumbleEffect), Unsupported { id: i16, kind: u16, }, }
```

Trait implementations:

```rust
impl Clone for ForceFeedbackEffect
impl Debug for ForceFeedbackEffect
impl PartialEq for ForceFeedbackEffect
impl Copy for ForceFeedbackEffect
impl Eq for ForceFeedbackEffect
impl StructuralPartialEq for ForceFeedbackEffect
impl Freeze for ForceFeedbackEffect
impl RefUnwindSafe for ForceFeedbackEffect
impl Send for ForceFeedbackEffect
impl Sync for ForceFeedbackEffect
impl Unpin for ForceFeedbackEffect
impl UnsafeUnpin for ForceFeedbackEffect
impl UnwindSafe for ForceFeedbackEffect
```

### `ForceFeedbackEvent`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub enum ForceFeedbackEvent { Uploaded { request_id: u32, effect: ForceFeedbackEffect, status: i32, }, Erased { request_id: u32, effect_id: u32, status: i32, }, Playback { effect: RumbleEffect, repetitions: u32, }, }
```

Trait implementations:

```rust
impl Clone for ForceFeedbackEvent
impl Debug for ForceFeedbackEvent
impl PartialEq for ForceFeedbackEvent
impl Copy for ForceFeedbackEvent
impl Eq for ForceFeedbackEvent
impl StructuralPartialEq for ForceFeedbackEvent
impl Freeze for ForceFeedbackEvent
impl RefUnwindSafe for ForceFeedbackEvent
impl Send for ForceFeedbackEvent
impl Sync for ForceFeedbackEvent
impl Unpin for ForceFeedbackEvent
impl UnsafeUnpin for ForceFeedbackEvent
impl UnwindSafe for ForceFeedbackEvent
```

### `HostLifecycle`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum HostLifecycle { Started, Stopped, Opened, Closed, }
```

Trait implementations:

```rust
impl Clone for HostLifecycle
impl Debug for HostLifecycle
impl PartialEq for HostLifecycle
impl Copy for HostLifecycle
impl Eq for HostLifecycle
impl StructuralPartialEq for HostLifecycle
impl Freeze for HostLifecycle
impl RefUnwindSafe for HostLifecycle
impl Send for HostLifecycle
impl Sync for HostLifecycle
impl Unpin for HostLifecycle
impl UnsafeUnpin for HostLifecycle
impl UnwindSafe for HostLifecycle
```

### `InputTopologyError`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum InputTopologyError { EmptyIdentifier, DuplicateIdentifier(InputControlId), DuplicatePlacement(InputControlId), InvalidPlacement(InputControlId), InvalidAxisRange(InputControlId), InvalidTouchpad(InputControlId), InvalidScale(InputControlId), }
```

Trait implementations:

```rust
impl Clone for InputTopologyError
impl Debug for InputTopologyError
impl Display for InputTopologyError
impl Error for InputTopologyError
impl PartialEq for InputTopologyError
impl Copy for InputTopologyError
impl Eq for InputTopologyError
impl StructuralPartialEq for InputTopologyError
impl Freeze for InputTopologyError
impl RefUnwindSafe for InputTopologyError
impl Send for InputTopologyError
impl Sync for InputTopologyError
impl Unpin for InputTopologyError
impl UnsafeUnpin for InputTopologyError
impl UnwindSafe for InputTopologyError
```

### `RealizationValidationStatus`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
#[non_exhaustive]pub enum RealizationValidationStatus { ResearchBacked, HostValidated, PhysicallyValidated, }
```

Trait implementations:

```rust
impl Clone for RealizationValidationStatus
impl Debug for RealizationValidationStatus
impl PartialEq for RealizationValidationStatus
impl Copy for RealizationValidationStatus
impl Eq for RealizationValidationStatus
impl StructuralPartialEq for RealizationValidationStatus
impl Freeze for RealizationValidationStatus
impl RefUnwindSafe for RealizationValidationStatus
impl Send for RealizationValidationStatus
impl Sync for RealizationValidationStatus
impl Unpin for RealizationValidationStatus
impl UnsafeUnpin for RealizationValidationStatus
impl UnwindSafe for RealizationValidationStatus
```

### `SampleDirection`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
#[non_exhaustive]pub enum SampleDirection { HostToController, ControllerToHost, }
```

Trait implementations:

```rust
impl Clone for SampleDirection
impl Debug for SampleDirection
impl PartialEq for SampleDirection
impl Copy for SampleDirection
impl Eq for SampleDirection
impl StructuralPartialEq for SampleDirection
impl Freeze for SampleDirection
impl RefUnwindSafe for SampleDirection
impl Send for SampleDirection
impl Sync for SampleDirection
impl Unpin for SampleDirection
impl UnsafeUnpin for SampleDirection
impl UnwindSafe for SampleDirection
```

### `ServiceReadiness`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
#[non_exhaustive]pub enum ServiceReadiness<'a> { Poll, Descriptor(ReadinessDescriptor<'a>), }
```

Trait implementations:

```rust
impl<'a> Debug for ServiceReadiness<'a>
impl<'a> Freeze for ServiceReadiness<'a>
impl<'a> RefUnwindSafe for ServiceReadiness<'a>
impl<'a> Send for ServiceReadiness<'a>
impl<'a> Sync for ServiceReadiness<'a>
impl<'a> Unpin for ServiceReadiness<'a>
impl<'a> UnsafeUnpin for ServiceReadiness<'a>
impl<'a> UnwindSafe for ServiceReadiness<'a>
```

### `SwitchProControl`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub enum SwitchProControl { Show 14 variants B, A, Y, X, L, R, Zl, Zr, Minus, Plus, Home, Capture, LeftStickPress, RightStickPress, }
```

Trait implementations:

```rust
impl Clone for SwitchProControl
impl Debug for SwitchProControl
impl PartialEq for SwitchProControl
impl Copy for SwitchProControl
impl Eq for SwitchProControl
impl StructuralPartialEq for SwitchProControl
impl Freeze for SwitchProControl
impl RefUnwindSafe for SwitchProControl
impl Send for SwitchProControl
impl Sync for SwitchProControl
impl Unpin for SwitchProControl
impl UnsafeUnpin for SwitchProControl
impl UnwindSafe for SwitchProControl
```

### `SwitchProOutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum SwitchProOutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), Output { report_id: Option<u8>, bytes: Vec<u8>, }, }
pub fn rumble(&self) -> Option<SwitchProRumble>
```

Trait implementations:

```rust
impl Clone for SwitchProOutputEvent
impl Debug for SwitchProOutputEvent
impl PartialEq for SwitchProOutputEvent
impl Eq for SwitchProOutputEvent
impl StructuralPartialEq for SwitchProOutputEvent
impl Freeze for SwitchProOutputEvent
impl RefUnwindSafe for SwitchProOutputEvent
impl Send for SwitchProOutputEvent
impl Sync for SwitchProOutputEvent
impl Unpin for SwitchProOutputEvent
impl UnsafeUnpin for SwitchProOutputEvent
impl UnwindSafe for SwitchProOutputEvent
```

### `TouchSlot`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub enum TouchSlot { First, Second, }
```

Trait implementations:

```rust
impl Clone for TouchSlot
impl Debug for TouchSlot
impl PartialEq for TouchSlot
impl Copy for TouchSlot
impl Eq for TouchSlot
impl StructuralPartialEq for TouchSlot
impl Freeze for TouchSlot
impl RefUnwindSafe for TouchSlot
impl Send for TouchSlot
impl Sync for TouchSlot
impl Unpin for TouchSlot
impl UnsafeUnpin for TouchSlot
impl UnwindSafe for TouchSlot
```

### `TouchpadActuation`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum TouchpadActuation { None, Button(AuxiliaryButtonInput), Deflection { id: InputControlId, range: InputAxisRange, }, }
```

Trait implementations:

```rust
impl Clone for TouchpadActuation
impl Debug for TouchpadActuation
impl PartialEq for TouchpadActuation
impl Copy for TouchpadActuation
impl Eq for TouchpadActuation
impl StructuralPartialEq for TouchpadActuation
impl Freeze for TouchpadActuation
impl RefUnwindSafe for TouchpadActuation
impl Send for TouchpadActuation
impl Sync for TouchpadActuation
impl Unpin for TouchpadActuation
impl UnsafeUnpin for TouchpadActuation
impl UnwindSafe for TouchpadActuation
```

### `TriggerInputKind`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub enum TriggerInputKind { Button { id: InputControlId, }, Axis { id: InputControlId, range: InputAxisRange, }, }
```

Trait implementations:

```rust
impl Clone for TriggerInputKind
impl Debug for TriggerInputKind
impl PartialEq for TriggerInputKind
impl Copy for TriggerInputKind
impl Eq for TriggerInputKind
impl StructuralPartialEq for TriggerInputKind
impl Freeze for TriggerInputKind
impl RefUnwindSafe for TriggerInputKind
impl Send for TriggerInputKind
impl Sync for TriggerInputKind
impl Unpin for TriggerInputKind
impl UnsafeUnpin for TriggerInputKind
impl UnwindSafe for TriggerInputKind
```

### `Xbox360Control`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub enum Xbox360Control { A, B, X, Y, LeftShoulder, RightShoulder, Back, Start, Guide, LeftStickPress, RightStickPress, }
```

Trait implementations:

```rust
impl Clone for Xbox360Control
impl Debug for Xbox360Control
impl PartialEq for Xbox360Control
impl Copy for Xbox360Control
impl Eq for Xbox360Control
impl StructuralPartialEq for Xbox360Control
impl Freeze for Xbox360Control
impl RefUnwindSafe for Xbox360Control
impl Send for Xbox360Control
impl Sync for Xbox360Control
impl Unpin for Xbox360Control
impl UnsafeUnpin for Xbox360Control
impl UnwindSafe for Xbox360Control
```

### `Xbox360OutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum Xbox360OutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput { report_id: Option<u8>, bytes: Vec<u8>, }, }
```

Trait implementations:

```rust
impl Clone for Xbox360OutputEvent
impl Debug for Xbox360OutputEvent
impl PartialEq for Xbox360OutputEvent
impl Eq for Xbox360OutputEvent
impl StructuralPartialEq for Xbox360OutputEvent
impl Freeze for Xbox360OutputEvent
impl RefUnwindSafe for Xbox360OutputEvent
impl Send for Xbox360OutputEvent
impl Sync for Xbox360OutputEvent
impl Unpin for Xbox360OutputEvent
impl UnsafeUnpin for Xbox360OutputEvent
impl UnwindSafe for Xbox360OutputEvent
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

Trait implementations:

```rust
impl Clone for AbsoluteAxisSurface
impl Debug for AbsoluteAxisSurface
impl PartialEq for AbsoluteAxisSurface
impl Copy for AbsoluteAxisSurface
impl Eq for AbsoluteAxisSurface
impl StructuralPartialEq for AbsoluteAxisSurface
impl Freeze for AbsoluteAxisSurface
impl RefUnwindSafe for AbsoluteAxisSurface
impl Send for AbsoluteAxisSurface
impl Sync for AbsoluteAxisSurface
impl Unpin for AbsoluteAxisSurface
impl UnsafeUnpin for AbsoluteAxisSurface
impl UnwindSafe for AbsoluteAxisSurface
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

Trait implementations:

```rust
impl Clone for AudioDiagnostics
impl Debug for AudioDiagnostics
impl PartialEq for AudioDiagnostics
impl Eq for AudioDiagnostics
impl StructuralPartialEq for AudioDiagnostics
impl Freeze for AudioDiagnostics
impl RefUnwindSafe for AudioDiagnostics
impl Send for AudioDiagnostics
impl Sync for AudioDiagnostics
impl Unpin for AudioDiagnostics
impl UnsafeUnpin for AudioDiagnostics
impl UnwindSafe for AudioDiagnostics
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

Trait implementations:

```rust
impl Clone for AudioEndpoint
impl Debug for AudioEndpoint
impl PartialEq for AudioEndpoint
impl Eq for AudioEndpoint
impl StructuralPartialEq for AudioEndpoint
impl Freeze for AudioEndpoint
impl RefUnwindSafe for AudioEndpoint
impl Send for AudioEndpoint
impl Sync for AudioEndpoint
impl Unpin for AudioEndpoint
impl UnsafeUnpin for AudioEndpoint
impl UnwindSafe for AudioEndpoint
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

Trait implementations:

```rust
impl Clone for AudioOptions
impl Debug for AudioOptions
impl Default for AudioOptions
impl PartialEq for AudioOptions
impl Copy for AudioOptions
impl Eq for AudioOptions
impl StructuralPartialEq for AudioOptions
impl Freeze for AudioOptions
impl RefUnwindSafe for AudioOptions
impl Send for AudioOptions
impl Sync for AudioOptions
impl Unpin for AudioOptions
impl UnsafeUnpin for AudioOptions
impl UnwindSafe for AudioOptions
```

### `AudioRead`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
#[non_exhaustive]pub struct AudioRead { pub frames: usize, pub first_frame: u64, pub discontinuity: bool, }
```

Trait implementations:

```rust
impl Clone for AudioRead
impl Debug for AudioRead
impl PartialEq for AudioRead
impl Copy for AudioRead
impl Eq for AudioRead
impl StructuralPartialEq for AudioRead
impl Freeze for AudioRead
impl RefUnwindSafe for AudioRead
impl Send for AudioRead
impl Sync for AudioRead
impl Unpin for AudioRead
impl UnsafeUnpin for AudioRead
impl UnwindSafe for AudioRead
```

### `AuxiliaryButtonInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct AuxiliaryButtonInput { pub id: InputControlId, pub label: &'static str, }
```

Trait implementations:

```rust
impl Clone for AuxiliaryButtonInput
impl Debug for AuxiliaryButtonInput
impl PartialEq for AuxiliaryButtonInput
impl Copy for AuxiliaryButtonInput
impl Eq for AuxiliaryButtonInput
impl StructuralPartialEq for AuxiliaryButtonInput
impl Freeze for AuxiliaryButtonInput
impl RefUnwindSafe for AuxiliaryButtonInput
impl Send for AuxiliaryButtonInput
impl Sync for AuxiliaryButtonInput
impl Unpin for AuxiliaryButtonInput
impl UnsafeUnpin for AuxiliaryButtonInput
impl UnwindSafe for AuxiliaryButtonInput
```

### `BatteryLevel`

Owner/source: `../src/gr_curated_controllers/lib.rs.html`

```rust
pub struct BatteryLevel(/* private fields */);
pub const fn percent(self) -> u8
pub fn new(percent: u8) -> Result<BatteryLevel, ControlError>
```

Trait implementations:

```rust
impl Clone for BatteryLevel
impl Debug for BatteryLevel
impl PartialEq for BatteryLevel
impl Copy for BatteryLevel
impl Eq for BatteryLevel
impl StructuralPartialEq for BatteryLevel
impl Freeze for BatteryLevel
impl RefUnwindSafe for BatteryLevel
impl Send for BatteryLevel
impl Sync for BatteryLevel
impl Unpin for BatteryLevel
impl UnsafeUnpin for BatteryLevel
impl UnwindSafe for BatteryLevel
```

### `BatteryState`

Owner/source: `../src/gr_curated_controllers/lib.rs.html`

```rust
pub struct BatteryState { /* private fields */ }
pub const fn is_exposed(self) -> bool
pub const fn level(self) -> BatteryLevel
```

Trait implementations:

```rust
impl Clone for BatteryState
impl Debug for BatteryState
impl Default for BatteryState
impl PartialEq for BatteryState
impl Copy for BatteryState
impl Eq for BatteryState
impl StructuralPartialEq for BatteryState
impl Freeze for BatteryState
impl RefUnwindSafe for BatteryState
impl Send for BatteryState
impl Sync for BatteryState
impl Unpin for BatteryState
impl UnsafeUnpin for BatteryState
impl UnwindSafe for BatteryState
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

Trait implementations:

```rust
impl Clone for ClusterPlacement
impl Debug for ClusterPlacement
impl Hash for ClusterPlacement
impl PartialEq for ClusterPlacement
impl Copy for ClusterPlacement
impl Eq for ClusterPlacement
impl StructuralPartialEq for ClusterPlacement
impl Freeze for ClusterPlacement
impl RefUnwindSafe for ClusterPlacement
impl Send for ClusterPlacement
impl Sync for ClusterPlacement
impl Unpin for ClusterPlacement
impl UnsafeUnpin for ClusterPlacement
impl UnwindSafe for ClusterPlacement
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

Trait implementations:

```rust
impl Clone for ComponentAssociation
impl Debug for ComponentAssociation
impl PartialEq for ComponentAssociation
impl Eq for ComponentAssociation
impl StructuralPartialEq for ComponentAssociation
impl Freeze for ComponentAssociation
impl RefUnwindSafe for ComponentAssociation
impl Send for ComponentAssociation
impl Sync for ComponentAssociation
impl Unpin for ComponentAssociation
impl UnsafeUnpin for ComponentAssociation
impl UnwindSafe for ComponentAssociation
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

Trait implementations:

```rust
impl Clone for ControllerAssociation
impl Debug for ControllerAssociation
impl PartialEq for ControllerAssociation
impl Eq for ControllerAssociation
impl StructuralPartialEq for ControllerAssociation
impl Freeze for ControllerAssociation
impl RefUnwindSafe for ControllerAssociation
impl Send for ControllerAssociation
impl Sync for ControllerAssociation
impl Unpin for ControllerAssociation
impl UnsafeUnpin for ControllerAssociation
impl UnwindSafe for ControllerAssociation
```

### `ControllerAudio`

Owner/source: `../src/virtualgamepad/audio.rs.html`

```rust
pub struct ControllerAudio { /* private fields */ }
pub fn diagnostics(&self) -> AudioDiagnostics
pub fn endpoints(&self) -> &[AudioEndpoint]
pub const fn onboard_speaker_source(&self) -> Option<AudioChannel>
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

Trait implementations:

```rust
impl Drop for ControllerAudio
impl Freeze for ControllerAudio
impl !RefUnwindSafe for ControllerAudio
impl Send for ControllerAudio
impl Sync for ControllerAudio
impl Unpin for ControllerAudio
impl UnsafeUnpin for ControllerAudio
impl !UnwindSafe for ControllerAudio
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

Trait implementations:

```rust
impl Clone for ControllerDiagnostics
impl Debug for ControllerDiagnostics
impl PartialEq for ControllerDiagnostics
impl Eq for ControllerDiagnostics
impl StructuralPartialEq for ControllerDiagnostics
impl Freeze for ControllerDiagnostics
impl RefUnwindSafe for ControllerDiagnostics
impl Send for ControllerDiagnostics
impl Sync for ControllerDiagnostics
impl Unpin for ControllerDiagnostics
impl UnsafeUnpin for ControllerDiagnostics
impl UnwindSafe for ControllerDiagnostics
```

### `ControllerId`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub struct ControllerId(/* private fields */);
pub const fn new(value: &'static str) -> ControllerId
pub const fn as_str(self) -> &'static str
```

Trait implementations:

```rust
impl Clone for ControllerId
impl Debug for ControllerId
impl Display for ControllerId
impl Hash for ControllerId
impl PartialEq for ControllerId
impl Copy for ControllerId
impl Eq for ControllerId
impl StructuralPartialEq for ControllerId
impl Freeze for ControllerId
impl RefUnwindSafe for ControllerId
impl Send for ControllerId
impl Sync for ControllerId
impl Unpin for ControllerId
impl UnsafeUnpin for ControllerId
impl UnwindSafe for ControllerId
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

Trait implementations:

```rust
impl Clone for ControllerSurface
impl Debug for ControllerSurface
impl PartialEq for ControllerSurface
impl Copy for ControllerSurface
impl Eq for ControllerSurface
impl StructuralPartialEq for ControllerSurface
impl Freeze for ControllerSurface
impl RefUnwindSafe for ControllerSurface
impl Send for ControllerSurface
impl Sync for ControllerSurface
impl Unpin for ControllerSurface
impl UnsafeUnpin for ControllerSurface
impl UnwindSafe for ControllerSurface
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

Trait implementations:

```rust
impl Clone for CreationOptions
impl Debug for CreationOptions
impl PartialEq for CreationOptions
impl Copy for CreationOptions
impl Eq for CreationOptions
impl StructuralPartialEq for CreationOptions
impl Freeze for CreationOptions
impl RefUnwindSafe for CreationOptions
impl Send for CreationOptions
impl Sync for CreationOptions
impl Unpin for CreationOptions
impl UnsafeUnpin for CreationOptions
impl UnwindSafe for CreationOptions
```

### `CustomInputModule`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct CustomInputModule { pub id: InputControlId, pub title: &'static str, }
```

Trait implementations:

```rust
impl Clone for CustomInputModule
impl Debug for CustomInputModule
impl PartialEq for CustomInputModule
impl Copy for CustomInputModule
impl Eq for CustomInputModule
impl StructuralPartialEq for CustomInputModule
impl Freeze for CustomInputModule
impl RefUnwindSafe for CustomInputModule
impl Send for CustomInputModule
impl Sync for CustomInputModule
impl Unpin for CustomInputModule
impl UnsafeUnpin for CustomInputModule
impl UnwindSafe for CustomInputModule
```

### `DigitalControlSurface`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct DigitalControlSurface { pub control: &'static str, pub event_code: u16, }
```

Trait implementations:

```rust
impl Clone for DigitalControlSurface
impl Debug for DigitalControlSurface
impl PartialEq for DigitalControlSurface
impl Copy for DigitalControlSurface
impl Eq for DigitalControlSurface
impl StructuralPartialEq for DigitalControlSurface
impl Freeze for DigitalControlSurface
impl RefUnwindSafe for DigitalControlSurface
impl Send for DigitalControlSurface
impl Sync for DigitalControlSurface
impl Unpin for DigitalControlSurface
impl UnsafeUnpin for DigitalControlSurface
impl UnwindSafe for DigitalControlSurface
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

Trait implementations:

```rust
impl Clone for DpadCluster
impl Debug for DpadCluster
impl PartialEq for DpadCluster
impl Copy for DpadCluster
impl Eq for DpadCluster
impl StructuralPartialEq for DpadCluster
impl Freeze for DpadCluster
impl RefUnwindSafe for DpadCluster
impl Send for DpadCluster
impl Sync for DpadCluster
impl Unpin for DpadCluster
impl UnsafeUnpin for DpadCluster
impl UnwindSafe for DpadCluster
```

### `DualSenseAxis`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseAxis(/* private fields */);
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> DualSenseAxis
pub const fn neutral() -> DualSenseAxis
```

Trait implementations:

```rust
impl Clone for DualSenseAxis
impl Debug for DualSenseAxis
impl PartialEq for DualSenseAxis
impl Copy for DualSenseAxis
impl Eq for DualSenseAxis
impl StructuralPartialEq for DualSenseAxis
impl Freeze for DualSenseAxis
impl RefUnwindSafe for DualSenseAxis
impl Send for DualSenseAxis
impl Sync for DualSenseAxis
impl Unpin for DualSenseAxis
impl UnsafeUnpin for DualSenseAxis
impl UnwindSafe for DualSenseAxis
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

Trait implementations:

```rust
impl Freeze for DualSenseController
impl !RefUnwindSafe for DualSenseController
impl Send for DualSenseController
impl !Sync for DualSenseController
impl Unpin for DualSenseController
impl UnsafeUnpin for DualSenseController
impl !UnwindSafe for DualSenseController
```

### `DualSenseIdentity`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct DualSenseIdentity(/* private fields */);
pub fn generate() -> Result<Self, ControllerError>
pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self>
pub const fn to_bytes(self) -> [u8; 6]
```

Trait implementations:

```rust
impl Clone for DualSenseIdentity
impl Debug for DualSenseIdentity
impl Hash for DualSenseIdentity
impl PartialEq for DualSenseIdentity
impl Copy for DualSenseIdentity
impl Eq for DualSenseIdentity
impl StructuralPartialEq for DualSenseIdentity
impl Freeze for DualSenseIdentity
impl RefUnwindSafe for DualSenseIdentity
impl Send for DualSenseIdentity
impl Sync for DualSenseIdentity
impl Unpin for DualSenseIdentity
impl UnsafeUnpin for DualSenseIdentity
impl UnwindSafe for DualSenseIdentity
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

Trait implementations:

```rust
impl Clone for DualSenseState
impl Debug for DualSenseState
impl Default for DualSenseState
impl PartialEq for DualSenseState
impl Eq for DualSenseState
impl StructuralPartialEq for DualSenseState
impl Freeze for DualSenseState
impl RefUnwindSafe for DualSenseState
impl Send for DualSenseState
impl Sync for DualSenseState
impl Unpin for DualSenseState
impl UnsafeUnpin for DualSenseState
impl UnwindSafe for DualSenseState
```

### `DualSenseSurface`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseSurface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

Trait implementations:

```rust
impl Clone for DualSenseSurface
impl ControllerSurfaceInfo for DualSenseSurface
impl Debug for DualSenseSurface
impl PartialEq for DualSenseSurface
impl Copy for DualSenseSurface
impl Eq for DualSenseSurface
impl StructuralPartialEq for DualSenseSurface
impl Freeze for DualSenseSurface
impl RefUnwindSafe for DualSenseSurface
impl Send for DualSenseSurface
impl Sync for DualSenseSurface
impl Unpin for DualSenseSurface
impl UnsafeUnpin for DualSenseSurface
impl UnwindSafe for DualSenseSurface
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

Trait implementations:

```rust
impl Clone for DualSenseTouchContact
impl Debug for DualSenseTouchContact
impl PartialEq for DualSenseTouchContact
impl Copy for DualSenseTouchContact
impl Eq for DualSenseTouchContact
impl StructuralPartialEq for DualSenseTouchContact
impl Freeze for DualSenseTouchContact
impl RefUnwindSafe for DualSenseTouchContact
impl Send for DualSenseTouchContact
impl Sync for DualSenseTouchContact
impl Unpin for DualSenseTouchContact
impl UnsafeUnpin for DualSenseTouchContact
impl UnwindSafe for DualSenseTouchContact
```

### `DualSenseTrigger`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseTrigger(/* private fields */);
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> DualSenseTrigger
```

Trait implementations:

```rust
impl Clone for DualSenseTrigger
impl Debug for DualSenseTrigger
impl PartialEq for DualSenseTrigger
impl Copy for DualSenseTrigger
impl Eq for DualSenseTrigger
impl StructuralPartialEq for DualSenseTrigger
impl Freeze for DualSenseTrigger
impl RefUnwindSafe for DualSenseTrigger
impl Send for DualSenseTrigger
impl Sync for DualSenseTrigger
impl Unpin for DualSenseTrigger
impl UnsafeUnpin for DualSenseTrigger
impl UnwindSafe for DualSenseTrigger
```

### `DualShock4Axis`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Axis(/* private fields */);
pub const fn new(raw: u8) -> DualShock4Axis
pub const fn raw(self) -> u8
```

Trait implementations:

```rust
impl Clone for DualShock4Axis
impl Debug for DualShock4Axis
impl PartialEq for DualShock4Axis
impl Copy for DualShock4Axis
impl Eq for DualShock4Axis
impl StructuralPartialEq for DualShock4Axis
impl Freeze for DualShock4Axis
impl RefUnwindSafe for DualShock4Axis
impl Send for DualShock4Axis
impl Sync for DualShock4Axis
impl Unpin for DualShock4Axis
impl UnsafeUnpin for DualShock4Axis
impl UnwindSafe for DualShock4Axis
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

Trait implementations:

```rust
impl Freeze for DualShock4Controller
impl !RefUnwindSafe for DualShock4Controller
impl Send for DualShock4Controller
impl !Sync for DualShock4Controller
impl Unpin for DualShock4Controller
impl UnsafeUnpin for DualShock4Controller
impl !UnwindSafe for DualShock4Controller
```

### `DualShock4Identity`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct DualShock4Identity(/* private fields */);
pub fn generate() -> Result<Self, ControllerError>
pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self>
pub const fn to_bytes(self) -> [u8; 6]
```

Trait implementations:

```rust
impl Clone for DualShock4Identity
impl Debug for DualShock4Identity
impl Hash for DualShock4Identity
impl PartialEq for DualShock4Identity
impl Copy for DualShock4Identity
impl Eq for DualShock4Identity
impl StructuralPartialEq for DualShock4Identity
impl Freeze for DualShock4Identity
impl RefUnwindSafe for DualShock4Identity
impl Send for DualShock4Identity
impl Sync for DualShock4Identity
impl Unpin for DualShock4Identity
impl UnsafeUnpin for DualShock4Identity
impl UnwindSafe for DualShock4Identity
```

### `DualShock4MotionSample`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4MotionSample { pub accelerometer: [i16; 3], pub gyroscope: [i16; 3], }
```

Trait implementations:

```rust
impl Clone for DualShock4MotionSample
impl Debug for DualShock4MotionSample
impl PartialEq for DualShock4MotionSample
impl Copy for DualShock4MotionSample
impl Eq for DualShock4MotionSample
impl StructuralPartialEq for DualShock4MotionSample
impl Freeze for DualShock4MotionSample
impl RefUnwindSafe for DualShock4MotionSample
impl Send for DualShock4MotionSample
impl Sync for DualShock4MotionSample
impl Unpin for DualShock4MotionSample
impl UnsafeUnpin for DualShock4MotionSample
impl UnwindSafe for DualShock4MotionSample
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

Trait implementations:

```rust
impl Clone for DualShock4State
impl Debug for DualShock4State
impl Default for DualShock4State
impl PartialEq for DualShock4State
impl Eq for DualShock4State
impl StructuralPartialEq for DualShock4State
impl Freeze for DualShock4State
impl RefUnwindSafe for DualShock4State
impl Send for DualShock4State
impl Sync for DualShock4State
impl Unpin for DualShock4State
impl UnsafeUnpin for DualShock4State
impl UnwindSafe for DualShock4State
```

### `DualShock4Surface`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Surface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

Trait implementations:

```rust
impl Clone for DualShock4Surface
impl ControllerSurfaceInfo for DualShock4Surface
impl Debug for DualShock4Surface
impl PartialEq for DualShock4Surface
impl Copy for DualShock4Surface
impl Eq for DualShock4Surface
impl StructuralPartialEq for DualShock4Surface
impl Freeze for DualShock4Surface
impl RefUnwindSafe for DualShock4Surface
impl Send for DualShock4Surface
impl Sync for DualShock4Surface
impl Unpin for DualShock4Surface
impl UnsafeUnpin for DualShock4Surface
impl UnwindSafe for DualShock4Surface
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

Trait implementations:

```rust
impl Clone for DualShock4TouchContact
impl Debug for DualShock4TouchContact
impl PartialEq for DualShock4TouchContact
impl Copy for DualShock4TouchContact
impl Eq for DualShock4TouchContact
impl StructuralPartialEq for DualShock4TouchContact
impl Freeze for DualShock4TouchContact
impl RefUnwindSafe for DualShock4TouchContact
impl Send for DualShock4TouchContact
impl Sync for DualShock4TouchContact
impl Unpin for DualShock4TouchContact
impl UnsafeUnpin for DualShock4TouchContact
impl UnwindSafe for DualShock4TouchContact
```

### `DualShock4Trigger`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Trigger(/* private fields */);
pub const fn new(raw: u8) -> DualShock4Trigger
pub const fn raw(self) -> u8
```

Trait implementations:

```rust
impl Clone for DualShock4Trigger
impl Debug for DualShock4Trigger
impl PartialEq for DualShock4Trigger
impl Copy for DualShock4Trigger
impl Eq for DualShock4Trigger
impl StructuralPartialEq for DualShock4Trigger
impl Freeze for DualShock4Trigger
impl RefUnwindSafe for DualShock4Trigger
impl Send for DualShock4Trigger
impl Sync for DualShock4Trigger
impl Unpin for DualShock4Trigger
impl UnsafeUnpin for DualShock4Trigger
impl UnwindSafe for DualShock4Trigger
```

### `FaceButtonCluster`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct FaceButtonCluster { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn buttons(&self) -> &'static [FaceButtonInput]
```

Trait implementations:

```rust
impl Clone for FaceButtonCluster
impl Debug for FaceButtonCluster
impl PartialEq for FaceButtonCluster
impl Copy for FaceButtonCluster
impl Eq for FaceButtonCluster
impl StructuralPartialEq for FaceButtonCluster
impl Freeze for FaceButtonCluster
impl RefUnwindSafe for FaceButtonCluster
impl Send for FaceButtonCluster
impl Sync for FaceButtonCluster
impl Unpin for FaceButtonCluster
impl UnsafeUnpin for FaceButtonCluster
impl UnwindSafe for FaceButtonCluster
```

### `FaceButtonInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct FaceButtonInput { pub button: FaceButton, pub label: &'static str, pub placement: ClusterPlacement, }
```

Trait implementations:

```rust
impl Clone for FaceButtonInput
impl Debug for FaceButtonInput
impl PartialEq for FaceButtonInput
impl Copy for FaceButtonInput
impl Eq for FaceButtonInput
impl StructuralPartialEq for FaceButtonInput
impl Freeze for FaceButtonInput
impl RefUnwindSafe for FaceButtonInput
impl Send for FaceButtonInput
impl Sync for FaceButtonInput
impl Unpin for FaceButtonInput
impl UnsafeUnpin for FaceButtonInput
impl UnwindSafe for FaceButtonInput
```

### `InputAxisRange`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct InputAxisRange { pub minimum: i32, pub maximum: i32, pub neutral: i32, }
pub const fn is_valid(self) -> bool
```

Trait implementations:

```rust
impl Clone for InputAxisRange
impl Debug for InputAxisRange
impl PartialEq for InputAxisRange
impl Copy for InputAxisRange
impl Eq for InputAxisRange
impl StructuralPartialEq for InputAxisRange
impl Freeze for InputAxisRange
impl RefUnwindSafe for InputAxisRange
impl Send for InputAxisRange
impl Sync for InputAxisRange
impl Unpin for InputAxisRange
impl UnsafeUnpin for InputAxisRange
impl UnwindSafe for InputAxisRange
```

### `InputControlId`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct InputControlId(/* private fields */);
pub const fn new(value: &'static str) -> InputControlId
pub const fn as_str(self) -> &'static str
```

Trait implementations:

```rust
impl Clone for InputControlId
impl Debug for InputControlId
impl Hash for InputControlId
impl PartialEq for InputControlId
impl Copy for InputControlId
impl Eq for InputControlId
impl StructuralPartialEq for InputControlId
impl Freeze for InputControlId
impl RefUnwindSafe for InputControlId
impl Send for InputControlId
impl Sync for InputControlId
impl Unpin for InputControlId
impl UnsafeUnpin for InputControlId
impl UnwindSafe for InputControlId
```

### `InputScale`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct InputScale { pub numerator: i32, pub denominator: u32, }
pub const IDENTITY: InputScale
```

Trait implementations:

```rust
impl Clone for InputScale
impl Debug for InputScale
impl PartialEq for InputScale
impl Copy for InputScale
impl Eq for InputScale
impl StructuralPartialEq for InputScale
impl Freeze for InputScale
impl RefUnwindSafe for InputScale
impl Send for InputScale
impl Sync for InputScale
impl Unpin for InputScale
impl UnsafeUnpin for InputScale
impl UnwindSafe for InputScale
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

Trait implementations:

```rust
impl Clone for InputTopology
impl Debug for InputTopology
impl PartialEq for InputTopology
impl Copy for InputTopology
impl Eq for InputTopology
impl StructuralPartialEq for InputTopology
impl Freeze for InputTopology
impl RefUnwindSafe for InputTopology
impl Send for InputTopology
impl Sync for InputTopology
impl Unpin for InputTopology
impl UnsafeUnpin for InputTopology
impl UnwindSafe for InputTopology
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

Trait implementations:

```rust
impl Clone for MotionInput
impl Debug for MotionInput
impl PartialEq for MotionInput
impl Copy for MotionInput
impl Eq for MotionInput
impl StructuralPartialEq for MotionInput
impl Freeze for MotionInput
impl RefUnwindSafe for MotionInput
impl Send for MotionInput
impl Sync for MotionInput
impl Unpin for MotionInput
impl UnsafeUnpin for MotionInput
impl UnwindSafe for MotionInput
```

### `MotionSample`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct MotionSample { pub accelerometer: [i16; 3], pub gyroscope: [i16; 3], }
```

Trait implementations:

```rust
impl Clone for MotionSample
impl Debug for MotionSample
impl PartialEq for MotionSample
impl Copy for MotionSample
impl Eq for MotionSample
impl StructuralPartialEq for MotionSample
impl Freeze for MotionSample
impl RefUnwindSafe for MotionSample
impl Send for MotionSample
impl Sync for MotionSample
impl Unpin for MotionSample
impl UnsafeUnpin for MotionSample
impl UnwindSafe for MotionSample
```

### `OutputSurface`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct OutputSurface { pub name: &'static str, pub event_type: u16, pub event_code: u16, }
```

Trait implementations:

```rust
impl Clone for OutputSurface
impl Debug for OutputSurface
impl PartialEq for OutputSurface
impl Copy for OutputSurface
impl Eq for OutputSurface
impl StructuralPartialEq for OutputSurface
impl Freeze for OutputSurface
impl RefUnwindSafe for OutputSurface
impl Send for OutputSurface
impl Sync for OutputSurface
impl Unpin for OutputSurface
impl UnsafeUnpin for OutputSurface
impl UnwindSafe for OutputSurface
```

### `PcmFormat`

Owner/source: `../src/gr_audio_contract/pcm.rs.html`

```rust
pub struct PcmFormat { /* private fields */ }
pub fn new( rate: u32, channels: &[AudioChannel], ) -> Result<PcmFormat, AudioError>
pub const fn sample_rate_hz(&self) -> u32
pub fn channels(&self) -> &[AudioChannel]
```

Trait implementations:

```rust
impl Clone for PcmFormat
impl Debug for PcmFormat
impl PartialEq for PcmFormat
impl Eq for PcmFormat
impl StructuralPartialEq for PcmFormat
impl Freeze for PcmFormat
impl RefUnwindSafe for PcmFormat
impl Send for PcmFormat
impl Sync for PcmFormat
impl Unpin for PcmFormat
impl UnsafeUnpin for PcmFormat
impl UnwindSafe for PcmFormat
```

### `ReadinessDescriptor`

Owner/source: `../src/virtualgamepad/application.rs.html`

```rust
pub struct ReadinessDescriptor<'a> { /* private fields */ }
pub const fn raw_fd(&self) -> i32
```

Trait implementations:

```rust
impl<'a> Debug for ReadinessDescriptor<'a>
impl<'a> Freeze for ReadinessDescriptor<'a>
impl<'a> RefUnwindSafe for ReadinessDescriptor<'a>
impl<'a> Send for ReadinessDescriptor<'a>
impl<'a> Sync for ReadinessDescriptor<'a>
impl<'a> Unpin for ReadinessDescriptor<'a>
impl<'a> UnsafeUnpin for ReadinessDescriptor<'a>
impl<'a> UnwindSafe for ReadinessDescriptor<'a>
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

Trait implementations:

```rust
impl Clone for RealizationId
impl Debug for RealizationId
impl Display for RealizationId
impl Hash for RealizationId
impl Ord for RealizationId
impl PartialEq for RealizationId
impl PartialOrd for RealizationId
impl Copy for RealizationId
impl Eq for RealizationId
impl StructuralPartialEq for RealizationId
impl Freeze for RealizationId
impl RefUnwindSafe for RealizationId
impl Send for RealizationId
impl Sync for RealizationId
impl Unpin for RealizationId
impl UnsafeUnpin for RealizationId
impl UnwindSafe for RealizationId
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

Trait implementations:

```rust
impl Clone for RealizationTargetSet
impl Debug for RealizationTargetSet
impl Default for RealizationTargetSet
impl Hash for RealizationTargetSet
impl PartialEq for RealizationTargetSet
impl Copy for RealizationTargetSet
impl Eq for RealizationTargetSet
impl Freeze for RealizationTargetSet
impl RefUnwindSafe for RealizationTargetSet
impl Send for RealizationTargetSet
impl Sync for RealizationTargetSet
impl Unpin for RealizationTargetSet
impl UnsafeUnpin for RealizationTargetSet
impl UnwindSafe for RealizationTargetSet
```

### `RumbleEffect`

Owner/source: `../src/gr_realization_api/lib.rs.html`

```rust
pub struct RumbleEffect { pub id: i16, pub strong: u16, pub weak: u16, pub length_ms: u16, pub delay_ms: u16, pub trigger_button: u16, pub trigger_interval_ms: u16, }
```

Trait implementations:

```rust
impl Clone for RumbleEffect
impl Debug for RumbleEffect
impl PartialEq for RumbleEffect
impl Copy for RumbleEffect
impl Eq for RumbleEffect
impl StructuralPartialEq for RumbleEffect
impl Freeze for RumbleEffect
impl RefUnwindSafe for RumbleEffect
impl Send for RumbleEffect
impl Sync for RumbleEffect
impl Unpin for RumbleEffect
impl UnsafeUnpin for RumbleEffect
impl UnwindSafe for RumbleEffect
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

Trait implementations:

```rust
impl Clone for StickInput
impl Debug for StickInput
impl PartialEq for StickInput
impl Copy for StickInput
impl Eq for StickInput
impl StructuralPartialEq for StickInput
impl Freeze for StickInput
impl RefUnwindSafe for StickInput
impl Send for StickInput
impl Sync for StickInput
impl Unpin for StickInput
impl UnsafeUnpin for StickInput
impl UnwindSafe for StickInput
```

### `SwitchProAxis`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProAxis(/* private fields */);
pub const fn new(raw: i16) -> SwitchProAxis
pub const fn raw(self) -> i16
```

Trait implementations:

```rust
impl Clone for SwitchProAxis
impl Debug for SwitchProAxis
impl PartialEq for SwitchProAxis
impl Copy for SwitchProAxis
impl Eq for SwitchProAxis
impl StructuralPartialEq for SwitchProAxis
impl Freeze for SwitchProAxis
impl RefUnwindSafe for SwitchProAxis
impl Send for SwitchProAxis
impl Sync for SwitchProAxis
impl Unpin for SwitchProAxis
impl UnsafeUnpin for SwitchProAxis
impl UnwindSafe for SwitchProAxis
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

Trait implementations:

```rust
impl Freeze for SwitchProController
impl !RefUnwindSafe for SwitchProController
impl Send for SwitchProController
impl !Sync for SwitchProController
impl Unpin for SwitchProController
impl UnsafeUnpin for SwitchProController
impl !UnwindSafe for SwitchProController
```

### `SwitchProMotionSample`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProMotionSample { pub accelerometer: [i16; 3], pub gyroscope: [i16; 3], }
```

Trait implementations:

```rust
impl Clone for SwitchProMotionSample
impl Debug for SwitchProMotionSample
impl PartialEq for SwitchProMotionSample
impl Copy for SwitchProMotionSample
impl Eq for SwitchProMotionSample
impl StructuralPartialEq for SwitchProMotionSample
impl Freeze for SwitchProMotionSample
impl RefUnwindSafe for SwitchProMotionSample
impl Send for SwitchProMotionSample
impl Sync for SwitchProMotionSample
impl Unpin for SwitchProMotionSample
impl UnsafeUnpin for SwitchProMotionSample
impl UnwindSafe for SwitchProMotionSample
```

### `SwitchProRumble`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProRumble { pub packet_counter: u8, pub left: [u8; 4], pub right: [u8; 4], }
```

Trait implementations:

```rust
impl Clone for SwitchProRumble
impl Debug for SwitchProRumble
impl PartialEq for SwitchProRumble
impl Copy for SwitchProRumble
impl Eq for SwitchProRumble
impl StructuralPartialEq for SwitchProRumble
impl Freeze for SwitchProRumble
impl RefUnwindSafe for SwitchProRumble
impl Send for SwitchProRumble
impl Sync for SwitchProRumble
impl Unpin for SwitchProRumble
impl UnsafeUnpin for SwitchProRumble
impl UnwindSafe for SwitchProRumble
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

Trait implementations:

```rust
impl Clone for SwitchProState
impl Debug for SwitchProState
impl Default for SwitchProState
impl PartialEq for SwitchProState
impl Eq for SwitchProState
impl StructuralPartialEq for SwitchProState
impl Freeze for SwitchProState
impl RefUnwindSafe for SwitchProState
impl Send for SwitchProState
impl Sync for SwitchProState
impl Unpin for SwitchProState
impl UnsafeUnpin for SwitchProState
impl UnwindSafe for SwitchProState
```

### `SwitchProSurface`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProSurface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

Trait implementations:

```rust
impl Clone for SwitchProSurface
impl ControllerSurfaceInfo for SwitchProSurface
impl Debug for SwitchProSurface
impl PartialEq for SwitchProSurface
impl Copy for SwitchProSurface
impl Eq for SwitchProSurface
impl StructuralPartialEq for SwitchProSurface
impl Freeze for SwitchProSurface
impl RefUnwindSafe for SwitchProSurface
impl Send for SwitchProSurface
impl Sync for SwitchProSurface
impl Unpin for SwitchProSurface
impl UnsafeUnpin for SwitchProSurface
impl UnwindSafe for SwitchProSurface
```

### `TargetRestriction`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct TargetRestriction { pub feature: &'static str, pub reason: &'static str, }
```

Trait implementations:

```rust
impl Clone for TargetRestriction
impl Debug for TargetRestriction
impl PartialEq for TargetRestriction
impl Copy for TargetRestriction
impl Eq for TargetRestriction
impl StructuralPartialEq for TargetRestriction
impl Freeze for TargetRestriction
impl RefUnwindSafe for TargetRestriction
impl Send for TargetRestriction
impl Sync for TargetRestriction
impl Unpin for TargetRestriction
impl UnsafeUnpin for TargetRestriction
impl UnwindSafe for TargetRestriction
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

Trait implementations:

```rust
impl Clone for TouchpadInput
impl Debug for TouchpadInput
impl PartialEq for TouchpadInput
impl Copy for TouchpadInput
impl Eq for TouchpadInput
impl StructuralPartialEq for TouchpadInput
impl Freeze for TouchpadInput
impl RefUnwindSafe for TouchpadInput
impl Send for TouchpadInput
impl Sync for TouchpadInput
impl Unpin for TouchpadInput
impl UnsafeUnpin for TouchpadInput
impl UnwindSafe for TouchpadInput
```

### `TriggerInput`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct TriggerInput { pub label: &'static str, pub kind: TriggerInputKind, }
```

Trait implementations:

```rust
impl Clone for TriggerInput
impl Debug for TriggerInput
impl PartialEq for TriggerInput
impl Copy for TriggerInput
impl Eq for TriggerInput
impl StructuralPartialEq for TriggerInput
impl Freeze for TriggerInput
impl RefUnwindSafe for TriggerInput
impl Send for TriggerInput
impl Sync for TriggerInput
impl Unpin for TriggerInput
impl UnsafeUnpin for TriggerInput
impl UnwindSafe for TriggerInput
```

### `TriggerStack`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub struct TriggerStack { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn controls(&self) -> &'static [TriggerInput]
```

Trait implementations:

```rust
impl Clone for TriggerStack
impl Debug for TriggerStack
impl PartialEq for TriggerStack
impl Copy for TriggerStack
impl Eq for TriggerStack
impl StructuralPartialEq for TriggerStack
impl Freeze for TriggerStack
impl RefUnwindSafe for TriggerStack
impl Send for TriggerStack
impl Sync for TriggerStack
impl Unpin for TriggerStack
impl UnsafeUnpin for TriggerStack
impl UnwindSafe for TriggerStack
```

### `Xbox360Axis`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Axis(/* private fields */);
pub const fn raw(self) -> i16
pub const fn new(raw: i16) -> Xbox360Axis
```

Trait implementations:

```rust
impl Clone for Xbox360Axis
impl Debug for Xbox360Axis
impl PartialEq for Xbox360Axis
impl Copy for Xbox360Axis
impl Eq for Xbox360Axis
impl StructuralPartialEq for Xbox360Axis
impl Freeze for Xbox360Axis
impl RefUnwindSafe for Xbox360Axis
impl Send for Xbox360Axis
impl Sync for Xbox360Axis
impl Unpin for Xbox360Axis
impl UnsafeUnpin for Xbox360Axis
impl UnwindSafe for Xbox360Axis
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

Trait implementations:

```rust
impl Freeze for Xbox360Controller
impl !RefUnwindSafe for Xbox360Controller
impl Send for Xbox360Controller
impl !Sync for Xbox360Controller
impl Unpin for Xbox360Controller
impl UnsafeUnpin for Xbox360Controller
impl !UnwindSafe for Xbox360Controller
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

Trait implementations:

```rust
impl Clone for Xbox360State
impl Debug for Xbox360State
impl Default for Xbox360State
impl PartialEq for Xbox360State
impl Eq for Xbox360State
impl StructuralPartialEq for Xbox360State
impl Freeze for Xbox360State
impl RefUnwindSafe for Xbox360State
impl Send for Xbox360State
impl Sync for Xbox360State
impl Unpin for Xbox360State
impl UnsafeUnpin for Xbox360State
impl UnwindSafe for Xbox360State
```

### `Xbox360Surface`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Surface { /* private fields */ }
pub const fn common(&self) -> &ControllerSurface
```

Trait implementations:

```rust
impl Clone for Xbox360Surface
impl ControllerSurfaceInfo for Xbox360Surface
impl Debug for Xbox360Surface
impl PartialEq for Xbox360Surface
impl Copy for Xbox360Surface
impl Eq for Xbox360Surface
impl StructuralPartialEq for Xbox360Surface
impl Freeze for Xbox360Surface
impl RefUnwindSafe for Xbox360Surface
impl Send for Xbox360Surface
impl Sync for Xbox360Surface
impl Unpin for Xbox360Surface
impl UnsafeUnpin for Xbox360Surface
impl UnwindSafe for Xbox360Surface
```

### `Xbox360Trigger`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Trigger(/* private fields */);
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> Xbox360Trigger
```

Trait implementations:

```rust
impl Clone for Xbox360Trigger
impl Debug for Xbox360Trigger
impl PartialEq for Xbox360Trigger
impl Copy for Xbox360Trigger
impl Eq for Xbox360Trigger
impl StructuralPartialEq for Xbox360Trigger
impl Freeze for Xbox360Trigger
impl RefUnwindSafe for Xbox360Trigger
impl Send for Xbox360Trigger
impl Sync for Xbox360Trigger
impl Unpin for Xbox360Trigger
impl UnsafeUnpin for Xbox360Trigger
impl UnwindSafe for Xbox360Trigger
```

### `ControllerSurfaceInfo`

Owner/source: `../src/gr_controller_contract/lib.rs.html`

```rust
pub trait ControllerSurfaceInfo { // Required method fn common_surface(&self) -> &ControllerSurface; }
```

Trait implementations:

```rust
impl ControllerSurfaceInfo for DualSenseSurface
impl ControllerSurfaceInfo for DualShock4Surface
impl ControllerSurfaceInfo for SwitchProSurface
impl ControllerSurfaceInfo for Xbox360Surface
```
