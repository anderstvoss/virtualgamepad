# Working root API inventory

Baseline: final VG-1b a–d API through #390 (`a0398b7`); step e reconciles consumers and docs.
Generated from Linux all-feature rustdoc. Supporting crates and
`experimental` are outside the alpha application compatibility promise.
This inventory records final input/status names; no new compatibility freeze is declared.
Ordinary-root declarations are available with every feature set on all
compiling platforms. Provider creation is separately gated by Linux,
features and host prerequisites; see [the historical audit](../history/alpha-1/ALPHA_ROOT_API_AUDIT.md).

## Review decisions

| Group | Decision / rationale | Availability / future stress |
| --- | --- | --- |
| Native controller inputs, status, controls and output | Keep: package-owned native semantics | Seven realized families; input levels and status remain separate |
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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

### `DisplayFormat`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub enum DisplayFormat { Monochrome1, Rgb888, }
```

Trait implementations:

```rust
impl Clone for DisplayFormat
impl Debug for DisplayFormat
impl PartialEq for DisplayFormat
impl Copy for DisplayFormat
impl Eq for DisplayFormat
impl StructuralPartialEq for DisplayFormat
impl Freeze for DisplayFormat
impl RefUnwindSafe for DisplayFormat
impl Send for DisplayFormat
impl Sync for DisplayFormat
impl Unpin for DisplayFormat
impl UnsafeUnpin for DisplayFormat
impl UnwindSafe for DisplayFormat
```

### `DpadDirection`

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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
pub enum DualSenseFeature { Touch, Motion, Lightbar, PlayerIndicators, MicrophoneLed, AdaptiveTriggers, Audio, }
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
#[non_exhaustive]pub enum DualSenseHidOutput { UsbOutput {Show 20 fields raw: Vec<u8>, valid_flag0: u8, valid_flag1: u8, valid_flag2: u8, right_motor: Option<u8>, left_motor: Option<u8>, right_trigger_effect: [u8; 11], left_trigger_effect: [u8; 11], mute_button_led: Option<DualSenseMuteLed>, microphone_muted: Option<bool>, audio_path: Option<DualSenseAudioPath>, speaker_volume: Option<u8>, microphone_volume: Option<u8>, speaker_preamp: Option<u8>, led_brightness: u8, lightbar_setup: Option<u8>, release_leds: Option<bool>, player_leds: Option<u8>, player_led_instant: Option<bool>, lightbar_rgb: Option<[u8; 3]>, }, Unknown { report_id: Option<u8>, raw: Vec<u8>, }, }
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

### `DualSenseMuteLed`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub enum DualSenseMuteLed { Off, On, Pulse, Unknown(u8), }
```

Trait implementations:

```rust
impl Clone for DualSenseMuteLed
impl Debug for DualSenseMuteLed
impl PartialEq for DualSenseMuteLed
impl Copy for DualSenseMuteLed
impl Eq for DualSenseMuteLed
impl StructuralPartialEq for DualSenseMuteLed
impl Freeze for DualSenseMuteLed
impl RefUnwindSafe for DualSenseMuteLed
impl Send for DualSenseMuteLed
impl Sync for DualSenseMuteLed
impl Unpin for DualSenseMuteLed
impl UnsafeUnpin for DualSenseMuteLed
impl UnwindSafe for DualSenseMuteLed
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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

### `InputKind`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub enum InputKind { PressureButton { range: InputAxisRange, }, CapacitiveTouch, Relative2D { x: InputAxisRange, y: InputAxisRange, }, PointSet { points: u8, x: InputAxisRange, y: InputAxisRange, }, LoadCell { range: InputAxisRange, unit: &'static str, }, }
```

Trait implementations:

```rust
impl Clone for InputKind
impl Debug for InputKind
impl PartialEq for InputKind
impl Copy for InputKind
impl Eq for InputKind
impl StructuralPartialEq for InputKind
impl Freeze for InputKind
impl RefUnwindSafe for InputKind
impl Send for InputKind
impl Sync for InputKind
impl Unpin for InputKind
impl UnsafeUnpin for InputKind
impl UnwindSafe for InputKind
```

### `InputTopologyError`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub enum InputTopologyError { EmptyIdentifier, DuplicateIdentifier(InputControlId), DuplicatePlacement(InputControlId), InvalidPlacement(InputControlId), InvalidAxisRange(InputControlId), InvalidTouchpad(InputControlId), InvalidTypedInput(InputControlId), InvalidScale(InputControlId), }
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

### `LedKind`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub enum LedKind { Mono, Rgb, }
```

Trait implementations:

```rust
impl Clone for LedKind
impl Debug for LedKind
impl PartialEq for LedKind
impl Copy for LedKind
impl Eq for LedKind
impl StructuralPartialEq for LedKind
impl Freeze for LedKind
impl RefUnwindSafe for LedKind
impl Send for LedKind
impl Sync for LedKind
impl Unpin for LedKind
impl UnsafeUnpin for LedKind
impl UnwindSafe for LedKind
```

### `MotorRole`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub enum MotorRole { Left, Right, Strong, Weak, LeftTrigger, RightTrigger, }
```

Trait implementations:

```rust
impl Clone for MotorRole
impl Debug for MotorRole
impl PartialEq for MotorRole
impl Copy for MotorRole
impl Eq for MotorRole
impl StructuralPartialEq for MotorRole
impl Freeze for MotorRole
impl RefUnwindSafe for MotorRole
impl Send for MotorRole
impl Sync for MotorRole
impl Unpin for MotorRole
impl UnsafeUnpin for MotorRole
impl UnwindSafe for MotorRole
```

### `OutputKind`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub enum OutputKind { Rumble { motors: &'static [MotorRole], }, ImpulseTrigger, Led { kind: LedKind, }, PlayerIndicator { count: u8, }, AdaptiveTrigger, Display { w: u32, h: u32, format: DisplayFormat, }, Speaker { format: SpeakerFormat, }, }
```

Trait implementations:

```rust
impl Clone for OutputKind
impl Debug for OutputKind
impl PartialEq for OutputKind
impl Copy for OutputKind
impl Eq for OutputKind
impl StructuralPartialEq for OutputKind
impl Freeze for OutputKind
impl RefUnwindSafe for OutputKind
impl Send for OutputKind
impl Sync for OutputKind
impl Unpin for OutputKind
impl UnsafeUnpin for OutputKind
impl UnwindSafe for OutputKind
```

### `RealizationValidationStatus`

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

### `SpeakerFormat`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub enum SpeakerFormat { Signed16Le, Unsigned8, }
```

Trait implementations:

```rust
impl Clone for SpeakerFormat
impl Debug for SpeakerFormat
impl PartialEq for SpeakerFormat
impl Copy for SpeakerFormat
impl Eq for SpeakerFormat
impl StructuralPartialEq for SpeakerFormat
impl Freeze for SpeakerFormat
impl RefUnwindSafe for SpeakerFormat
impl Send for SpeakerFormat
impl Sync for SpeakerFormat
impl Unpin for SpeakerFormat
impl UnsafeUnpin for SpeakerFormat
impl UnwindSafe for SpeakerFormat
```

### `SteamController2026Button`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026.rs.html`

```rust
#[non_exhaustive]pub enum SteamController2026Button { Show 22 variants A, B, X, Y, Qam, RightStick, Menu, R4, R5, RightShoulder, Down, Right, Left, Up, View, LeftStick, Steam, L4, L5, LeftShoulder, RightTrigger, LeftTrigger, }
```

Trait implementations:

```rust
impl Clone for SteamController2026Button
impl Debug for SteamController2026Button
impl PartialEq for SteamController2026Button
impl Copy for SteamController2026Button
impl Eq for SteamController2026Button
impl StructuralPartialEq for SteamController2026Button
impl Freeze for SteamController2026Button
impl RefUnwindSafe for SteamController2026Button
impl Send for SteamController2026Button
impl Sync for SteamController2026Button
impl Unpin for SteamController2026Button
impl UnsafeUnpin for SteamController2026Button
impl UnwindSafe for SteamController2026Button
```

### `SteamController2026Haptic`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026/codec.rs.html`

```rust
#[non_exhaustive]pub enum SteamController2026Haptic { Stream(Box<SteamController2026StreamPacket>), Rumble { kind: u8, intensity: u16, left_speed: u16, left_gain: u8, right_speed: u16, right_gain: u8, }, Pulse { side: u8, on_us: u16, off_us: u16, count: u16, }, Command { side: u8, command: u8, gain_db: i8, }, Lfo { side: u8, gain_db: i8, frequency: u16, duration_ms: u16, lfo_frequency: u16, depth: u8, }, Sweep { side: u8, gain_db: i8, duration_ms: u16, start: u16, end: u16, }, Script { side: u8, script: u8, gain_db: i8, }, }
```

Trait implementations:

```rust
impl Clone for SteamController2026Haptic
impl Debug for SteamController2026Haptic
impl PartialEq for SteamController2026Haptic
impl Eq for SteamController2026Haptic
impl StructuralPartialEq for SteamController2026Haptic
impl Freeze for SteamController2026Haptic
impl RefUnwindSafe for SteamController2026Haptic
impl Send for SteamController2026Haptic
impl Sync for SteamController2026Haptic
impl Unpin for SteamController2026Haptic
impl UnsafeUnpin for SteamController2026Haptic
impl UnwindSafe for SteamController2026Haptic
```

