# ADR-0014 — Transactional caller-driven protocol edits

Accepted after [EXP-0020](../experiments/EXP-0020-internal-topology.md).

`Runtime::update_protocol` supplies a fallible closure over an isolated personality
clone. It rejects closed sessions, validates current semantic state against the
candidate, then installs the candidate and schedules service. Failed edits leave
state and scheduling unchanged. This follows input-generation clone semantics.

The controller owns topology validity and bounded status queues. The runtime does
not expose a mutable live protocol reference, perform provider operations during
an edit, or discard/rewrite accepted input/reply bytes. Pending transport work
keeps its order; subsequent generation observes the updated personality.

Public controller APIs should wrap this seam in typed controller-specific actions.
No arbitrary extension API or production Wii support follows from its existence.
Closures must be side-effect-free outside the candidate and protocol clones must
isolate mutable resources. These requirements also apply to input generation.
