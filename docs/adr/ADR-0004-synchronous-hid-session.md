# ADR-0004 — Synchronous HID session and bounded delivery

Status: accepted for the tested USB/HID contract. Evidence: EXP-0002.

Use a controller-neutral `gr-hid` crate with typed reports, requests, lifecycle events, protocol actions, transport delivery outcomes, and a deterministic runtime. Logical report IDs are optional, nonzero when numbered; payloads exclude IDs. USB/BT personality framing remains controller-owned; kernel envelopes remain provider-owned. Providers must preserve authoritative START numbering flags.

Personalities clone for transactional generation. Sequences advance at queue acceptance; definitely-unsent retries reuse exact bytes. Uncertain submission terminates the session. The desired semantic state is separate from protocol timing and delivery. Input batches cannot promise atomic consumer visibility. Default input capacity is 32 and each service call permits 16 submissions plus one host event. Each runtime represents one component; callers service components round-robin.

Reserve one required-reply slot with a 100 ms transport retry deadline. Validate SET before producing its success reply. Provider-assigned monotonically increasing request ordinals prevent replay independently of reusable kernel IDs. Read, answer, and attempt completion in one service cycle. Invalid/late requests, invalid reply contracts, uncertain delivery, or reply expiry close the session. A required reply never depends on optional subscribers.

Typed observations are separately bounded. Acknowledged observations remain recoverable after a later service error; overflow is explicit in a cumulative counter and does not evict protocol actions. Close is terminal and idempotent, including cleanup failure. Host OPEN/CLOSE is not library close.

Expose borrowed readiness, write interest, and next monotonic deadline; service while input is unchanged. The outer embedding owns scheduling. The tested DualSense cadence is a compatibility policy, not fresh physical timing evidence. No internal executor dependency is justified.

Revisit for a concrete personality that cannot express its scheduling or resynchronization requirements. Generic wheel/HOTAS support is not added; typed output may represent long-lived effects without changing provider meaning.