### `SteamController2026OutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum SteamController2026OutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput { report_id: Option<u8>, bytes: Vec<u8>, }, }
```

Trait implementations:

```rust
impl Clone for SteamController2026OutputEvent
impl Debug for SteamController2026OutputEvent
impl PartialEq for SteamController2026OutputEvent
impl Eq for SteamController2026OutputEvent
impl StructuralPartialEq for SteamController2026OutputEvent
impl Freeze for SteamController2026OutputEvent
impl RefUnwindSafe for SteamController2026OutputEvent
impl Send for SteamController2026OutputEvent
impl Sync for SteamController2026OutputEvent
impl Unpin for SteamController2026OutputEvent
impl UnsafeUnpin for SteamController2026OutputEvent
impl UnwindSafe for SteamController2026OutputEvent
```

### `SteamController2026StreamPacket`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026/streaming.rs.html`

```rust
pub enum SteamController2026StreamPacket { Configure { operation: u8, target: u8, mode: u8, }, Mono { target: u8, data: Vec<u8>, }, Stereo { left: Vec<u8>, right: Vec<u8>, }, }
pub fn from_output( report_id: u8, payload: &[u8], ) -> Option<SteamController2026StreamPacket>
```

Trait implementations:

```rust
impl Clone for SteamController2026StreamPacket
impl Debug for SteamController2026StreamPacket
impl PartialEq for SteamController2026StreamPacket
impl Eq for SteamController2026StreamPacket
impl StructuralPartialEq for SteamController2026StreamPacket
impl Freeze for SteamController2026StreamPacket
impl RefUnwindSafe for SteamController2026StreamPacket
impl Send for SteamController2026StreamPacket
impl Sync for SteamController2026StreamPacket
impl Unpin for SteamController2026StreamPacket
impl UnsafeUnpin for SteamController2026StreamPacket
impl UnwindSafe for SteamController2026StreamPacket
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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

### `WiiExtension`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
#[non_exhaustive]pub enum WiiExtension { None, Nunchuk(WiiNunchukInputs), Classic(WiiClassicInputs), ClassicPro(WiiClassicInputs), }
```

Trait implementations:

```rust
impl Clone for WiiExtension
impl Debug for WiiExtension
impl Default for WiiExtension
impl PartialEq for WiiExtension
impl Eq for WiiExtension
impl StructuralPartialEq for WiiExtension
impl Freeze for WiiExtension
impl RefUnwindSafe for WiiExtension
impl Send for WiiExtension
impl Sync for WiiExtension
impl Unpin for WiiExtension
impl UnsafeUnpin for WiiExtension
impl UnwindSafe for WiiExtension
```

### `WiiRemoteButton`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
#[non_exhaustive]pub enum WiiRemoteButton { Left, Right, Up, Down, Plus, Minus, One, Two, A, B, Home, }
```

Trait implementations:

```rust
impl Clone for WiiRemoteButton
impl Debug for WiiRemoteButton
impl PartialEq for WiiRemoteButton
impl Copy for WiiRemoteButton
impl Eq for WiiRemoteButton
impl StructuralPartialEq for WiiRemoteButton
impl Freeze for WiiRemoteButton
impl RefUnwindSafe for WiiRemoteButton
impl Send for WiiRemoteButton
impl Sync for WiiRemoteButton
impl Unpin for WiiRemoteButton
impl UnsafeUnpin for WiiRemoteButton
impl UnwindSafe for WiiRemoteButton
```

### `WiiRemoteOutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum WiiRemoteOutputEvent { HostLifecycle(HostLifecycle), ForceFeedback(ForceFeedbackEvent), HidOutput { report_id: Option<u8>, bytes: Vec<u8>, }, }
```

Trait implementations:

```rust
impl Clone for WiiRemoteOutputEvent
impl Debug for WiiRemoteOutputEvent
impl PartialEq for WiiRemoteOutputEvent
impl Eq for WiiRemoteOutputEvent
impl StructuralPartialEq for WiiRemoteOutputEvent
impl Freeze for WiiRemoteOutputEvent
impl RefUnwindSafe for WiiRemoteOutputEvent
impl Send for WiiRemoteOutputEvent
impl Sync for WiiRemoteOutputEvent
impl Unpin for WiiRemoteOutputEvent
impl UnsafeUnpin for WiiRemoteOutputEvent
impl UnwindSafe for WiiRemoteOutputEvent
```

### `WiiRemoteRevision`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
#[non_exhaustive]pub enum WiiRemoteRevision { Original, EarlyIntegratedPlus, Tr, }
pub const fn has_identity_evidence(self) -> bool
```

Trait implementations:

```rust
impl Clone for WiiRemoteRevision
impl Debug for WiiRemoteRevision
impl Default for WiiRemoteRevision
impl PartialEq for WiiRemoteRevision
impl Copy for WiiRemoteRevision
impl Eq for WiiRemoteRevision
impl StructuralPartialEq for WiiRemoteRevision
impl Freeze for WiiRemoteRevision
impl RefUnwindSafe for WiiRemoteRevision
impl Send for WiiRemoteRevision
impl Sync for WiiRemoteRevision
impl Unpin for WiiRemoteRevision
impl UnsafeUnpin for WiiRemoteRevision
impl UnwindSafe for WiiRemoteRevision
```

### `WiiResourceDisposition`

Owner/source: `../src/gr_curated_controllers/wii_remote/memory.rs.html`

```rust
pub enum WiiResourceDisposition { KeepSnapshot, Discard, }
```

Trait implementations:

```rust
impl Clone for WiiResourceDisposition
impl Debug for WiiResourceDisposition
impl PartialEq for WiiResourceDisposition
impl Copy for WiiResourceDisposition
impl Eq for WiiResourceDisposition
impl StructuralPartialEq for WiiResourceDisposition
impl Freeze for WiiResourceDisposition
impl RefUnwindSafe for WiiResourceDisposition
impl Send for WiiResourceDisposition
impl Sync for WiiResourceDisposition
impl Unpin for WiiResourceDisposition
impl UnsafeUnpin for WiiResourceDisposition
impl UnwindSafe for WiiResourceDisposition
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

### `XboxSeriesButton`

Owner/source: `../src/gr_curated_controllers/xbox_series.rs.html`

```rust
#[non_exhaustive]pub enum XboxSeriesButton { Show 16 variants A, B, X, Y, LeftShoulder, RightShoulder, View, Menu, Guide, Share, LeftStickPress, RightStickPress, DpadUp, DpadDown, DpadLeft, DpadRight, }
```

Trait implementations:

```rust
impl Clone for XboxSeriesButton
impl Debug for XboxSeriesButton
impl PartialEq for XboxSeriesButton
impl Copy for XboxSeriesButton
impl Eq for XboxSeriesButton
impl StructuralPartialEq for XboxSeriesButton
impl Freeze for XboxSeriesButton
impl RefUnwindSafe for XboxSeriesButton
impl Send for XboxSeriesButton
impl Sync for XboxSeriesButton
impl Unpin for XboxSeriesButton
impl UnsafeUnpin for XboxSeriesButton
impl UnwindSafe for XboxSeriesButton
```

### `XboxSeriesOutputEvent`

Owner/source: `../src/virtualgamepad/output.rs.html`

```rust
#[non_exhaustive]pub enum XboxSeriesOutputEvent { ForceFeedback(ForceFeedbackEvent), HostLifecycle(HostLifecycle), HidOutput { report_id: Option<u8>, bytes: Vec<u8>, }, }
```

Trait implementations:

```rust
impl Clone for XboxSeriesOutputEvent
impl Debug for XboxSeriesOutputEvent
impl PartialEq for XboxSeriesOutputEvent
impl Eq for XboxSeriesOutputEvent
impl StructuralPartialEq for XboxSeriesOutputEvent
impl Freeze for XboxSeriesOutputEvent
impl RefUnwindSafe for XboxSeriesOutputEvent
impl Send for XboxSeriesOutputEvent
impl Sync for XboxSeriesOutputEvent
impl Unpin for XboxSeriesOutputEvent
impl UnsafeUnpin for XboxSeriesOutputEvent
impl UnwindSafe for XboxSeriesOutputEvent
```

### `create_dualsense`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_dualsense( options: CreationOptions, family: DualSenseOptions, ) -> Result<DualSenseController, ControllerError>
```

### `create_dualshock4`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_dualshock4( options: CreationOptions, family: DualShock4Options, ) -> Result<DualShock4Controller, ControllerError>
```

### `create_steam_controller_2026`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_steam_controller_2026( options: CreationOptions, family: SteamController2026Options, ) -> Result<SteamController2026Controller, ControllerError>
```

### `create_switch_pro`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_switch_pro( options: CreationOptions, family: SwitchProOptions, ) -> Result<SwitchProController, ControllerError>
```

### `create_wii_remote`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_wii_remote( options: CreationOptions, family: WiiRemoteOptions, ) -> Result<WiiRemoteController, ControllerError>
```

### `create_xbox360`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_xbox360( options: CreationOptions, family: Xbox360Options, ) -> Result<Xbox360Controller, ControllerError>
```

### `create_xbox_series`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub fn create_xbox_series( options: CreationOptions, family: XboxSeriesOptions, ) -> Result<XboxSeriesController, ControllerError>
```

### `AbsoluteAxisSurface`

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

