# Pre-alpha API refinement migration

This is a deliberate breaking candidate update before the final alpha freeze.
Start with the [final alpha.1 → alpha.2 consumer table](#alpha1--alpha2-final-consumer-table).
The PR sections below are chronological history: intermediate combined state
closures and standalone identity/image/options constructors are superseded by
the final table. This ledger distinguishes snapshot changes from behavior-only
changes and does not imply new hardware qualification.

- Inspect topology descriptors and `ControllerSurface` through same-named
  accessors (`topology.sticks()`, `stick.id()`, `surface.restrictions()`). Small
  range/placement/scale and label values retain public fields. Curated construction
  records are supporting-crate SPI; normal applications inspect supplied metadata.
- Remove `FaceButtonCluster::button_width`. UIs measure labels and available space.
- `RealizationId` names the exact primary realization. Explicit audio options add
  required associated endpoints; USB/IP remains its own composite realization.
- Use `ComponentAssociation::kind()` for input/audio classification. Treat `role()`
  as a display label. Audio components have no requested HID physical/unique IDs.
- Matching is unavailable. Use `AudioExposure::Disabled` or `Emulated`; do not
  reinterpret emulation as a controller-matching claim.
- Replace audio-selector `identity()` with `pipewire_node()` or `alsa_pcm()`.
  Resolve selectors anew after recreation; cached selectors do not keep nodes alive.
- `AudioRead` is root-owned and keeps `frames`, `first_frame`, `discontinuity`.
- `ControllerAudio::diagnostics()` returns opaque retained application health.
  Existing meaningful loss/error getters remain available. Graph timing and
  bridge scheduling now require `experimental::audio_instrumentation(audio)` and
  the opt-in `experimental` feature; acceptance examples declare that requirement.
- Rename `microphone_host_frames()` to `microphone_consumed_frames()`. It is
  optional sample-pacing progress, including silence, not proof of host delivery.
- Match `AudioError::InvalidSampleBuffer`, `OwnershipMismatch`, `Unsupported`,
  `Unavailable`, `Closed`, and `Backend` for their respective conditions.
  `AccessDenied` retains OS/backend permission meaning.
- Failed creation may return `ControllerError::Cleanup { cause, cleanup }`.
  Inspect the typed initiating cause; cleanup details supplement it.
- Audio is required when requested. Failure discovery closes the logical controller;
  recreate without audio explicitly if desired. No reopening or silent degradation.

The unused old audio session/factory traits are removed from internal SPI. Manifest
sidecar requirements remain distinct descriptive contracts, not another live backend.

Post-GUI audit: the empty root `audio-alsa` feature is removed. ALSA host-device
routing belongs to the demo's `audio-alsa` feature, whose defaults are unchanged.
Library users enable `audio-pipewire` for local UHID audio or `audio-usbip` for the
WIP composite; ALSA routing does not add a library realization.

## Historical capability and supporting-API ledger

These entries were checked against the introducing main commits and their
checked-in snapshots, then against current code for final defaults. Both root
feature views receive each stated root delta; supporting scopes are separate.

### #225 · Typed capabilities and retained outputs (`3e241e9`)

Root 110 → 122 pages: add `DisplayFormat`, `InputKind`, `LedKind`, `MotorRole`,
`OutputKind`, `SpeakerFormat`, `TypedInput`, `RumblePlayback`,
`DualSenseOutputState`, `DualShock4OutputState`, `SwitchProOutputState` and
`Xbox360OutputState`. `TypedInput` exposes `id()`, `title()` and `kind()`.
`InputTopology::typed_inputs()` and `ControllerSurface::output_kinds()` describe
native capabilities. `InputTopologyError` becomes non-exhaustive and adds
`InvalidTypedInput(InputControlId)`; downstream matches need a wildcard.
The four original handles add `host_outputs() -> Result<FamilyOutputState,
ControlError>`. `RumblePlayback` retains `strong: u16`, `weak: u16` and
`repetitions: u32`. Read retained outputs instead of reconstructing accepted
state from optional service events. #350 later adds USB/IP recovery with no
snapshot delta.

### #233 / #250 · Initial Wii supporting API and resources (`714958d` / `0c36691`)

C5 #233 introduces the production family. #250 first records twenty supporting
pages: `WiiExtension`, `WiiRemoteButton`, `WiiRemoteRevision`, `WiiResourceDisposition`, `create_wii_remote`, `create_wii_remote_with_image`, `ir_kind`, `BLUETOOTH`, `TOPOLOGY`, `UINPUT`, `WiiClassicState`, `WiiEepromImage`, `WiiExtensionCalibration`, `WiiIrPoint`, `WiiMotionPlusState`, `WiiNunchukState`, `WiiRemoteController`, `WiiRemoteDefinition`, `WiiRemoteOutputs`, `WiiRemoteState`.
The historical constructors accept target/session, with an image/revision path.
`WiiExtensionCalibration::import` initially takes optional MotionPlus `[u8; 32]`
and Nunchuk/Classic `[u8; 16]` blocks; `WiiEepromImage::with_extension_calibration`
attaches checked caller-provided resources. These are not synthesized factory
calibration. Uinput refuses the addressed-memory resource path. #319 replaces
the constructor and positional import shapes; #390 replaces the input names.

### #256 · Wii passthrough creation options (`1c810c5`)

Supporting 20 → 22 pages: add `WiiRemoteCreationOptions` and
`create_wii_remote_with_options`. Its public booleans
`motionplus_passthrough_id_mirror` and `motionplus_passthrough_calibration_copy`
both default to true. #319 later replaces public option fields with builders.

### #263 · Wii inactive identity option (`29e4724`)

Add `WiiRemoteCreationOptions::motionplus_inactive_id_external_style: bool`,
default false. This compatibility behavior applies to the Tr revision, not a
new controller identity or inferred physical capability. No page-count change.

### #264 · Wii activation status edge (`f40ea45`)

Add `motionplus_activation_reattach_status: bool`, default true, to the historical
creation options. It controls the activation-from-inactive reattach status edge
when a child is present. #319 retains it as an explicit family option builder.

### #301 · Initial wired Steam supporting surface (`ad80ae6`)

C6 first records eleven supporting pages: `STEAM_CONTROLLER_2026_ID`, `SteamController2026Button`, `SteamController2026Haptic`, `create_steam_controller_2026`, `create_steam_controller_2026_with_options`, `SteamController2026Controller`, `SteamController2026CreationOptions`, `SteamController2026Outputs`, `SteamController2026State`, `SteamMotionSample`, `SteamTrackpadState`.
The target/session constructors support wired UHID and reduced uinput; historical
`SteamController2026CreationOptions` has `serial: Option<String>`, default None
for a generated per-instance identity. Native input, motion and trackpad domains
remain family-specific. Subsequent PRs add live options, outputs and scheduling;
#319 and #387 establish the final constructor and input/status names.

### #305 · Steam client compatibility option (`f3b5412`)

Add `steam_client_compat: bool` to historical creation options, initially false.
It selects the recorded wired HID compatibility profile; uinput has no feature
channel. #307 (`baf9d4e`) changes this default to true without a snapshot delta.
The current `SteamController2026Options::default()` keeps compatibility on.

### #306 · Steam haptic PCM compatibility stream (`bc95936`)

Supporting 11 → 13 pages: add `SteamController2026StreamPacket` and
`SteamController2026StreamOutputs`. Add opt-in `haptic_streaming: bool` (default
false), `SteamController2026Haptic::Stream(Box<SteamController2026StreamPacket>)`
and `SteamController2026Outputs::streaming`. Packets are `Configure { operation,
target, mode }` (u8 fields), `Mono { target: u8, data: Vec<u8> }` and
`Stereo { left: Vec<u8>, right: Vec<u8> }`, decoded by `from_output`.
Retained stream fields are `enabled: bool`, `configured: [bool; 4]`,
`buffered_bytes: [u16; 4]`, `received_bytes: [u64; 4]`,
`overrun_bytes: [u64; 4]` and `last_packet: Option<SteamController2026StreamPacket>`.
These compatibility payload paths remain until the separately scoped typed
stream migration under ADR-0024; step e does not implement that migration.

### #308 · Steam retained LED intent (`f7fec82`)

Add `led_brightness: Option<u16>`, `led_override: Option<bool>` and
`led_rgbw: Option<[u8; 4]>` to `SteamController2026Outputs`, without new pages.
Unknown/unobserved intent is None; brightness/color payloads are native values.

### #310 · Steam desktop support option (`547be27`)

Add `lizard_mode: bool` to historical creation options, default false. The final
`SteamController2026Options::with_lizard_mode` builder still defaults to false;
compatibility remains true and haptic streaming false. Configured desktop-report
support and the host's last accepted lizard setting are separate status reads,
`lizard_mode()` and `lizard_setting()`. Reduced uinput has no desktop HID channel.

### #314 · Complete Wii resource export (`38cef2b`)

Add `WiiEepromImage::extension_calibration() -> &WiiExtensionCalibration` and
calibration readers `motion_plus() -> Option<[u8; 32]>`,
`nunchuk() -> Option<[u8; 16]>`, `classic() -> Option<[u8; 16]>`.
Export all imported blocks with the EEPROM bytes; no snapshot page-count change.

### #319 · Curated Wii and Steam constructor convergence (`12295d4`)

Both supporting families converge on `create_<family>(CreationOptions,
<Family>Options) -> Result<FamilyController, ProviderError>` before C7 root
promotion. Remove `create_wii_remote_with_image`, `create_wii_remote_with_options`,
`WiiRemoteCreationOptions`, `create_steam_controller_2026_with_options` and
`SteamController2026CreationOptions`; add `WiiRemoteOptions`, `WiiRemoteIdentity`,
`SteamController2026Options`, `SteamController2026Identity` and handle `identity()`.
Wii supporting 22 → 21 pages; Steam stays at 13; root unchanged.

Wii builder/reader pairs cover `revision`, `eeprom_image`, `identity`,
`motionplus_passthrough_id_mirror`, `motionplus_passthrough_calibration_copy`,
`motionplus_inactive_id_external_style`, `motionplus_activation_reattach_status`.
Defaults are Original revision, empty EEPROM, no restored identity, and true,
true, false, true for the four compatibility flags. Steam pairs cover `identity`,
`steam_client_compat`, `haptic_streaming`, `lizard_mode`, with None, true, false,
false defaults. Each builder uses the `with_` prefix; read through the matching
name. Wii identity uses `from_bytes`, `generate`, `bytes`; Steam uses
`from_serial`, `generate`, `serial`. Validation is fallible; no fallback identity.

Replace positional `WiiExtensionCalibration::import` with fallible
`with_motion_plus`, `with_nunchuk`, `with_classic`, keeping the three readers from
#314. The old standalone constructors and public option fields are historical.

### #320 · Steam scheduling and extensibility (`bf8564e`)

Add handle `next_service_in() -> Option<Duration>`, `wants_write() -> bool`,
`is_dirty() -> bool`, `dropped_output_events() -> u64`.
`SteamController2026Haptic`, `SteamController2026Outputs` and
`SteamController2026StreamOutputs` become non-exhaustive: use wildcard matching
and do not construct the output structs using external literals. Supporting page
count remains 13. Observe readiness, deadlines and write interest independently
of input edits; output loss is not a count of lost required replies.

## C7 step 20a · Curated controller creation

The curated crate and corresponding `virtualgamepad::experimental` exports
now use common realization options plus family options. The ordinary root,
Wii and Steam snapshots are unchanged in this step.

| Before | After |
| --- | --- |
| Curated `create_dualsense(creation)` | `create_dualsense(creation, DualSenseOptions::default())` |
| Curated `create_dualshock4(creation)` | `create_dualshock4(creation, DualShock4Options::default())` |
| Curated `create_switch_pro(creation)` | `create_switch_pro(creation, SwitchProOptions::default())` |
| Curated `create_xbox360(creation)` | `create_xbox360(creation, Xbox360Options::default())` |
| Curated `create_dualsense_with_identity(creation, id)` | `create_dualsense(creation, DualSenseOptions::default().with_identity(id))` |
| Curated `create_dualshock4_with_identity(creation, id)` | `create_dualshock4(creation, DualShock4Options::default().with_identity(id))` |
| Curated `create_switch_pro_with_identity(creation, id)` | `create_switch_pro(creation, SwitchProOptions::default().with_identity(id))` |
| Curated Sony identity `to_bytes()` | `bytes()` |
| Curated Sony identity `from_bytes(bytes) -> Option<_>` | `from_bytes(bytes) -> Result<_, ProviderError>` |

Curated Sony handles expose their local USB/UHID pairing identity via
`identity()`. Sony address bytes use pairing-feature order. Switch Pro and Wii
use display order. All require locally administered unicast flags. The wired
Xbox 360 options carry no persistent identity for the existing standard-HID
presentation (research `x360-l0-consumer-sdl360-vs-generic-hid`).

Curated USB worker constructors still bind an already opened bridge. The
application-side USB/IP path resolves identity before opening that bridge.
Registry entries now describe the older families' existing audio components
and realization restrictions. Sony input-lab isolation is an environment
setting for acceptance runs, so it has no family option or demo toggle.

## C7 step 20b · Ordinary-root controller creation

Root `CreationOptions` still selects the realization and audio.
The family options and identity types are re-exported from the curated crate.
The ordinary API remains open to changes during alpha.

| Before | After |
| --- | --- |
| Root `create_dualsense(creation)` | `create_dualsense(creation, DualSenseOptions::default())` |
| Root `create_dualshock4(creation)` | `create_dualshock4(creation, DualShock4Options::default())` |
| Root `create_switch_pro(creation)` | `create_switch_pro(creation, SwitchProOptions::default())` |
| Root `create_xbox360(creation)` | `create_xbox360(creation, Xbox360Options::default())` |
| Root `create_dualsense_with_identity(creation, id)` | `create_dualsense(creation, DualSenseOptions::default().with_identity(id))` |
| Root `create_dualshock4_with_identity(creation, id)` | `create_dualshock4(creation, DualShock4Options::default().with_identity(id))` |
| Root Sony identity `to_bytes()` | `bytes()` |
| Root Sony identity `const from_bytes(bytes) -> Option<_>` | `from_bytes(bytes) -> Result<_, ProviderError>`. Use `.ok()` for an option. Validate at runtime if migrating a const initializer. |
| Root Sony identity `generate() -> Result<_, ControllerError>` | `generate() -> Result<_, ProviderError>` from the shared curated type. Handle the error at the persistence boundary. |

The root adds `SwitchProIdentity` and `SwitchProController::identity()`.
Restore a stored Switch Pro address via `SwitchProOptions::with_identity`.
The existing Sony root handle identity accessors still return an option.
Wii and Steam supporting snapshots are unchanged.

## C7 step 20c · Wii and Steam at the ordinary root

Wii Remote and Steam Controller 2026 constructors now also live at
`virtualgamepad::create_wii_remote` and
`virtualgamepad::create_steam_controller_2026`. Call them with ordinary
`CreationOptions::new(realization)` and the existing curated family options.
The curated constructors retain their signatures. The root creates the
process-local session token internally. The root re-exports family option,
identity, state, retained-output and resource types.

The root `service` callbacks receive `WiiRemoteOutputEvent` or
`SteamController2026OutputEvent`. For curated callers switching to the root,
match `HostLifecycle`, `ForceFeedback` and `HidOutput` on the corresponding
typed enum in place of the raw reverse event. `HidOutput` preserves the report
ID and payload. Use `host_outputs()` for retained accepted values. Steam
`readiness()`, `wants_write()` and `next_service_in()` preserve curated
scheduling. Wii `attach()`, topology accessors, `resource_dirty()` and
`resolve_resource()` preserve the curated ownership and cleanup rules.
`association()` exposes application components, refreshed after a successful
Wii attachment change.

The Wii and Steam supporting snapshots are unchanged. No existing ordinary-root
declaration is removed or renamed in this step.

C7 #329 (`692f41b`) adds exactly these root pages (historical names at
promotion; the d entries below record their final replacements):

