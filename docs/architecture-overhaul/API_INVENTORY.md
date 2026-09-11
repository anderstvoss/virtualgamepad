# Preliminary public API inventory

Baseline: merged PR108 `6f7b084`; this inventory describes the working preliminary
candidate, not a final post-refinement freeze. Rebuild it after hands-on feedback.

The root API includes every exported type and all its inherent public methods,
not just use statements. Normal consumers need no implementation-crate import.
Derived standard traits (Debug, Clone, Copy, comparison/hash where applicable)
remain on value types; no Deref to implementation handles is provided.

## Classification

| Group | Classification | Evolution decision |
| --- | --- | --- |
| Root application types/functions | Intended alpha application contract | Opaque options/handles/identity/metadata; non-exhaustive service/output/error enums |
| Curated state/numeric/control/surface types | Controller-specific intended contract | Private semantic storage; fixed physical controls; explicit validated construction and native getters |
| Shared controls and presentation metadata | Intended descriptive contract | Native domains remain separate from Linux event surfaces; restrictions carry reasons; no mutable universal state |
| Old broad root exports | Experimental SPI only | Available under opt-in experimental module; no stability or plugin promise |
| Internal workspace public items / fake-provider seam | Implementation/testing SPI | Excluded from the root stability boundary |

## Application symbols and inherent methods

### application

**`ComponentAssociation`** — struct; [application.rs](../../src/application.rs).

Storage/fields are private.

```rust
pub const fn role(&self) -> &'static str
pub const fn surface(&self) -> &'static crate::ControllerSurface
pub fn requested_physical_path(&self) -> Option<&str>
pub fn requested_unique_id(&self) -> Option<&str>
pub fn observed_host_path(&self) -> Option<&str>
```

**`ControllerAssociation`** — struct; [application.rs](../../src/application.rs).

Storage/fields are private.

```rust
pub const fn controller(&self) -> crate::ControllerId
pub const fn realization(&self) -> RealizationId
pub const fn creation(&self) -> u64
pub fn components(&self) -> &[ComponentAssociation]
```

**`ControllerDiagnostics`** — struct; [application.rs](../../src/application.rs).

Storage/fields are private.

```rust
pub const fn status(&self) -> ControllerStatus
pub const fn frames_sent(&self) -> u64
pub const fn reverse_events_drained(&self) -> u64
pub const fn write_failures(&self) -> u64
pub const fn lifecycle_events(&self) -> u64
pub const fn dropped_output_events(&self) -> u64
pub fn last_error(&self) -> Option<&str>
```

**`ControllerError`** — enum; [application.rs](../../src/application.rs).

```rust
pub enum ControllerError {
MissingDeviceNode { target: RealizationId, path: String },
    AccessDenied { target: RealizationId, path: String },
    UnsupportedPlatform { target: RealizationId },
    InvalidRequest { reason: String },
    Open { reason: String },
    Write { reason: String },
    Read { reason: String },
    Unsupported { reason: String },
    Closed,
    WouldBlock,
}
```

**`ControllerStatus`** — enum; [application.rs](../../src/application.rs).

```rust
pub enum ControllerStatus {
NotOpen,
    Open,
    Closed,
    Failed,
}
```

**`CreationOptions`** — struct; [application.rs](../../src/application.rs).

Storage/fields are private.

```rust
pub const fn new(target: RealizationId) -> Self
pub const fn realization(self) -> RealizationId
```

**`ReadinessDescriptor`** — struct; [application.rs](../../src/application.rs).

Opaque value; construction uses methods below.

```rust
pub const fn raw_fd(&self) -> i32
```

**`ServiceReadiness`** — enum; [application.rs](../../src/application.rs).

```rust
pub enum ServiceReadiness<'a> {
Poll,
    Descriptor(ReadinessDescriptor<'a>),
}
```

### controllers

**`DualSenseController`** — struct; [controllers.rs](../../src/controllers.rs).

Storage/fields are private.