```rust
pub struct ControllerSurface { /* private fields */ }
pub const fn target(&self) -> RealizationId
pub const fn validation_status(&self) -> RealizationValidationStatus
pub const fn digital_controls(&self) -> &'static [DigitalControlSurface]
pub const fn axes(&self) -> &'static [AbsoluteAxisSurface]
pub const fn output_kinds(&self) -> &'static [OutputKind]
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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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
pub fn inputs(&self) -> &DualSenseInputs
pub fn status(&self) -> DualSenseStatus
pub fn edit_inputs( &mut self, edit: impl FnOnce(&mut DualSenseInputs) -> Result<(), ControlError>, ) -> Result<(), ControlError>
pub fn set_inputs(&mut self, state: DualSenseInputs) -> Result<(), ControlError>
pub fn host_outputs(&self) -> Result<DualSenseOutputState, ControlError>
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
pub fn refresh_motion(&mut self) -> Result<(), CommitError>
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

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseIdentity(/* private fields */);
pub fn generate() -> Result<DualSenseIdentity, ProviderError>
pub fn from_bytes(bytes: [u8; 6]) -> Result<DualSenseIdentity, ProviderError>
pub const fn bytes(self) -> [u8; 6]
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

### `DualSenseInputs`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseInputs { /* private fields */ }
pub const fn left_stick(&self) -> (DualSenseAxis, DualSenseAxis)
pub const fn right_stick(&self) -> (DualSenseAxis, DualSenseAxis)
pub const fn triggers(&self) -> (DualSenseTrigger, DualSenseTrigger)
pub const fn touch(&self, slot: TouchSlot) -> Option<DualSenseTouchContact>
pub const fn motion(&self) -> MotionSample
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub const fn native_pressed(&self, control: DualSenseControl) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
pub fn set_sticks( &mut self, left: (DualSenseAxis, DualSenseAxis), right: (DualSenseAxis, DualSenseAxis), )
pub fn set_triggers(&mut self, left: DualSenseTrigger, right: DualSenseTrigger)
pub fn set_touch( &mut self, slot: TouchSlot, contact: Option<DualSenseTouchContact>, )
pub fn set_motion(&mut self, motion: MotionSample)
pub fn set_digital(&mut self, update: DigitalControlUpdate)
pub fn set_native(&mut self, control: DualSenseControl, pressed: bool)
```

Trait implementations:

```rust
impl Clone for DualSenseInputs
impl Debug for DualSenseInputs
impl Default for DualSenseInputs
impl PartialEq for DualSenseInputs
impl Eq for DualSenseInputs
impl StructuralPartialEq for DualSenseInputs
impl Freeze for DualSenseInputs
impl RefUnwindSafe for DualSenseInputs
impl Send for DualSenseInputs
impl Sync for DualSenseInputs
impl Unpin for DualSenseInputs
impl UnsafeUnpin for DualSenseInputs
impl UnwindSafe for DualSenseInputs
```

### `DualSenseOptions`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
#[non_exhaustive]pub struct DualSenseOptions { /* private fields */ }
pub const fn with_identity( self, identity: DualSenseIdentity, ) -> DualSenseOptions
pub const fn identity(self) -> Option<DualSenseIdentity>
```

Trait implementations:

```rust
impl Clone for DualSenseOptions
impl Debug for DualSenseOptions
impl Default for DualSenseOptions
impl Copy for DualSenseOptions
impl Freeze for DualSenseOptions
impl RefUnwindSafe for DualSenseOptions
impl Send for DualSenseOptions
impl Sync for DualSenseOptions
impl Unpin for DualSenseOptions
impl UnsafeUnpin for DualSenseOptions
impl UnwindSafe for DualSenseOptions
```

### `DualSenseOutputState`

Owner/source: `../src/gr_curated_controllers/retained.rs.html`

```rust
#[non_exhaustive]pub struct DualSenseOutputState {Show 13 fields pub revision: u64, pub evdev_rumble: Option<RumblePlayback>, pub motors: Option<[u8; 2]>, pub right_trigger: Option<[u8; 11]>, pub left_trigger: Option<[u8; 11]>, pub mute_led: Option<u8>, pub microphone_muted: Option<bool>, pub audio_path: Option<DualSenseAudioPath>, pub speaker_volume: Option<u8>, pub microphone_volume: Option<u8>, pub speaker_preamp: Option<u8>, pub player_leds: Option<u8>, pub lightbar_rgb: Option<[u8; 3]>, }
```

Trait implementations:

```rust
impl Clone for DualSenseOutputState
impl Debug for DualSenseOutputState
impl Default for DualSenseOutputState
impl PartialEq for DualSenseOutputState
impl Eq for DualSenseOutputState
impl StructuralPartialEq for DualSenseOutputState
impl Freeze for DualSenseOutputState
impl RefUnwindSafe for DualSenseOutputState
impl Send for DualSenseOutputState
impl Sync for DualSenseOutputState
impl Unpin for DualSenseOutputState
impl UnsafeUnpin for DualSenseOutputState
impl UnwindSafe for DualSenseOutputState
```

### `DualSenseStatus`

Owner/source: `../src/gr_curated_controllers/dualsense.rs.html`

```rust
pub struct DualSenseStatus { /* private fields */ }
pub const fn battery(self) -> BatteryState
pub const fn input_sequence(self) -> u8
pub const fn sensor_timestamp(self) -> u32
```

Trait implementations:

```rust
impl Clone for DualSenseStatus
impl Debug for DualSenseStatus
impl Default for DualSenseStatus
impl PartialEq for DualSenseStatus
impl Copy for DualSenseStatus
impl Eq for DualSenseStatus
impl StructuralPartialEq for DualSenseStatus
impl Freeze for DualSenseStatus
impl RefUnwindSafe for DualSenseStatus
impl Send for DualSenseStatus
impl Sync for DualSenseStatus
impl Unpin for DualSenseStatus
impl UnsafeUnpin for DualSenseStatus
impl UnwindSafe for DualSenseStatus
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
pub fn inputs(&self) -> &DualShock4Inputs
pub fn status(&self) -> DualShock4Status
pub fn edit_inputs( &mut self, edit: impl FnOnce(&mut DualShock4Inputs) -> Result<(), ControlError>, ) -> Result<(), ControlError>
pub fn set_inputs( &mut self, state: DualShock4Inputs, ) -> Result<(), ControlError>
pub fn host_outputs(&self) -> Result<DualShock4OutputState, ControlError>
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
pub fn refresh_motion(&mut self) -> Result<(), CommitError>
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

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Identity(/* private fields */);
pub fn generate() -> Result<DualShock4Identity, ProviderError>
pub fn from_bytes(bytes: [u8; 6]) -> Result<DualShock4Identity, ProviderError>
pub const fn bytes(self) -> [u8; 6]
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

### `DualShock4Inputs`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Inputs { /* private fields */ }
pub const fn motion(&self) -> DualShock4MotionSample
pub const fn left_stick(&self) -> (DualShock4Axis, DualShock4Axis)
pub const fn right_stick(&self) -> (DualShock4Axis, DualShock4Axis)
pub const fn triggers(&self) -> (DualShock4Trigger, DualShock4Trigger)
pub const fn touch( &self, slot: DualShock4TouchSlot, ) -> Option<DualShock4TouchContact>
pub const fn native_pressed(&self, control: DualShock4Control) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub fn set_sticks( &mut self, left: (DualShock4Axis, DualShock4Axis), right: (DualShock4Axis, DualShock4Axis), )
pub fn set_triggers( &mut self, left: DualShock4Trigger, right: DualShock4Trigger, )
pub fn set_touch( &mut self, slot: DualShock4TouchSlot, contact: Option<DualShock4TouchContact>, )
pub fn set_motion(&mut self, motion: DualShock4MotionSample)
pub fn set_digital(&mut self, update: DigitalControlUpdate)
pub fn set_native(&mut self, control: DualShock4Control, pressed: bool)
```

Trait implementations:

```rust
impl Clone for DualShock4Inputs
impl Debug for DualShock4Inputs
impl Default for DualShock4Inputs
impl PartialEq for DualShock4Inputs
impl Eq for DualShock4Inputs
impl StructuralPartialEq for DualShock4Inputs
impl Freeze for DualShock4Inputs
impl RefUnwindSafe for DualShock4Inputs
impl Send for DualShock4Inputs
impl Sync for DualShock4Inputs
impl Unpin for DualShock4Inputs
impl UnsafeUnpin for DualShock4Inputs
impl UnwindSafe for DualShock4Inputs
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

