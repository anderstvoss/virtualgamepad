# ADR-0002 — Early-development rewrite and source authority

Status: accepted planning direction from the user's 2026-09-05 clarification. No experiment result is implied. Refines ADR-0001 and supersedes compatibility-preservation wording in earlier plan revisions.

## Decision

The library is early development. Replace public APIs, crates, provider internals, binaries, and obsolete tests when justified by the target architecture. No compatibility shim or old/new runtime coexistence is required. Retain recoverable Git revisions and meaningful regression evidence; do not preserve implementation accidents as requirements.

Keep correctness goals: validated state changes, recoverable delivery, explicit bounded request ownership, terminal library close, no implicit provider fallback, curated privilege boundaries, and honest feature evidence. Re-express tests at the new seams; an old snapshot is not proof the old behavior was correct.

Memories and generated chat/context files inform investigations only. Product authority comes from current user direction and reviewed decisions. Code at an exact SHA establishes implementation; primary specifications and controlled observations establish applicable protocol/host facts. Merge status establishes integration history, not correctness. Unrecovered historical claims remain reported, not physically validated.

## Branch handling

Inventory actual refs and branch-only work before retiring it. Check PR merge status, patch/content equivalence, and later reversions: ancestry counts alone misclassify squash merges. Reuse narrow tests, fixtures, and design lessons without merging obsolete profile/tier stacks. Do not delete or rewrite branch history during an assessment.

## Consequences and revisit

E0 is a short evidence/branch triage, not an obligation to finish every old phase or stabilize discarded code. Deterministic architecture work can proceed while consumer acceptance is unavailable; unsupported claims remain pending. Revisit compatibility policy only when a real downstream support commitment emerges or the user changes scope.
