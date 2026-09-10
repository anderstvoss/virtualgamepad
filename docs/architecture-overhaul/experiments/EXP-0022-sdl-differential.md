# EXP-0022 — Differential consumer observations (Gate U)

Question: Can the existing exact-device SDL probe compare reference and virtual
observations without confusing source decoding or API success with physical truth?
Prerequisites: existing C probe, pinned SDL 3.2.0
`535d80badefc83c5c527ec5748f2a20d6a9310fe`, deterministic tooling tests.

Experiment: extend the existing probe to contract version 2, add optional reopen,
and compare measured fields with `scripts/compare-sdl3-observations.py`. Both
reference and virtual use the same probe and schema. Existing positional modes
and mapping_ready synchronization remain. Historical v1 inputs remain partial;
the unconditional historical consumer_closed field is deliberately not trusted.

Evidence: contract tests cover failed/open/reopen lifecycle helpers; four fake-SDL
integration tests execute the actual C probe through duplicate/absent selection,
failed initialization/open/reopen, extra buttons, touch, output calls, control
warmup and close counts. Nine comparator tests cover expected/unexpected/missing
fields, exact evidence-linked exceptions, incompatible scenarios/builds, failed
captures, v1 records CLI parsing and hash-bound external reverse/cleanup evidence. The probe compiles with strict C warnings
against the pinned real SDL 3.2.0 build; invalid-argument output parses as v2 and
correctly reports no consumer close. No host devices were opened in that check.

Result: deterministic harness/schema tests pass. Full Gate U remains **blocked**
for measured reference-versus-virtual acceptance and corpus adoption. Verified
backend identity/mapping database provenance are not supplied by this SDL probe;
they are explicitly unmeasured, not guessed from requested hints. External
controller-side reverse output and provider removal observations are also required
before those dimensions can be claimed. Companion SDL source claims exist at
corpus revision `e677a26` ([companion PR](https://github.com/anderstvoss/controller-protocol-corpus/pull/2)); they are not yet the adopted corpus pin.

Decision: accept the versioned comparison workflow with structured differences.
Consequences: zero unexpected differences may still contain unmeasured fields;
exit 0 alone is not full compatibility. Successful rumble/LED calls are not
physical effects. Contact masks cover the first touchpad and at most 32 fingers;
other contact events produce an explicit unavailable reason instead of truncation.
Revisit condition: exact backend/source provenance or broader contact capture is
available through a versioned consumer API/harness extension.
No new dependencies, production controllers, privileges or physical support claims.