### `DualShock4Options`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
#[non_exhaustive]pub struct DualShock4Options { /* private fields */ }
pub const fn with_identity( self, identity: DualShock4Identity, ) -> DualShock4Options
pub const fn identity(self) -> Option<DualShock4Identity>
```

Trait implementations:

```rust
impl Clone for DualShock4Options
impl Debug for DualShock4Options
impl Default for DualShock4Options
impl Copy for DualShock4Options
impl Freeze for DualShock4Options
impl RefUnwindSafe for DualShock4Options
impl Send for DualShock4Options
impl Sync for DualShock4Options
impl Unpin for DualShock4Options
impl UnsafeUnpin for DualShock4Options
impl UnwindSafe for DualShock4Options
```

### `DualShock4OutputState`

Owner/source: `../src/gr_curated_controllers/retained.rs.html`

```rust
#[non_exhaustive]pub struct DualShock4OutputState { pub revision: u64, pub evdev_rumble: Option<RumblePlayback>, pub motors: Option<[u8; 2]>, pub lightbar_rgb: Option<[u8; 3]>, pub blink: Option<[u8; 2]>, }
```

Trait implementations:

```rust
impl Clone for DualShock4OutputState
impl Debug for DualShock4OutputState
impl Default for DualShock4OutputState
impl PartialEq for DualShock4OutputState
impl Eq for DualShock4OutputState
impl StructuralPartialEq for DualShock4OutputState
impl Freeze for DualShock4OutputState
impl RefUnwindSafe for DualShock4OutputState
impl Send for DualShock4OutputState
impl Sync for DualShock4OutputState
impl Unpin for DualShock4OutputState
impl UnsafeUnpin for DualShock4OutputState
impl UnwindSafe for DualShock4OutputState
```

### `DualShock4Status`

Owner/source: `../src/gr_curated_controllers/dualshock4.rs.html`

```rust
pub struct DualShock4Status { /* private fields */ }
pub const fn battery(self) -> BatteryState
pub const fn sequence(self) -> u8
pub const fn sensor_timestamp(self) -> u16
pub const fn touch_sequence(self) -> u8
```

Trait implementations:

```rust
impl Clone for DualShock4Status
impl Debug for DualShock4Status
impl Default for DualShock4Status
impl PartialEq for DualShock4Status
impl Copy for DualShock4Status
impl Eq for DualShock4Status
impl StructuralPartialEq for DualShock4Status
impl Freeze for DualShock4Status
impl RefUnwindSafe for DualShock4Status
impl Send for DualShock4Status
impl Sync for DualShock4Status
impl Unpin for DualShock4Status
impl UnsafeUnpin for DualShock4Status
impl UnwindSafe for DualShock4Status
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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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
pub const fn typed_inputs(&self) -> &'static [TypedInput]
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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

### `RumblePlayback`

Owner/source: `../src/gr_curated_controllers/retained.rs.html`

```rust
#[non_exhaustive]pub struct RumblePlayback { pub strong: u16, pub weak: u16, pub repetitions: u32, }
```

Trait implementations:

```rust
impl Clone for RumblePlayback
impl Debug for RumblePlayback
impl PartialEq for RumblePlayback
impl Copy for RumblePlayback
impl Eq for RumblePlayback
impl StructuralPartialEq for RumblePlayback
impl Freeze for RumblePlayback
impl RefUnwindSafe for RumblePlayback
impl Send for RumblePlayback
impl Sync for RumblePlayback
impl Unpin for RumblePlayback
impl UnsafeUnpin for RumblePlayback
impl UnwindSafe for RumblePlayback
```

### `SteamController2026Controller`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct SteamController2026Controller { /* private fields */ }
pub const fn identity(&self) -> &SteamController2026Identity
pub fn inputs(&self) -> &SteamController2026Inputs
pub fn status(&self) -> SteamController2026Status
pub fn edit_inputs( &mut self, edit: impl FnOnce(&mut SteamController2026Inputs) -> Result<(), ControlError>, ) -> Result<(), ControlError>
pub fn host_outputs(&self) -> Result<SteamController2026Outputs, ControlError>
pub fn surface(&self) -> &'static ControllerSurface
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn is_dirty(&self) -> bool
pub fn is_closed(&self) -> bool
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(SteamController2026OutputEvent), ) -> Result<(), ControllerError>
pub fn set_digital( &mut self, update: DigitalControlUpdate, ) -> Result<(), ControlError>
pub fn set_inputs( &mut self, state: SteamController2026Inputs, ) -> Result<(), ControlError>
pub fn set_button( &mut self, button: SteamController2026Button, pressed: bool, ) -> Result<(), ControlError>
```

Trait implementations:

```rust
impl Freeze for SteamController2026Controller
impl !RefUnwindSafe for SteamController2026Controller
impl Send for SteamController2026Controller
impl !Sync for SteamController2026Controller
impl Unpin for SteamController2026Controller
impl UnsafeUnpin for SteamController2026Controller
impl !UnwindSafe for SteamController2026Controller
```

### `SteamController2026Identity`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026/controller.rs.html`

```rust
pub struct SteamController2026Identity { /* private fields */ }
pub fn from_serial( serial: impl Into<String>, ) -> Result<SteamController2026Identity, ProviderError>
pub fn generate() -> Result<SteamController2026Identity, ProviderError>
pub fn serial(&self) -> &str
```

Trait implementations:

```rust
impl Clone for SteamController2026Identity
impl Debug for SteamController2026Identity
impl PartialEq for SteamController2026Identity
impl Eq for SteamController2026Identity
impl StructuralPartialEq for SteamController2026Identity
impl Freeze for SteamController2026Identity
impl RefUnwindSafe for SteamController2026Identity
impl Send for SteamController2026Identity
impl Sync for SteamController2026Identity
impl Unpin for SteamController2026Identity
impl UnsafeUnpin for SteamController2026Identity
impl UnwindSafe for SteamController2026Identity
```

### `SteamController2026Inputs`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026.rs.html`

```rust
pub struct SteamController2026Inputs { pub sticks: [[i16; 2]; 2], pub triggers: [i16; 2], pub pads: [SteamTrackpadState; 2], pub stick_touch: [bool; 2], pub grip_touch: [bool; 2], pub motion: SteamMotionSample, /* private fields */ }
pub const fn input_topology(&self) -> &'static InputTopology
pub fn neutralize(&mut self)
pub const fn pressed(&self, button: SteamController2026Button) -> bool
pub fn set_button(&mut self, button: SteamController2026Button, pressed: bool)
```

Trait implementations:

```rust
impl Clone for SteamController2026Inputs
impl Debug for SteamController2026Inputs
impl Default for SteamController2026Inputs
impl PartialEq for SteamController2026Inputs
impl Eq for SteamController2026Inputs
impl StructuralPartialEq for SteamController2026Inputs
impl Freeze for SteamController2026Inputs
impl RefUnwindSafe for SteamController2026Inputs
impl Send for SteamController2026Inputs
impl Sync for SteamController2026Inputs
impl Unpin for SteamController2026Inputs
impl UnsafeUnpin for SteamController2026Inputs
impl UnwindSafe for SteamController2026Inputs
```

### `SteamController2026Options`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026/controller.rs.html`

```rust
#[non_exhaustive]pub struct SteamController2026Options { /* private fields */ }
pub fn with_identity( self, identity: SteamController2026Identity, ) -> SteamController2026Options
pub const fn identity(&self) -> Option<&SteamController2026Identity>
pub fn with_steam_client_compat( self, enabled: bool, ) -> SteamController2026Options
pub const fn steam_client_compat(&self) -> bool
pub fn with_haptic_streaming(self, enabled: bool) -> SteamController2026Options
pub const fn haptic_streaming(&self) -> bool
pub fn with_lizard_mode(self, enabled: bool) -> SteamController2026Options
pub const fn lizard_mode(&self) -> bool
```

Trait implementations:

```rust
impl Clone for SteamController2026Options
impl Debug for SteamController2026Options
impl Default for SteamController2026Options
impl PartialEq for SteamController2026Options
impl Eq for SteamController2026Options
impl StructuralPartialEq for SteamController2026Options
impl Freeze for SteamController2026Options
impl RefUnwindSafe for SteamController2026Options
impl Send for SteamController2026Options
impl Sync for SteamController2026Options
impl Unpin for SteamController2026Options
impl UnsafeUnpin for SteamController2026Options
impl UnwindSafe for SteamController2026Options
```

### `SteamController2026Outputs`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026/codec.rs.html`

```rust
#[non_exhaustive]pub struct SteamController2026Outputs { pub revision: u64, pub haptics: [Option<SteamController2026Haptic>; 6], pub streaming: SteamController2026StreamOutputs, pub led_brightness: Option<u16>, pub led_override: Option<bool>, pub led_rgbw: Option<[u8; 4]>, }
```

Trait implementations:

```rust
impl Clone for SteamController2026Outputs
impl Debug for SteamController2026Outputs
impl Default for SteamController2026Outputs
impl PartialEq for SteamController2026Outputs
impl Eq for SteamController2026Outputs
impl StructuralPartialEq for SteamController2026Outputs
impl Freeze for SteamController2026Outputs
impl RefUnwindSafe for SteamController2026Outputs
impl Send for SteamController2026Outputs
impl Sync for SteamController2026Outputs
impl Unpin for SteamController2026Outputs
impl UnsafeUnpin for SteamController2026Outputs
impl UnwindSafe for SteamController2026Outputs
```

### `SteamController2026Status`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026.rs.html`

```rust
pub struct SteamController2026Status { /* private fields */ }
pub const fn report_sequence(self) -> Option<u8>
pub const fn steam_client_compat(self) -> Option<bool>
pub const fn haptic_streaming(self) -> Option<bool>
pub const fn lizard_mode(self) -> Option<bool>
pub const fn mappings(self) -> Option<bool>
pub const fn lizard_setting(self) -> Option<bool>
pub const fn imu_mode(self) -> Option<u16>
```

Trait implementations:

