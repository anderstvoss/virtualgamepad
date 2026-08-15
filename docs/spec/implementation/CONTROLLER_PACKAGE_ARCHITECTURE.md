# Controller package architecture

This specification defines the required shape of a first-party curated
controller package. It is intentionally not a plugin contract.

## Required ownership

Each controller module owns its private complete state, typed native controls,
native numeric value types, normalized digital mapping, target manifest,
prepared realization, encoder, reverse decoder, typed output event, and typed
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
