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

Evidence: contract tests cover failed/open/reopen lifecycle helpers; six fake-SDL
integration tests execute the actual C probe through duplicate/absent selection,
failed initialization/open/reopen, extra buttons, touch, output calls, control
warmup, close counts and source-derived backend identification. Ten comparator
tests cover retained inference provenance and expected/unexpected/missing
fields, exact evidence-linked exceptions, incompatible scenarios/builds, failed
captures, v1 records CLI parsing and hash-bound external reverse/cleanup evidence. The probe compiles with strict C warnings
against the pinned real SDL 3.2.0 build; invalid-argument output parses as v2 and
correctly reports no consumer close. No host devices were opened in that check.

Result: Gate U **passes** its harness/schema and source-evidence scope. Core adopts
the claims from published corpus main `a1789d6ed92b2325016dd78be765342f3ca19aa4`. Physical comparisons
are controller-acceptance work, not a prerequisite for this harness/schema pass.
The reviewed Linux SDL 3.2.0 GUID/path combinations now supply source-derived
backend identity; ambiguous combinations remain unmeasured. Mapping database
provenance, controller-side reverse output and provider removal require external
observations before those dimensions can be claimed. Companion SDL source claims exist at
corpus revision `0468aeb` ([companion PR](https://github.com/anderstvoss/controller-protocol-corpus/pull/2)); they are included in the adopted corpus merge revision.

Decision: accept the versioned comparison workflow with structured differences.
Consequences: zero unexpected differences may still contain unmeasured fields;
exit 0 alone is not full compatibility. Successful rumble/LED calls are not
physical effects. Contact masks cover the first touchpad and at most 32 fingers;
other contact events produce an explicit unavailable reason instead of truncation.
Revisit condition: controller-specific reference runs, exact
mapping provenance or broader contact capture becomes available.
No new dependencies, production controllers, privileges or physical support claims.

## Follow-up investigation — 2026-09-10

Gate U's original definition explicitly permits a harness/schema pass without
every physical reference. The previous blanket physical/backend blocker was too
broad. No physical gamepads are attached in the read-only host inventory, but the
backend instrumentation gap is independently fixable.

The probe now uses the [reviewed GUID construction](https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/SDL_joystick.c)
plus the exact opened Linux path, constrained to the baseline version/revision
label and a structured matching VID/PID. Root `backend_evidence` records the
method and source pin. Vendor-less GUIDs can contain name bytes in the signature
position, so they are deliberately excluded. Conflicting requested hints cannot
change the interpretation. Unknown builds, malformed GUIDs, mismatched identity,
unsupported paths and failed opens produce no backend claim. This identifies the
SDL driver family by source inference, not a kernel binding or physical bus.

The real baseline build compiles with strict C warnings; a no-device invocation
parses as v2 and records no open/close/backend. Deterministic tests exercise both
positive backend paths and rejection cases without opening host devices. Full
mapping database provenance remains unavailable: an active mapping string does
not establish which built-in, generated or user-supplied entry won. The existing
capture-hash evidence attachment remains available for independent observations.