- Wii (17): `WiiExtension`, `WiiRemoteButton`, `WiiRemoteOutputEvent`, `WiiRemoteRevision`, `WiiResourceDisposition`, `create_wii_remote`, `WiiClassicState`, `WiiEepromImage`, `WiiExtensionCalibration`, `WiiIrPoint`, `WiiMotionPlusState`, `WiiNunchukState`, `WiiRemoteController`, `WiiRemoteIdentity`, `WiiRemoteOptions`, `WiiRemoteOutputs`, `WiiRemoteState`.
- Steam (13): `SteamController2026Button`, `SteamController2026Haptic`, `SteamController2026OutputEvent`, `SteamController2026StreamPacket`, `create_steam_controller_2026`, `SteamController2026Controller`, `SteamController2026Identity`, `SteamController2026Options`, `SteamController2026Outputs`, `SteamController2026State`, `SteamController2026StreamOutputs`, `SteamMotionSample`, `SteamTrackpadState`.

Together with five family option/identity pages and removal of the two standalone
Sony identity constructors, root grows 122 → 155 pages (+35 / −2).

## C9 step 15a · Xbox Series native types

The curated crate adds `xbox_series::{XboxSeriesOptions, XboxSeriesState,
XboxSeriesButton, XboxSeriesTrigger}`. State uses signed stick coordinates in
the xpad evdev orientation (positive Y down), independent Guide and Share
buttons, and validated trigger positions from 0 through 1023. Use
`XboxSeriesState::set_triggers(left, right)` to validate both trigger positions
before replacing either one. The options value has no settings for uinput.

