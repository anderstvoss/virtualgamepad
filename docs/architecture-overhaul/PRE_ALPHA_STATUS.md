# Pre-alpha stabilization: active execution ledger

This replaces the earlier 72-step execution checklist. Imported guidance and
conversation proposals are background; current maintainer decisions govern.

## Baseline and release contract

Implementation starts from merged PR108, `6f7b084ac8f1d4c8cad0c108c7228574fd60469c`.
The preceding local tip `3b12004` and merged PR108 have identical Git trees
(`c15a3c3a74aa9a93da79f80eda9ba4cc864c2301`). Local main was fast-forwarded and
implementation moved to `codex/pre-alpha-stabilization` at the maintainer's request.
No prior history was rewritten.

The maintainer confirmed post-reboot UHID creation and cleanup after PR108.
That acceptance is complete; it is not a new hardware measurement by this batch.
Historical experiment records retain their original observations.

The first alpha is a Git-based source release, not crates.io publication. The
root application API is the stability boundary; `experimental` and supporting
workspace crates are implementation/research interfaces. Alpha breaking changes
remain possible but require a reason, migration notes, and refreshed consumer tests.
Registry publish restrictions and release workflows remain unchanged.

Required sequence: API/stress work → preliminary candidate → controller/demo
refinement and maintainer hands-on feedback → fresh API review → separate quality
review → release. An unattended demo run cannot complete the feedback checkpoint.

## Work packages

| Package | Status | Evidence / exit |
| --- | --- | --- |
| Baseline and contract | Complete | Baseline above; [API inventory](API_INVENTORY.md), [application contract](../APPLICATION_API.md) |
| Application API contraction | Implemented; preliminary | Opaque creation options; controller handles, errors, diagnostics, identity and component metadata in root; old machinery under opt-in experimental module |
| Consumer and exposure stress | Deterministic scope complete | Root consumer lifecycle tests; executable negative/positive doctests; existing Gate T, memory, topology and compound regressions; benchmark conclusions below |
| Current controllers and demo | In refinement | Migrated demo; native state readback and DS4/Switch labels; deterministic regressions; selected live tests recorded in completion evidence |
| Maintainer hands-on checkpoint | **Pending** | Exercise creation, controls, feedback, arbitrary controller selection/removal, diagnostics and recreation; incorporate feedback |
| Final independent API review | Not started | Re-inventory after feedback, review every refinement delta, rerun boundary/consumer tests |
| Separate final quality review | Not started | After final API review: resource/unsafe/concurrency/dependency/package audit |
| Git alpha release | Not started | Exact-revision consumer build, cached offline build, changelog/versions, approved gates, tag and post-tag smoke |

## Issue register

| ID | Class | Disposition and acceptance condition | Evidence |
| --- | --- | --- | --- |
| API-01 | API blocker | Resolved: normal callers supply realization only; each creation gets a non-wrapping internal token | `src/application.rs`; creation-token tests |
| API-02 | API blocker | Resolved: no provider/protocol types or reply methods on root handles, including return signatures; experimental gadget rejected before opening | Root exports; doctests; controller/output tests |
| API-03 | API blocker | Resolved for current models: component collection with roles, exposure and observed/requested identity; no implicit single-component public contract | `ControllerAssociation::components`; component metadata tests |
| API-04 | API blocker | Resolved: accepted native/face/D-pad button state is readable; DS4/Switch native face labels preserve existing spatial encoding | Curated state getters; native-label encoding regression |
| API-05 | API blocker | Resolved: compile-fail fixtures are included in root doctests, paired with positive compilation; obsolete stderr snapshots replaced | `tests/ui`; root doctests |
| COR-01 | Correctness | Guarded: required requests cannot become application callbacks; unexpected ownership leaks close in the same service cycle | Four public-handle fake-provider lifecycle tests |
| COR-02 | Correctness | Guarded: rejected delivery retains accepted state; independent removal and cleanup remain terminal/idempotent with retained errors | Root consumer tests and existing runtime/provider regressions |
| REF-01 | Refinement | Open: user-directed controller/demo refinements; complete only after actual hands-on feedback | Checkpoint below |
| EVD-01 | Deferred evidence | Full Gate T physical contrasting exposure/remap-only evidence remains blocked; retain unknowns and do not admit independent controls from macro documentation | EXP-0021 / ADR-0015 |
| EVD-02 | Deferred evidence | DS4 isolated touch behavior, Switch compressed-rumble fidelity, consumer/backend comparisons require scoped evidence before support promotion | Existing gate ledger and controller support matrix |
| EVD-03 | Deferred host validation | Uinput and gadget live validation require prepared infrastructure; absence does not count as a pass | Host preflight and completion evidence |
| REL-01 | Release blocker | Pending hands-on checkpoint, final API pass and final quality pass; no alpha tag/version claim until these close | Sequence above |