```rust
impl Clone for SteamController2026Status
impl Debug for SteamController2026Status
impl Default for SteamController2026Status
impl PartialEq for SteamController2026Status
impl Copy for SteamController2026Status
impl Eq for SteamController2026Status
impl StructuralPartialEq for SteamController2026Status
impl Freeze for SteamController2026Status
impl RefUnwindSafe for SteamController2026Status
impl Send for SteamController2026Status
impl Sync for SteamController2026Status
impl Unpin for SteamController2026Status
impl UnsafeUnpin for SteamController2026Status
impl UnwindSafe for SteamController2026Status
```

### `SteamController2026StreamOutputs`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026/streaming.rs.html`

```rust
#[non_exhaustive]pub struct SteamController2026StreamOutputs { pub enabled: bool, pub configured: [bool; 4], pub buffered_bytes: [u16; 4], pub received_bytes: [u64; 4], pub overrun_bytes: [u64; 4], pub last_packet: Option<SteamController2026StreamPacket>, }
```

Trait implementations:

```rust
impl Clone for SteamController2026StreamOutputs
impl Debug for SteamController2026StreamOutputs
impl Default for SteamController2026StreamOutputs
impl PartialEq for SteamController2026StreamOutputs
impl Eq for SteamController2026StreamOutputs
impl StructuralPartialEq for SteamController2026StreamOutputs
impl Freeze for SteamController2026StreamOutputs
impl RefUnwindSafe for SteamController2026StreamOutputs
impl Send for SteamController2026StreamOutputs
impl Sync for SteamController2026StreamOutputs
impl Unpin for SteamController2026StreamOutputs
impl UnsafeUnpin for SteamController2026StreamOutputs
impl UnwindSafe for SteamController2026StreamOutputs
```

### `SteamMotionSample`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026.rs.html`

```rust
pub struct SteamMotionSample { pub timestamp_us: u32, pub accelerometer: [i16; 3], pub gyroscope: [i16; 3], pub quaternion: [i16; 4], }
```

Trait implementations:

```rust
impl Clone for SteamMotionSample
impl Debug for SteamMotionSample
impl Default for SteamMotionSample
impl PartialEq for SteamMotionSample
impl Eq for SteamMotionSample
impl StructuralPartialEq for SteamMotionSample
impl Freeze for SteamMotionSample
impl RefUnwindSafe for SteamMotionSample
impl Send for SteamMotionSample
impl Sync for SteamMotionSample
impl Unpin for SteamMotionSample
impl UnsafeUnpin for SteamMotionSample
impl UnwindSafe for SteamMotionSample
```

### `SteamTrackpadState`

Owner/source: `../src/gr_curated_controllers/steam_controller_2026.rs.html`

```rust
pub struct SteamTrackpadState { pub position: [i16; 2], pub pressure: u16, pub touch: bool, pub click: bool, }
```

Trait implementations:

```rust
impl Clone for SteamTrackpadState
impl Debug for SteamTrackpadState
impl Default for SteamTrackpadState
impl PartialEq for SteamTrackpadState
impl Copy for SteamTrackpadState
impl Eq for SteamTrackpadState
impl StructuralPartialEq for SteamTrackpadState
impl Freeze for SteamTrackpadState
impl RefUnwindSafe for SteamTrackpadState
impl Send for SteamTrackpadState
impl Sync for SteamTrackpadState
impl Unpin for SteamTrackpadState
impl UnsafeUnpin for SteamTrackpadState
impl UnwindSafe for SteamTrackpadState
```

### `StickInput`

Owner/source: `source unavailable`

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
pub const fn identity(&self) -> SwitchProIdentity
pub fn stream_enabled(&self) -> bool
pub fn motion_report_counter(&self) -> u8
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn inputs(&self) -> &SwitchProInputs
pub fn status(&self) -> SwitchProStatus
pub fn edit_inputs( &mut self, edit: impl FnOnce(&mut SwitchProInputs) -> Result<(), ControlError>, ) -> Result<(), ControlError>
pub fn set_inputs(&mut self, state: SwitchProInputs) -> Result<(), ControlError>
pub fn host_outputs(&self) -> Result<SwitchProOutputState, ControlError>
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

### `SwitchProIdentity`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProIdentity(/* private fields */);
pub fn from_bytes(address: [u8; 6]) -> Result<SwitchProIdentity, ProviderError>
pub fn generate() -> Result<SwitchProIdentity, ProviderError>
pub const fn bytes(self) -> [u8; 6]
```

Trait implementations:

```rust
impl Clone for SwitchProIdentity
impl Debug for SwitchProIdentity
impl PartialEq for SwitchProIdentity
impl Copy for SwitchProIdentity
impl Eq for SwitchProIdentity
impl StructuralPartialEq for SwitchProIdentity
impl Freeze for SwitchProIdentity
impl RefUnwindSafe for SwitchProIdentity
impl Send for SwitchProIdentity
impl Sync for SwitchProIdentity
impl Unpin for SwitchProIdentity
impl UnsafeUnpin for SwitchProIdentity
impl UnwindSafe for SwitchProIdentity
```

### `SwitchProInputs`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProInputs { /* private fields */ }
pub const fn left_stick(&self) -> (SwitchProAxis, SwitchProAxis)
pub const fn right_stick(&self) -> (SwitchProAxis, SwitchProAxis)
pub const fn motion(&self) -> SwitchProMotionSample
pub const fn native_pressed(&self, control: SwitchProControl) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub fn set_sticks( &mut self, left: (SwitchProAxis, SwitchProAxis), right: (SwitchProAxis, SwitchProAxis), )
pub fn set_motion(&mut self, motion: SwitchProMotionSample)
pub fn set_digital(&mut self, update: DigitalControlUpdate)
pub fn set_native(&mut self, c: SwitchProControl, p: bool)
```

Trait implementations:

```rust
impl Clone for SwitchProInputs
impl Debug for SwitchProInputs
impl Default for SwitchProInputs
impl PartialEq for SwitchProInputs
impl Eq for SwitchProInputs
impl StructuralPartialEq for SwitchProInputs
impl Freeze for SwitchProInputs
impl RefUnwindSafe for SwitchProInputs
impl Send for SwitchProInputs
impl Sync for SwitchProInputs
impl Unpin for SwitchProInputs
impl UnsafeUnpin for SwitchProInputs
impl UnwindSafe for SwitchProInputs
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

### `SwitchProOptions`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
#[non_exhaustive]pub struct SwitchProOptions { /* private fields */ }
pub const fn with_identity( self, identity: SwitchProIdentity, ) -> SwitchProOptions
pub const fn identity(self) -> Option<SwitchProIdentity>
```

Trait implementations:

```rust
impl Clone for SwitchProOptions
impl Debug for SwitchProOptions
impl Default for SwitchProOptions
impl Copy for SwitchProOptions
impl Freeze for SwitchProOptions
impl RefUnwindSafe for SwitchProOptions
impl Send for SwitchProOptions
impl Sync for SwitchProOptions
impl Unpin for SwitchProOptions
impl UnsafeUnpin for SwitchProOptions
impl UnwindSafe for SwitchProOptions
```

### `SwitchProOutputState`

Owner/source: `../src/gr_curated_controllers/retained.rs.html`

```rust
#[non_exhaustive]pub struct SwitchProOutputState { pub revision: u64, pub evdev_rumble: Option<RumblePlayback>, pub motor_words: Option<[[u8; 4]; 2]>, pub player_lights: Option<u8>, }
```

Trait implementations:

```rust
impl Clone for SwitchProOutputState
impl Debug for SwitchProOutputState
impl Default for SwitchProOutputState
impl PartialEq for SwitchProOutputState
impl Eq for SwitchProOutputState
impl StructuralPartialEq for SwitchProOutputState
impl Freeze for SwitchProOutputState
impl RefUnwindSafe for SwitchProOutputState
impl Send for SwitchProOutputState
impl Sync for SwitchProOutputState
impl Unpin for SwitchProOutputState
impl UnsafeUnpin for SwitchProOutputState
impl UnwindSafe for SwitchProOutputState
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

### `SwitchProStatus`

Owner/source: `../src/gr_curated_controllers/switch_pro.rs.html`

```rust
pub struct SwitchProStatus { /* private fields */ }
pub const fn timer(self) -> u8
pub const fn stream_enabled(self) -> bool
```

Trait implementations:

```rust
impl Clone for SwitchProStatus
impl Debug for SwitchProStatus
impl Default for SwitchProStatus
impl PartialEq for SwitchProStatus
impl Copy for SwitchProStatus
impl Eq for SwitchProStatus
impl StructuralPartialEq for SwitchProStatus
impl Freeze for SwitchProStatus
impl RefUnwindSafe for SwitchProStatus
impl Send for SwitchProStatus
impl Sync for SwitchProStatus
impl Unpin for SwitchProStatus
impl UnsafeUnpin for SwitchProStatus
impl UnwindSafe for SwitchProStatus
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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

Owner/source: `source unavailable`

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

### `TypedInput`

Owner/source: `source unavailable`

```rust
#[non_exhaustive]pub struct TypedInput { /* private fields */ }
pub const fn id(&self) -> InputControlId
pub const fn title(&self) -> &'static str
pub const fn kind(&self) -> InputKind
```

Trait implementations:

```rust
impl Clone for TypedInput
impl Debug for TypedInput
impl PartialEq for TypedInput
impl Copy for TypedInput
impl Eq for TypedInput
impl StructuralPartialEq for TypedInput
impl Freeze for TypedInput
impl RefUnwindSafe for TypedInput
impl Send for TypedInput
impl Sync for TypedInput
impl Unpin for TypedInput
impl UnsafeUnpin for TypedInput
impl UnwindSafe for TypedInput
```