## C9 step 15b · Xbox Series curated uinput controller

The curated `xbox_series` module adds
`create_xbox_series(CreationOptions, XboxSeriesOptions)` and the
`XboxSeriesController` and `XboxSeriesOutputState` types. Select
`LINUX_UINPUT`. The handle provides native edits, lifecycle servicing and
retained conventional rumble playback. Its surface explains the limits of
uinput force feedback and device metadata.

## C9 step 15c · Xbox Series at the ordinary root

The ordinary root adds `create_xbox_series(CreationOptions, XboxSeriesOptions)`,
`XboxSeriesController`, `XboxSeriesOutputEvent` and the re-exported family types
`XboxSeriesOptions`, `XboxSeriesButton`, `XboxSeriesState`,
`XboxSeriesTrigger` and `XboxSeriesOutputState`. The root constructor handles
the internal session and association. It refuses audio and any target other
than `LINUX_UINPUT`. The typed service callback carries force-feedback
observations and `host_outputs()` retains the most recent playback request.

The curated state adds `XboxSeriesState::set_digital` for spatial face and
D-pad edits. The compiled registry adds Xbox Series with no USB worker,
audio subsystem or creation toggles. Existing family signatures and the
separate Wii and Steam snapshots are unchanged.

## USB/IP retained host outputs (#350, tracking #203)

- The existing DualSense, DualShock 4, and Xbox 360 `host_outputs()` getters now
  return typed snapshots on the USB/IP realization. Read after `service()` for
  a refreshed view and after orderly close for the final view. On worker failure,
  read the last validated snapshot and consult controller diagnostics.
- The USB/IP surfaces and compiled registry no longer refuse retained host
  outputs. The Xbox 360 standard-HID worker returns an empty snapshot at
  revision zero because that personality accepts no host output reports.
