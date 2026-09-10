# EXP-0020 — Live internal topology (Gate S)

Question: Can controller-owned attachments and a downstream bridge change inside
one live personality without compound host components or provider changes?

Prerequisites: Gate R clone ownership result, fake transport and manual clock.
Experiment: `gr-hid/tests/internal_topology.rs` implements test-only Wii-like
Remote/Nunchuk/MotionPlus concepts using entirely synthetic IDs and bytes.
No Linux provider or production controller manifest references this prototype.

Evidence: five tests exercise one session from base through direct attachment,
calibration reads, control changes, detach, bridge activation and repeated
pass-through/downstream transitions. They cover invalid topology and semantic
validation rollback, four-edge queue saturation, rapid detach during initialization,
malformed host writes, every report class, exact error replies, queued input and
reply byte preservation, uncertain submission, close and edits after close.
Run with `cargo test -p gr-hid`.

Result: passed at the deterministic architecture seam. The pre-existing API only
allowed semantic input edits; caller-owned topology needed a controlled personality
edit. `Runtime::update_protocol` now clones, edits, validates current semantic
input and installs atomically. Failed edits do not alter deadlines or state.
Accepted edits mark work dirty. Previously accepted reports and required replies
retain their bytes; new edges follow queued work through bounded servicing.

Decision: accept the small transactional protocol-edit seam (ADR-0014).
Consequences: attachment validation, edge queues and register semantics remain
controller-owned. No generic attachment trait, host-component composition,
provider family branch or production Wii package is introduced. At this seam
callers must keep edits free of external side effects and clone state isolated.
Revisit condition: a future controller demonstrates that preserved queued reports
cannot satisfy its physical topology ordering, or an accessory is independently
host-visible. Do not silently purge queued work to manufacture a pass.
No physical Wii fidelity, actual Bluetooth or new privileges are claimed.