### `WiiClassicInputs`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiClassicInputs { pub left: [u8; 2], pub right: [u8; 2], pub triggers: [u8; 2], pub buttons: [bool; 15], }
```

Trait implementations:

```rust
impl Clone for WiiClassicInputs
impl Debug for WiiClassicInputs
impl Default for WiiClassicInputs
impl PartialEq for WiiClassicInputs
impl Eq for WiiClassicInputs
impl StructuralPartialEq for WiiClassicInputs
impl Freeze for WiiClassicInputs
impl RefUnwindSafe for WiiClassicInputs
impl Send for WiiClassicInputs
impl Sync for WiiClassicInputs
impl Unpin for WiiClassicInputs
impl UnsafeUnpin for WiiClassicInputs
impl UnwindSafe for WiiClassicInputs
```

### `WiiEepromImage`

Owner/source: `../src/gr_curated_controllers/wii_remote/memory.rs.html`

```rust
pub struct WiiEepromImage { /* private fields */ }
pub fn import(bytes: Vec<u8>) -> Result<WiiEepromImage, ControlError>
pub fn with_extension_calibration( self, calibration: WiiExtensionCalibration, ) -> WiiEepromImage
pub fn extension_calibration(&self) -> &WiiExtensionCalibration
pub fn bytes(&self) -> &[u8] ⓘ
```

Trait implementations:

```rust
impl Clone for WiiEepromImage
impl Debug for WiiEepromImage
impl Default for WiiEepromImage
impl PartialEq for WiiEepromImage
impl Eq for WiiEepromImage
impl StructuralPartialEq for WiiEepromImage
impl Freeze for WiiEepromImage
impl RefUnwindSafe for WiiEepromImage
impl Send for WiiEepromImage
impl Sync for WiiEepromImage
impl Unpin for WiiEepromImage
impl UnsafeUnpin for WiiEepromImage
impl UnwindSafe for WiiEepromImage
```

### `WiiExtensionCalibration`

Owner/source: `../src/gr_curated_controllers/wii_remote/calibration.rs.html`

```rust
pub struct WiiExtensionCalibration { /* private fields */ }
pub fn with_motion_plus( self, block: Option<[u8; 32]>, ) -> Result<WiiExtensionCalibration, ControlError>
pub fn with_nunchuk( self, block: Option<[u8; 16]>, ) -> Result<WiiExtensionCalibration, ControlError>
pub fn with_classic( self, block: Option<[u8; 16]>, ) -> Result<WiiExtensionCalibration, ControlError>
pub fn motion_plus(&self) -> Option<[u8; 32]>
pub fn nunchuk(&self) -> Option<[u8; 16]>
pub fn classic(&self) -> Option<[u8; 16]>
```

Trait implementations:

```rust
impl Clone for WiiExtensionCalibration
impl Debug for WiiExtensionCalibration
impl Default for WiiExtensionCalibration
impl PartialEq for WiiExtensionCalibration
impl Eq for WiiExtensionCalibration
impl StructuralPartialEq for WiiExtensionCalibration
impl Freeze for WiiExtensionCalibration
impl RefUnwindSafe for WiiExtensionCalibration
impl Send for WiiExtensionCalibration
impl Sync for WiiExtensionCalibration
impl Unpin for WiiExtensionCalibration
impl UnsafeUnpin for WiiExtensionCalibration
impl UnwindSafe for WiiExtensionCalibration
```

### `WiiIrPoint`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiIrPoint { /* private fields */ }
pub fn new(x: u16, y: u16) -> Result<WiiIrPoint, ControlError>
pub const fn coordinates(self) -> (u16, u16)
```

Trait implementations:

```rust
impl Clone for WiiIrPoint
impl Debug for WiiIrPoint
impl PartialEq for WiiIrPoint
impl Copy for WiiIrPoint
impl Eq for WiiIrPoint
impl StructuralPartialEq for WiiIrPoint
impl Freeze for WiiIrPoint
impl RefUnwindSafe for WiiIrPoint
impl Send for WiiIrPoint
impl Sync for WiiIrPoint
impl Unpin for WiiIrPoint
impl UnsafeUnpin for WiiIrPoint
impl UnwindSafe for WiiIrPoint
```

### `WiiMotionPlusInputs`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiMotionPlusInputs { pub gyroscope: [u16; 3], pub slow: [bool; 3], }
```

Trait implementations:

```rust
impl Clone for WiiMotionPlusInputs
impl Debug for WiiMotionPlusInputs
impl Default for WiiMotionPlusInputs
impl PartialEq for WiiMotionPlusInputs
impl Eq for WiiMotionPlusInputs
impl StructuralPartialEq for WiiMotionPlusInputs
impl Freeze for WiiMotionPlusInputs
impl RefUnwindSafe for WiiMotionPlusInputs
impl Send for WiiMotionPlusInputs
impl Sync for WiiMotionPlusInputs
impl Unpin for WiiMotionPlusInputs
impl UnsafeUnpin for WiiMotionPlusInputs
impl UnwindSafe for WiiMotionPlusInputs
```

### `WiiNunchukInputs`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiNunchukInputs { pub stick: [u8; 2], pub accelerometer: [u16; 3], pub c: bool, pub z: bool, }
```

Trait implementations:

```rust
impl Clone for WiiNunchukInputs
impl Debug for WiiNunchukInputs
impl Default for WiiNunchukInputs
impl PartialEq for WiiNunchukInputs
impl Eq for WiiNunchukInputs
impl StructuralPartialEq for WiiNunchukInputs
impl Freeze for WiiNunchukInputs
impl RefUnwindSafe for WiiNunchukInputs
impl Send for WiiNunchukInputs
impl Sync for WiiNunchukInputs
impl Unpin for WiiNunchukInputs
impl UnsafeUnpin for WiiNunchukInputs
impl UnwindSafe for WiiNunchukInputs
```

### `WiiRemoteBattery`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiRemoteBattery(/* private fields */);
pub const fn new(raw: u8) -> WiiRemoteBattery
pub const fn raw(self) -> u8
```

Trait implementations:

```rust
impl Clone for WiiRemoteBattery
impl Debug for WiiRemoteBattery
impl Default for WiiRemoteBattery
impl PartialEq for WiiRemoteBattery
impl Copy for WiiRemoteBattery
impl Eq for WiiRemoteBattery
impl StructuralPartialEq for WiiRemoteBattery
impl Freeze for WiiRemoteBattery
impl RefUnwindSafe for WiiRemoteBattery
impl Send for WiiRemoteBattery
impl Sync for WiiRemoteBattery
impl Unpin for WiiRemoteBattery
impl UnsafeUnpin for WiiRemoteBattery
impl UnwindSafe for WiiRemoteBattery
```

### `WiiRemoteController`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct WiiRemoteController { /* private fields */ }
pub const fn identity(&self) -> WiiRemoteIdentity
pub fn battery(&self) -> WiiRemoteBattery
pub fn current_topology(&self) -> &'static InputTopology
pub fn topology_generation(&self) -> u64
pub fn attach( &mut self, extension: WiiExtension, motion_plus: bool, ) -> Result<(), ControlError>
pub fn resource_dirty(&self) -> bool
pub fn resolve_resource( &mut self, choice: WiiResourceDisposition, ) -> Result<Option<&WiiEepromImage>, ControlError>
pub fn inputs(&self) -> &WiiRemoteInputs
pub fn status(&self) -> WiiRemoteStatus
pub fn edit_inputs( &mut self, edit: impl FnOnce(&mut WiiRemoteInputs) -> Result<(), ControlError>, ) -> Result<(), ControlError>
pub fn host_outputs(&self) -> Result<WiiRemoteOutputs, ControlError>
pub fn surface(&self) -> &'static ControllerSurface
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn is_dirty(&self) -> bool
pub fn is_closed(&self) -> bool
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(WiiRemoteOutputEvent), ) -> Result<(), ControllerError>
pub fn set_digital( &mut self, update: DigitalControlUpdate, ) -> Result<(), ControlError>
pub fn set_inputs(&mut self, state: WiiRemoteInputs) -> Result<(), ControlError>
pub fn set_button( &mut self, button: WiiRemoteButton, pressed: bool, ) -> Result<(), ControlError>
pub fn set_accelerometer( &mut self, values: [u16; 3], ) -> Result<(), ControlError>
pub fn set_ir( &mut self, points: [Option<WiiIrPoint>; 4], ) -> Result<(), ControlError>
pub fn set_battery( &mut self, battery: WiiRemoteBattery, ) -> Result<(), ControlError>
pub fn set_extension(&mut self, value: WiiExtension) -> Result<(), ControlError>
pub fn set_motion_plus( &mut self, gyroscope: [u16; 3], slow: [bool; 3], ) -> Result<(), ControlError>
```

Trait implementations:

```rust
impl Freeze for WiiRemoteController
impl !RefUnwindSafe for WiiRemoteController
impl Send for WiiRemoteController
impl !Sync for WiiRemoteController
impl Unpin for WiiRemoteController
impl UnsafeUnpin for WiiRemoteController
impl !UnwindSafe for WiiRemoteController
```

