# ADR-0013 — Controller-owned resource values and explicit snapshots

Accepted for the tested lifecycle pattern after
[EXP-0019](../experiments/EXP-0019-persistent-resources.md).

A protocol personality may own typed persistent resource values. Validate import
before opening its provider, isolate protocol clones, and apply host writes once
when accepted. A controller-specific handle can copy the final image from the
closed runtime. Transport close is terminal and independent of persistence.

Deliberate shutdown exposes an explicit retain/discard path. Implicit drop never
persists; failed export leaves a retained snapshot recoverable. File paths and
atomic file replacement belong to a future controller/caller helper. The test
exporter establishes failure isolation, not actual file durability.

Do not introduce a universal memory state field or EEPROM trait. Cross-realization
resource continuity and large-image optimization require a new measured need.
