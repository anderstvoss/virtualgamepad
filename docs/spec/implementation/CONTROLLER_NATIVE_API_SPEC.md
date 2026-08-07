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

Updates are local at the controller-state layer. Callers batch them and call
`commit()` explicitly. Every update runs against a cloned candidate, validates
the complete candidate, and replaces the current state only on success. This
makes direct typed-state edits and individual control methods obey the same
atomicity rule. An encode/send failure leaves the last valid state dirty and
the handle live for retry. `close()` stops reverse delivery, attempts provider
cleanup, and makes the handle terminal even if cleanup itself reports an
error. Dropping a live handle performs the same bounded cleanup.

`CreationOptions` bounds the number of output subscriptions and defines the
slow-callback diagnostic threshold. Callbacks are typed per controller, while
heterogeneous callers receive `CuratedControllerOutputEvent`. Dropping an
`OutputSubscription` cancels it. Capacity exhaustion is a recoverable
`SubscriptionError`, not an allocation-growth policy.

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
normalization mapping, validation, `PreparedControllerFrame`, reverse decoders,
and immutable OS realization specifications. A realization contains identity,
descriptor, report, evdev capability, feature-report, or USB-gadget data. A
frame is an immutable tagged controller-specific value prepared at commit
time. Linux providers report only OS transport capabilities and consume a
matching realization shape; they do not select identities, define control
labels, or branch on a controller family.

The root opens the caller-selected backend through `NativeBackendOpenContext`.
`PreparedControllerFrame` is encoded to a provider `BackendFrame` by the
compiled controller, so creation and commits do not construct profile input,
query a registry, use the legacy session actor, or select a translator. Older
profile/planner interfaces remain only for quarantined pre-redesign workspace
tools while they are retired; the public crate does not re-export their
provider inventories or accept their identifiers.

The root reverse worker drains provider reports outside the commit path,
decodes all recognized native meanings, and delivers tagged controller-native
events. Unknown or malformed reports use lossless controller-specific fallback
variants. It contains callback panics by cancelling only the offending
subscription. Slow callbacks may delay other subscribers on that controller's
delivery worker, but never run on its update/commit path or another controller's
worker. Diagnostics expose backend state, lifecycle, dirty state, active
subscriptions, delivered-event count, callback panics, slow callbacks, and a
terminal reverse-worker error.

## Lifecycle and failure contract

| Operation | Recoverable failure | State after failure |
|---|---|---|
| create | feature absent, target incompatible, host open failed | no handle |
| normalized/native update | unsupported control, invalid index/range, closed | prior state unchanged |
| typed full-state edit | complete-state validation failed, closed | prior state unchanged |
| commit | encode or provider send failed | handle live and dirty; retry allowed |
| subscribe | closed, capacity reached, lock unavailable | existing subscriptions unchanged |
| callback | panic | only that subscription cancelled and counted |
| close | worker join or provider close failed | handle terminally closed |

No public method panics for caller-controlled values. Internal `unreachable!`
checks are restricted to invariants established by concrete handle
construction and cannot be selected through public input.

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
   normalized mappings, complete-state validation, deterministic encoder,
   typed reverse-event decoder, and controller-owned realization specs.
2. Implement `ControllerDefinition` and `ControllerDriver`; declare only the
   immutable realization requirements the controller actually needs.
3. Add the tagged state/frame/output variants, heterogeneous dispatch arm, and
   one root typed constructor. Do not change the generic runtime or insert
   controller branches in a Linux provider.
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
boundary must be documented. State, serialization, identity, and capability
decisions must be deterministic for a given instance ID. Session IDs and
host-visible unique identifiers must not collide during ordinary concurrent
creation.

The normal commit path has no registry lookup, YAML parsing, reflection, or
string-map dispatch. Mutable state is owned by the caller's handle; provider
access is serialized per handle; each handle owns one reverse worker; callback
lists and callback invocations use separate locks so callback code never runs
while the subscriber-list lock is held. Queue-like collections are bounded by
API configuration or kernel/provider boundaries.

Test coverage includes unit and property tests, generated lifecycle operation
sequences, injected sink and backend failures, drop/close tests, callback panic
and worker-failure containment, compile-fail API coverage, realization and raw
report regression checks, feature-minimal builds, and privileged Linux checks
separated from the hermetic suite. `cargo-fuzz` targets exercise untrusted
reverse-report bytes and generated control sequences. Only minimized,
synthetic corpora may be committed.
