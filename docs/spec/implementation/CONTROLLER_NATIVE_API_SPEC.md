# Controller-native API specification

## Purpose

`virtualgamepad` is a curated virtual-controller library. It creates a small
set of high-fidelity controller types through an explicit Linux provider
target. It is not a runtime profile engine, a YAML-defined controller system,
or a third-party plugin host.

The initial curated types are Generic Gamepad, Xbox 360, DualSense, and Steam
Controller. Each is compiled Rust code with its own state model, report codec,
descriptor, and reverse-event decoder. Adding a future curated type must be a
new controller implementation plus a root creation function; runtime lifecycle
and Linux provider code must not acquire controller-specific branches.

## API contract

Applications choose a target explicitly with `CreationOptions` and call a
controller-specific constructor such as `create_dualsense`. Creation is exact:
the selected provider must realize the whole declared controller surface at its
documented tier, or creation returns `CreationError` and no handle. The library
never silently substitutes a lower-fidelity provider.

Controller state is mutable and local. Applications apply one or more updates
then call `commit()`. Updates do not perform provider I/O. A failed update
leaves state unchanged; a failed commit leaves the valid state dirty and the
controller live so the caller may retry, inspect it, or close it.

Normalized inputs are spatial and controller independent where that is honest:
`FaceButton::North`, `South`, `East`, and `West`; D-pad directions; sticks;
and triggers. Native labels are explicit, typed controller operations:
`DualSenseControl::Cross`, `XboxControl::A`, and
`SteamControllerControl::Steam`. Native labels never use ambiguous names such
as `button_x`. A native and normalized operation map to the same stored state
when they identify the same physical position.

Concrete handles expose only controls that exist on the type. Runtime
collections use `ControllerHandle`; incompatible normalized/native updates
return recoverable `ControlError` values without mutating or closing the
controller.

## Core and provider boundaries

`gr-controller-contract` owns controller-neutral identifiers, normalized input
primitives, creation/commit/control errors, realization requirements, and the
controller definition/driver contracts. `gr-controller-runtime` owns the
controller-independent mutable-state lifecycle: reusable encoding storage,
atomic updates, dirty tracking, retry-safe commit, and closure. Its `FrameSink`
is the only provider-facing commit boundary. `gr-controllers` owns all curated
controller state, native controls, mappings, and conversion preparation. Linux
providers own OS device realization only.

The first migration slice uses the existing profile/report pipeline as a
strictly isolated provider seam. `ControllerState::legacy_payload()` is the
only bridge. It is not a public profile API and must disappear when providers
accept prepared controller encoders directly.

The root façade also converts the existing generic reverse command exactly once
into `ControllerOutputEvent`, then invokes the application callback through the
bounded delivery worker. New controller implementations must replace this
adapter with their own typed output event enum; applications and controller
state types must not import the legacy runtime-model output container.

`ControllerRuntime` must be the destination for all new controller creation
paths. The legacy session actor is temporary infrastructure only; future
controller additions may not add profile/session branches to it.

## Reliability requirements

The commit hot path must have no YAML parsing, registry lookup, reflection, or
string/map control dispatch. User-controlled input and recoverable provider
failures never panic. Memory and callback queues are bounded. Reverse output
must be typed per concrete controller and delivered off the input path.

Every controller addition requires unit, property, state-machine,
fault-injection, report regression, and Linux integration coverage. YAML is
permitted only for fixtures and snapshots; it never defines a runtime
controller or public controller configuration.
