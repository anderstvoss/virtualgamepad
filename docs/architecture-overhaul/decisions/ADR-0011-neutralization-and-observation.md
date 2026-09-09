# ADR-0011: native neutralization and observation contract

Status: implemented for the four current controller handles.

Each handle exposes neutralize() as a single validated semantic edit. It releases
buttons, dpad, sticks, triggers and contacts and restores the controller-defined
neutral motion. It performs no transport I/O; the existing explicit commit()
delivers the accepted state. Backpressure leaves that state dirty and retryable.
Closed handles reject neutralization without mutation.

Identity, battery metadata, protocol clocks/counters, Switch stream configuration
and host-owned output state are preserved. DS4's touch sample sequence advances
once when releasing active contacts, as for a normal contact update; repeated
neutralization with no active contacts does not advance it. Neutralization is not
recreation, a battery reset, rumble stop, or cancellation of required host replies.
It introduces no normalized state model, runtime thread or adapter dependency.

All handles expose provider_diagnostics(). Native session cleanup now retains
close failure even when the provider's own diagnostics do not, and reports the
library state as closed. HID runtime automatic-close failures are also retained.
Idempotent close does not retry uncertain cleanup or erase its diagnostic record.

Optional outputs are observed in consuming service order, preserving local
component order. Required replies precede optional callbacks; component-group
observations are returned only after required work or terminal cancellation.
There is no cross-device hardware chronology guarantee. Existing bounded queues
and omitted counts remain; no speculative timestamp fields are introduced.

Regressions cover native controls, both contacts, motion, retained metadata,
repeated neutralization, no I/O before commit, retryable failed commit, rejected
edits and closed-session rejection across all current families. Cleanup tests
require retained failure diagnostics after repeated close. Production DS4 split,
physical fidelity and support-level promotion remain separately gated.
