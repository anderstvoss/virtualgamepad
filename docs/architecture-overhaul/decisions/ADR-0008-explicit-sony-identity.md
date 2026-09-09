# ADR-0008 — Explicit Sony USB/UHID identity restoration

Status: accepted API and deterministic contract; live consumer association pending.

## Problem

Fresh identity generation prevents accidental collision between virtual creations,
but a caller cannot intentionally recreate the same emulated controller after a
real disconnect. Application session IDs must not become pairing addresses, and
restoring a pairing address must not restore transport handles or protocol state.

## Decision

Expose separate `DualSenseIdentity` and `DualShock4Identity` types. Each can generate
six valid bytes from the existing OS entropy source, return bytes for caller-owned
storage and validate restored bytes. The byte order is the controller pairing
feature's wire order, and the existing local/unicast flag constraint is retained.
The application owns storage and logical slot policy. There is no registry, file
format dependency, database, new runtime dependency or implicit persistence.

Add `create_dualsense_with_identity` and `create_dualshock4_with_identity`. These
explicit constructors support USB/UHID only. Pairing feature bytes and the
controller-defined UHID `uniq` value remain stable for a supplied identity.
Physical-path instance labels always receive a fresh process/creation suffix.
Every creation builds a new personality, neutral semantic state, clock origin,
request ownership and queues. Closing the previous controller remains a real
removal; no kernel object is cached to conceal disconnect/reconnect.

Existing constructors retain their fresh-per-creation behavior and transport
labels. Callers choosing persistence should generate/store an identity and use
its explicit constructor from the first creation. The current default is preserved
for compatibility, not declared a permanent persistence policy.

Use distinct identities for concurrently connected logical controllers. Deliberate
reuse in simultaneous sessions may collide in host-driver/consumer identity
registries; the library does not provide global arbitration or promise that such
creations succeed. A new transport session isolates protocol state but does not
make duplicate emulated hardware identities meaningful to every consumer.

## Scope and alternatives

Evdev and compiled gadget identity behavior has not been established. Supplying
an identity for those targets fails before provider open, with no fallback. Generic
providers receive the controller's prepared metadata and transport the protocol;
they do not generate or interpret Sony identity. No new realization is introduced.

An arbitrary shared identity string would lose controller-owned validation and
encoding. Reusing application IDs would repeat the previous identity-collision
bug. Persisting entire protocol sessions would retain stale requests and prevent
physical-like reconnect. None of these alternatives is adopted.

## Tests and remaining acceptance

Tests cover all first-byte flag combinations, byte round-trip, target rejection,
stable restored labels with fresh physical paths, and unchanged ephemeral label
uniqueness. Fake Sony sessions restore the same pairing reply after terminal close,
return to neutral state and initial report sequencing, and cannot resurrect a
previous definitely-unsent reply. Reused request/application IDs stay scoped to
their independent session. Existing entropy failure and lifecycle regressions stay.

Live Linux reconnect and SDL/Steam association remain separate evidence. No such
association pass is claimed from stable protocol bytes. The physical DualSense
was unplugged during this batch; neither this implementation nor its tests need
it. DS4 remains source-backed without physical hardware. Compound identity-derived
roles and logical failure policy remain subsequent work. Steam Controller
implementation is deferred until the current overhaul lands, per maintainer scope.
