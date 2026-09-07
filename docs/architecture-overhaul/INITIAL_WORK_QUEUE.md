# Initial execution work queue

The [gate dependency table](ARCHITECTURE_DECISION_EXPERIMENTS.md) is authoritative. These are bounded first work packages, not permission to begin all extensions. Claim ownership in the gate ledger before running experiments.

## W0 — Finish E0 baseline and handoff

Inputs: [baseline](BASELINE.md), repository instructions, current working tree.

1. Verify actual SHA/dirty state and the [branch review](BRANCH_REVIEW.md); reconcile later changes without overwriting work. Classify branch-only candidates as reimplement, extract evidence, already integrated, or obsolete before retiring them.
2. Complete baseline Cargo/security checks and the [host survey](HOST_READINESS.md). Inventory relevant ignored tests without treating them as passed.
3. Recover available historical evidence by hash/revision; label unrecovered reports as historical claims. Do not copy private raw logs into this plan.
4. Establish the reviewed plan commit before dispatching work to another checkout. Preserve the baseline commit as rollback/reference; do not switch this checkout or create a tag incidentally.

Done: commands/results and owners are recorded, live prerequisites are known, plans are reproducible from a commit, and missing evidence is explicit. Existing defects become separate regression items rather than being silently fixed as baseline preparation.

## W1 — Establish minimal independent corpus (E1 / A)

Resolve repository owner, name, visibility, and URL from actual user/repository context before creation. The working name is `controller-protocol-corpus`; no remote has been created by this kit. Keep implementation-independent validation.

Build the minimal versioned source/claim/experiment/fixture schemas; seed a small DualSense USB fact set plus the SC2 physical-versus-compatibility contradiction. Record source lineage and independent expected bytes. Implement reference/hash/status validation and the minimum safe fixture transformation support. Do not wait for exhaustive audio or SC2 capture campaigns.

Done: Gate A can answer provenance/conflict questions, synthetic and physical fixtures are distinct, corpus commit is durably reachable, and `protocol-corpus/` pins it. Prepare CI changes within explicitly authorized scope; an absent integration check remains pending, not presumed present.

## W2 — Settle the protocol contract (E2 / C,D,J,O,K)

Build one Linux-independent fake-clock/host/adapter harness. Use DualSense plus synthetic SC2-inspired cadence/edge cases. Verify partial success, definitely-unsent/uncertain delivery, bounded queues, sequence advancement, exact request completion, close/reopen distinctions, framing, and idle-time service.

Done: deterministic regressions and ADRs select the contract, corpus generation scope is fixed, and independent golden cases guard against shared encoder/expected-data mistakes. No production provider replacement yet.

## W3 — Measure the current host path (E3 / B,P)

Can proceed independently of W2 once its baseline/input provenance is recorded. Use current code or the minimum bus-selectable adapter. Freeze versions, identity/timing controls, repetitions, consumers, and pass criteria before collecting results. Use session-specific discovery and sequential A/B runs.

Done: B/P have reproducible scoped evidence or explicit blocked/failure outcomes. Do not call the existing BUS_USB setting a newly implemented fix. Required host setup is separate from library execution.

## W4 — Probe broker feasibility early (G)

Inventory supported gadget control metadata/completion APIs and test probing before create returns. A minimal staged channel and fake startup harness precede a full broker rewrite. Live latency/capability evidence requires a provisioned host.

Done: report what can be forwarded, startup ordering, version compatibility, failure cleanup, and unresolved kernel limits. A failed G does not block valid UHID work.

## First production checkpoint

Only after the E4 prerequisites are satisfied, implement one complete DualSense USB/UHID slice: shared personality, bidirectional runtime, reply ownership, lifecycle, realization selection, and preserved semantic/uinput regressions. Preserve the old revision and valuable branch work in Git, not in the active build. Crate/API replacement is allowed. Defer audio, btvirt, and broad family migration from the first complete slice.
