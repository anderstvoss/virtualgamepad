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
silently truncating them. Controller-owned feature addresses are a separate protocol concern. DualSense and DS4
acquire a locally administered unicast address from OS entropy before
provider open, independently of application session IDs. Its personality retains
the address across GETs/retries; entropy failure rejects creation. Deterministic
tests inject identities without OS entropy.

## Conventional evdev feedback ownership

All curated evdev controllers service required upload/erase replies internally
during `poll_output`. `ForceFeedback(ForceFeedbackEvent)` reports completed uploads,
erasures and playback commands with stored magnitudes and replay timing. Callers
must remove the old manual force-feedback reply calls. Low-level provider users
still reply explicitly to typed mechanism requests. See
[ADR-0006](../../architecture-overhaul/decisions/ADR-0006-conventional-feedback.md)
for bounded ownership, rejected fields and terminal error behavior.

## Explicit curated service entry point

Each current curated handle exposes `service(callback)`; `poll_output` remains a
compatible alias. Embeddings service unchanged input on readiness and monotonic
deadlines, adding write interest only when requested. Recompute interest after
state submission and servicing. Terminal native sessions expose no readiness or
deadline, matching HID terminal behavior.

Required curated HID/evdev protocol work precedes optional observer delivery in
each bounded service cycle. Evdev observations are capped at 32 and eviction is
reported by `dropped_output_events`. Observers must return promptly; they cannot
extend the timing guarantee to subsequent cycles. Service/edit scheduling stays
with the embedding, without mandatory workers or an async runtime. The legacy
compiled gadget path remains subject to its separate request-interface gate.

## Optional Sony identity restoration

`DualSenseIdentity` and `DualShock4Identity` own generation, byte validation and
pairing representation for the explicit USB/UHID restoration constructors. The
caller may store their bytes but does not persist protocol state. Controllers
prepare stable UHID `uniq`; shared creation always gives the physical path a fresh
instance suffix. The default constructors remain unchanged, and other targets
reject supplied identities before opening resources. Compound component identity
must later derive from one logical controller identity under its own accepted
contract. See [ADR-0008](../../architecture-overhaul/decisions/ADR-0008-explicit-sony-identity.md).

## Post-106 controller-owned extensions

Populate `NativeHidRealization.target` with the exact compiled realization ID.
For local UHID use USB bus metadata with `linux.uhid.usb` or Bluetooth bus metadata
with `linux.uhid.bluetooth`; the provider rejects mismatches without fallback.
Only declare paths the controller actually implements. No current USB personality
is automatically Bluetooth-capable because the mechanism supports both IDs.

Use typed controller methods around `Runtime::update_protocol` for caller-driven
configuration/topology. Validate attachment edits inside the transaction, keep
resource clones isolated, and bound protocol status queues. Queued transport bytes
are preserved and new input generation follows them. Internal attachments are not
`RequiredGroup` host components. No arbitrary extension plugin interface is added.

The persistent-resource experiment recommends typed, value-owned protocol memory
and exact snapshots after terminal close. Deliberate shutdown exposes retain or
discard; an external exporter may retry after failure without reopening transport.
Actual file helpers belong outside runtime/providers and require their own atomic
replacement tests. This is an accepted pattern, not a universal public resource API.

Controller-native independent controls remain typed even when a target has a
technical restriction; document the reason in its surface. Remap-only or unknown
raw controls are not invented as independent state. Consumer/source/physical
observations are separately scoped. Production Wii Remote is deferred pending
explicit maintainer authorization; its test prototype grants no package support.
