# Application API: preliminary alpha candidate

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

Association exposes `components()`, each with a role, selected surface, requested
identity and optional observed host path. Current root realizations have one
primary component; consumers must not assume a fixed count. A path cached at
creation does not establish continuing ownership. Future internal Wii expansions
would remain protocol state unless independently represented as host components.

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

The root contract is a **preliminary** candidate. Hands-on refinement precedes
the final API review and separate quality review. The intended alpha is Git-based;
registry publication remains disabled. Changes after alpha require rationale and
migration notes, not a promise of 1.0 compatibility.