```rust
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<std::time::Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &DualSenseState
pub fn surface(&self) -> &'static DualSenseSurface
pub fn is_dirty(&self) -> bool
pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError>
pub fn set_native( &mut self, control: DualSenseControl, pressed: bool, ) -> Result<(), ControlError>
pub fn set_left_stick( &mut self, x: DualSenseAxis, y: DualSenseAxis, ) -> Result<(), ControlError>
pub fn set_right_stick( &mut self, x: DualSenseAxis, y: DualSenseAxis, ) -> Result<(), ControlError>
pub fn set_triggers( &mut self, left: DualSenseTrigger, right: DualSenseTrigger, ) -> Result<(), ControlError>
pub fn set_touch( &mut self, slot: TouchSlot, contact: Option<DualSenseTouchContact>, ) -> Result<(), ControlError>
pub fn set_motion(&mut self, motion: MotionSample) -> Result<(), ControlError>
pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError>
pub fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), ControlError>
pub fn feature_available(&self, feature: DualSenseFeature) -> Result<(), ControlError>
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(DualSenseOutputEvent), ) -> Result<(), ControllerError>
pub const fn identity(&self) -> Option<DualSenseIdentity>
```

**`DualSenseIdentity`** — struct; [controllers.rs](../../src/controllers.rs).

Opaque value; construction uses methods below.

```rust
pub fn generate() -> Result<Self, ControllerError>
pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self>
pub const fn to_bytes(self) -> [u8; 6]
```

**`DualShock4Controller`** — struct; [controllers.rs](../../src/controllers.rs).

Storage/fields are private.

