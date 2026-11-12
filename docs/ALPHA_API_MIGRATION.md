# Pre-alpha API refinement migration

This is a deliberate breaking candidate update before the final alpha freeze.

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

## USB/IP retained host outputs (#203)

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

## VG-1 · Whole-state input replacement

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

## VG-1b/b · Transactional native input edits

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
