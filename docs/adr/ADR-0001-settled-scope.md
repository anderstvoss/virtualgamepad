# ADR-0001 — Settled scope and architecture constraints

Status: accepted product/design constraints from the September 4 planning discussion; not an experimental result.

> **Partially superseded (2026-10-14):** the corpus is no longer a Git submodule.
> PR #152 replaced it with the one-line `protocol-corpus.rev` pin, read only by CI,
> so Git consumers need no corpus access. The corpus remains an independent
> repository and is still not a runtime controller definition language.
>
> **Updated (2026-10-15):** the public `controller-protocol-research` repository
> replaces the private corpus. It is a submodule again, at `protocol-research/`,
> marked `update = none` so Cargo and recursive clones skip it.

## Decision

- Keep virtualgamepad a standalone curated Rust library, independent of Gamepad Manager slot/routing policy.
- Keep controller-native typed semantic state and exact concrete realization selection with no fallback.
- Reuse stateful controller protocol personalities across compatible realization mechanisms; providers do not interpret controller semantics.
- Retain dummy_hcd as a selectable realization. The earlier proposal to abandon its product role is superseded.
- Maintain the research corpus as an independent repository pinned by a Git submodule. Reviewed compiled Rust implements behavior; corpus YAML is not a runtime controller definition language.
- Preserve transactional edits, retryable accepted state, terminal library close, controller-owned compound components, private fake-I/O seams, and narrow privileged construction.
- Treat wheel/HOTAS as a review benchmark, not a requested implementation target.

## Evidence boundary

These decisions reflect user-approved direction, not proof of host behavior. BUS_USB is already present at the inspected baseline; Steam sensor acceptance remains a scoped experiment. Mutable broker protocol forwarding, compound UHID usefulness, audio implementations, and btvirt viability remain governed by A–P results.

## Consequences

Implement the E0–E6 dependency sequence. Keep mandatory replies independent of observer callbacks and preserve shared protocol ownership during migration. Optional backend failures do not invalidate unrelated realizations.

## Revisit condition

Reopen product scope only with user direction. Record evidence-driven technical refinements in separate ADRs linked to EXP records; do not silently replace these constraints or rewrite negative experiment history.