- Update the installed worker alongside the application. USB/IP creation
  requires private retained-output operation v1. Ordinary-root API signatures
  and the reviewed signature snapshots are unchanged.

## VG-1 · Whole-state input replacement (#352, source #348)

Each of these methods is available on the ordinary root handle and its
curated counterpart. Each takes the named state by value and returns
`Result<(), ControlError>`.

- `DualSenseController::set_inputs(DualSenseState)`
- `DualShock4Controller::set_inputs(DualShock4State)`
- `SwitchProController::set_inputs(SwitchProState)`
- `Xbox360Controller::set_inputs(Xbox360State)`

Start with `State::default()` for a neutral snapshot or clone `controller.state()`
to edit an accepted snapshot. The state types exposed by both layers gained
these methods for building a candidate independently of the controller.

| State type | Added methods |
| --- | --- |
| `DualSenseState` | `set_native`, `set_digital`, `set_sticks`, `set_triggers`, `set_touch`, `set_motion` |
| `DualShock4State` | `set_native`, `set_digital`, `set_sticks`, `set_triggers`, `set_touch`, `set_motion` |
| `SwitchProState` | `set_native`, `set_digital`, `set_sticks`, `set_motion` |
| `Xbox360State` | `set_native`, `set_digital`, `set_sticks`, `set_triggers` |

A successful replacement stages one complete input edit. Rejection preserves
the accepted state and its dirty flag. The destination session keeps its
battery metadata, identity, host outputs, and protocol handshakes. Changed
motion advances the destination's motion counters. DS4 touch changes also
advance the touch packet sequence. Repeating an identical snapshot keeps the
accepted counters at their current values.
Call `commit()` for delivery and continue servicing the session on its advertised
readiness and deadlines. A commit can produce zero or multiple reports, and
consumers can observe intermediate deliveries. See [ADR-0004](adr/ADR-0004-synchronous-hid-session.md).

Whole-state edits reject opposing D-pad directions when the target encodes
them as a hat. Switch Pro USB reports carry separate directional bits. On
uinput, nonzero DualSense, DualShock 4 and Switch Pro motion and an active
DualSense microphone mute button return `UnavailableInRealization`. DualShock
4 touch IDs and DualSense USB touch IDs are limited to `0..=127`. DualSense
uinput touch IDs use `0..=255`.

## VG-1b/a · Final-state validation and input clocks

The root API snapshot adds two methods,
`DualSenseController::refresh_motion() -> Result<(), CommitError>` and
`DualShock4Controller::refresh_motion() -> Result<(), CommitError>`. The curated
handles expose the same methods. The Wii and Steam snapshots have zero
additions, removals or signature changes.

- The existing setters and whole-state replacements share final-state
  validation. `set_digital` rejects opposing directions on hat targets.
  Switch Pro USB continues to accept independent directions.
- `set_touch` enforces the same contact ID bounds as `set_inputs`. Setting
  the DualSense microphone mute button to pressed on uinput returns
  `UnavailableInRealization` because that target has no mute event.
- The three motion setters return `UnavailableInRealization` on uinput.
  A snapshot with neutral motion is valid there.
- An accepted motion change advances the DualSense sensor timestamp by
  12,000, the DS4 timestamp by 750, or the Switch Pro state timer by 1.
  This includes a return to neutral via `neutralize()`. Repeating the
  same motion value leaves these counters at their current positions.
- DS4 advances the touch sequence once when contacts change, including the
  last release. A simultaneous motion change cannot add a second touch
  increment. Repeating the same contact edit leaves the sequence in place.

- `refresh_motion()` explicitly advances the Sony motion clock and sequence
  even when the motion values are equal, then commits the current input.
  DS4 also advances the touch sequence when a contact is active.
  The demo uses this operation for periodic motion samples. Uinput refuses
  this operation because it has no motion presentation. The method returns
  `CommitError`, matching Switch Pro's refresh operation. After backpressure,
  retry with `commit()` to deliver the accepted sample at its current clock.

## VG-1b/b · Transactional native input edits (#363)

This section records the intermediate combined-state closure shape; step d
supersedes it with input-only types. Use the final consumer table for new code.

All seven curated and ordinary-root controller handles add
`edit_inputs(&mut self, edit: impl FnOnce(&mut FamilyState) -> Result<(), ControlError>) -> Result<(), ControlError>`.
The candidate type is the family's existing native state type.

| Handle | Candidate |
| --- | --- |
| `DualSenseController` | `DualSenseState` |
| `DualShock4Controller` | `DualShock4State` |
| `SwitchProController` | `SwitchProState` |
| `Xbox360Controller` | `Xbox360State` |
| `WiiRemoteController` | `WiiRemoteState` |
| `SteamController2026Controller` | `SteamController2026State` |
| `XboxSeriesController` | `XboxSeriesState` |

The root snapshot adds seven methods in each feature view. The Wii and Steam
supporting snapshots each add one method. There are zero removed declarations
or changed existing signatures. The supporting runtime crates also add
`ControllerRuntime::edit_state` and `gr_hid::Runtime::edit_state` for validated
candidate installation and change detection.

The closure edits an isolated candidate. Its error, or a final-state validation
error, rejects the entire edit and preserves accepted inputs, clocks, dirty work,
queued reports and host outputs. Closed handles reject before invoking the closure.
Keep external side effects outside the closure.

`set_inputs`, field setters, digital edits and `neutralize` use the same input
transition. Only the final levels determine validity and motion/contact clock
changes. An accepted unchanged candidate adds no dirty work and preserves existing
dirty work and queued bytes. Wii companion nodes also skip unchanged state when
committing.

```rust
controller.edit_inputs(|state| {
    state.set_native(DualSenseControl::Cross, true);
    state.set_sticks((left_x, left_y), (right_x, right_y));
    Ok(())
})?;
controller.commit()?;
```

Existing family ownership rules apply to edits. Sony and Xbox 360 battery,
Sony and Switch counters, and Switch streaming state remain controller-owned.
Wii revision and attachment changes use the live attachment API. Wii raw battery
ownership and the input-only value types remain scheduled for steps c and d.
Steam's motion timestamp remains caller-supplied. Explicit Sony motion refresh,
host servicing, Wii attachment and EEPROM, and Steam protocol modes retain their
separate operations.

## VG-1b/c1 · Steam reduced-uinput input validation

The root, Wii and Steam signature snapshots each have zero additions, removals
or changed declarations.

