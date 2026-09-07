# Implementation assessment — 2026-09-06

Review branch: `architecture/protocol-session-rewrite`, [PR #106](https://github.com/anderstvoss/virtualgamepad/pull/106). Foundation and production changes are committed in coherent batches; no remote-main push or history rewrite is required. The [gate ledger](GATE_STATUS.md) controls acceptance.

The implemented UHID path now keeps state, autonomous cadence, GET/SET decisions, and output validation in synchronous controller personalities. The neutral runtime owns bounded delivery, request identity, retry deadlines, and terminal cleanup. UHID retains transport framing, kernel request mapping, and lifecycle metadata. DualSense, DS4, Switch Pro, and existing standard-HID Xbox behavior share this boundary. uinput presentation and the existing compiled dummy_hcd path remain intact.

Review fixes include rejection before SET success, one-event UHID writes after partial/uncertain delivery, terminal cleanup after a failed close, preserving accepted state and reply identity under pressure, and exposing writable polling interest through controller handles. Deterministic tests retain transaction, compound rollback, framing, concurrency, removal, and demo-selection regressions. Independent synthetic DualSense fixtures are pinned to published corpus evidence; they are not physical captures.

All required workspace formatting, check, strict Clippy, and test commands pass. Gitleaks passes. Five corpus workflow tests cover regeneration/staleness, absent or mismatched submodules, dirty contents, unpublished revisions, and recursive checkout. Remote revision verification passes. `cargo package -p gr-hid --allow-dirty` packages and verifies the new dependency-free crate. Live uinput creation/destruction passed separately; ignored hardware tests remain skipped.

The independent corpus has a separate [parent-validation PR #1](https://github.com/anderstvoss/controller-protocol-corpus/pull/1), with seven tests and schema validation passing. This fixes transformation identifier validation without prematurely moving the superproject gitlink. The published seed remains adopted.

Remaining acceptance and implementation: Basic UHID access and production DualSense kernel startup are now validated; B/P still require controlled baseline/bus/driver and consumer comparisons. See [EXP-0006](experiments/EXP-0006-dualsense-live-startup.md). G has a source-level metadata/completion mismatch and requires a supported kernel/profile decision plus live startup, latency, and cleanup evidence before broker replacement; see [EXP-0004](experiments/EXP-0004-gadget-capability.md). Compound, host/USB audio, Bluetooth, compatibility, physical SC2 expansion, and wheel/HOTAS acceptance are not implemented or promoted by this batch. Explicit development-helper provisioning and restoration are recorded in EXP-0005; EXP-0006 used existing access without further host changes. The authenticated corpus CI job needs the operator-provided `PROTOCOL_CORPUS_READ_TOKEN`; ordinary builds do not require it. The full E0–E6 roadmap is therefore still incomplete, and PR #106 remains a draft.

The transport identity review removed aliasing caused by caller-session reuse and
made UHID reject silent identity truncation. A compact process/creation ordinal
fits curated phys/uniq fields. This does not change controller-owned pairing
addresses or claim namespace-independent physical identity. The new opt-in live
probe validates the production personality beyond the minimal provider test.

The test-only bus apparatus now runs against both the archived baseline and the
rewrite. Three repetitions per bus/revision all enumerated and cleaned up; USB
selected playstation and virtual bus selected hid-generic. This narrows the
remaining B/P work to controlled consumer/binding/reference evidence and observed
report behavior; it does not diagnose the historical Steam issue. See
[EXP-0007](experiments/EXP-0007-bus-baseline-comparison.md).
