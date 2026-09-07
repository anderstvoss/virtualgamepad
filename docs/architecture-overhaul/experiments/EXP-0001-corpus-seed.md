# EXP-0001 — Minimal evidence corpus

Owner: Codex. Date: 2026-09-05. Gate: A. Result: passed for minimal E1 source/synthetic readiness, not physical conformance.

Inputs: baseline `9b466e0`, reviewed kit `e3bd06f`, corpus `9d0d56e`. Corpus source registry pins Linux v6.12, OpenPuck, and SC2 research revisions with SHA-256 hashes. No raw captures were imported.

Predeclared criteria: validate references, schema versions, fixture hashes, transformation lineage, source independence, and prevent unsupported physical promotion. Retain the historical compatibility-versus-physical identity contradiction and a minimal DualSense USB subset.

Commands: `python tools/validate.py` and `python -m unittest discover -s tests` in the corpus development environment. Results: 20 records validated; six tests passed. `gitleaks detect --redact` and no-Git content scan passed. The installed Gitleaks does not support its newer `dir` command; use `detect --no-git` for content scanning.

Finding: pinned current OpenPuck source selects 28de:1304. The old 28de:1142 and physical-capture claims remain historical/conflicted with unrecovered originating artifacts. No source-only evidence was promoted to physical truth.

Implementation-independent schemas/validator live only in the corpus. Superproject adoption verifies clean matching gitlink/HEAD, retained remote main reachability, fixture hashes, and reproducible checked-in artifacts. See ADR-0003.