Steam Controller 2026 now rejects non-neutral IMU fields, capacitive stick or
grip contact, and pad pressure on `LINUX_UINPUT` with
`ControlError::UnavailableInRealization`. IMU fields include the source timestamp,
acceleration, gyro and quaternion. The reduced gamepad has no corresponding
sensor node, contact keys or pressure axes. The same final-state validation runs
for `set_inputs`, `edit_inputs` and field operations. Rejected edits preserve
accepted state and dirty work.

Pad contact-gated position and independent clicks remain available on reduced
uinput. The demo uses raw-zero motion for reduced sessions and its existing
synthetic stationary pose for wired UHID. Xtask samples include IMU, capacitance
and pressure only for wired UHID.


## VG-1b/c2 · Wii battery metadata

The root and Wii supporting snapshots each add `WiiRemoteBattery`, with
`new(raw: u8) -> Self` and `raw(self) -> u8`, plus these handle methods.

- `WiiRemoteController::battery(&self) -> WiiRemoteBattery`
- `WiiRemoteController::set_battery(&mut self, battery: WiiRemoteBattery) -> Result<(), ControlError>`

Each snapshot removes three methods.

- `WiiRemoteState::battery_raw`
- `WiiRemoteState::set_battery_raw`
- `WiiRemoteController::set_battery_raw`

The root delta applies to both feature views. The Steam snapshot has zero
additions, removals or changed declarations.

Replace `controller.set_battery_raw(raw)` with
`controller.set_battery(WiiRemoteBattery::new(raw))`. Read the byte through
`controller.battery().raw()`. Battery is absent from `WiiRemoteState`;
`set_inputs` and `edit_inputs` own input levels. The native raw domain is
`0..=255`, with the existing default of zero and low-battery status threshold
below `0x20`. There is no inferred percentage conversion.

Accepted battery updates are metadata transactions. Future Wii status reports
carry the accepted byte, while queued reports retain their existing bytes.
Input replacement, neutralization and attachment changes preserve battery.
Closed handles retain the last accepted metadata and reject updates. Uinput
retains the typed metadata locally; its published restriction identifies the
missing power-supply endpoint.

The in-repo live probe uses the typed accessor. Revision and attachment remain
managed by live topology and `attach`, and EEPROM and host outputs retain their
separate ownership.

## Xbox Series Bluetooth-bus UHID (#369, `5f83165`)

`create_xbox_series` accepts `LINUX_UHID_BLUETOOTH` with the existing
options and native input state. The selected surface advertises native
four-channel Bluetooth HID rumble. `XboxSeriesOutputEvent` adds HID
lifecycle and raw output observations, and `XboxSeriesOutputState` adds
`bluetooth_rumble` with native motor amplitudes and timing fields.

The motor-role vocabulary adds `LeftTrigger` and `RightTrigger` for
native trigger channels. Bluetooth retained magnitudes are logical
0–100 values, while evdev retained magnitudes stay 0–65535.

`XboxSeriesBluetoothRumble` is non-exhaustive and has eight u8 fields:
`motor_mask`, `left_trigger`, `right_trigger`, `strong`, `weak`, `duration`,
`delay`, `repeat`. Four motor amplitudes use logical 0–100 units; `duration`
and `delay` use descriptor-defined 10 ms ticks; `repeat` is a raw count and
`motor_mask` selects channels. `XboxSeriesOutputState::bluetooth_rumble` is
`Option<XboxSeriesBluetoothRumble>`. The exact added output variants are
`HostLifecycle(HostLifecycle)` and `HidOutput { report_id: Option<u8>, bytes:
Vec<u8> }`; `ForceFeedback` remains. This is local Bluetooth-bus emulation,
not radio pairing. Motor role variants are `LeftTrigger` and `RightTrigger`.

## VG-1b/d · DualSense input-only values

The ordinary root and curated DualSense handle now use `DualSenseInputs` for
caller-owned native levels. The public combined `DualSenseState` type is removed.
Its input getters and native editing helpers live on `DualSenseInputs`;
battery is read through `DualSenseStatus`.

| Previous API | Current API |
| --- | --- |
| `DualSenseState` | `DualSenseInputs` |
| `controller.state()` | `controller.inputs()` |
| `controller.state().battery()` | `controller.status().battery()` |
| `set_inputs(DualSenseState)` | `set_inputs(DualSenseInputs)` |
| `edit_inputs(FnOnce(&mut DualSenseState) -> Result<(), ControlError>)` | `edit_inputs(FnOnce(&mut DualSenseInputs) -> Result<(), ControlError>)` |

The root snapshot removes one type and adds two types, `DualSenseInputs` and
`DualSenseStatus`, in each feature view. The handle removes `state()`, adds
`inputs() -> &DualSenseInputs` and `status() -> DualSenseStatus`, and changes the
two input mutation signatures above. The fourteen input getters/mutators from
the old state live on the new input type. Status adds `battery()`,
`input_sequence()` and `sensor_timestamp()` read accessors. Both new value types
support defaults, clone and equality; status is also copyable. The Wii and Steam
supporting snapshots have zero changes.

Battery setters remain handle operations. The accepted motion clock advances
on changed motion and explicit `refresh_motion()`. Replacing input values,
including with defaults, preserves battery and derives clock transitions from
the accepted input change. Identity and host outputs retain their existing
accessors.

Supporting worker/runtime code uses `DualSenseSessionState`, containing separate
input and status values. The in-repo worker, root-audio adapter, demo and tests
are migrated together. The worker envelope and HID report bytes are pinned
against the pre-split synthetic payload; their encodings retain the existing
format and version.

## VG-1b/d · DualShock 4 input-only values

`DualShock4Inputs` replaces `DualShock4State` on the ordinary root and curated
surfaces. It contains native controls, contacts and motion only. `inputs()`
borrows accepted levels; `status()` returns a copyable `DualShock4Status` with
battery, accepted motion sequence, sensor timestamp and touch packet sequence.

| Previous API | Current API |
| --- | --- |
| `DualShock4State` | `DualShock4Inputs` |
| `controller.state()` | `controller.inputs()` |
| `set_inputs(DualShock4State)` | `set_inputs(DualShock4Inputs)` |
| `edit_inputs(FnOnce(&mut DualShock4State) -> Result<(), ControlError>)` | `edit_inputs(FnOnce(&mut DualShock4Inputs) -> Result<(), ControlError>)` |

