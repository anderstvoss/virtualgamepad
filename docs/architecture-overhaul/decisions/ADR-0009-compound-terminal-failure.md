# ADR-0009: required-component terminal failure

Status: accepted for the native compound helper; production compound acceptance
remains gated.

The compound helper previously returned provider failures with component context
but left siblings open until its caller explicitly closed them. This could leave
a logical controller with only part of its required presentation working.

All components selected into `CompoundSession` are required. A send, reply or
reverse-read provider error other than `WouldBlock` closes the entire group before
returning the original error. This includes uncertain writes: no automatic retry
is safe. `WouldBlock` preserves the group for controller-owned bounded retry;
read backpressure remains an ordinary end of drain. Invalid caller operations
are rejected before I/O and do not terminate a healthy group.

Cleanup attempts every component once in reverse order. Cleanup failure cannot
mask the initiating error or prevent later close attempts. Diagnostics expose
cleanup failures; terminal library state does not prove successful kernel cleanup.
Already delivered reverse events remain observable, but terminal teardown cancels
their requests. Later I/O cannot revive the group or replay requests.

The runtime does not interpret controller lifecycle records or request deadlines.
The logical owner must explicitly close on protocol removal, expired bounded
retry or another protocol terminal condition. STOP/CLOSE consumer events are not
automatically library removal. Optional hot-removable components require a future
controller-owned model; this change adds no optional-component flags, generic
roles, identity database or new realization.

Deterministic tests cover both failure positions across input, reply and read,
cleanup failure containment, uncertain writes, partial reverse delivery, explicit
backpressure retry and idempotent terminal cleanup. Existing rollback, exact reply
routing and independent-controller tests remain. No live-host or OS atomicity
claim follows. DS4 split production acceptance still requires isolated consumers;
compound identity and association remain separate work.
