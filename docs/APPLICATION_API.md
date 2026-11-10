# Application API: reviewed ordinary-root alpha boundary

`virtualgamepad` is a standalone curated-controller appliance. The root API owns
virtual controllers, not physical discovery, normalization, routing, player slots,
host provisioning, application threads, or filesystem persistence.

```rust,no_run
use virtualgamepad::{create_dualsense, CreationOptions, RealizationId, DualSenseControl, DualSenseOptions};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut controller = create_dualsense(
    CreationOptions::new(RealizationId::LINUX_UHID_USB),
    DualSenseOptions::default(),
)?;
let identity = controller.identity().map(|identity| identity.bytes());
controller.set_native(DualSenseControl::Cross, true)?;
controller.commit()?;
controller.service(&mut |output| { /* consume native output promptly */ })?;
controller.neutralize()?;
controller.commit()?;
controller.close();
if let Some(error) = controller.diagnostics().last_error() {
    eprintln!("cleanup: {error}");
}
# Ok(()) }
```

The example is a short lifecycle illustration. A live endpoint requires continuous
servicing. See [`controller_loop`](../examples/controller_loop.rs) for a bounded
application owner for the original four families.

## Selection and identity

Choose an exact realization with `CreationOptions::new(target)`. All seven
families offer `LINUX_UINPUT`. Wii Remote also offers `LINUX_UHID_BLUETOOTH`,
which uses Bluetooth bus metadata through local UHID. `DualSense`, `DualShock4`,
Switch Pro, Xbox 360 and Steam Controller 2026 also offer `LINUX_UHID_USB`.
Xbox Series selects `LINUX_UINPUT` with `XboxSeriesOptions::default()`. Each realization has its own published restrictions
and acceptance status. Creation has no automatic fallback.

Use common `CreationOptions` for realization and audio and the matching
family options for controller-specific settings. Family options implement
`Default` and can be reused. Each creation gets fresh internal request and
session ownership. `association().creation()` is a process-local observation.

For Sony USB/UHID and USB/IP, or Switch Pro USB/UHID, save the handle's
typed `identity().bytes()` (unwrap the Sony option when present). Restore it
with the matching `Identity::from_bytes(bytes)?` and pass it to
`FamilyOptions::default().with_identity(identity)`. The root re-exports the
curated identity types, so generation and restoration return the same errors
at both layers. Sony uses pairing-feature byte order, and Switch Pro uses
display order. Both require locally administered unicast flags. Switch Pro
also retains the address on uinput, where the kernel provider has no unique-ID
setter. Wii Remote options restore a locally administered unicast address in
display order, and Steam Controller 2026 options restore a validated serial
via `SteamController2026Identity::from_serial`. Both handles expose their
resolved identity on either realization. Wired Xbox 360 uses options with no
persistent identity. Use distinct identities for simultaneously connected controllers. Storage belongs to the
application.

