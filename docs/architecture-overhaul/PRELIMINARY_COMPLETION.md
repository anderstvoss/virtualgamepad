# Completion Summary — preliminary candidate

This completes the implemented API/stress batch and initial controller/demo
refinement, not the entire alpha release plan. Hands-on feedback, the final API
review, separate quality review and release remain pending.

## Files changed

- `Cargo.lock`: Reflect root internal dependency and removal of the demo broker dependency.
- `Cargo.toml`: Application experimental feature, internal HID translation dependency, dev-only fake-provider seam.
- `README.md`: Describe the current application API and active pre-alpha sequence.
- `crates/gr-controller-contract/src/lib.rs`: Evolution-harden commit/validation enums and migrate realization constants.
- `crates/gr-controller-runtime/src/compound.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-controller-runtime/src/lib.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-curated-controllers/Cargo.toml`: Dev-only fake-provider feature for consumer tests.
- `crates/gr-curated-controllers/src/common.rs`: Private injected-provider constructor for deterministic tests and canonical realization constants.
- `crates/gr-curated-controllers/src/common/session.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-curated-controllers/src/common/session_tests.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-curated-controllers/src/dualsense.rs`: Native button readback, typed face labels where missing, fake-provider seam, and canonical IDs.
- `crates/gr-curated-controllers/src/dualsense/host_probe.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-curated-controllers/src/dualshock4.rs`: Native button readback, typed face labels where missing, fake-provider seam, and canonical IDs.
- `crates/gr-curated-controllers/src/dualshock4/evdev.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-curated-controllers/src/switch_pro.rs`: Native button readback, typed face labels where missing, fake-provider seam, and canonical IDs.
- `crates/gr-curated-controllers/src/xbox360.rs`: Native button readback, typed face labels where missing, fake-provider seam, and canonical IDs.
- `crates/gr-curated-controllers/tests/dualsense_uhid_live.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-curated-controllers/tests/dualshock4_uhid_live.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-curated-controllers/tests/evdev_feedback_live.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-privileged-broker/src/lib.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-privileged-broker/src/main.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-provider-linux-dummy-hcd/src/lib.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-provider-linux-uhid/src/lib.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-provider-linux-uinput/src/lib.rs`: Mechanical migration from ambiguous realization aliases to exact constants; existing behavior retained.
- `crates/gr-realization-api/src/lib.rs`: Remove ambiguous associated realization aliases; canonical constants throughout.
- `demo/Cargo.toml`: Remove direct broker dependency from the ordinary application demo.
- `demo/src/gui.rs`: Use root APIs; separate lab correlation from session identity; mark gadget unavailable.
- `demo/src/gui/editor.rs`: Read-only application diagnostics and component association snapshots.
- `docs/APPLICATION_API.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/CONTROLLER_SUPPORT.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/CURRENT_CONTROLLER_REFINEMENT.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/DEPLOYMENT_AND_VALIDATION.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/AGENT_START_HERE.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/API_INVENTORY.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/GATE_STATUS.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/LANDING_AND_ALPHA_PLAN.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/POST_106_PLAN.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/PRELIMINARY_COMPLETION.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/PRE_ALPHA_STATUS.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `docs/architecture-overhaul/UHID_REBOOT_ACCESS_PLAN.md`: Record API contract, inventory, current evidence or superseding execution sequence.
- `examples/controller_loop.rs`: Standalone four-family application service owner using only root imports.
- `src/application.rs`: Opaque options, fresh creation tokens, application errors/readiness/diagnostics and component metadata.
- `src/controllers.rs`: Typed root handles, identity restoration/readback and lifecycle delegation.
- `src/controllers/tests.rs`: Fake-provider public-handle lifecycle, retry, feedback, cleanup and native-label regressions.
- `src/experimental.rs`: Opt-in former implementation/research surface.
- `src/lib.rs`: Intentional root exports and executable positive/negative API doctests.
- `src/output.rs`: Native output translation and fail-closed request-ownership guard.
- `tests/root_uhid_live.rs`: Opt-in root identity restoration and fresh-session acceptance.
- `tests/ui/dualsense_rejects_xbox_native_control.rs`: Correct native types and wire the preserved negative case into root doctests.
- `tests/ui/dualsense_rejects_xbox_native_control.stderr`: Removed obsolete unexecuted diagnostic snapshot; behavior now guarded by doctests.
- `tests/ui/xbox_has_no_touch_surface.rs`: Correct native types and wire the preserved negative case into root doctests.
- `tests/ui/xbox_has_no_touch_surface.stderr`: Removed obsolete unexecuted diagnostic snapshot; behavior now guarded by doctests.

## Behavior changed

- Normal callers select a realization through opaque options; internal session IDs
  are generated per creation and cannot wrap/reuse silently.
- Application handles expose typed state/output, native identity and component
  metadata, borrowed readiness, errors and retained diagnostics without raw reply
  methods or provider construction.
- Native controls are readable. DS4 and Switch native face labels preserve existing
  spatial wire encoding. Required request ownership remains internal.
- Experimental gadget creation is explicitly excluded from the ordinary API/demo;
  research implementations and prior regression coverage remain available.
- The active ledger records bounded Gate T acceptance and a Git-based alpha;
  no version, tag, registry publication or release workflow was changed.

## Tests added or modified

- 11 root unit tests cover creation-token exhaustion/reuse, component metadata,
  all-family consumer lifecycles, retries, exact feedback acknowledgements,
  independent removal, cleanup diagnostics, output preservation and native labels.
- Eight compile-fail and two positive doctests guard typed controls, opaque options,
  removed aliases, raw-reply boundaries and borrowed readiness lifetimes.
- One opt-in root UHID identity-restoration test was added and passed live.
- Demo regressions were migrated without dropping previous coverage. Removed
  stderr snapshots were previously unwired; their corrected Rust cases now run.

## Validation

- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets --all-features`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: passed, 280 tests; 32 opt-in cases ignored
  by the aggregate run. Three selected live cases separately passed below.
