# Device specification validation

Every curated controller must keep its identity, descriptor, report layouts,
feature reports, transport parameters, native controls, and reverse semantics
in reviewed compiled Rust code. Acceptance requires reproducible public
evidence or sanitized captures, deterministic codec fixtures, malformed-input
tests, and prepared-host comparison against the intended host-visible device.

Evidence must state the controller revision and transport, source provenance,
known gaps, and exact fidelity claim. Unknown bytes stay losslessly available
to typed controller-specific fallback events until their semantics are proven.
Captured serials, host paths, account data, and private traffic must be removed.

Adding support follows
[ADDING_A_CURATED_CONTROLLER.md](../implementation/ADDING_A_CURATED_CONTROLLER.md).
YAML may serialize sanitized evidence fixtures but cannot define runtime
behavior or register a controller.
