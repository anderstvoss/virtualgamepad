# EXP-0019 — Persistent resource ownership (Gate R)

Question: Can host-writable resources survive terminal cleanup without breaking
transactional input clones or putting persistence in the transport?

Prerequisites: merged synchronous runtime and deterministic fake transport.
Experiment: `gr-hid/tests/persistent_resource.rs` implements a synthetic 256-byte
image, protected prefix, range selection, reads and writes. Import occurs before
the fake transport factory. All wire bytes are synthetic, not Wii fixtures.

Evidence: seven tests cover ephemeral recreation; exact export/import; invalid
size/protected import; protected/out-of-range writes; all report classes and
exact acknowledgements; definitely-unsent acknowledgement/input retry; rejected
input generation; uncertain delivery; close/export failure; drop; and repeated
explicit disposition. Tests run using `cargo test -p gr-hid`.

Result: passed at the deterministic lifecycle seam. A write mutates live protocol
memory once, before acknowledgement submission. Reply retries reuse bytes. Input
clones copy the current image by value; rejected generation cannot roll back a
host write. Closed Runtime retains the personality and cleanup error.

## Ownership comparison

| Candidate | Lifecycle / transaction result | Decision |
| --- | --- | --- |
| Protocol-owned value | Live request writes and input clone isolation use existing semantics; snapshot survives terminal close | Selected, exercised end to end |
| Handle-owned independent value | Host requests would need a second mutation/synchronization channel or duplicated state | No demonstrated benefit for this experiment; not implemented |
| Shared controller-owned object | Interior-mutability clones could leak edits on failed generation; requires additional transaction contract | Rejected as unnecessary complexity, not claimed experimentally equivalent |

The test-owned controller has explicit KeepSnapshot/Discard. First disposition
wins; repeated close cannot repeat an export. A fallible injected exporter runs
only on explicit caller invocation after close. Failure keeps the exact snapshot
for retry; cleanup failure stays available separately. Drop retains no exporter
and cannot write files. Dirty means a successful write changed the imported/reference
image at least once, even if later writes restore original bytes.

Decision: accept the controller-owned value/snapshot pattern. No shared primitive,
generic EEPROM trait, public persistence API or controller package is justified.
Consequences: actual file helpers must implement atomic replacement outside
runtime/providers. File replacement and crash durability are not implemented or
validated here; the injected exporter tests failure isolation only.
Revisit condition: a production resource genuinely needs continuity across
realizations or image size makes deep clones materially costly.
No new privileges or dependencies; no production Wii support.
