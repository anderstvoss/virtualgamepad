# ADR-0007 — Single service owner with bounded demo edits

Status: accepted for the demo integration; live GUI acceptance remains pending.

## Problem and inspected boundary

The per-controller service worker shared a controller mutex with GUI drawing.
Nonblocking GUI acquisition avoided waiting for the worker, but drawing after
acquisition could still stall required protocol service. Optional display isolation
alone did not resolve this. This is the handoff's service/control contention issue,
not evidence that synchronous personalities or executor-neutral runtimes fail.

## Decision

Move each live controller into its worker. The GUI holds a detached clone of that
controller's native semantic state and static surface, plus sampled diagnostics.
It queues only private, typed native setter commands; it cannot supply arbitrary
application callbacks to the worker. No normalized authoritative state is added.

One channel slot carries at most 64 edits. The worker executes at most one batch
before servicing; all setters must succeed before commit. Any rejected edit or
commit failure closes the session before further servicing can submit partially
edited state. This is failure containment, not rollback to a still-live controller.
Required protocol semantics remain in the existing curated handles.

The GUI waits for a matching applied snapshot before accepting another batch.
Stale snapshots cannot reopen editing. Full/disconnected queues or sequence
exhaustion become visible controller failures, never silent release loss. This
preserves accepted input order but does not guarantee that an external consumer
samples every short pulse. Input while controls are awaiting acknowledgement is
not accepted. Stop uses a separate channel, checked before queued edits each cycle.

Display publication uses a nonblocking lock and may be skipped. The next successful
publication includes current state and acknowledgement. A stalled GUI therefore
cannot block the live controller; OS scheduling, provider operations and finite
native encoding work still bound practical timing. No hard realtime claim follows.

## Alternatives and API impact

A longer-lived shared mutex leaves the original hazard. A latest-state mailbox can
collapse press/release edges. An unbounded command queue can grow without limit.
A mandatory library worker or public split handle is unnecessary for this proof:
the demo can own scheduling with existing native setters and `service`.

Library APIs and persistence policy are unchanged. Per-controller threads remain
a demo choice; a downstream owner can schedule multiple controllers in one loop.
Future reusable adapters need their own concrete integration requirements.

## Validation and remaining evidence

Fake-owner tests cover service without UI, locked display, independent removal,
short deadlines, bounded display loss, ordered edit batches, stale acknowledgements,
queue saturation/disconnection, rejected edits without commit, sequence exhaustion,
unexpected worker exit and stop with queued input. The detached editor's bound is
also tested. Existing selection, conversion, touch-latch and lifecycle regressions
remain, with the obsolete mutex test replaced by ownership and queue regressions.

No live touch test or display restart is needed for these deterministic claims.
Live GUI and consumer timing remain unvalidated; isolated DS4 touch acceptance is
still blocked. Persistent identity and compound failure/association remain separate
next batches, as do Gate G and all extension-specific acceptance requirements.