In each root snapshot feature view, remove the old type and add `DualShock4Inputs`
and `DualShock4Status`. Remove the handle's `state()`, add
`inputs() -> &DualShock4Inputs` and `status() -> DualShock4Status`, and change
both input mutation signatures above. The fourteen native getters/mutators move
to the input type. Status adds four read accessors: `battery()`, `sequence()`,
`sensor_timestamp()` and `touch_sequence()`. Both types support defaults, clone
and equality; status is also copyable. Wii and Steam supporting snapshots have
zero delta.

The experimental surface exports both new types, plus the supporting worker
`DualShock4SessionState`; it also now exports `DualSenseInputs` and
`DualSenseStatus` alongside the session type introduced in #370.

The runtime retains one authoritative status value. Input edits preserve battery
and apply the existing change-triggered motion and contact timing, including
last-contact release and the motion/contact double-bump guard. Explicit
`refresh_motion()` remains a separate clock update. Worker and HID bytes match
synthetic baselines captured before the split, with family byte and field order
unchanged. All compiling worker, root-audio, demo and test consumers migrate in
this PR; no compatibility alias remains.

## VG-1b/d · Switch Pro input-only values

`SwitchProInputs` replaces `SwitchProState` on the ordinary root, curated and
experimental surfaces. Native buttons, sticks and motion remain caller-owned;
accepted motion timing and host-selected stream state are read through copyable
`SwitchProStatus`.

| Previous API | Current API |
| --- | --- |
| `SwitchProState` | `SwitchProInputs` |
| `controller.state()` | `controller.inputs()` |
| `set_inputs(SwitchProState)` | `set_inputs(SwitchProInputs)` |
| `edit_inputs(FnOnce(&mut SwitchProState) -> Result<(), ControlError>)` | `edit_inputs(FnOnce(&mut SwitchProInputs) -> Result<(), ControlError>)` |

In each root snapshot feature view, remove the old type and add `SwitchProInputs`
and `SwitchProStatus`. Remove the handle's `state()`, add
`inputs() -> &SwitchProInputs` and `status() -> SwitchProStatus`, and change both
input mutation signatures above. Ten native getters/mutators move to the input
type. Status adds `timer()` and `stream_enabled()` read accessors. Both types
support defaults, clone and equality; status is also copyable. Wii and Steam
supporting snapshots have zero delta.

`status().timer()` is the accepted, change-triggered motion clock. The existing
`motion_report_counter()` retains its realization-specific transport counter
semantics; UHID cadence can advance that counter independently of input edits.
`status().stream_enabled()` reads the live host-selected stream state, also
available through the existing handle accessor. No battery telemetry is invented
for this family. Input edits preserve stream state, while motion changes advance
the accepted timer once. `refresh_motion()` retains its separate runtime update
and delivery behavior.

The supporting runtime holds the separate input and status values as a pair;
there is no additional combined public family state type or worker codec. The
demo and tests migrate with the API, retaining protocol, provider, handshake,
queued-report and cleanup regressions.

## VG-1b/d · Xbox 360 input-only values

`Xbox360Inputs` replaces `Xbox360State` on the ordinary root and curated surfaces.
Native buttons, sticks and triggers are caller-owned input levels; battery is
separate controller metadata returned by copyable `Xbox360Status`.

| Previous API | Current API |
| --- | --- |
| `Xbox360State` | `Xbox360Inputs` |
| `controller.state()` | `controller.inputs()` |
| `controller.state().battery()` | `controller.status().battery()` |
| `set_inputs(Xbox360State)` | `set_inputs(Xbox360Inputs)` |
| `edit_inputs(FnOnce(&mut Xbox360State) -> Result<(), ControlError>)` | `edit_inputs(FnOnce(&mut Xbox360Inputs) -> Result<(), ControlError>)` |

In each root snapshot feature view, remove the old type and add `Xbox360Inputs`
and `Xbox360Status`. Remove the handle's `state()`, add
`inputs() -> &Xbox360Inputs` and `status() -> Xbox360Status`, and change both input
mutation signatures above. Ten native getters/mutators move to the input type;
status adds the `battery()` getter. Both values support defaults, clone and
equality; status is also copyable. Wii and Steam supporting snapshots have zero
delta.

Root, curated and experimental exports migrate together. The supporting worker
representation is the doc-hidden `Xbox360SessionState`, with separate inputs and
status. Worker envelope and HID bytes are pinned against synthetic levels
captured before the split: family byte 3, version and field order are unchanged.
Battery setters remain separate runtime updates, while input edits and
neutralization preserve accepted metadata. No input clock is invented for this
family. Worker, root-audio, demo and test consumers migrate in the same PR with
no compatibility alias.

## VG-1b/d · Steam Controller 2026 input-only values (#387, `685479b`)

`SteamController2026Inputs` replaces `SteamController2026State`. Its native input
fields, including caller-supplied `motion.timestamp_us`, retain their domains and
encoding. `inputs()` borrows accepted levels. Copyable `SteamController2026Status`
reads the live wired protocol's counter and mode metadata.

| Previous API | Current API |
| --- | --- |
| `SteamController2026State` | `SteamController2026Inputs` |
| `controller.state()` | `controller.inputs()` |
| `set_inputs(SteamController2026State)` | `set_inputs(SteamController2026Inputs)` |
| `edit_inputs(FnOnce(&mut SteamController2026State) -> Result<(), ControlError>)` | `edit_inputs(FnOnce(&mut SteamController2026Inputs) -> Result<(), ControlError>)` |

In both root snapshot feature views and the Steam supporting snapshot, remove
the old type and add `SteamController2026Inputs` and `SteamController2026Status`.
Remove the handle's `state()`, add `inputs() -> &SteamController2026Inputs` and
`status() -> SteamController2026Status`, and change both mutation signatures above.
The six public native fields and four input methods move to the renamed input
type unchanged. Status adds seven read accessors: `report_sequence()`,
`steam_client_compat()`, `haptic_streaming()`, `lizard_mode()`, `mappings()`,
`lizard_setting()` and `imu_mode()`. Both value types support defaults, clone and
equality; status is also copyable. The root and experimental modules export both
new types. The Wii supporting snapshot has zero delta.

The report sequence is the next wired transport sequence, independent of the
caller-owned source timestamp. Compatibility, haptic-stream and desktop support
are configured protocol modes; mappings, lizard setting and IMU mode are retained
accepted host intent, with unknown settings represented by `None`. Desktop
support opt-in and the host's lizard setting are separate observations. Reduced
uinput has no HID protocol or feature channel, so all status fields are `None`.
There is no fabricated battery value or wire counter for that realization.

