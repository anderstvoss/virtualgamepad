# Controller package architecture

This specification defines the required shape of a first-party curated
controller package. It is intentionally not a plugin contract.

## Required ownership

Each controller module owns its private complete state, typed native controls,
native numeric value types, normalized digital mapping, target manifest,
prepared realization, stateful protocol personality, evdev encoder, reverse decoder, typed output event, and typed
target-surface descriptor. A module may use shared helpers only where values
have identical units and semantics.

The module must not depend on profile IDs, YAML, maps of optional features,
stringly typed native controls, or a shared mutable gamepad state.

## State and operations

Native numeric values are public range-validated newtypes with raw accessors.
The controller package documents their exact native report or semantic domain.
No core numeric normalization API exists. If a package later offers a
convenience conversion, it is explicitly named, controller-owned, and
documents rounding, bounds, and non-finite-value handling.

Face-button position and D-pad direction are the only shared convenience
operations. Every other semantic input is a concrete controller operation.
State fields remain private. A controller handle must make changes through a
candidate-edit operation or a typed setter that uses the same candidate-edit
path; validation failure discards the candidate.

## Target surfaces

Every declared realization exposes a typed immutable surface. Its common view
contains target, digital input codes, absolute axes with ranges/neutral/flat,
output channels, and target restrictions. A concrete surface may add native
presentation facts. Surface metadata describes host presentation only; it
never creates an alternate generic mutation API.

The package declares unavailable features with `UnavailableInRealization` and
keeps state unchanged. `UnsupportedControl` means the controller family never
implements that operation. A package may not silently map an unfaithful
feature to a similarly named generic operation.

## Evidence and acceptance checklist

Before a target feature is advertised, the package records protocol and Linux
presentation evidence, supplies deterministic encoder/decoder tests, and
validates its realization on an appropriate host where applicable. It must
test neutral state, digital/native equivalence, numeric bounds, rejected-edit
atomicity, deterministic full-state encoding, surface/realization consistency,
reverse output, retry after failed commit, and terminal closure.

Adding a controller changes only its package, root constructor/re-exports,
tests, and documentation. It must not change core/runtime/provider logic.

## Stateful HID execution

UHID packages implement the `gr-hid::Protocol` contract through a controller-owned personality. Feature tables are personality data, never provider configuration. Personality state owns report sequence, timing, initialization, and required replies. Logical reports carry a report class, optional nonzero ID, and payload without that ID. Kernel envelopes belong to the adapter.

The runtime clones personality generation before queue acceptance, retains definitely-unsent bytes, and closes on uncertain delivery. Required SET validation precedes acknowledgement. Optional observations must not own completion tokens. Service readiness and deadlines independently of semantic edits; test idle cadence and startup probes without subscribers.

Compiled manifests declare cohesive `RealizationId` values and static `RealizationTargetSet::new(&[...])` membership. The existing uinput and broker paths retain their earlier frame runtime pending their relevant migration gates. See [current architecture](../../CORE_ARCHITECTURE.md) for implementation boundaries.

## Ephemeral UHID transport identity

UHID phys/uniq values receive a compact process/creation ordinal suffix. Caller
session IDs remain application identifiers and may be reused without duplicating
those transport fields. This identity is process-local and distinguishes concurrent
processes in the same PID namespace; it is not a physical serial guarantee.
The provider rejects oversized or embedded-NUL identity strings rather than
silently truncating them. Controller-owned feature addresses are a separate
protocol concern and are unchanged by this transport suffix.
