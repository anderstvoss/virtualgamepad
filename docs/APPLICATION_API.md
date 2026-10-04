# Application API: current pre-alpha candidate

`virtualgamepad` is a standalone curated-controller appliance. The root API owns
virtual controllers, not physical discovery, normalization, routing, player slots,
host provisioning, application threads, or filesystem persistence.

```rust,no_run
use virtualgamepad::{create_dualsense, CreationOptions, RealizationId, DualSenseControl};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut controller = create_dualsense(CreationOptions::new(RealizationId::LINUX_UHID_USB))?;
let identity = controller.identity().map(|identity| identity.to_bytes());
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
four-family application owner.

## Selection and identity

Use `CreationOptions::new(RealizationId::LINUX_UINPUT)` or
`RealizationId::LINUX_UHID_USB` for the current application controllers. There is
no automatic fallback. `LINUX_UHID_BLUETOOTH` is an exact research target identifier,
not a supported current controller personality or actual Bluetooth transport.

Options are reusable and contain no application/session identifier. Each creation
gets fresh internal request/session ownership. `association().creation()` is a
process-local observation, never persistent identity. Sony identities are typed,
can be read from USB/UHID handles, and can be restored through the explicit
`create_*_with_identity` constructors. No files are read/written for persistence.
Concurrent controllers should use distinct persistent identities.

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

Gadget creation is rejected by normal root constructors with its Gate G reason.
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