The protocol remains the metadata owner; the status view is read-only and remains
readable after close. Input replacement, rejection and neutralization preserve
protocol modes and retained host outputs. No combined session type or worker
codec is added. The c1 reduced-uinput refusals and existing encoded report fixtures
remain in force. Demo, xtask, live haptics probe and other compiling callers migrate
with this PR; no compatibility alias remains.

## VG-1b/d · Xbox Series input-only values (#389, `edeea68`)

`XboxSeriesInputs` replaces `XboxSeriesState` on both uinput and Bluetooth UHID.
Native button, stick and trigger domains and payload encodings are unchanged.
`inputs()` borrows accepted input levels; copyable `XboxSeriesStatus` reports
selected realization metadata through `target()`, including after close.

| Previous API | Current API |
| --- | --- |
| `XboxSeriesState` | `XboxSeriesInputs` |
| `controller.state()` | `controller.inputs()` |
| `set_inputs(XboxSeriesState)` | `set_inputs(XboxSeriesInputs)` |
| `edit_inputs(FnOnce(&mut XboxSeriesState) -> Result<(), ControlError>)` | `edit_inputs(FnOnce(&mut XboxSeriesInputs) -> Result<(), ControlError>)` |

In each root snapshot feature view, remove the old type and add `XboxSeriesInputs`
and `XboxSeriesStatus`. Remove the handle's `state()`, add
`inputs() -> &XboxSeriesInputs` and `status() -> XboxSeriesStatus`, and change both
mutation signatures above. The public `sticks` field and six native input methods
move to the input type unchanged. Status adds `target() -> RealizationTarget`.
Inputs retain default, clone and equality support; status is copyable with clone
and equality support. Both new types are exported by root and experimental
modules. Wii and Steam supporting snapshots have zero delta.

This family models no battery telemetry or caller-visible input clock; status
adds no fabricated values for them. Host rumble stays in `host_outputs()`, separate
from input edits and selected-target metadata. No combined session type or worker
codec is added. Demo, xtask, the Xbox Series probe and other compiling users migrate
together, retaining the existing uinput and Bluetooth report fixtures and cleanup
regressions. No compatibility alias remains.


## VG-1b/d · Wii input-only values (#390, `a0398b7`) and metadata

`WiiRemoteInputs`, `WiiMotionPlusInputs`, `WiiNunchukInputs` and
`WiiClassicInputs` replace the four corresponding `*State` types on root,
curated, experimental and Wii supporting surfaces. No compatibility aliases or
public combined session type remain. Native input domains and report bytes are
unchanged. `WiiExtension` payloads use the renamed accessory input types.

| Previous API | Current API |
| --- | --- |
| `controller.state()` | `controller.inputs() -> &WiiRemoteInputs` |
| `set_inputs(WiiRemoteState)` | `set_inputs(WiiRemoteInputs)` |
| `edit_inputs(FnOnce(&mut WiiRemoteState) -> Result<(), ControlError>)` | `edit_inputs(FnOnce(&mut WiiRemoteInputs) -> Result<(), ControlError>)` |
| `WiiRemoteState::new(revision)` and `state.revision()` | creation options and `controller.status().revision()` |
| `state.set_attachment(...)` | live `controller.attach(...)` |
| `state.motion_plus().attached` | `controller.status().motion_plus_attached()` |
| `state.input_topology()` | `inputs.projected_input_topology(motion_plus_attached)` or live `controller.current_topology()` |

Each root feature view and the Wii supporting snapshot removes four old types,
adds the four input types and `WiiRemoteStatus`, removes handle `state()`, adds
`inputs()` and `status()`, and changes both input mutation signatures. Accessory
payload declarations follow the renames. Remote input methods retain native
editing helpers except `new`, `revision` and `set_attachment`; `input_topology`
is renamed to `projected_input_topology` and takes a presence boolean. `WiiMotionPlusInputs` removes `attached`, retaining
only gyro and slow-range levels. Status adds four read getters: `revision()`,
`motion_plus_attached()`, `battery()` and `topology_generation()`. All input types
and status support default, clone and equality; status is copyable. Handle
`battery()` is no longer `const`, because it reads live protocol state. The Steam
supporting snapshot has zero delta.

Detached `set_extension_values` validates the native payload domain; live input
transactions also require the existing extension kind. Detached MotionPlus
setters validate fourteen-bit samples; live transactions also reject non-neutral
samples when MotionPlus is absent. Only `attach` changes live presence, companion
nodes and topology generation. Neutralizing inputs preserves the extension
payload kind and live metadata. Revision stays in creation options.

The protocol is the single owner of battery for both UHID and uinput. Status
reads that owner and retains the last accepted value after close. Attachment,
neutralization and input replacement preserve battery; queued status reports
retain accepted bytes. Closed handles reject updates. The demo keeps a detached
attachment projection separately from input levels and revalidates queued edits
against live status. Existing EEPROM, host-output, report and cleanup regressions
remain in force.


### Wii review refinements before merge

The private runtime is a named-field struct (`inputs`, `revision`,
`motion_plus_attached`), with no extension trait or tuple positions. The public
supporting `WiiRemoteDefinition` driver's associated `State` is now
`WiiRemoteInputs`; its default core-frame encoding is unchanged. Configured live
sessions use the private runtime struct. This keeps runtime identity/presence out
of the public supporting driver state as well as out of application inputs.
The public-driver regression pins that associated type and its unchanged frame
encoding. The snapshot guard records trait implementations but does not record
their associated types.

A protocol-owning session is required at Wii creation. Native sessions store a
required protocol beside the runtime; HID sessions retain their required runtime
protocol. Worker sessions cannot inhabit this type. `battery()` accesses the
required owner directly, without `Option`, `expect` or a panic path, including
after close. Protocol edits retain the existing transactional clone/validation
rules. EEPROM and wire encodings remain unchanged.

After attaching an extension, `set_inputs(WiiRemoteInputs::default())` is rejected:
default inputs describe extension `None`. Use `controller.neutralize()`, or clone
`controller.inputs()`, call `.neutralize()` on that snapshot, then `set_inputs`.
Both preserve the extension payload kind and live revision, presence and battery.

Use `controller.current_topology()` for live topology. Rename the detached input
helper from `input_topology(motion_plus_attached)` to
`projected_input_topology(motion_plus_attached)`: pending editor attachment context
can differ from accepted status. The explicit projection name and documentation
direct live callers to the handle rather than asking them to supply a boolean.
Both root feature views and the Wii supporting snapshot record this method
rename; the Steam supporting snapshot is unchanged.


## Alpha.1 → alpha.2 final consumer table

