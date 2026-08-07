# Controller-native API specification

## Product boundary

`virtualgamepad` creates a deliberately small set of tightly integrated,
statically compiled virtual controllers. It is not a profile platform, a YAML
controller-definition engine, or a plugin host. The initial curated set is
Generic Gamepad, Xbox 360, DualSense, and Steam Controller.

This is a breaking pre-1.0 design. A future controller is reviewed first-party
Rust code, not a file loaded at runtime. That choice keeps report encoding,
identity/descriptors, reverse output, and feature semantics together and lets
the hot path avoid registry lookup, reflection, stringly typed controls, and
configuration parsing.

## Public API

Applications select an exact Linux target and call one of
`create_generic_gamepad`, `create_xbox360`, `create_dualsense`, or
`create_steam_controller` with `CreationOptions`. There is no target default,
automatic selection, or silent fallback. Creation validates the immutable
`ControllerDefinition` requirements against the selected provider capabilities
before opening a handle. A missing identity, transport, or reverse-output
surface returns `CreationError` and no handle.

Each concrete handle owns mutable typed state. Normalized updates use spatial
names—`FaceButton::{North, South, East, West}`, D-pad directions, sticks, and
triggers. Native updates have explicit controller-specific types, for example
`DualSenseControl::Cross`, `XboxControl::A`, and
`SteamControllerControl::Steam`. `button_x`-style ambiguous public names are
prohibited. A controller implementation defines and tests every normalized ↔
native equivalence once.

Controller-specific physical features remain native: DualSense touch contacts,
motion, trigger effects, LEDs, audio, and reports; Steam pads and grips; and
any future capacitive sensors or accessories. They are never flattened into a
misleading universal control set. A concrete handle omits unavailable methods,
which makes misuse a compile error. `ControllerHandle` is the closed
heterogeneous wrapper; incompatible runtime updates return `ControlError` and
preserve the handle and prior state.

Updates are local and allocation-free at the controller-state layer. Callers
batch them and call `commit()` explicitly. A rejected update does not mutate
state. An encode/send failure leaves the last valid state dirty and the handle
live for retry. `close()` makes further updates and commits return their typed
closed error. Callbacks are typed per controller, while heterogeneous callers
receive `CuratedControllerOutputEvent`.

## Core contracts

`gr-controller-contract` is controller-agnostic. It owns normalized value
types, lifecycle errors, Linux target IDs, realization requirements,
`ProviderCapabilities`, `ControllerDefinition`, and `ControllerDriver`.
`validate_realization` is the one creation-time compatibility predicate.

`gr-controller-runtime` owns cloned-next-state update atomicity, dirty state,
retry-safe commit, closure, and the typed `FrameSink` boundary. It has no
controller-family branches. Its invariants are tested with generated update
sequences and injected sink failures.

`gr-controllers` owns all compiled controller state, native control enums,
normalization mapping, validation, and `PreparedControllerFrame`. A frame is
an immutable tagged controller-specific value prepared once at commit time.
Linux providers report only OS transport capabilities; they do not define
control labels or controller behaviour.

The root opens the caller-selected backend directly. `PreparedControllerFrame`
is encoded to a provider `BackendFrame` by the compiled controller, so commits
do not construct `ProfileInputFrame`, use the session actor, or select a
translator. The current Linux provider implementations retain internal legacy
identity lookup while their device-spec construction is moved into controller
modules; that internal compatibility detail is not reachable from the public
API and must not receive new profile/YAML extension points.

The root reverse worker drains provider reports outside the commit path and
delivers tagged controller-native events. It contains callback panics by
dropping only the offending subscription; slow callbacks remain isolated from
input progress. Raw report variants preserve device semantics while a
controller-specific decoder is expanded.

## Realization guarantees

The target must meet the controller's complete declared surface at the target's
documented fidelity. Current Linux boundaries are explicit:

- uinput supplies compatibility realization for Generic Gamepad and Xbox 360;
- UHID supplies the identity-aware DualSense path;
- USB transport supplies the supported hardware-faithful DualSense path;
- Steam Controller creation is rejected until a provider can realize its full
  declared surface.

Windows and macOS have no creation API until native providers exist. Provider
availability is never inferred from a planner-only foundation.

## Adding a curated controller

1. Add a compiled controller module with typed neutral state, native enums,
   normalized mappings, validation, descriptors, deterministic encoder, and
   typed reverse-event decoder.
2. Implement `ControllerDefinition` and `ControllerDriver`; declare only the
   immutable realization requirements the controller actually needs.
3. Add the tagged state/frame/output variants and one root typed constructor.
   Do not change the generic runtime or insert controller branches in a Linux
   provider.
4. Add unit, property, mapping-equivalence, rejected-update, deterministic
   frame, decoder robustness, fault-injection, compile-fail, and Linux target
   realization tests.
5. Add sanitized raw-report fixtures only when they test codec behaviour.
   YAML may describe those fixtures or snapshots, never runtime behaviour.
6. Document host prerequisites, exact fidelity claim, native feature semantics,
   and every unsupported target before advertising the controller.

## Quality and threat model

No caller-controlled input or recoverable provider failure may panic. Unsafe
code is forbidden outside narrowly scoped kernel/provider modules, where every
boundary must be documented. State, serialization, and capability decisions
must be deterministic. Provider and callback failure must be observable and
must not corrupt state. Test coverage includes unit and property tests,
state-machine operation sequences, fault-injection sinks, compile-fail API
coverage, raw-report regression fixtures, and privileged Linux checks separated
from the hermetic suite. Fuzz targets for untrusted reverse-report decoding are
maintained with sanitized reproducible corpora.
