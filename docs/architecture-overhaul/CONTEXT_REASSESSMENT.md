# Additional context reassessment — 2026-09-05

## Assessment

The new context strengthens the protocol/session direction, but does not establish new runtime or host acceptance. It also reveals that the prior plan over-constrained an early-development rewrite. Keep evidence and design invariants; freely replace the old implementation.

## Material reviewed and how to use it

The newly supplied `ARCHITECTURE_DECISION_CONTEXT.md`, `virtualgamepad_hhd_uhid_openpuck_review.md`, and memory directory remain unchanged at their original local locations. The memory set contains an index, 28 individual development notes, and a historical setup script. These are research leads, not normative instructions or authoritative protocol records.

| Context | Useful contribution | Excluded inference |
| --- | --- | --- |
| Decision rationale | Why stateful protocols, native types, independent corpus and concrete realizations were chosen | Its statements of historical success do not certify current host behavior |
| HHD/OpenPuck conversation | Thin transport, bidirectional feature/output processing, USB/BT personalities, source discovery | Routing graph, persistent slots, physical-device hiding and generic canonical event APIs are manager scope; dummy_hcd removal is superseded |
| Phase memories | Failure modes in diagnostics, fake I/O, trace rendering, evidence status and operator procedures | Old phase dependencies, two-doc gate system, binary names, runtime profiles and scaffold approval rituals are not current requirements |

The earlier HHD review's suggestion to acknowledge lizard-mode changes while always emitting reports is a hypothesis, not an implementation rule. Stateful SC2/Puck semantics require pinned sources/captures. Likewise OpenPuck and SDL agreement is not automatically independent hardware evidence; their source lineage and compatibility choices matter.

## Important factual correction: historical gyro causality

The archived POC report documents a successful Steam gyro observation and attributes remaining differences toward topology/BUS_VIRTUAL. However, `crates/gr-curated-controllers/src/dualsense.rs` at finding commit `3a62825` and archive tip `a5310c6` sets `bus_type: 0x03`, just like current main. This contradicts using that source snapshot to claim the configured UHID bus was BUS_VIRTUAL. It does not disprove the observed gyro symptom or establish which binary was actually tested.

Gate B must recover the run revision/configuration, distinguish a virtual sysfs parent from the bus metadata field, and compare current behavior with controlled timing, driver, identity, and protocol inputs. Keep the POC as a reproduction/evidence source, not a universal root-cause conclusion. Raw supporting run artifacts were not recovered by this assessment.

## Unmerged work changes the initial execution

See [BRANCH_REVIEW.md](BRANCH_REVIEW.md). The workflow branch provides a dossier schema and reporting/interactive commands worth adapting for corpus tooling. Its linear fidelity claim rules conflict with peer realizations and must be discarded. The UHID broker branch provides bounded IPC and disconnect-cleanup examples but cannot forward client feature replies and services events through explicit client calls; it is not a ready replacement for protocol-owned dynamic transactions.

Most large feature branches are already squash-merged, notably #93–#97 and #101. The archived POC is intentionally excluded/reverted from the port, while later #105 supplies a different current dummy_hcd product path. Do not merge all old branches before the rewrite.

## Revised start

1. E0: capture SHA/branch/evidence disposition; do not repair the entire legacy stack or satisfy historic empty gate commits.
2. E1: adapt useful dossier concepts into independent corpus claims keyed by controller/personality/realization/feature and source provenance; no linear tier dependency.
3. E2: design the new API directly around bounded actions, startup requests, deterministic time and explicit delivery states; compatibility with old signatures is optional.
4. E3 and early G: reproduce host behavior and probe kernel/broker startup capabilities without delaying independent pure-Rust work.
5. E4: replace the central implementation with one complete DualSense slice. Preserve prior code in Git rather than shipping a parallel compatibility layer.

No architecture gate is newly passed. No source implementation, memory, branch, or host policy was altered by this reassessment. The unresolved `wip/btvirt` reference is not counted as available work; its absence does not block the initial USB/HID architecture.