- `gitleaks detect --redact`: passed.
- `python3 -m unittest discover -s scripts/tests`: passed, 43 tests.
- Strict root rustdoc build with no default features: passed.
- Corpus-free standalone source-snapshot consumer and cached offline rebuild:
  passed; ordinary dependency graph excludes test-support and GUI dependencies.
- Live demo two-DualSense lifecycle/cleanup: passed.
- Live three-DS4 independence/cleanup: passed.
- Live root Sony identity restoration/fresh-session test: passed.
- Whitespace validation: passed.
- Demo launched successfully for maintainer feedback.

## New dependencies

- No new third-party crates.
- Root now directly references existing workspace `gr-hid` solely to translate
  private readiness/lifecycle types into the application API.
- Existing curated dependency enables `test-support` only as a dev dependency.
- Demo direct broker dependency removed.

## Unresolved issues

- Maintainer hands-on refinement checkpoint is pending; no final API freeze,
  final quality pass or alpha release is claimed.
- Full physical Gate T evidence, physical fidelity, DS4 isolated touch and Switch
  compressed-rumble fidelity remain scoped follow-ups.
- Uinput/gadget infrastructure was not prepared; corresponding live cases remain
  deferred. No host policy was changed.
- Exact-tag Git dependency validation belongs to release preparation; the current
  standalone test used a corpus-free snapshot of the working source.


## Completion Summary — pre-checkpoint general polish

The maintainer will create a separate task for interactive controller/demo
refinement. General polish ends at that checkpoint, following the required
execution sequence. This supplement records changes after the preliminary
implementation commit `e50b9ba`; the earlier validation record remains historical.

Files changed:
- `README.md`: replace obsolete session-ID reuse wording with lab correlation.
- `docs/CORE_ARCHITECTURE.md`: clarify the root API excludes `poll_output`.
- `docs/DEPLOYMENT_AND_VALIDATION.md`: instruct applications to call `service`.
- `docs/DEMO_LAB.md`: align lab labels, record v3, gadget availability and measured
  identity/lifecycle evidence with the implemented demo.
- `src/lib.rs`: include the application guide in crate documentation and doctests.
- `docs/architecture-overhaul/PRE_ALPHA_STATUS.md`: record separate-task ownership,
  resumption context, acceptance requirements and latest deterministic validation.
- `docs/architecture-overhaul/PRELIMINARY_COMPLETION.md`: this supplement.

Behavior changed:
- Documentation and handoff only; controller and GUI runtime behavior is unchanged.

Tests added or modified:
- The application guide's lifecycle example now compiles in the root doctests;
  root doctests contain three positive examples and eight compile-fail cases.

Validation:
- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets --all-features`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: passed, 281 tests; 32 opt-in tests ignored.
- `gitleaks detect --redact`: passed.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p virtualgamepad --no-deps --no-default-features`: passed.
- `git diff --check`: passed.
- Hardware tests and tooling tests were not repeated for this documentation-only
  supplement; the earlier results above remain the recorded evidence.

New dependencies:
- None.

Unresolved issues:
- Focused hands-on refinement remains pending in the maintainer-created task.
- Final API review, subsequent final quality pass and alpha release remain pending.
- Previously deferred physical and host evidence remains deferred.