For opt-in acceptance tests only, `VIRTUALGAMEPAD_INPUT_LAB=1` relabels
DualSense/DS4 UHID or uinput `phys` into the corresponding
`virtualgamepad/input-lab/{uhid,uinput}/` namespace.
Applications must not set this variable. See the
[optional input-lab setup](DEPLOYMENT_AND_VALIDATION.md#optional-sony-input-lab-isolation).

Association exposes `components()`, each with typed `kind()`, a display role,
optional input surface or audio endpoint metadata, and transport-specific requested
identity. Audio components do not populate requested HID identity fields. Consumers
must not assume a fixed count. Cached paths and selectors do not establish
continuing ownership. Internal Wii expansions remain package protocol state unless
independently represented as host components.

## State, exposure and servicing

Each family retains its own state, numeric units/ranges, native button enum,
motion/touch types and output vocabulary. Native and spatial button readback
describe accepted state, not host/SDL visibility. DS4 Cross and Switch B both map
to spatial South; selecting by label is an explicit application choice.

Rejected edits preserve accepted state. Failed commits preserve retryable dirty
state where delivery remains retryable. `neutralize()` edits semantic input;
commit it to send. Neutralization does not erase identity, battery metadata,
protocol clocks or host-owned outputs. Commit does not promise one wire report.

Service on readiness **and** monotonic deadlines while input is unchanged.
Recompute readiness, write interest and `next_service_in()` after service/commit.
Zero means service now; `None` removes timer interest, not necessarily read
interest. Polling sessions require bounded polling. Readiness descriptors borrow
the controller; never close them or retain a raw descriptor after that borrow.

Required host requests complete internally before optional native observations.
Applications cannot reply to them. Native output retains Sony flags, LED changes,
adaptive-trigger bytes and unknown payloads, plus Switch encoded motor words and
subcommand payloads. Optional observations are bounded; loss is reported by
`dropped_output_events()`. Conventional force-feedback upload is not playback.
No decoded Switch amplitude/frequency fidelity is implied.

Close is terminal and idempotent. Diagnostics remain available after closure,
including cleanup failure and observation loss. Recreate a new controller instead
of resetting/reopening a closed session. Host OPEN/CLOSE observations are distinct
from library session lifetime.

## Migration from the pre-108 surface

| Before | Preliminary application API |
| --- | --- |
| `CreationOptions { target, session }` | `CreationOptions::new(target)`; keep lab/correlation IDs in the application |
| `RealizationTarget::{Evdev,Uhid,DummyHcd}` | Explicit `RealizationId::LINUX_*` constants; ambiguous associated aliases removed |
| `poll_output(...)` | `service(...)`, including idle protocol work |
| `ProviderError` / `ProviderDiagnostics` | `ControllerError` / read-only `ControllerDiagnostics` |
| `provider_diagnostics()` | `diagnostics()` and accessors |
| Single association fields and `role()` | `association().components()` with component accessors |
| `gr_hid::Readiness` on handles | Borrowed `ServiceReadiness` |
| Raw provider events and manual GET/SET replies | Controller-native observations; replies are internal |
| Switch `refresh_motion()` | Continue servicing advertised deadlines; do not synthesize protocol cadence in the application |

## Experimental boundary and compatibility

The `experimental` feature exposes the former broad root surface under
`virtualgamepad::experimental`. Supporting workspace crates remain unstable SPI,
not an arbitrary controller/provider plugin contract. The `test-support` feature
of the curated crate supplies fake-provider construction only for workspace tests;
ordinary builds do not enable it.

Each ordinary family handle now exposes `host_outputs()` under owner D2
(#176 comments 6325147128 and 6328169640). It returns a typed snapshot
(`DualSenseOutputState`, `DualShock4OutputState`, `SwitchProOutputState`,
`Xbox360OutputState`, `WiiRemoteOutputs`, `SteamController2026Outputs`, or
`XboxSeriesOutputState`) of accepted in-process host output values and a revision,
independently of the bounded observation queue, including after close. The
revision advances once per accepted value change; repeated identical values and
malformed requests do not advance it. Read the snapshot for latest levels and
use events for edges and diagnostics. These values do not prove physical delivery.
The same views remain available through the experimental curated handles.
On UHID, Xbox 360 retains no supported host outputs (an empty snapshot at revision
zero). The `evdev_rumble` field on the original four retained state types is
`None` on UHID because evdev playback belongs to the uinput realization. USB-worker retained views explicitly return
`UnavailableInRealization`: their lossy observation channel has no recovery query.

Ordinary-root `surface().restrictions()` also exposes the retained-host-outputs
restriction on USB/IP and `dummy_hcd` surfaces. This is visible restriction metadata,
independent of whether an application reads the new ordinary-root snapshot.

Under owner decision D5 (#176 comment 6325079527), `dummy_hcd` stays in the tree
as experimental and not recommended because it needs root and host provisioning.
Controller buildout targets near feature-complete UHID realizations; `dummy_hcd`
and USB/IP remain optional faithful tiers, never default paths. Gadget creation
is rejected by normal root constructors with its Gate G reason.
Research implementations/tests remain available; no supported function was
silently mapped to a lower-fidelity realization. USB gadget, Bluetooth and audio
support are not promoted by API cleanup. The demo disables gadget selection and
uses the same application API as ordinary consumers.

The ordinary root signatures are checked against the reviewed alpha snapshot.
The holistic release review and remediation retain unresolved acceptance gates;
a signature freeze alone does not establish readiness. The intended alpha is
Git-based; registry publication remains disabled. Changes after alpha require rationale and
migration notes, not a promise of 1.0 compatibility.


## Audio and read-only metadata

Selected surfaces expose `output_kinds()` alongside the existing `outputs()`
event-code descriptors. Native kinds describe rumble motor roles, mono/RGB LEDs,
player indicators and adaptive triggers. uinput exposes conventional rumble only;
Xbox UHID has no output kinds. The compiled registry carries family-native kinds
(Xbox uses its evdev rumble kind). Display, speaker and impulse-trigger vocabulary
is available but undeclared by the current families; no capability is implied by
an enum variant alone. Existing restriction reasons still apply.

`InputTopology::typed_inputs()` describes pressure buttons, binary capacitive
contact, signed relative 2D displacement, bounded point sets and load cells.
Descriptors retain native ranges and load-cell units; controller-native state
and operations own values. The original four families leave this slice empty.
Future controller packages can declare these kinds without hiding them in custom
modules. New kind and retained-state types are non-exhaustive; consumers must
allow future variants/fields. `InputTopologyError` is also non-exhaustive;
downstream matches must include a wildcard arm to handle future validation errors.
The reviewed D2 update added retained outputs, descriptor accessors and a
typed-input validation error.

Topology descriptors are read-only semantic metadata, not external controller
construction or GUI sizing policy. Inspect their accessor slices and native ranges;
controller-native state remains authoritative.

Audio is disabled by default. `AudioOptions::new(AudioExposure::Emulated)` explicitly
requests functional audio. Playback/microphone ownership each applies to all groups
in that direction. UHID may own associated caller-session `PipeWire` endpoints;
USB/IP is a distinct opt-in WIP composite realization. Both reject unsupported
feature/platform/topology combinations without fallback.

Borrow audio briefly with `controller.audio()`. PCM processing runs on owned workers;
`service()` handles controller protocol/output work and discovers propagated audio
failure. Readiness and write interest describe the controller provider, not PCM
queue availability. Audio health participates in deadlines without promising a
fixed polling cadence. Recompute readiness/write interest/deadlines after service
or commit; callbacks and PCM batches should remain bounded.

A requested audio component is required. Audio or HID failure closes the logical
creation when discovered; close is terminal and idempotent. Neutralization does
not flush audio. Endpoint selectors invalidate on close/recreation; retained
association/diagnostic snapshots neither keep endpoints alive nor prove ownership.

Use `AudioDiagnostics` for retained health and loss, typed errors for sample
alignment/ownership/closure, and optional `microphone_consumed_frames()` for pacing.
Graph timing is experimental instrumentation, not measured end-to-end latency.
See [audio](CONTROLLER_AUDIO.md) and [migration](ALPHA_API_MIGRATION.md).

## Lifecycle observations and companion topology

Host Start/Open/Close/Stop observations arrive through the same bounded output
path as controller feedback after required protocol handling in `service`. Their
order is preserved; diagnostics account for observation overflow. Closing an
application controller does not invent a host lifecycle event. Subscription close
disconnects publication, drains accepted events and waits for finite callbacks;
a callback can close its own subscription without joining itself. A panicking
callback terminates that subscription and increments its panic diagnostics.

DS4 uinput creation owns a gamepad node and a separate touch companion. The
primary component surface describes the complete logical controller; the touch
component exposes its association role and creation-scoped identity. Applications
should iterate topology components rather than assume one input component per
controller. Touch requires an isolated consumer for safe acceptance testing.

The supporting `gr_hid::Protocol::lifecycle` hook now returns `Option<Output>`;
custom implementations must return their observation or `None` after state
handling. Supporting `ControllerAssociation` constructors also need the new
`companions` field. These SPI changes leave ordinary root signatures unchanged.

The supporting curated and ordinary-root handles expose retained host outputs.
Optional observation callbacks remain bounded and lossy. The internal retained
view survives event eviction and terminal close. USB-worker snapshots are
explicitly unavailable because the lossy worker channel has no recovery query
(tracked separately in #203).

## Wii attachment and EEPROM, and Steam scheduling

Use `create_wii_remote(creation, WiiRemoteOptions::default())` with the
selected target. `attach` applies the attachment and `MotionPlus` presence in
one transaction. Re-read `current_topology()`, `topology_generation()` and
`association().components()` after an accepted change. The Bluetooth-bus UHID
realization owns an addressed EEPROM image. Supply it through
`WiiRemoteOptions::with_eeprom_image` and check `resource_dirty()` before or
after closing. Resolve the terminal resource with
`resolve_resource(WiiResourceDisposition::KeepSnapshot)` or `Discard`.
Persist a kept image in the application. Uinput lacks the addressed memory
channel and reports the resource unavailable. See
[ADR-0013](adr/ADR-0013-persistent-resources.md) and
[ADR-0019](adr/ADR-0019-live-topology.md).

```rust,no_run
use virtualgamepad::{
    create_wii_remote, CreationOptions, RealizationId, WiiExtension,
    WiiRemoteOptions, WiiResourceDisposition,
};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut wii = create_wii_remote(
    CreationOptions::new(RealizationId::LINUX_UHID_BLUETOOTH),
    WiiRemoteOptions::default(),
)?;
wii.attach(WiiExtension::None, true)?;
let topology = wii.current_topology();
wii.service(&mut |event| { /* consume optional native observations */ })?;
wii.close();
let snapshot = wii.resolve_resource(WiiResourceDisposition::KeepSnapshot)?;
# Ok(()) }
```

```rust,no_run
use virtualgamepad::{
    create_steam_controller_2026, CreationOptions, RealizationId,
    SteamController2026Options,
};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut steam = create_steam_controller_2026(
    CreationOptions::new(RealizationId::LINUX_UHID_USB),
    SteamController2026Options::default(),
)?;
steam.service(&mut |event| { /* consume optional native observations */ })?;
let readiness = steam.readiness();
let watch_write = steam.wants_write();
let service_in = steam.next_service_in();
let latest = steam.host_outputs()?;
# Ok(()) }
```

Use the same readiness and deadline loop for Wii and Steam. Steam compatibility
options retain their curated defaults. The opt-in PCM observation payload and
retained last-packet view follow the existing curated behavior described in
[ADR-0023](adr/ADR-0023-controller-subsystems.md).

## Xbox Series

```rust,no_run
use virtualgamepad::{
    create_xbox_series, CreationOptions, RealizationId, XboxSeriesButton,
    XboxSeriesOptions,
};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut series = create_xbox_series(
    CreationOptions::new(RealizationId::LINUX_UINPUT),
    XboxSeriesOptions::default(),
)?;
series.set_button(XboxSeriesButton::Share, true)?;
series.commit()?;
series.service(&mut |event| { /* consume conventional force feedback */ })?;
let latest = series.host_outputs()?;
series.close();
# Ok(()) }
```

`XboxSeriesState` uses signed xpad evdev coordinates with positive Y down.
Trigger positions are 10-bit. `set_triggers` on the state validates both
values. Share and Guide are independent. `XboxSeriesOutputEvent` carries
conventional force-feedback observations, and the retained state exposes
the latest accepted strong/weak playback request. The selected surface
lists the unavailable impulse-trigger motors and other uinput limits.