## Bounded Gate T decision

The maintainer selected bounded acceptance: alpha requires tested separation of
native controls, selected realization exposure, and consumer mapping. It does not
wait indefinitely for unavailable hardware observations. Full evidence Gate T
remains blocked in the architecture ledger; this is not relabeled as a pass.
Unknown raw independence remains unknown. Source evidence or a synthetic test
cannot promote a physical-fidelity support cell.

Existing `TargetRestriction` carries unavailable features with technical reasons;
component metadata now exposes the applicable surface per host component. Native
state remains independently typed. No universal mutable controller model or
routing capability vocabulary was added.

## Structural benchmark conclusions

| Benchmark | Reviewed evidence | Conclusion and revisit condition |
| --- | --- | --- |
| Exhaustive external consumer | Root handle tests and `examples/controller_loop.rs` | Multiple families and same-family sessions can use lifecycle, native state and typed output without transport knowledge. Capture, mappings, slot identity and feedback routing stay in the caller. |
| Wii resources | EXP-0019 and seven persistent-resource regressions | Controller-owned values cover import, host writes, dirty state, retry, post-close snapshot, export failure and discard. No shared memory/filesystem abstraction. Revisit if a real resource requires cross-realization continuity or large-copy semantics. |
| Wii internal topology | EXP-0020 and six topology regressions | Existing protocol edits cover live attach/detach, initialization, calibration reads, bounded edges and rollback with queued replies preserved. Internal expansion does not automatically become a host component. Revisit if evidenced topology ordering cannot preserve queued work. |
| Steam/Puck structural hypothesis | Prior research reviewed against current synchronous protocol/deadline and compound-runtime tests, including synthetic accessory/display lifecycle | Initialization, mode state, timing, unusual typed controls, output and multiple required host components have existing owners. No demonstrated missing shared primitive. Exact production wire/receiver topology remains unknown; no new personality or identity claims. |
| Wheel/HOTAS thought experiment | Controller-owned typed state, manifests, evdev presentation and compound ownership | Unequal axes, pedals and button sets need controller packages rather than a universal gamepad state. Rich force effects are not promised by conventional-rumble support; a concrete wheel's force semantics would require evidence and controller-specific output work. |

The old Steam/Puck review included persistent slot policy and suggestions to
acknowledge mode commands without full state modeling. Neither is adopted here:
slots remain external, and required controller semantics must be evidence-backed.
These are bounded structural conclusions, not new physical protocol verification.
Q/R/S/U were not reimplemented or promoted beyond their existing scoped results.

## Hands-on checkpoint

Run `cargo run -p virtualgamepad-demo` on the prepared desktop. Select USB/UHID
when uinput is not prepared. Review all four existing families, same-family
duplicates, lists beyond the viewport, removal from different positions, neutral
input/recreation, visible host feedback and retained cleanup diagnostics. Record
consumer/backend versions when assessing mappings. DS4 touch injection requires
an isolated test environment; omit it on an ordinary desktop.

Report confusing behavior or desired refinements. The next API review must start
from that feedback and the actual updated public inventory, not merely approve
this preliminary candidate. Final quality review and release remain subsequent
work packages.

## Preliminary validation record

- Workspace formatting, all-target/all-feature check and strict Clippy pass.
- Full workspace suite: 280 passed, 32 intentionally ignored hardware cases;
  no failures at this checkpoint.
- Python tooling suite: 43 passed.
- Strict root rustdoc build passes with default features disabled.
- Corpus-free source snapshot builds as a standalone application dependency with
  default features disabled, then rebuilds offline with cached dependencies.
  The ordinary consumer graph contains neither test-support nor the GUI.
- Live demo two-DualSense lifecycle, separate three-DS4 independence, and root
  Sony identity-restoration tests pass on prepared UHID. Uinput/gadget/physical-reference tests remain deferred.
- Gitleaks full-history scan and whitespace checks pass.

The snapshot consumer build is not the final exact-tag Git-dependency validation;
that remains REL-01 work after the interactive and final review gates.