### `WiiRemoteIdentity`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiRemoteIdentity(/* private fields */);
pub fn from_bytes(address: [u8; 6]) -> Result<WiiRemoteIdentity, ProviderError>
pub fn generate() -> Result<WiiRemoteIdentity, ProviderError>
pub const fn bytes(self) -> [u8; 6]
```

Trait implementations:

```rust
impl Clone for WiiRemoteIdentity
impl Debug for WiiRemoteIdentity
impl PartialEq for WiiRemoteIdentity
impl Copy for WiiRemoteIdentity
impl Eq for WiiRemoteIdentity
impl StructuralPartialEq for WiiRemoteIdentity
impl Freeze for WiiRemoteIdentity
impl RefUnwindSafe for WiiRemoteIdentity
impl Send for WiiRemoteIdentity
impl Sync for WiiRemoteIdentity
impl Unpin for WiiRemoteIdentity
impl UnsafeUnpin for WiiRemoteIdentity
impl UnwindSafe for WiiRemoteIdentity
```

### `WiiRemoteInputs`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiRemoteInputs { /* private fields */ }
pub fn neutralize(&mut self)
pub fn projected_input_topology( &self, motion_plus_attached: bool, ) -> &'static InputTopology
pub const fn pressed(&self, button: WiiRemoteButton) -> bool
pub fn set_button(&mut self, button: WiiRemoteButton, pressed: bool)
pub const fn accelerometer(&self) -> [u16; 3]
pub fn set_accelerometer( &mut self, values: [u16; 3], ) -> Result<(), ControlError>
pub const fn ir(&self) -> &[Option<WiiIrPoint>; 4]
pub fn set_ir(&mut self, points: [Option<WiiIrPoint>; 4])
pub const fn extension(&self) -> &WiiExtension
pub fn set_extension_values( &mut self, value: WiiExtension, ) -> Result<(), ControlError>
pub fn set_motion_plus_values( &mut self, gyroscope: [u16; 3], slow: [bool; 3], ) -> Result<(), ControlError>
pub const fn motion_plus(&self) -> &WiiMotionPlusInputs
```

Trait implementations:

```rust
impl Clone for WiiRemoteInputs
impl Debug for WiiRemoteInputs
impl Default for WiiRemoteInputs
impl PartialEq for WiiRemoteInputs
impl Eq for WiiRemoteInputs
impl StructuralPartialEq for WiiRemoteInputs
impl Freeze for WiiRemoteInputs
impl RefUnwindSafe for WiiRemoteInputs
impl Send for WiiRemoteInputs
impl Sync for WiiRemoteInputs
impl Unpin for WiiRemoteInputs
impl UnsafeUnpin for WiiRemoteInputs
impl UnwindSafe for WiiRemoteInputs
```

### `WiiRemoteOptions`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
#[non_exhaustive]pub struct WiiRemoteOptions { /* private fields */ }
pub fn with_revision(self, revision: WiiRemoteRevision) -> WiiRemoteOptions
pub fn with_eeprom_image(self, image: WiiEepromImage) -> WiiRemoteOptions
pub fn with_identity(self, identity: WiiRemoteIdentity) -> WiiRemoteOptions
pub const fn revision(&self) -> WiiRemoteRevision
pub const fn eeprom_image(&self) -> &WiiEepromImage
pub const fn identity(&self) -> Option<WiiRemoteIdentity>
pub fn with_motionplus_passthrough_id_mirror( self, enabled: bool, ) -> WiiRemoteOptions
pub const fn motionplus_passthrough_id_mirror(&self) -> bool
pub fn with_motionplus_passthrough_calibration_copy( self, enabled: bool, ) -> WiiRemoteOptions
pub const fn motionplus_passthrough_calibration_copy(&self) -> bool
pub fn with_motionplus_inactive_id_external_style( self, enabled: bool, ) -> WiiRemoteOptions
pub const fn motionplus_inactive_id_external_style(&self) -> bool
pub fn with_motionplus_activation_reattach_status( self, enabled: bool, ) -> WiiRemoteOptions
pub const fn motionplus_activation_reattach_status(&self) -> bool
```

Trait implementations:

```rust
impl Clone for WiiRemoteOptions
impl Debug for WiiRemoteOptions
impl Default for WiiRemoteOptions
impl PartialEq for WiiRemoteOptions
impl Eq for WiiRemoteOptions
impl StructuralPartialEq for WiiRemoteOptions
impl Freeze for WiiRemoteOptions
impl RefUnwindSafe for WiiRemoteOptions
impl Send for WiiRemoteOptions
impl Sync for WiiRemoteOptions
impl Unpin for WiiRemoteOptions
impl UnsafeUnpin for WiiRemoteOptions
impl UnwindSafe for WiiRemoteOptions
```

### `WiiRemoteOutputs`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiRemoteOutputs { pub revision: u64, pub leds: [bool; 4], pub rumble: bool, }
```

Trait implementations:

```rust
impl Clone for WiiRemoteOutputs
impl Debug for WiiRemoteOutputs
impl Default for WiiRemoteOutputs
impl PartialEq for WiiRemoteOutputs
impl Copy for WiiRemoteOutputs
impl Eq for WiiRemoteOutputs
impl StructuralPartialEq for WiiRemoteOutputs
impl Freeze for WiiRemoteOutputs
impl RefUnwindSafe for WiiRemoteOutputs
impl Send for WiiRemoteOutputs
impl Sync for WiiRemoteOutputs
impl Unpin for WiiRemoteOutputs
impl UnsafeUnpin for WiiRemoteOutputs
impl UnwindSafe for WiiRemoteOutputs
```

### `WiiRemoteStatus`

Owner/source: `../src/gr_curated_controllers/wii_remote.rs.html`

```rust
pub struct WiiRemoteStatus { /* private fields */ }
pub const fn revision(self) -> WiiRemoteRevision
pub const fn motion_plus_attached(self) -> bool
pub const fn battery(self) -> WiiRemoteBattery
pub const fn topology_generation(self) -> u64
```

Trait implementations:

```rust
impl Clone for WiiRemoteStatus
impl Debug for WiiRemoteStatus
impl Default for WiiRemoteStatus
impl PartialEq for WiiRemoteStatus
impl Copy for WiiRemoteStatus
impl Eq for WiiRemoteStatus
impl StructuralPartialEq for WiiRemoteStatus
impl Freeze for WiiRemoteStatus
impl RefUnwindSafe for WiiRemoteStatus
impl Send for WiiRemoteStatus
impl Sync for WiiRemoteStatus
impl Unpin for WiiRemoteStatus
impl UnsafeUnpin for WiiRemoteStatus
impl UnwindSafe for WiiRemoteStatus
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
pub fn inputs(&self) -> &Xbox360Inputs
pub fn status(&self) -> Xbox360Status
pub fn edit_inputs( &mut self, edit: impl FnOnce(&mut Xbox360Inputs) -> Result<(), ControlError>, ) -> Result<(), ControlError>
pub fn set_inputs(&mut self, state: Xbox360Inputs) -> Result<(), ControlError>
pub fn host_outputs(&self) -> Result<Xbox360OutputState, ControlError>
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

### `Xbox360Inputs`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Inputs { /* private fields */ }
pub const fn left_stick(&self) -> (Xbox360Axis, Xbox360Axis)
pub const fn right_stick(&self) -> (Xbox360Axis, Xbox360Axis)
pub const fn triggers(&self) -> (Xbox360Trigger, Xbox360Trigger)
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub const fn native_pressed(&self, control: Xbox360Control) -> bool
pub const fn dpad_pressed(&self, direction: DpadDirection) -> bool
pub fn set_sticks( &mut self, left: (Xbox360Axis, Xbox360Axis), right: (Xbox360Axis, Xbox360Axis), )
pub fn set_triggers(&mut self, left: Xbox360Trigger, right: Xbox360Trigger)
pub fn set_digital(&mut self, update: DigitalControlUpdate)
pub fn set_native(&mut self, control: Xbox360Control, pressed: bool)
```

Trait implementations:

```rust
impl Clone for Xbox360Inputs
impl Debug for Xbox360Inputs
impl Default for Xbox360Inputs
impl PartialEq for Xbox360Inputs
impl Eq for Xbox360Inputs
impl StructuralPartialEq for Xbox360Inputs
impl Freeze for Xbox360Inputs
impl RefUnwindSafe for Xbox360Inputs
impl Send for Xbox360Inputs
impl Sync for Xbox360Inputs
impl Unpin for Xbox360Inputs
impl UnsafeUnpin for Xbox360Inputs
impl UnwindSafe for Xbox360Inputs
```

