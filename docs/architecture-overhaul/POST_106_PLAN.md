# Post-106 core refinement

**Current execution:** [pre-alpha stabilization ledger](PRE_ALPHA_STATUS.md), based
on merged PR108. It supersedes the execution order below; historical experiment
results remain unchanged. Gate T representation is required for alpha, while its
unavailable physical evidence remains explicitly deferred. Final API review and
release require the maintainer's controller/demo refinement checkpoint.

## Historical planning and evidence


Baseline: merged PR #106, `d3a6832`; corpus `e65c044d062ed833ad13641f54c826db16d176e4`.
One staged core PR and a companion corpus change. No second architecture rewrite.

| Gate | Required experiment | Acceptance boundary |
| --- | --- | --- |
| Q | Exact USB/BT UHID selection and fake CREATE2 | Shared mechanism, exact selection, mismatch before I/O; not Bluetooth Gate L/M |
| R | Synthetic persistent memory lifecycle | Typed resource, isolated clones, exact recoverable snapshot after terminal failure; no implicit persistence |
| S | Test-only Wii-like live topology | One session, bounded attach/detach edges, bridge/downstream, transactional edits; no provider branches |
| T | Native control/exposure evidence | Physical/raw/semantic/realization/Linux/SDL/profile dimensions remain separate, including unknowns |
| U | Existing SDL probe differential observations | Versioned comparable observations, exact selection, structured differences and explicit missing evidence |

Write EXP results and ADR decisions after deterministic experiments; do not turn
hypotheses into accepted APIs. Preserve current controller regressions, exact
realization selection, irreversible close, delivery certainty and bounded work.
No new privileges, host policy changes, background executor, generic memory API,
arbitrary extension plugins, or new production controller packages are authorized.

## Production Wii Remote gate

**Blocked pending explicit later maintainer authorization**, even if Q/R/S pass.
The Wii-like prototype is test-only, uses synthetic bytes, and cannot appear in
compiled production controller manifests or the demo. Gate L (BT personality),
Gate M (actual Bluetooth), and hardware/corpus acceptance remain independent.

## Evidence and delivery

Source, synthetic, physical and consumer evidence remain distinct. Existing DS4
split-touch, Switch rumble fidelity, Steam/Eden, broker, audio and Bluetooth
limitations remain. No support cell is automatically promoted by a gate result.
The companion corpus revision must be published on its main before adoption by
the core repository. A pending corpus PR is not an adopted corpus revision.
Run workspace checks, tooling/corpus tests and repository hooks for delivery.
