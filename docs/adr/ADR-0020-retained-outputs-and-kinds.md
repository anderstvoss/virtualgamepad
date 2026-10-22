# ADR-0020: Retained accepted host outputs and typed surface kinds

Status: Proposed. Public additions require D2; this document does not grant it.

## Context

The bounded observation queue is useful for edges, but eviction can hide the
latest lightbar, LED, rumble or trigger setting. Every family benefits from a
retained accepted-output view; DualSense lightbar and adaptive triggers are the
first concrete consumers. Evdev-shaped `OutputSurface` alone cannot describe
these native outputs or the richer inputs required by future families.

## Decision

Each controller owns the latest accepted value of every retained output, with a
monotonically increasing revision. Update the value and revision exactly once
when validated controller-native output changes it. Malformed/unsupported
requests and repeated identical values do not advance the revision. Transport
retries of an already accepted required acknowledgement never reapply the output.
A value is accepted protocol state, not proof of physical playback or delivery.

Observation events remain for edges and can be evicted under their existing
bounded policy. The retained view and its revision survive observation eviction
and terminal close. Event loss is reported independently; it cannot invalidate
the retained latest values. Command-like actions that have no meaningful latest
state remain explicitly typed events, rather than being misrepresented as levels.

Output kinds describe native capabilities alongside `OutputSurface`: rumble with
motor roles, impulse triggers, mono/RGB LEDs, player indicators with count,
adaptive triggers, display geometry/format and speaker format. A kind describes
capability, not an implemented claim: unevidenced display/speaker formats remain
undeclared. A provider limitation stays explicit in its realization surface.

Input vocabulary must support pressure buttons, capacitive touch, relative 2D
motion, point sets and load cells without treating every feature as an untyped
`CustomInputModule`. Units, ranges and accepted values stay controller-owned;
this is no generic mutable gamepad or runtime profile interface.

## Alternatives and consequences

A larger observation queue still loses the last output under sustained traffic.
Reconstructing state from lossy events is rejected. Keeping raw unvalidated report
bytes as the public output state is also rejected: report meaning, acceptance and
errors belong to each controller personality.

Step 10b implements retained state below the root, with a burst exceeding queue
capacity, accepted-change revision checks and malformed-output regressions. Step
10c remains draft while D2 is open. Proposed ordinary-root additions requiring
that decision are `host_outputs()` on typed handles, retained typed output state
and revision access, typed `OutputKind`/input-kind descriptors, and associated
surface accessors. Public enums are non-exhaustive. The exact snapshot diff must
be listed in 10c; this ADR/C2 changes none of the reviewed 110 items.