All seven root and curated handles use the same native input types and
transactional final-state validation. These are the final names, superseding the
combined `<Family>State` closures from #363 and historical standalone constructors.
No compatibility aliases remain. Step e adds no public API delta; all three
snapshots stay byte-identical to the end of step d (root 172, Wii 23, Steam 14).

| Family | Caller input type | Copyable status read view |
| --- | --- | --- |
| DualSense | `DualSenseInputs` | `DualSenseStatus`: battery, input sequence, sensor timestamp |
| DualShock 4 | `DualShock4Inputs` | `DualShock4Status`: battery, sequence, sensor timestamp, touch sequence |
| Switch Pro | `SwitchProInputs` | `SwitchProStatus`: accepted timer, live stream mode |
| Xbox 360 | `Xbox360Inputs` | `Xbox360Status`: battery |
| Steam Controller 2026 | `SteamController2026Inputs` | `SteamController2026Status`: optional protocol sequence and modes; caller motion timestamp stays input |
| Xbox Series | `XboxSeriesInputs` | `XboxSeriesStatus`: selected `target()` only |
| Wii Remote | `WiiRemoteInputs` | `WiiRemoteStatus`: revision, MotionPlus presence, battery, topology generation |

| Consumer task | Final alpha.2 API and semantics |
| --- | --- |
| Create a family | `create_<family>(CreationOptions, <Family>Options)`; restore identity with a family option builder. The old constructors with identity/image/options suffixes are historical. |
| Read accepted levels / metadata | `controller.inputs()` borrows inputs; `controller.status()` returns a small copyable view. Every realized family has both, including target-only Xbox Series status. |
| Replace levels | `set_inputs(<Family>Inputs)`; clone accepted inputs when preserving attachment-dependent payload shape. |
| Edit several fields | `edit_inputs(|inputs| -> Result<(), ControlError> { ... })`; intermediate values may be invalid, but the final candidate must pass all native/realization/topology rules. No external side effects in closures. |
| Edit one native field | Family setters use the same final-state transaction as replacement, edits and neutralization. Rejection preserves accepted inputs, metadata, dirty work and pending bytes. |
| Release held controls | `neutralize()`, then `commit()`; preserve live metadata and host outputs. Wii attached inputs can also be cloned, neutralized and passed to `set_inputs`. Default Wii inputs have extension None and are rejected when an extension is attached. |
| Produce an explicit motion sample | Sony `refresh_motion()` advances the accepted clocks even for equal motion, then commits. Switch Pro retains its realization/stream-specific refresh behavior and HID cadence. Retry an accepted sample with `commit()` after backpressure; do not refresh again merely to retry. |
| Manage Wii battery | `battery()` / `set_battery(WiiRemoteBattery::new(raw))`; read raw byte with `.raw()` or `status().battery()`. Metadata survives input edits, attach, neutralize and close. Old `set_battery_raw` is removed. |
| Read Wii live / projected topology | Live `current_topology()` uses accepted attachment metadata. Detached editors use `projected_input_topology(bool)` with explicit pending presence. Only live `attach()` changes companions and topology generation. |
| Deliver or service | Call `commit()` explicitly and service readiness/deadlines independently. Input levels coalesce: press/release before commit may disappear. There is no input event queue or guarantee of atomic host observation. |

```rust,no_run
use virtualgamepad::{ControlError, DualSenseController, WiiRemoteController};
fn update(sony: &mut DualSenseController, wii: &mut WiiRemoteController)
    -> Result<(), ControlError>
{
    let candidate = sony.inputs().clone();
    sony.set_inputs(candidate)?;
    sony.edit_inputs(|inputs| { *inputs = Default::default(); Ok(()) })?;
    let mut neutral = wii.inputs().clone();
    neutral.neutralize();
    wii.set_inputs(neutral)?;
    let _accepted_topology = wii.current_topology();
    Ok(()) // The caller commits and services each handle explicitly.
}
```

Behavior-only changes with no snapshot delta: #307 (`baf9d4e`) enables Steam
client compatibility by default; #350 (`abcdaf8`) recovers retained host outputs
for the three USB/IP worker families and refuses outdated workers; #364
(`77ec6e3`) rejects unsupported non-neutral IMU/contact/pressure input on reduced
Steam uinput. These are acceptance/refusal and retention changes, not additional
input APIs. USB/IP retained snapshots are not a typed host-output event stream.

The accepted input contract is [ADR-0025](adr/ADR-0025-input-model.md).
VG-1's main-introducing PR is #352 (`c5d8340`), which lands #348's delta.
VG-1b final merge ledger: a #353 `2890223`; b #363 `5bea7a0`; c1 #364
`77ec6e3`; c2 #366 `b41c82d`; d #370 `f27e611`, #371 `031dd11`, #374
`8cd1000`, #375 `26168b3`, #387 `685479b`, #389 `edeea68`, #390 `a0398b7`.
Step e records this consumer/documentation completion; its merge SHA belongs in
the closing #322 status after owner merge, not as a predicted SHA here.

## Switch Pro HD rumble typed decode (Alpha 2 G2)

- Existing `SwitchProRumble` fields and `SwitchProOutputEvent::rumble()` remain
  unchanged. New `decode()` and `decode_with_previous(left, right)` return
  `SwitchProRumbleDecoded` with independent `SwitchProRumbleWord` sides;
  each carries the original four bytes and bounded `SwitchProRumbleSamples`.
- The new root functions `decode_switch_pro_rumble_word(raw)` and
  `decode_switch_pro_rumble_word_with_previous(raw, previous)` are pure. Each
  `SwitchProRumbleSample` exposes high/low `SwitchProRumbleBand` values;
  `frequency_hz()` and `amplitude_ratio()` return optional quantities.
  Amplitude is the dimensionless compatibility-model ratio, not physical
  output or the community table's amplitude scale.
- Recognized relative encodings can leave individual values unknown unless
  prior contiguous motor state is provided. Preserve previous state across
  no-sample words, but discard it on unpinned encodings, gaps, or resets.
  Raw words remain available in all cases; no uinput or journal change.
- Source-only research: [pinned draft corpus commit](https://github.com/anderstvoss/controller-protocol-research/commit/79f919f22d9c587eb4b3b15383b41c296de1e797);
  `swpro-l0-rumble-absolute-layout`, `swpro-l0-rumble-sample-layouts`,
  `swpro-l0-rumble-logarithmic-units`, `swpro-l0-rumble-relative-commands`.
  Formal acceptance evidence awaits corpus merge and adopted pin. No host or
  physical run was performed.
