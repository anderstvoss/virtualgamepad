# Post-106 core refinement assessment

The scoped implementation extends the landed architecture without a new production
controller, new dependency, new privilege, or corpus-pin promotion. Production Wii
Remote remains blocked on explicit later maintainer authorization. The synthetic
prototype is only compiled as an integration test.

## Review findings and decisions

- Q: native HID target and provider capability now preserve exact USB/Bluetooth
  selection. Bus mismatch/unknown IDs reject before preflight or creation. Current
  four controller manifests/descriptors remain USB-only and source aliases remain.
  Public `NativeHidRealization` literals gain a target field; preflight adds an
  invalid-request error. Bluetooth bus metadata is not a Bluetooth link.
- R: value-owned protocol memory passes import, dirty write, retry, clone rollback,
  terminal failure, snapshot/discard and exporter-failure cases. No generic memory
  contract or filesystem helper is needed. Atomic file persistence remains future.
- S: a small transactional protocol edit closes the demonstrated caller-topology
  gap. It preserves accepted reports/replies, rejects partial/invalid edits and
  schedules bounded controller-owned status work. No compound/provider semantics
  are changed. Both direct and host-driven bridge/memory cases pass.
- T: typed independent controls and restrictions are representable; macro-only
  documentation cannot admit independent raw state. Real mode/backend evidence
  remains incomplete. Full Gate T is blocked, not passed by synthetic examples.
- U: one v2 SDL probe, exact selection, actual close/reopen records and classified
  comparison pass deterministic tests. Required external observations can attach
  only to a matching capture hash and cannot overwrite existing values. Physical
  comparisons/backend evidence and corpus adoption remain outstanding; Gate U
  stays blocked. Exit 0 is not a claim of complete compatibility.

Review corrected lint findings in the typed-button fixture and C output formatting.
The final warning-free build and tooling suite pass. Follow-up regressions also
verify independent mixed-bus cleanup and successful host reads/activation in one
live topology session. Existing current-controller regressions are retained.

## Corpus delivery boundary

[Companion corpus PR #2](https://github.com/anderstvoss/controller-protocol-corpus/pull/2)
contains revision `e677a26`: three pinned/scoped sources, twelve claims and three
provenance tests. It validates 46 records and nine tests. Vendor excerpt hashing
is explicitly scoped; no whole-page digest or physical capture is invented.
Core remains pinned to `e65c044d062ed833ad13641f54c826db16d176e4`, verified against
published corpus main. Adopt the companion only after it satisfies that policy.
Neither PR is merged automatically.

## Completion Summary

Files changed:
- `crates/gr-controller-contract/tests/control_exposure.rs`
- `crates/gr-curated-controllers/src/common.rs`
- `crates/gr-curated-controllers/src/common/session.rs`
- `crates/gr-curated-controllers/src/dualsense.rs`
- `crates/gr-curated-controllers/src/dualshock4.rs`
- `crates/gr-curated-controllers/src/switch_pro.rs`
- `crates/gr-hid/src/lib.rs`
- `crates/gr-hid/tests/internal_topology.rs`
- `crates/gr-hid/tests/persistent_resource.rs`
- `crates/gr-hid/tests/support/mod.rs`
- `crates/gr-provider-linux-uhid/src/lib.rs`
- `crates/gr-realization-api/src/lib.rs`
- `docs/CONTROLLER_SUPPORT.md`
- `docs/CORE_ARCHITECTURE.md`
- `docs/SDL_DIFFERENTIAL.md`
- `docs/architecture-overhaul/AGENT_START_HERE.md`
- `docs/architecture-overhaul/ASSESSMENT.md`
- `docs/architecture-overhaul/GATE_STATUS.md`
- `docs/architecture-overhaul/LANDING_AND_ALPHA_PLAN.md`
- `docs/architecture-overhaul/POST_106_ASSESSMENT.md`
- `docs/architecture-overhaul/POST_106_PLAN.md`
- `docs/architecture-overhaul/decisions/ADR-0012-multiple-uhid-targets.md`
- `docs/architecture-overhaul/decisions/ADR-0013-persistent-resources.md`
- `docs/architecture-overhaul/decisions/ADR-0014-protocol-edits.md`
- `docs/architecture-overhaul/decisions/ADR-0015-control-exposure.md`
- `docs/architecture-overhaul/decisions/ADR-0016-sdl-observations.md`
- `docs/architecture-overhaul/experiments/EXP-0018-multi-uhid.md`
- `docs/architecture-overhaul/experiments/EXP-0019-persistent-resources.md`
- `docs/architecture-overhaul/experiments/EXP-0020-internal-topology.md`
- `docs/architecture-overhaul/experiments/EXP-0021-control-exposure.md`
- `docs/architecture-overhaul/experiments/EXP-0022-sdl-differential.md`
- `docs/spec/implementation/CONTROLLER_PACKAGE_ARCHITECTURE.md`
- `scripts/compare-sdl3-observations.py`
- `scripts/sdl3-gamepad-probe.c`
- `scripts/sdl3-probe-contract.h`
- `scripts/tests/fixtures/SDL3/SDL.h`
- `scripts/tests/test_sdl_comparison.py`
- `scripts/tests/test_sdl_contract.py`
- `scripts/tests/test_sdl_probe.py`

Behavior changed:
- Exact multi-UHID targets and transactional protocol configuration are available.
- SDL observations distinguish unavailable measurements and actual lifecycle;
  expected differences require exact values and evidence.
- Persistent resources and Wii-like topology are test-only experiments.

Tests added or modified:
- 19 Rust regressions/examples across realization, lifecycle, topology and exposure.
- Tooling suite expanded from 27 to 40 tests, including the actual C probe with fake
  SDL and hash-bound external observation evidence.
- Companion corpus adds three provenance regressions (six to nine tests).

Validation:
- `cargo fmt --all -- --check`: passed.
- `cargo check --workspace --all-targets --all-features`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo test --workspace --all-features`: passed, 258 tests; 30 hardware tests ignored.
- `gitleaks detect --redact`: passed.
- `python3 -m unittest discover -s scripts/tests`: passed, 40 tests.
- Pinned corpus validator/tests and `scripts/check-protocol-corpus.py --verify-remote`: passed.
- Real SDL 3.2.0 strict C compilation and failed-open v2 output contract: passed.
- Configured commit hooks: passed for implementation stages; delivery uses the
  configured push hooks and reports their outcome with the PR.

New dependencies:
- None.

Unresolved issues:
- Corpus companion adoption awaits publication on main; no merge is authorized.
- Gate T real backend/remap-only evidence and Gate U physical/backend/removal
  observations remain blocked. This is not complete hardware fidelity acceptance.
- Existing DS4 touch isolation, Switch rumble fidelity, Steam/Eden, broker, audio
  and actual Bluetooth limits remain. No host changes or physical tests performed.
