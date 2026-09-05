# Architecture overhaul execution kit

Status: planning files prepared; implementation and experiment execution have not begun in this task. The code baseline is recorded in [BASELINE.md](BASELINE.md). These files still require a reviewed commit before other checkouts can rely on them.

## Reading order and authority

1. [Agent Start Here](AGENT_START_HERE.md): scope and settled decisions.
2. [Corpus handoff](CONTROLLER_PROTOCOL_CORPUS_AGENT_HANDOFF.md): evidence and schema requirements.
3. [Decision experiments](ARCHITECTURE_DECISION_EXPERIMENTS.md): experiment definitions; section 17 exclusively owns E0–E6 dependencies.
4. [Rewrite handoff](VIRTUALGAMEPAD_ARCHITECTURE_REWRITE_AGENT_HANDOFF.md): runtime/provider contracts.
5. [Submodule workflow](PROTOCOL_CORPUS_SUBMODULE_WORKFLOW.md): adoption and reproducibility.

## Operational files

- [Baseline](BASELINE.md): inspected source, reusable tests/tools, and unknown evidence.
- [Gate status](GATE_STATUS.md): the authoritative current execution status of A–P. Update it with links to actual run records; do not copy another status table into the handoffs.
- [Initial work queue](INITIAL_WORK_QUEUE.md): concrete first deliverables and completion criteria.
- [Host readiness](HOST_READINESS.md): environment survey to complete before live experiments.
- [Settled decisions](decisions/ADR-0001-settled-scope.md): accepted product constraints, separated from untested technical hypotheses.
- [Experiment template](templates/EXPERIMENT.md) and [ADR template](templates/ADR.md): records for evidence and resulting decisions.
- [Experiment index](experiments/README.md): where sanitized, reproducible run summaries belong.

## Recording policy

Use repository-relative paths in public records. Keep private/raw captures and host identifiers outside Git; reference safe aliases and hashes. Corpus research records belong to the independent corpus when it exists; runtime realization experiments belong here and reference the pinned corpus commit. Do not maintain duplicate evidence authorities.

No blanket implementation, publishing, host-policy, or hardware-operation permission is implied by a checklist. Follow the actual task authorization and repository rules. Proposed CI/release work requires the appropriate explicit scope; do not modify protected files incidentally.

## Reassessment — 2026-09-05

Read [context reassessment](CONTEXT_REASSESSMENT.md), [branch review](BRANCH_REVIEW.md), and [early-development rewrite ADR](decisions/ADR-0002-early-development-rewrite.md). These reconcile newly supplied historical notes, establish memory authority limits, and remove unnecessary API/dual-runtime compatibility constraints. Source context remains unchanged in its original local files; it is not copied into the normative plan.
