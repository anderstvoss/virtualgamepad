# ADR-0003 — Pinned source evidence and test-only generation

Status: accepted for minimal corpus and development fixtures. Evidence: EXP-0001.

Use an independent private corpus pinned by a Git submodule. Source-derived and synthetic records permit deterministic architecture experiments. They do not establish physical fidelity. Retain conflicting hypotheses and source lineage; require controlled evidence for physical promotion.

Generation initially copies only reviewed synthetic test fixtures and records corpus commit, schema version, generator version, and input hashes. Runtime protocol behavior remains handwritten. Ordinary Rust builds and downstream source packages require neither Git metadata nor private corpus access.

Authenticated CI is separate from ordinary Rust jobs. Existing private-repository job gating remains intact; public fork jobs cannot run private evidence checks. Missing access is an explicit failure in requested corpus jobs, never a conformance pass. An operator must provision a corpus read credential before enabling those jobs; no credential or repository secret is created by this change.

Revisit when production generation needs additional reviewed constants or physical fixtures. Such expansion requires independent golden tests and regeneration checks, not runtime profile interpretation.