```rust
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<std::time::Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &DualShock4State
pub fn is_dirty(&self) -> bool
pub fn surface(&self) -> &'static DualShock4Surface
pub fn set_digital(&mut self, u: DigitalControlUpdate) -> Result<(), ControlError>
pub fn set_native(&mut self, c: DualShock4Control, p: bool) -> Result<(), ControlError>
pub fn set_left_stick( &mut self, x: DualShock4Axis, y: DualShock4Axis, ) -> Result<(), ControlError>
pub fn set_right_stick( &mut self, x: DualShock4Axis, y: DualShock4Axis, ) -> Result<(), ControlError>
pub fn set_triggers( &mut self, l: DualShock4Trigger, r: DualShock4Trigger, ) -> Result<(), ControlError>
pub fn set_motion(&mut self, m: DualShock4MotionSample) -> Result<(), ControlError>
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

**`DualShock4Identity`** — struct; [controllers.rs](../../src/controllers.rs).

Opaque value; construction uses methods below.

```rust
pub fn generate() -> Result<Self, ControllerError>
pub const fn from_bytes(bytes: [u8; 6]) -> Option<Self>
pub const fn to_bytes(self) -> [u8; 6]
```

**`SwitchProController`** — struct; [controllers.rs](../../src/controllers.rs).

Storage/fields are private.

```rust
pub fn stream_enabled(&self) -> bool
pub fn motion_report_counter(&self) -> u8
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<std::time::Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &SwitchProState
pub fn is_dirty(&self) -> bool
pub fn surface(&self) -> &'static SwitchProSurface
pub fn set_digital(&mut self, u: DigitalControlUpdate) -> Result<(), ControlError>
pub fn set_native(&mut self, c: SwitchProControl, p: bool) -> Result<(), ControlError>
pub fn set_left_stick( &mut self, x: SwitchProAxis, y: SwitchProAxis, ) -> Result<(), ControlError>
pub fn set_right_stick( &mut self, x: SwitchProAxis, y: SwitchProAxis, ) -> Result<(), ControlError>
pub fn set_motion(&mut self, m: SwitchProMotionSample) -> Result<(), ControlError>
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(SwitchProOutputEvent), ) -> Result<(), ControllerError>
```

**`Xbox360Controller`** — struct; [controllers.rs](../../src/controllers.rs).

Storage/fields are private.

```rust
pub fn wants_write(&self) -> bool
pub fn next_service_in(&self) -> Option<std::time::Duration>
pub fn dropped_output_events(&self) -> u64
pub fn state(&self) -> &Xbox360State
pub fn surface(&self) -> &'static Xbox360Surface
pub fn is_dirty(&self) -> bool
pub fn set_digital(&mut self, update: DigitalControlUpdate) -> Result<(), ControlError>
pub fn set_battery_exposed(&mut self, exposed: bool) -> Result<(), ControlError>
pub fn set_battery_level(&mut self, level: BatteryLevel) -> Result<(), ControlError>
pub fn set_native( &mut self, control: Xbox360Control, pressed: bool, ) -> Result<(), ControlError>
pub fn set_left_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError>
pub fn set_right_stick(&mut self, x: Xbox360Axis, y: Xbox360Axis) -> Result<(), ControlError>
pub fn set_triggers( &mut self, left: Xbox360Trigger, right: Xbox360Trigger, ) -> Result<(), ControlError>
pub fn neutralize(&mut self) -> Result<(), ControlError>
pub fn commit(&mut self) -> Result<(), CommitError>
pub fn close(&mut self)
pub fn readiness(&self) -> Option<ServiceReadiness<'_>>
pub fn association(&self) -> &ControllerAssociation
pub fn diagnostics(&mut self) -> ControllerDiagnostics
pub fn service( &mut self, callback: &mut dyn FnMut(Xbox360OutputEvent), ) -> Result<(), ControllerError>
```

**`create_dualsense`** — fn; [controllers.rs](../../src/controllers.rs).

```rust
pub fn create_dualsense(options: CreationOptions) -> Result<DualSenseController, ControllerError>
```

**`create_dualsense_with_identity`** — fn; [controllers.rs](../../src/controllers.rs).

```rust
pub fn create_dualsense_with_identity( options: CreationOptions, identity: DualSenseIdentity, ) -> Result<DualSenseController, ControllerError>
```

**`create_dualshock4`** — fn; [controllers.rs](../../src/controllers.rs).

```rust
pub fn create_dualshock4( options: CreationOptions, ) -> Result<DualShock4Controller, ControllerError>
```

**`create_dualshock4_with_identity`** — fn; [controllers.rs](../../src/controllers.rs).

```rust
pub fn create_dualshock4_with_identity( options: CreationOptions, identity: DualShock4Identity, ) -> Result<DualShock4Controller, ControllerError>
```

**`create_switch_pro`** — fn; [controllers.rs](../../src/controllers.rs).

```rust
pub fn create_switch_pro(options: CreationOptions) -> Result<SwitchProController, ControllerError>
```

**`create_xbox360`** — fn; [controllers.rs](../../src/controllers.rs).

```rust
pub fn create_xbox360(options: CreationOptions) -> Result<Xbox360Controller, ControllerError>
```

### output

**`DualSenseOutputEvent`** — enum; [output.rs](../../src/output.rs).

```rust
pub enum DualSenseOutputEvent {
HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    HidOutput(DualSenseHidOutput),
}
```

**`DualShock4OutputEvent`** — enum; [output.rs](../../src/output.rs).

```rust
pub enum DualShock4OutputEvent {
HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    HidOutput(DualShock4HidOutput),
}
```

**`HostLifecycle`** — enum; [output.rs](../../src/output.rs).

```rust
pub enum HostLifecycle {
Started,
    Stopped,
    Opened,
    Closed,
}
```

**`SwitchProOutputEvent`** — enum; [output.rs](../../src/output.rs).

```rust
pub enum SwitchProOutputEvent {
HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    Output {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
}
```
```rust
pub fn rumble(&self) -> Option<SwitchProRumble>
```

**`Xbox360OutputEvent`** — enum; [output.rs](../../src/output.rs).

```rust
pub enum Xbox360OutputEvent {
HostLifecycle(HostLifecycle),
    ForceFeedback(ForceFeedbackEvent),
    HidOutput {
        report_id: Option<u8>,
        bytes: Vec<u8>,
    },
}
```

### gr_controller_contract

**`AbsoluteAxisSurface`** — struct; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

Public descriptive fields: `control: &'static str`, `event_code: u16`, `minimum: i32`, `maximum: i32`, `neutral: i32`, `flat: i32`.


**`CommitError`** — enum; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

```rust
pub enum CommitError {
Closed,
    Backend { reason: String },
}
```

**`ControlError`** — enum; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

```rust
pub enum ControlError {
UnsupportedControl { control: &'static str },
    ValueOutOfRange {
        control: &'static str,
        value: u32,
        maximum: u32,
    },
    UnavailableInRealization {
        selected_target: RealizationTarget,
        available_in: RealizationTargetSet,
    },
    Closed,
}
```

**`ControllerSurface`** — struct; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

Public descriptive fields: `target: RealizationTarget`, `validation_status: RealizationValidationStatus`, `digital_controls: &'static [DigitalControlSurface]`, `axes: &'static [AbsoluteAxisSurface]`, `outputs: &'static [OutputSurface]`, `restrictions: &'static [TargetRestriction]`.


**`ControllerSurfaceInfo`** — trait; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

```rust
pub trait ControllerSurfaceInfo {
fn common_surface(&self) -> &ControllerSurface;
}
```

**`DigitalControlSurface`** — struct; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

Public descriptive fields: `control: &'static str`, `event_code: u16`.


**`DigitalControlUpdate`** — enum; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

```rust
pub enum DigitalControlUpdate {
FaceButton {
        button: FaceButton,
        pressed: bool,
    },
    Dpad {
        direction: DpadDirection,
        pressed: bool,
    },
}
```

**`DpadDirection`** — enum; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

```rust
pub enum DpadDirection {
Up,
    Down,
    Left,
    Right,
}
```

**`FaceButton`** — enum; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

```rust
pub enum FaceButton {
North,
    South,
    East,
    West,
}
```

**`OutputSurface`** — struct; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

Public descriptive fields: `name: &'static str`, `event_type: u16`, `event_code: u16`.


**`RealizationValidationStatus`** — enum; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

```rust
pub enum RealizationValidationStatus {
ResearchBacked,
    HostValidated,
    PhysicallyValidated,
}
```

**`TargetRestriction`** — struct; [lib.rs](../../crates/gr-controller-contract/src/lib.rs).

Public descriptive fields: `feature: &'static str`, `reason: &'static str`.


### gr_curated_controllers

**`BatteryLevel`** — struct; [lib.rs](../../crates/gr-curated-controllers/src/lib.rs).

Opaque value; construction uses methods below.

```rust
pub const fn percent(self) -> u8
pub fn new(percent: u8) -> Result<Self, ControlError>
```

**`BatteryState`** — struct; [lib.rs](../../crates/gr-curated-controllers/src/lib.rs).

Storage/fields are private.

```rust
pub const fn is_exposed(self) -> bool
pub const fn level(self) -> BatteryLevel
```

**`DualSenseAxis`** — struct; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

Opaque value; construction uses methods below.

```rust
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> Self
pub const fn neutral() -> Self
```

**`DualSenseControl`** — enum; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

```rust
pub enum DualSenseControl {
Cross,
    Circle,
    Square,
    Triangle,
    L1,
    R1,
    Create,
    Options,
    PlayStation,
    TouchpadClick,
    MicrophoneMute,
    LeftStickPress,
    RightStickPress,
}
```

**`DualSenseFeature`** — enum; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

```rust
pub enum DualSenseFeature {
Touch,
    Motion,
    Lightbar,
    AdaptiveTriggers,
    Audio,
}
```

**`DualSenseHidOutput`** — enum; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

```rust
pub enum DualSenseHidOutput {
UsbOutput {
        raw: Vec<u8>,
        valid_flag0: u8,
        valid_flag1: u8,
        valid_flag2: u8,
        right_motor: Option<u8>,
        left_motor: Option<u8>,
        right_trigger_effect: [u8; 11],
        left_trigger_effect: [u8; 11],
        mute_button_led: Option<bool>,
        player_leds: Option<u8>,
        lightbar_rgb: Option<[u8; 3]>,
    },
    Unknown {
        report_id: Option<u8>,
        raw: Vec<u8>,
    },
}
```

**`DualSenseState`** — struct; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

Storage/fields are private.

```rust
pub const fn left_stick(&self) -> (DualSenseAxis, DualSenseAxis)
pub const fn right_stick(&self) -> (DualSenseAxis, DualSenseAxis)
pub const fn triggers(&self) -> (DualSenseTrigger, DualSenseTrigger)
pub const fn touch(&self, slot: TouchSlot) -> Option<DualSenseTouchContact>
pub const fn motion(&self) -> MotionSample
pub const fn battery(&self) -> BatteryState
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub const fn native_pressed(&self, control: DualSenseControl) -> bool
pub const fn dpad_pressed(&self, direction: gr_controller_contract::DpadDirection) -> bool
```

**`DualSenseSurface`** — struct; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

Storage/fields are private.

```rust
pub const fn common(&self) -> &ControllerSurface
```

**`DualSenseTouchContact`** — struct; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

Storage/fields are private.

```rust
pub fn new(id: u8, x: u16, y: u16) -> Result<Self, ControlError>
pub const fn id(self) -> u8
pub const fn x(self) -> u16
pub const fn y(self) -> u16
```

**`DualSenseTrigger`** — struct; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

Opaque value; construction uses methods below.

```rust
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> Self
```

**`DualShock4Axis`** — struct; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

Opaque value; construction uses methods below.

```rust
pub const fn new(raw: u8) -> Self
pub const fn raw(self) -> u8
```

**`DualShock4Control`** — enum; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

```rust
pub enum DualShock4Control {
Cross,
    Circle,
    Square,
    Triangle,

    L1,
    R1,
    Share,
    Options,
    PlayStation,
    TouchpadClick,
    LeftStickPress,
    RightStickPress,
}
```

**`DualShock4HidOutput`** — enum; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

```rust
pub enum DualShock4HidOutput {
UsbOutput {
        raw: Vec<u8>,
        right_motor: u8,
        left_motor: u8,
        lightbar_rgb: Option<[u8; 3]>,
    },
    Unknown {
        report_id: Option<u8>,
        raw: Vec<u8>,
    },
}
```

**`DualShock4MotionSample`** — struct; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

Public descriptive fields: `accelerometer: [i16; 3]`, `gyroscope: [i16; 3]`.


**`DualShock4State`** — struct; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

Storage/fields are private.

```rust
pub const fn motion(&self) -> DualShock4MotionSample
pub const fn left_stick(&self) -> (DualShock4Axis, DualShock4Axis)
pub const fn right_stick(&self) -> (DualShock4Axis, DualShock4Axis)
pub const fn triggers(&self) -> (DualShock4Trigger, DualShock4Trigger)
pub const fn touch(&self, slot: DualShock4TouchSlot) -> Option<DualShock4TouchContact>
pub const fn native_pressed(&self, control: DualShock4Control) -> bool
pub const fn dpad_pressed(&self, direction: gr_controller_contract::DpadDirection) -> bool
pub const fn face_pressed(&self, button: gr_controller_contract::FaceButton) -> bool
```

**`DualShock4Surface`** — struct; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

Storage/fields are private.

```rust
pub const fn common(&self) -> &ControllerSurface
```

**`DualShock4TouchContact`** — struct; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

Storage/fields are private.

```rust
pub fn new(id: u8, x: u16, y: u16) -> Result<Self, ControlError>
pub const fn id(self) -> u8
pub const fn x(self) -> u16
pub const fn y(self) -> u16
```

**`DualShock4TouchSlot`** — enum; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

```rust
pub enum DualShock4TouchSlot {
First,
    Second,
}
```

**`DualShock4Trigger`** — struct; [dualshock4.rs](../../crates/gr-curated-controllers/src/dualshock4.rs).

Opaque value; construction uses methods below.

```rust
pub const fn new(raw: u8) -> Self
pub const fn raw(self) -> u8
```

**`MotionSample`** — struct; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

Public descriptive fields: `accelerometer: [i16; 3]`, `gyroscope: [i16; 3]`.


**`SwitchProAxis`** — struct; [switch_pro.rs](../../crates/gr-curated-controllers/src/switch_pro.rs).

Opaque value; construction uses methods below.

```rust
pub const fn new(raw: i16) -> Self
pub const fn raw(self) -> i16
```

**`SwitchProControl`** — enum; [switch_pro.rs](../../crates/gr-curated-controllers/src/switch_pro.rs).

```rust
pub enum SwitchProControl {
B,
    A,
    Y,
    X,

    L,
    R,
    Zl,
    Zr,
    Minus,
    Plus,
    Home,
    Capture,
    LeftStickPress,
    RightStickPress,
}
```

**`SwitchProMotionSample`** — struct; [switch_pro.rs](../../crates/gr-curated-controllers/src/switch_pro.rs).

Public descriptive fields: `accelerometer: [i16; 3]`, `gyroscope: [i16; 3]`.


**`SwitchProRumble`** — struct; [switch_pro.rs](../../crates/gr-curated-controllers/src/switch_pro.rs).

Public descriptive fields: `packet_counter: u8`, `left: [u8; 4]`, `right: [u8; 4]`.


**`SwitchProState`** — struct; [switch_pro.rs](../../crates/gr-curated-controllers/src/switch_pro.rs).

Storage/fields are private.

```rust
pub const fn left_stick(&self) -> (SwitchProAxis, SwitchProAxis)
pub const fn right_stick(&self) -> (SwitchProAxis, SwitchProAxis)
pub const fn motion(&self) -> SwitchProMotionSample
pub const fn native_pressed(&self, control: SwitchProControl) -> bool
pub const fn dpad_pressed(&self, direction: gr_controller_contract::DpadDirection) -> bool
pub const fn face_pressed(&self, button: gr_controller_contract::FaceButton) -> bool
```

**`SwitchProSurface`** — struct; [switch_pro.rs](../../crates/gr-curated-controllers/src/switch_pro.rs).

Storage/fields are private.

```rust
pub const fn common(&self) -> &ControllerSurface
```

**`TouchSlot`** — enum; [dualsense.rs](../../crates/gr-curated-controllers/src/dualsense.rs).

```rust
pub enum TouchSlot {
First,
    Second,
}
```

**`Xbox360Axis`** — struct; [xbox360.rs](../../crates/gr-curated-controllers/src/xbox360.rs).

Opaque value; construction uses methods below.

```rust
pub const fn raw(self) -> i16
pub const fn new(raw: i16) -> Self
```

**`Xbox360Control`** — enum; [xbox360.rs](../../crates/gr-curated-controllers/src/xbox360.rs).

```rust
pub enum Xbox360Control {
A,
    B,
    X,
    Y,
    LeftShoulder,
    RightShoulder,
    Back,
    Start,
    Guide,
    LeftStickPress,
    RightStickPress,
}
```

**`Xbox360State`** — struct; [xbox360.rs](../../crates/gr-curated-controllers/src/xbox360.rs).

Storage/fields are private.

```rust
pub const fn left_stick(&self) -> (Xbox360Axis, Xbox360Axis)
pub const fn right_stick(&self) -> (Xbox360Axis, Xbox360Axis)
pub const fn triggers(&self) -> (Xbox360Trigger, Xbox360Trigger)
pub const fn battery(&self) -> BatteryState
pub const fn face_pressed(&self, button: FaceButton) -> bool
pub const fn native_pressed(&self, control: Xbox360Control) -> bool
pub const fn dpad_pressed(&self, direction: gr_controller_contract::DpadDirection) -> bool
```

**`Xbox360Surface`** — struct; [xbox360.rs](../../crates/gr-curated-controllers/src/xbox360.rs).

Storage/fields are private.

```rust
pub const fn common(&self) -> &ControllerSurface
```

**`Xbox360Trigger`** — struct; [xbox360.rs](../../crates/gr-curated-controllers/src/xbox360.rs).

Opaque value; construction uses methods below.

```rust
pub const fn raw(self) -> u8
pub const fn new(raw: u8) -> Self
```

### gr_realization_api

**`ControllerId`** — struct; [lib.rs](../../crates/gr-realization-api/src/lib.rs).

Opaque value; construction uses methods below.

```rust
pub const fn new(value: &'static str) -> Self
pub const fn as_str(self) -> &'static str
```

**`ForceFeedbackEffect`** — enum; [lib.rs](../../crates/gr-realization-api/src/lib.rs).

```rust
pub enum ForceFeedbackEffect {
Rumble(RumbleEffect),
    Unsupported { id: i16, kind: u16 },
}
```

**`ForceFeedbackEvent`** — enum; [lib.rs](../../crates/gr-realization-api/src/lib.rs).

```rust
pub enum ForceFeedbackEvent {
Uploaded {
        request_id: u32,
        effect: ForceFeedbackEffect,
        status: i32,
    },
    Erased {
        request_id: u32,
        effect_id: u32,
        status: i32,
    },
    Playback {
        effect: RumbleEffect,
        repetitions: u32,
    },
}
```

**`RealizationId`** — struct; [lib.rs](../../crates/gr-realization-api/src/lib.rs).

Opaque value; construction uses methods below.

```rust
pub const fn new(value: &'static str) -> Self
pub const fn as_str(self) -> &'static str
```

**`RealizationTargetSet`** — struct; [lib.rs](../../crates/gr-realization-api/src/lib.rs).

Opaque value; construction uses methods below.

```rust
pub const fn new(ids: &'static [RealizationId]) -> Self
pub const fn contains(self, target: RealizationId) -> bool
pub const fn is_empty(self) -> bool
```

**`RumbleEffect`** — struct; [lib.rs](../../crates/gr-realization-api/src/lib.rs).

Public descriptive fields: `id: i16`, `strong: u16`, `weak: u16`, `length_ms: u16`, `delay_ms: u16`, `trigger_button: u16`, `trigger_interval_ms: u16`.


## Former root items now experimental only

`AudioBackendFactory`, `AudioDirection`, `AudioError`, `AudioFormat`, `AudioSession`, `AudioSidecarRequirement`, `AudioStreamRequirement`, `ChannelLayout`, `ClockRequirement`, `ControllerRuntime`, `EventReadiness`, `FrameSink`, `ManifestError`, `NativeControllerRealization`, `NativeHidReportKey`, `NativeProviderFactory`, `NativeProviderSession`, `NativeRealizationError`, `PreparedRealization`, `ProviderCapabilities`, `ProviderDiagnostics`, `ProviderError`, `ProviderFrame`, `ProviderOpenRequest`, `ProviderOpenValidationError`, `ProviderPreflightError`, `ProviderRequirements`, `ProviderReverseEvent`, `ProviderReverseEventSink`, `ProviderState`, `RawReverseEvent`, `RealizationControllerDefinition`, `RealizationError`, `RealizationManifest`, `RealizationManifestEntry`, `RealizationSelection`, `RealizationSessionId`, `RealizationTarget`, `RouteIntent`, `TargetAwareControllerDriver`, `prepare_realization`, `validate_provider`.

Same-named experimental handles/options/errors retain their SPI meaning and are
not interchangeable with the root application handles. Root handles expose no
provider factories, raw requests, manual replies, generic protocol edits, or
compatibility polling alias. `RealizationId` no longer has ambiguous aliases.