### `Xbox360Options`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
#[non_exhaustive]pub struct Xbox360Options {}
```

Trait implementations:

```rust
impl Clone for Xbox360Options
impl Debug for Xbox360Options
impl Default for Xbox360Options
impl Copy for Xbox360Options
impl Freeze for Xbox360Options
impl RefUnwindSafe for Xbox360Options
impl Send for Xbox360Options
impl Sync for Xbox360Options
impl Unpin for Xbox360Options
impl UnsafeUnpin for Xbox360Options
impl UnwindSafe for Xbox360Options
```

### `Xbox360OutputState`

Owner/source: `../src/gr_curated_controllers/retained.rs.html`

```rust
#[non_exhaustive]pub struct Xbox360OutputState { pub revision: u64, pub evdev_rumble: Option<RumblePlayback>, }
```

Trait implementations:

```rust
impl Clone for Xbox360OutputState
impl Debug for Xbox360OutputState
impl Default for Xbox360OutputState
impl PartialEq for Xbox360OutputState
impl Eq for Xbox360OutputState
impl StructuralPartialEq for Xbox360OutputState
impl Freeze for Xbox360OutputState
impl RefUnwindSafe for Xbox360OutputState
impl Send for Xbox360OutputState
impl Sync for Xbox360OutputState
impl Unpin for Xbox360OutputState
impl UnsafeUnpin for Xbox360OutputState
impl UnwindSafe for Xbox360OutputState
```

### `Xbox360Status`

Owner/source: `../src/gr_curated_controllers/xbox360.rs.html`

```rust
pub struct Xbox360Status { /* private fields */ }
pub const fn battery(self) -> BatteryState
```

Trait implementations:

```rust
impl Clone for Xbox360Status
impl Debug for Xbox360Status
impl Default for Xbox360Status
impl PartialEq for Xbox360Status
impl Copy for Xbox360Status
impl Eq for Xbox360Status
impl StructuralPartialEq for Xbox360Status
impl Freeze for Xbox360Status
impl RefUnwindSafe for Xbox360Status
impl Send for Xbox360Status
impl Sync for Xbox360Status
impl Unpin for Xbox360Status
impl UnsafeUnpin for Xbox360Status
impl UnwindSafe for Xbox360Status
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

### `XboxSeriesBluetoothRumble`

Owner/source: `../src/gr_curated_controllers/xbox_series/bluetooth.rs.html`

```rust
#[non_exhaustive]pub struct XboxSeriesBluetoothRumble { pub motor_mask: u8, pub left_trigger: u8, pub right_trigger: u8, pub strong: u8, pub weak: u8, pub duration: u8, pub delay: u8, pub repeat: u8, }
```

Trait implementations:

```rust
impl Clone for XboxSeriesBluetoothRumble
impl Debug for XboxSeriesBluetoothRumble
impl PartialEq for XboxSeriesBluetoothRumble
impl Copy for XboxSeriesBluetoothRumble
impl Eq for XboxSeriesBluetoothRumble
impl StructuralPartialEq for XboxSeriesBluetoothRumble
impl Freeze for XboxSeriesBluetoothRumble
impl RefUnwindSafe for XboxSeriesBluetoothRumble
impl Send for XboxSeriesBluetoothRumble
impl Sync for XboxSeriesBluetoothRumble
impl Unpin for XboxSeriesBluetoothRumble
impl UnsafeUnpin for XboxSeriesBluetoothRumble
impl UnwindSafe for XboxSeriesBluetoothRumble
```

### `XboxSeriesController`

Owner/source: `../src/virtualgamepad/controllers.rs.html`

```rust
pub struct XboxSeriesController { /* private fields */ }
pub fn inputs(&self) -> &XboxSeriesInputs
pub fn status(&self) -> XboxSeriesStatus
pub fn edit_inputs( &mut self, edit: impl FnOnce(&mut XboxSeriesInputs) -> Result<(), ControlError>, ) -> Result<(), ControlError>
pub fn host_outputs(&self) -> Result<XboxSeriesOutputState, ControlError>
pub fn surface(&self) -> &'static ControllerSurface
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<Duration>
pub fn dropped_output_events(&self) -> u64
pub fn is_dirty(&self) -> bool
pub fn is_closed(&self) -> bool
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(XboxSeriesOutputEvent), ) -> Result<(), ControllerError>
pub fn set_digital( &mut self, update: DigitalControlUpdate, ) -> Result<(), ControlError>
pub fn set_inputs( &mut self, state: XboxSeriesInputs, ) -> Result<(), ControlError>
pub fn set_button( &mut self, button: XboxSeriesButton, pressed: bool, ) -> Result<(), ControlError>
```

Trait implementations:

```rust
impl Freeze for XboxSeriesController
impl !RefUnwindSafe for XboxSeriesController
impl Send for XboxSeriesController
impl !Sync for XboxSeriesController
impl Unpin for XboxSeriesController
impl UnsafeUnpin for XboxSeriesController
impl !UnwindSafe for XboxSeriesController
```

### `XboxSeriesInputs`

Owner/source: `../src/gr_curated_controllers/xbox_series.rs.html`

```rust
pub struct XboxSeriesInputs { pub sticks: [[i16; 2]; 2], /* private fields */ }
pub const fn pressed(&self, button: XboxSeriesButton) -> bool
pub fn set_button(&mut self, button: XboxSeriesButton, pressed: bool)
pub fn set_digital(&mut self, update: DigitalControlUpdate)
pub const fn triggers(&self) -> [XboxSeriesTrigger; 2]
pub fn set_triggers( &mut self, left: u16, right: u16, ) -> Result<(), ControlError>
pub fn neutralize(&mut self)
```

Trait implementations:

```rust
impl Clone for XboxSeriesInputs
impl Debug for XboxSeriesInputs
impl Default for XboxSeriesInputs
impl PartialEq for XboxSeriesInputs
impl Eq for XboxSeriesInputs
impl StructuralPartialEq for XboxSeriesInputs
impl Freeze for XboxSeriesInputs
impl RefUnwindSafe for XboxSeriesInputs
impl Send for XboxSeriesInputs
impl Sync for XboxSeriesInputs
impl Unpin for XboxSeriesInputs
impl UnsafeUnpin for XboxSeriesInputs
impl UnwindSafe for XboxSeriesInputs
```

### `XboxSeriesOptions`

Owner/source: `../src/gr_curated_controllers/xbox_series.rs.html`

```rust
#[non_exhaustive]pub struct XboxSeriesOptions {}
```

Trait implementations:

```rust
impl Clone for XboxSeriesOptions
impl Debug for XboxSeriesOptions
impl Default for XboxSeriesOptions
impl PartialEq for XboxSeriesOptions
impl Copy for XboxSeriesOptions
impl Eq for XboxSeriesOptions
impl StructuralPartialEq for XboxSeriesOptions
impl Freeze for XboxSeriesOptions
impl RefUnwindSafe for XboxSeriesOptions
impl Send for XboxSeriesOptions
impl Sync for XboxSeriesOptions
impl Unpin for XboxSeriesOptions
impl UnsafeUnpin for XboxSeriesOptions
impl UnwindSafe for XboxSeriesOptions
```

### `XboxSeriesOutputState`

Owner/source: `../src/gr_curated_controllers/xbox_series/controller.rs.html`

```rust
#[non_exhaustive]pub struct XboxSeriesOutputState { pub revision: u64, pub evdev_rumble: Option<RumblePlayback>, pub bluetooth_rumble: Option<XboxSeriesBluetoothRumble>, }
```

Trait implementations:

```rust
impl Clone for XboxSeriesOutputState
impl Debug for XboxSeriesOutputState
impl Default for XboxSeriesOutputState
impl PartialEq for XboxSeriesOutputState
impl Eq for XboxSeriesOutputState
impl StructuralPartialEq for XboxSeriesOutputState
impl Freeze for XboxSeriesOutputState
impl RefUnwindSafe for XboxSeriesOutputState
impl Send for XboxSeriesOutputState
impl Sync for XboxSeriesOutputState
impl Unpin for XboxSeriesOutputState
impl UnsafeUnpin for XboxSeriesOutputState
impl UnwindSafe for XboxSeriesOutputState
```

### `XboxSeriesStatus`

Owner/source: `../src/gr_curated_controllers/xbox_series.rs.html`

```rust
pub struct XboxSeriesStatus { /* private fields */ }
pub const fn target(self) -> RealizationId
```

Trait implementations:

```rust
impl Clone for XboxSeriesStatus
impl Debug for XboxSeriesStatus
impl PartialEq for XboxSeriesStatus
impl Copy for XboxSeriesStatus
impl Eq for XboxSeriesStatus
impl StructuralPartialEq for XboxSeriesStatus
impl Freeze for XboxSeriesStatus
impl RefUnwindSafe for XboxSeriesStatus
impl Send for XboxSeriesStatus
impl Sync for XboxSeriesStatus
impl Unpin for XboxSeriesStatus
impl UnsafeUnpin for XboxSeriesStatus
impl UnwindSafe for XboxSeriesStatus
```

### `XboxSeriesTrigger`

Owner/source: `../src/gr_curated_controllers/xbox_series.rs.html`

```rust
pub struct XboxSeriesTrigger(/* private fields */);
pub fn new(raw: u16) -> Result<XboxSeriesTrigger, ControlError>
pub const fn raw(self) -> u16
```

Trait implementations:

```rust
impl Clone for XboxSeriesTrigger
impl Debug for XboxSeriesTrigger
impl Default for XboxSeriesTrigger
impl PartialEq for XboxSeriesTrigger
impl Copy for XboxSeriesTrigger
impl Eq for XboxSeriesTrigger
impl StructuralPartialEq for XboxSeriesTrigger
impl Freeze for XboxSeriesTrigger
impl RefUnwindSafe for XboxSeriesTrigger
impl Send for XboxSeriesTrigger
impl Sync for XboxSeriesTrigger
impl Unpin for XboxSeriesTrigger
impl UnsafeUnpin for XboxSeriesTrigger
impl UnwindSafe for XboxSeriesTrigger
```

### `ControllerSurfaceInfo`

Owner/source: `source unavailable`

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
