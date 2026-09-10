# ADR-0016 — Versioned SDL consumer comparison

Accepted for the harness after
[EXP-0022](../experiments/EXP-0022-sdl-differential.md); full Gate U remains blocked.

Reuse the existing exact-path C probe for physical and virtual observations.
Version 2 preserves legacy fields and introduces explicit measured/missing values,
actual consumer lifecycle, capability names and optional reopen. Source/build
revision is distinct from kernel backend evidence and physical device truth.

Comparison reports each measured field as match, expected realization limitation,
unexpected difference or not measured. Expected limitations require exact values,
a reason and an evidence locator. Missing values never become a match. Different
profiles/control cases cannot produce behavioral equivalence. Output API success,
controller reverse observations and physical effects remain separate facts.

The comparator reads historical v1 as partial input and does not trust its
unconditional close field. No GUI automation or second consumer framework is
introduced. External capture provenance and missing backend/removal observations
remain explicit release/evidence work, not automatic support promotion.
