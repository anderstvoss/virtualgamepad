# Rust Implementation Plan

This document defines the **sequencing and validation strategy** for the Rust buildout of `virtualgamepad`. It complements:

- [RUST_IMPLEMENTATION_SPEC.md](RUST_IMPLEMENTATION_SPEC.md) — authoritative for crate boundaries, runtime types, and contracts
- [TESTING_TOOLING_SPEC.md](TESTING_TOOLING_SPEC.md) — authoritative for the test stack (fixtures, fakes, snapshots, CLIs)
- [ARCHITECTURE_SPEC.md](../specs/ARCHITECTURE_SPEC.md) — architectural ground truth

If this plan disagrees with the implementation spec on **shape** (type names, crate ownership, public API), the spec wins and this plan is updated. This plan is authoritative for **sequencing** (what to build when, what proves it works, what blocks the next phase).

## How this plan works

### Phases and gates

The buildout is a sequence of **phases**. Between every pair of phases sits a **gate** — a checklist the user steps through before phase N+1 is allowed to start. Gates are intentionally manual at the end so that automated green doesn't disguise a regression in something only a human will catch (UX of CLI output, naming, ergonomics, perceived quality).

A gate has two halves:

1. **Automated portion** — runs in CI and locally via `vgpd-demo phase-gate <N>`. Must be 100% green before manual review starts.
2. **Manual portion** — a numbered checklist of commands the user runs and observations the user records. The user signs off with an empty commit `chore(phase-gate): Phase N gate passed`.

The gate commit is documentary. Tooling does not enforce it. Reviewers of subsequent PRs check for it.

### Within-phase iteration

Inside a phase, work follows an explicit loop. Agents may drive any step.

1. **Design pass** — sketch the new types / traits / data flow; update the implementation spec only if a real ambiguity is found
2. **Contract tests** — write failing tests that pin the new contract (snapshot, property, fixture-driven)
3. **Implementation** — make the tests pass with the minimum code that satisfies them
4. **Demo wiring** — add or extend `vgpd-demo` subcommands so the new functionality is observable by a human
5. **Refactor** — simplify with the test suite as a ratchet
6. **Gate-prep** — author the manual gate items, validate the automated portion, write the gate notes

Steps 2–4 are short and tight; steps 1 and 5 should be cheap once the test stack is solid. Step 6 happens once near the end of the phase.

### Testing tooling is liberal

[TESTING_TOOLING_SPEC.md](TESTING_TOOLING_SPEC.md) describes the test stack — `gr-testkit`, fixtures, `gr-cli`, `vgpd-demo`, snapshot and property testing, backend trace record/replay. This plan treats it as already designed and references the relevant sections per phase.

The user can author **custom test cases as YAML fixtures** starting in Phase 0; that workflow is exercised at every gate from Phase 1 onward.

### Manual gate scope by phase tier

- **Foundation phases (0–3)**: gates exercise authoring workflows and library output. The user reads dumps, validates samples, reviews snapshots.
- **Runtime phases (4–7)**: gates exercise fake-backend sessions end-to-end. The user runs scenarios and observes input/reverse flow.
- **Provider phases (8–11)**: gates exercise real Linux devices. The user plugs the virtual device into host software (jstest, SDL games, Steam, etc.) and confirms recognition + functional behavior.
- **Platform phases (12)**: gates exercise planner-only — Windows/macOS providers begin as inventory stubs and require no real host until later work.

## Authority and drift rule

If this plan and the implementation spec disagree on type shape, crate ownership, or contract surface, **the spec is authoritative**. Update this plan, not the spec.

If this plan and the demo's growth doc disagree on what `vgpd-demo` supports at a given phase, **this plan is authoritative**.

## Architectural alignment

The Rust buildout must hold these architectural decisions across every phase. Repeated here so they appear in plan-time reviews, not only in spec-time review.

- planning is runtime negotiation against backend inventory, not static profile lookup
- planning includes host-platform and provider negotiation, not just backend family
- backends are created per session and never shared as mutable singletons
- HID and transport translators are profile-family-specific where required
- reverse-path handling is core, not a later bolt-on
- the primary public API accepts profile-typed input, not a unified control model
- structured errors and telemetry are part of the public surface
- attached-function modeling is deferred from v1 (see [Decision 7](RUST_IMPLEMENTATION_SPEC.md#decision-7-deferred-attached-function-modeling))
- audio splits into discrete `OutputCommand::Audio` events and a separate `AudioStreamSink` for PCM (see [Audio stream contract](RUST_IMPLEMENTATION_SPEC.md#audio-stream-contract))
- backend sessions are sync + non-blocking; see [Backend blocking contract](RUST_IMPLEMENTATION_SPEC.md#backend-blocking-contract)

## Workspace layout

```text
virtualgamepad/
  Cargo.toml             # root crate + workspace
  src/                   # placeholder library crate (kept until Phase 0 splits crates)
  demo/                  # vgpd-demo (already shipping; grows phase by phase)
  crates/
    gr-core/
    gr-profiles/
    gr-config/
    gr-session-options/
    gr-runtime-model/
    gr-backend-api/
    gr-planner/
    gr-translators/
    gr-session/
    gr-host-bridge/
    gr-provider-linux-uinput/
    gr-provider-linux-uhid/
    gr-provider-linux-transport/
    gr-provider-windows-hid/
    gr-provider-macos-hid/
    gr-testkit/
    gr-cli/
```

Phase 0 establishes `crates/` and moves the existing root crate into `crates/gr-core/` (or the first crate it becomes). The demo stays at the workspace root under `demo/`.

## Dependency direction rules

Unchanged from the implementation spec:

- `gr-core` depends on no internal crates
- `gr-profiles` depends on `gr-core`
- `gr-config` depends on `gr-core`
- `gr-session-options` depends on `gr-core`, `gr-config`, `gr-profiles`
- `gr-runtime-model` depends on `gr-core`, `gr-session-options`
- `gr-backend-api` depends on `gr-core`, `gr-runtime-model`
- `gr-planner` depends on `gr-core`, `gr-profiles`, `gr-session-options`, `gr-runtime-model`, `gr-backend-api`
- `gr-translators` depends on `gr-core`, `gr-profiles`, `gr-runtime-model`, `gr-backend-api`
- `gr-session` depends on `gr-core`, `gr-runtime-model`, `gr-backend-api`, `gr-planner`, `gr-translators`
- `gr-host-bridge` depends on `gr-runtime-model`, `gr-session`
- concrete backend crates depend on `gr-core`, `gr-runtime-model`, `gr-backend-api`
- `gr-testkit` may depend on all runtime crates
- `gr-cli` depends on public crates only

## Phase index

| Phase | Theme | Primary crates touched |
| --- | --- | --- |
| 0 | Workspace split + testing foundation | (all stub) + `gr-testkit` + `gr-cli` + `demo` |
| 1 | Core domain model | `gr-core` |
| 2 | Profiles + capability registry | `gr-profiles` |
| 3 | Configuration + session options + runtime model | `gr-config`, `gr-session-options`, `gr-runtime-model` |
| 4 | Backend API + fake providers | `gr-backend-api`, `gr-testkit` fakes |
| 5 | Planner | `gr-planner` |
| 6 | Translators (forward + reverse) | `gr-translators` |
| 7 | Session engine + host bridge | `gr-session`, `gr-host-bridge` |
| 8 | Linux `uinput` provider (compatibility tier) | `gr-provider-linux-uinput` |
| 9 | Linux `UHID` provider (identity-aware tier) | `gr-provider-linux-uhid` |
| 10 | Linux transport foundation | `gr-provider-linux-transport` |
| 11 | First hardware-faithful target | `gr-provider-linux-transport` + translator additions |
| 12 | Windows + macOS provider foundations | `gr-provider-windows-hid`, `gr-provider-macos-hid` |

## Phase 0: Workspace split + testing foundation

### Goal

Move from the current single-crate scaffold to the full workspace layout. Stand up the testing tooling at zero cost so every later phase can rely on it. End with `vgpd-demo` capable of running a phase gate.

### Entry criteria

- workspace exists (`Cargo.toml` with `[workspace]`, demo as a member) — already true after [PR #34](https://github.com/anderstvoss/virtualgamepad/pull/34)
- pre-commit / pre-push hooks pass on `main`
- `vgpd-demo info` runs

### Deliverables

- Cargo workspace expanded with empty crates at the paths in the workspace layout above (each crate compiles, has a placeholder `lib.rs`, and is wired into the workspace `[workspace] members`)
- `gr-testkit` skeleton with the public module structure from [TESTING_TOOLING_SPEC.md gr-testkit module layout](TESTING_TOOLING_SPEC.md#module-layout); empty types, fixture loader parses the envelope only
- `gr-cli` skeleton with `gr-cli validate-fixture` operational; the automated portion of each phase gate is exposed as a library entry point (`gr_cli::run_phase_gate_auto`) consumed by the demo (no-op gates return success)
- `vgpd-demo phase-gate <N>` subcommand that runs the automated checks for Phase N via `gr_cli::run_phase_gate_auto` and prints the manual checklist (the checklist content is read from this plan file by section anchor)
- workspace dev-dependencies added: `insta`, `proptest`, `assert_matches`, `rstest`, `serde_yaml`

### Iteration loop

This phase is mostly plumbing. The loop runs once for the workspace split, then once for each tool stub.

- design pass: confirm crate names, module layouts, binary names against the implementation spec
- contract tests: each new crate ships with at least one trivial `#[test] fn smoke()` so empty crates still register in the workspace test suite
- implementation: minimum code per stub
- demo wiring: `vgpd-demo phase-gate` reads the gate from this file
- refactor: not applicable
- gate-prep: validate that the gate runner displays Phase 0's gate correctly

### Testing tooling additions

- workspace dev-deps in `Cargo.toml`
- `gr-testkit` skeleton + envelope-only fixture loader
- `gr-cli validate-fixture` accepting the envelope
- `vgpd-demo phase-gate <N>` driver

### Exit gate

Run `vgpd-demo phase-gate 0` and complete the checklist.

Automated portion:

- [ ] `cargo build --workspace --all-features` succeeds
- [ ] `cargo test --workspace --all-features` passes (every empty crate's smoke test runs)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `vgpd-demo phase-gate 0` exits 0

Manual portion:

- [ ] 1. `vgpd-demo phase-gate 0` prints Phase 0's checklist as defined here (proves the gate-runner can read this file)
- [ ] 2. `cargo metadata --format-version 1 | jq '.workspace_members[]'` lists every crate from the workspace layout (proves no crate is silently missing)
- [ ] 3. `gr-cli validate-fixture docs/spec/implementation/fixtures/envelope-only.yaml` accepts a minimal v1 envelope (you author this sample as part of this phase)
- [ ] 4. `gr-cli validate-fixture docs/spec/implementation/fixtures/envelope-bad-version.yaml` rejects with a clear error mentioning the version field

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 0 gate passed"`

## Phase 1: Core domain model (`gr-core`)

### Goal

Establish the primitive types every other crate depends on. No backend code, no Linux-specific code.

### Entry criteria

- Phase 0 gate signed off
- `gr-core` skeleton crate exists with smoke test

### Deliverables

- `ProfileId`, `SessionId`, `BackendId`, `VendorId`, `ProductId`, `SequenceId` newtypes
- `Timestamp` type
- fidelity-tier enum (`Compatibility`, `IdentityAware`, `HardwareFaithful`) with human-name parsing
- backend-level enum (`Evdev`, `Hid`, `Transport`)
- backend-family enum (`LinuxUinput`, `LinuxUhid`, `LinuxTransportUsb`, `LinuxTransportBluetooth`, `WindowsHid`, `MacosHid`)
- semantic input/output function enums
- capability-category enums
- shared error categories (`thiserror`-backed)
- `ProfileInputPayload` enum with `#[non_exhaustive]` and one variant per built-in profile (payload structs stubbed) per [`ProfileInputPayload` section](RUST_IMPLEMENTATION_SPEC.md#profileinputpayload)
- `ProfileInputFrame` and `ProfileInputDelta` types

### Iteration loop

- design pass: review enum shapes against the implementation spec; resolve any gaps via spec edits, not ad-hoc additions
- contract tests:
  - `proptest` strategies for every enum + newtype
  - serde round-trip property tests (YAML and JSON)
  - parse-from-human-name tests for fidelity tiers
- implementation: derive `Serialize`, `Deserialize`, `Clone`, `Debug`, `Eq`, `Hash` where appropriate; forbid unsafe at crate root
- demo wiring: `vgpd-demo show-types` prints every enum's variants
- refactor: collapse any duplication between variant payloads
- gate-prep: prepare a fixture sample exercising each `ProfileInputPayload` variant

### Testing tooling additions

- `gr_testkit::proptest_strategies` for every `gr-core` type
- `gr_testkit::builders::*_input()` builders begin to exist (stub payloads accepted)
- snapshot tests for the canonical YAML representations of each enum

### Exit gate

Run `vgpd-demo phase-gate 1` and complete the checklist.

Step-by-step reviewer guide:

- [Phase 1 Manual Gate](manual-gates/phase-1.md)

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] property tests run with `proptest` default budget without failures

Manual portion:

- [ ] 1. Run `cargo run -p virtual_gamepad_demo -- show-types`. Confirm the output order is stable and the names shown for fidelity tiers, backend levels, backend families, and capability categories are the exact spec-facing names you would want in docs, fixtures, and review output.
- [ ] 2. Run `cargo run -p gr-cli -- validate-fixture crates/gr-core/fixtures/payload-dualsense-neutral.yaml`. Confirm the fixture is accepted; confirm the demo's `profile_id`, the YAML's inner `profile` tag, and the reported `payload_type` are **all the literal word `dualsense`** with no hyphen. In the YAML, confirm booleans represent digital inputs (`buttons`, `dpad`, `touchpad.*.active`) while numbers represent analog inputs (`sticks`, `triggers`, `touchpad.*.(x|y)`).
- [ ] 3. Copy `crates/gr-core/fixtures/payload-dualsense-neutral.yaml` to `tests/fixtures/xbox360-neutral.yaml`, adjust it to an Xbox 360 neutral payload (same boolean/numeric split, using `lb`/`rb`/`ls`/`rs` and `lt`/`rt`), then run `cargo run -p gr-cli -- validate-fixture tests/fixtures/xbox360-neutral.yaml`. Confirm this succeeds without parser changes and that `cargo test -p gr-core workspace_xbox360_fixture_loads_as_profile_input_frame` passes against that exact file.
- [ ] 4. Author a sparse `ProfileInputDelta` fixture for DualSense under `tests/fixtures/` with only `dpad.left`, `triggers.l2`, and a single touch contact set (start from `tests/fixtures/dualsense-delta-sparse.yaml` if you want a working reference). Run `cargo run -p gr-cli -- validate-fixture <path>`; confirm acceptance. Then run `cargo test -p gr-core workspace_dualsense_sparse_delta_decodes_only_set_fields` and confirm only the named fields decode as `Some` — proves a "delta" really means sparse, not a relabelled full snapshot.
- [ ] 5. Open `crates/gr-core/src/snapshots/gr_core__tests__dualsense-neutral-payload.snap`. Confirm the payload clearly separates digital booleans from analog numeric sections and includes a `touchpad` block with `contact_1` and `contact_2`. Keep in mind that lower-level implementations may later realize the `dpad` block as hat axes even though the Phase 1 payload remains directional booleans.
- [ ] 6. Run `cargo insta test --check`, then review `crates/gr-core/src/snapshots/`. Confirm every variant of every Phase 1 enum (`FidelityTier`, `BackendLevel`, `BackendFamily`, `CapabilityCategory`) has a corresponding `*-<variant>.snap` file — cross-check `ls crates/gr-core/src/snapshots/ | grep -c <enum-prefix>` against each enum's `ALL.len()` (3, 3, 6, 9). Names should be canonical and payload variants visually distinguishable.
- [ ] 7. Run `cargo test -p gr-core`. Confirm the test list covers fidelity-tier parse/display behavior, serde round-trips for frames *and* deltas, fixture loading for both frames and deltas, sparse-delta absence-of-fields, and property tests — so the manual checks above are backed by executable tests rather than one-off demo output.

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 1 gate passed"`

## Phase 2: Profiles + capability registry (`gr-profiles`)

### Goal

Bring in the built-in profile set and the capability query API. The planner and translators in later phases consume this data; no backend code yet.

### Entry criteria

- Phase 1 gate signed off

### Deliverables

- `ControllerProfile` struct with fields per [the implementation spec](RUST_IMPLEMENTATION_SPEC.md#controllerprofile)
- built-in profiles: generic gamepad, Xbox 360, DualSense, Steam Controller (the four named in the prior plan; additional profiles can be added later, additively)
- `CapabilityRegistry` query API folded into `gr-profiles` (no separate crate; see [IMPLEMENTATION_FRAMEWORK.md — CapabilityRegistry](IMPLEMENTATION_FRAMEWORK.md#capabilityregistry) implementation note)
- per-profile input contracts (which fields are required / optional, value ranges)
- descriptor templates per supported fidelity level (placeholders for HID/transport; real bytes ship in later phases when the relevant providers are implemented)
- declared supported / required input and output functions per profile

### Iteration loop

- design pass: walk through each built-in profile against public sources (Linux kernel drivers, SDL gamecontroller mappings, public descriptors) — see [DEVICE_SPEC_VALIDATION_PLAN.md evidence ladder](../validation/DEVICE_SPEC_VALIDATION_PLAN.md#evidence-ladder)
- contract tests:
  - per-profile capability presence tests
  - capability-to-function consistency tests
  - duplicate-capability prevention tests
  - declared-but-unused capability tests
- implementation: encode profiles as static data; capability registry indexes by `ProfileId`
- demo wiring: `vgpd-demo list-profiles`, `vgpd-demo show-capabilities <profile>`
- refactor: factor common capability shapes (e.g. stick + trigger groupings) into reusable definitions
- gate-prep: produce one capability-coverage report per profile

### Testing tooling additions

- `gr-cli list-profiles`, `gr-cli show-capabilities`, `gr-cli capability-coverage`
- snapshot tests for capability dumps per profile
- fixture format extended with profile-tagged input frames per the four built-in profiles

### Exit gate

Step-by-step reviewer guide:

- [Phase 2 Manual Gate](manual-gates/phase-2.md)

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] `cargo run -p gr-cli -- capability-coverage` exits 0 (no declared-but-unsupported gaps)
- [ ] `vgpd-demo phase-gate 2` exits 0

Manual portion:

- [ ] 1. `vgpd-demo list-profiles` shows all four built-in profiles with stable display names
- [ ] 2. `vgpd-demo show-capabilities dualsense` lists touch surface, accelerometer, gyroscope, rumble, haptics, lighting, player indicators, trigger effects, and audio. In the stick entries, confirm the YAML explicitly shows the shared range applies to both axes rather than leaving that implicit. Cross-check against the [DualSense documentation in fidelity guide](../specs/FIDELITY_GUIDE.md#dualsense-profile_id-dualsense)
- [ ] 3. `vgpd-demo show-capabilities xbox360` lists analog sticks, triggers, d-pad, face buttons, shoulders, stick clicks, system buttons, rumble, lighting, and player indicators, with no DualSense-specific outputs such as trigger effects, audio, or haptics
- [ ] 4. Run `cargo test -p gr-profiles invalid_profiles_fail_with_field_specific_errors`. Confirm it reports 6 passing parametrized cases with the remainder filtered out, and that each case points to one specific missing field (`display_name`, `supported_fidelity`, `input_contract.required_fields`, `capabilities.input`, `identity.vendor_id`, `identity.product_id`)
- [ ] 5. Review `crates/gr-profiles/src/snapshots/` — capability dumps look correct to a human

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 2 gate passed"`

## Phase 3: Configuration + session options + runtime model (`gr-config`, `gr-session-options`, `gr-runtime-model`)

### Goal

Ship the three crates that together hold session intent: parse it, compile it, and carry the runtime types through the rest of the system. They build together because they are tightly coupled and reviewing them separately produces churn.

### Entry criteria

- Phase 2 gate signed off

### Deliverables

- `gr-config`: YAML schema (per [Configuration spec](../specs/CONFIGURATION_SPEC.md)), serde models, four-pass validator, structured `ConfigValidationReport`
- `gr-session-options`: compile `SessionConfig` into `CompiledSessionOptions`; provider hints, reverse delivery policy, backpressure policy
- `gr-runtime-model`: type definitions for `SessionRequest`, `SessionPlan` (skeleton — populated by the planner in Phase 5), `ControllerOutputCommand`, `PreparedTranslationContext`, status / diagnostics snapshots, `ReverseEventDeliveryPolicy`, `BackpressurePolicy`
- unknown-config-field policy implemented per [Configuration spec — unknown config fields](../specs/CONFIGURATION_SPEC.md#unknown-config-fields)

### Iteration loop

- design pass: review the YAML shape against the configuration spec; sanity-check unknown-field handling
- contract tests:
  - valid + invalid config fixtures (one each per validator pass)
  - serde round-trip property tests for `SessionConfig` and `CompiledSessionOptions`
  - delivery- and backpressure-policy compilation tests
  - unknown-field policy behavior tests
- implementation: serde derives + a hand-written validator for cross-field invariants
- demo wiring: `vgpd-demo validate-config <path>` (delegates to `gr-cli validate-config`), with friendly error formatting
- refactor: consolidate field-validation error variants
- gate-prep: assemble a starter sample-configs directory under `samples/`

### Testing tooling additions

- `gr-cli validate-config`, `vgpd-demo validate-config`
- fixture loader for `kind: input-frame` fully wired (uses Phase 1's payloads + Phase 2's contracts)
- snapshot tests for the compiled session-options shape per representative config

### Exit gate

Step-by-step reviewer guide:

- [Phase 3 Manual Gate](manual-gates/phase-3.md)

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] `cargo run -p gr-cli -- validate-config samples/configs/dualsense-identity.yaml` exits 0
- [ ] `vgpd-demo phase-gate 3` exits 0

Manual portion:

- [ ] 1. `vgpd-demo validate-config samples/configs/dualsense-identity.yaml` accepts and prints a structured summary
- [ ] 2. `vgpd-demo validate-config samples/configs/broken-mode.yaml` rejects with a clear, source-located error
- [ ] 3. Author a custom config that selects xbox360 at `compatibility`, references an unknown provider, and sets `validation.rejectUnsupportedProviderPreference: false`; verify the validator accepts it with a warning, not an error
- [ ] 4. Same custom config with `validation.rejectUnsupportedProviderPreference: true`; verify rejection
- [ ] 5. Author a custom config with an unknown top-level section; verify rejection
- [ ] 6. Author a custom config with an unknown key inside `session`; verify warning (default) and rejection when `validation.rejectUnknownConfigFields: true`

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 3 gate passed"`

## Phase 4: Backend API + fake providers (`gr-backend-api`, `gr-testkit` fakes)

### Goal

Lock down the trait shapes that providers must implement and ship a configurable fake. Until this phase ends, no provider can begin; once it does, the next four phases can be built almost entirely against the fake.

### Entry criteria

- Phase 3 gate signed off

### Deliverables

`gr-backend-api` trait + type vocabulary already shipped in the Phase 4 prep PR (#44): `BackendFactory`, `BackendSession`, `BackendReverseEventSink`, `BackendFrame` (+ `EvdevEvent`), `BackendReverseEvent` (+ `Kind`/`Target`/`Payload`), `BackendDiagnostics` (+ `BackendState`), `BackendOpenContext`, `BackendRealizationRequest`, `BackendSupportReport` (+ `SupportLevel`/`UnsupportedOutputFunction`), `BackendInventoryEntry`, `BackendError`, `EventReadiness` (with cfg-gated `ReadinessHandle`). Phase 4 itself implements behavior against those shapes:

- `gr-testkit::fakes`:
  - configurable `FakeBackendFactory` and `FakeBackendSession`
  - `FakeFailure` enum per [TESTING_TOOLING_SPEC.md failure injection](TESTING_TOOLING_SPEC.md#failure-injection)
  - `EventReadiness` flapping support
  - per-session capture of written frames
- backend trace recorder + replayer (records anything implementing `BackendSession`; replays from a `backend-trace` fixture)
- `kind: backend-trace` fixture loader in `gr-testkit::fixtures`
- demo + CLI wiring: `vgpd-demo simulate-session <scenario>` and `gr-cli simulate-session --record` / `replay-trace`

### Iteration loop

- design pass: validate the trait shapes against the existing implementation spec; if any shape needs to change, fix the spec, not the plan
- contract tests:
  - `BackendFactory::can_realize` returns sensible support reports for every combination of fidelity × profile family × inventory permutation we can express
  - `BackendSession::drain_reverse_events` accepts any `&mut dyn BackendReverseEventSink` (test with `Vec`, `SmallVec`, custom collector via the blanket `Extend<BackendReverseEvent>` impl)
  - non-blocking contract test: every fake method either returns immediately or returns `WouldBlock`
  - readiness round-trip via the cfg-gated handle on the build target
- implementation: traits + concrete fakes + recorder + replayer
- demo wiring: `vgpd-demo simulate-session <scenario>` running against a built-in fake (no planner yet — session is hand-constructed)
- refactor: simplify the fake builder; assertion helpers in `gr-testkit` start to bloom here
- gate-prep: author a `backend-trace` fixture by recording a fake session and replay it back

### Testing tooling additions

- `BackendFactory` / `BackendSession` traits (also part of the deliverable above)
- `gr-testkit::fakes` full surface
- recorder + replayer
- `kind: backend-trace` fixture loader

### Exit gate

Step-by-step reviewer guide:

- [Phase 4 Manual Gate](manual-gates/phase-4.md)

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] `cargo run -p virtual_gamepad_demo -- simulate-session crates/gr-testkit/fixtures/community/fake-session-rumble.yaml` exits 0
- [ ] `cargo run -p gr-cli -- replay-trace crates/gr-testkit/fixtures/community/fake-trace-rumble.yaml` exits 0
- [ ] property tests pass with default `proptest` budget
- [ ] `vgpd-demo phase-gate 4` exits 0

Manual portion:

- [ ] 1. `vgpd-demo simulate-session crates/gr-testkit/fixtures/community/fake-session-rumble.yaml` runs end-to-end; output shows input written, fake reverse rumble event delivered, command emitted
- [ ] 2. Inject `FakeFailure::SendWouldBlock` via a fixture; verify the session re-arms via readiness and recovers (visible in the demo's verbose output)
- [ ] 3. Record a fake session via `gr-cli simulate-session --record trace.yaml`; replay it via `gr-cli replay-trace trace.yaml`; outputs are identical
- [ ] 4. Author a custom `backend-trace` fixture interleaving a feature-report request and a malformed output report; replay it and verify the malformed event is logged but does not crash
- [ ] 5. Review `crates/gr-testkit/src/assertions/snapshots/` — assertion-helper failure messages are human-readable and stable (`assert_captured_frames`, `assert_trace_directions`, `assert_diagnostics_counters`)

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 4 gate passed"`

## Phase 5: Planner (`gr-planner`)

### Goal

Implement runtime negotiation: from a session request + compiled options + backend inventory to a `SessionPlan`, including degradation and rejection.

### Entry criteria

- Phase 4 gate signed off
- Built-in profiles in `gr-profiles` declare the capabilities the planner reasons over
- `gr-testkit` fakes can model arbitrary backend inventories

### Deliverables

- `gr-planner` with fidelity negotiation, backend-family selection, provider selection, degradation, unsupported-capability analysis, rejection reasons
- planner output: full `SessionPlan` per [the spec](RUST_IMPLEMENTATION_SPEC.md#sessionplan)
- planner accepts hints (provider preference, backend preference, host platform) without bypassing validation

### Iteration loop

- design pass: confirm planner inputs match the spec; identify any decision the planner needs to make that has no spec rule yet
- contract tests:
  - per-profile fidelity plan tests (Compatibility / IdentityAware / HardwareFaithful)
  - degradation tests (request HF, only Hid available → degrade to IA + record reasons)
  - rejection tests (`identity-aware` requested but provider lacks reverse output)
  - planner stability tests (same inputs → same plan; immutable after creation)
  - snapshot tests for representative plans
- implementation: a planner that's *readable* — each rule visible in a `PlannerRule` step or a clearly named function
- demo wiring: `vgpd-demo plan-session <profile> --goal <tier> --inventory <fixture>`
- refactor: extract reusable rule combinators
- gate-prep: ensure every documented degradation example from the [FIDELITY_GUIDE](../specs/FIDELITY_GUIDE.md#degradation-policy) has a planner test backing it

### Testing tooling additions

- `gr-cli plan-session`, `vgpd-demo plan-session`
- `kind: plan-snapshot` fixtures wired in
- `gr-testkit::builders::session_request` matures

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean (plan snapshots reviewed)
- [ ] `cargo run -p virtual_gamepad_demo -- plan-session dualsense --goal identity-aware --inventory samples/inventories/linux-uhid-only.yaml` exits 0
- [ ] `vgpd-demo phase-gate 5` exits 0

Manual portion:

- [ ] 1. `vgpd-demo plan-session dualsense --goal identity-aware --inventory samples/inventories/linux-uhid-only.yaml` produces an IA plan with `selected_backend_family: LinuxUhid` and no degradation
- [ ] 2. Same profile, `--goal hardware-faithful`, same inventory; produces a degraded plan whose `transport-not-realizable` reason names the requested transport level, the available lower-tier levels, and the concrete cause
- [ ] 3. Same profile, `--goal hardware-faithful`, an inventory with no providers; planner returns a structured rejection whose `no-backend-supports-profile` reason includes the requested backend level and the available backend list
- [ ] 4. `vgpd-demo plan-session xbox360 --goal compatibility --inventory samples/inventories/linux-uinput-only.yaml` produces a compatibility plan with `selected_backend_family: LinuxUinput`
- [ ] 5. Author a custom `plan-snapshot` fixture for an unusual edge case (e.g. Steam Controller at `identity-aware` with a fake that declares only LEDs); verify the snapshot test passes
- [ ] 6. Review `crates/gr-planner/snapshots/` — the YAML rationale strings read like a human wrote them, not a debug derive

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 5 gate passed"`

## Phase 6: Translators (`gr-translators`)

### Goal

Implement forward translators (profile input → backend frames) and reverse translators (backend reverse events → `ControllerOutputCommand`). Per the architectural rule, HID and transport translators are profile-family-specific; evdev translators may be shared where semantically valid.

### Entry criteria

- Phase 5 gate signed off

### Deliverables

Trait + registry + error types (`ForwardTranslator`, `ReverseTranslator`, `TranslatorRegistry`, `TranslationError`, `TranslationScratch`) already shipped in the Phase 6 prep PR. `PreparedTranslationContext.descriptor_template` already carries a live `&'static gr_profiles::DescriptorTemplate` reference. Phase 6 implements behavior against those shapes:

- forward translators:
  - `GenericEvdevTranslator`
  - `XboxStyleEvdevTranslator` (covers Xbox 360 + similar layouts)
  - `DualSenseEvdevTranslator` (only if identity-specific evdev shaping diverges materially — likely needed for trigger / touchpad)
  - `DualSenseUsbHidTranslator`
  - `SteamControllerHidTranslator`
- reverse translators:
  - `DualSenseHidReverseTranslator` (rumble, LEDs, trigger effects, mode commands, audio command discrete events)
  - `SteamControllerReverseTranslator` (LEDs, lighting commands per the family)
- `TranslatorRegistry` populated with `&'static dyn` references to the per-family implementations
- `prepared_translation_context(plan, registry)` body — currently `unimplemented!()` from prep
- real HID descriptor bytes for DualSense, Xbox 360, Steam Controller at `identity-aware` tier (Phase 2 shipped `EMPTY_DESCRIPTOR` placeholders; Phase 6 replaces them with bytes from public/community device specs)
- `gr-cli capability-coverage` translator-gap detection per the spec rules at [Translator contracts](RUST_IMPLEMENTATION_SPEC.md#capability-coverage-translator-gap-detection)
- descriptor compatibility contract: every HID profile has translator + descriptor template + reverse translator, all asserted consistent by the extended `capability-coverage`

### Iteration loop

- design pass: per profile family, walk the input contract and output capability list against translator coverage
- contract tests:
  - per-target input translation tests
  - descriptor / report compatibility tests (translator output never violates the descriptor)
  - reverse translation tests via canned reverse-event fixtures
  - reverse translator coverage property test (never emits a function the profile didn't declare)
- implementation: translators take a `PreparedTranslationContext` and a frame, return a backend frame; no per-frame allocation in the hot path
- demo wiring: `vgpd-demo replay-trace <path>` exercising forward + reverse translators
- refactor: factor common bit-fiddling helpers (e.g. signed 8-bit stick encoding)
- gate-prep: prepare per-profile-family golden traces for the gate

### Testing tooling additions

- `kind: reverse-event` fixtures fully wired
- backend trace replay drives translator tests directly
- per-translator capability coverage tested via `gr-cli capability-coverage`

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] property test: reverse translators never emit semantic outputs for undeclared capabilities — passes for every profile
- [ ] `cargo run -p gr-cli -- capability-coverage` exits 0
- [ ] `vgpd-demo phase-gate 6` exits 0

Manual portion:

- [ ] 1. `vgpd-demo replay-trace crates/gr-translators/fixtures/dualsense-buttons-roundtrip.yaml` shows every button mapped correctly between profile input and HID report bytes
- [ ] 2. `vgpd-demo replay-trace crates/gr-translators/fixtures/dualsense-rumble-from-host.yaml` decodes the host rumble request into an `OutputCommand::Rumble` with sensible payload
- [ ] 3. `vgpd-demo replay-trace crates/gr-translators/fixtures/xbox360-evdev-roundtrip.yaml` and `vgpd-demo replay-trace crates/gr-translators/fixtures/steam-controller-lighting.yaml` show the expected evdev/HID summaries and decoded output commands
- [ ] 4. Review the shipped Steam Controller lighting fixture output; the demo decodes it to `OutputCommand::Lighting`
- [ ] 5. Review snapshots — translator outputs are stable across runs

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 6 gate passed"`

## Phase 7: Session engine + host bridge (`gr-session`, `gr-host-bridge`)

### Goal

Glue planner + translators + backend together inside a session runtime that scales to many concurrent sessions. End-to-end forward and reverse flow works against the fake backend.

### Entry criteria

- Phase 6 gate signed off

### Deliverables

Type + error surface (`ManagerConfig`, `ManagerError`, `SessionError`, `SessionSendError`, `OutputSink`, `SessionOutputSubscription`, `AudioStreamSink`, `AudioStreamSource`, `AudioStreamError`, `DeliveryWorkerConfig`, `counter_keys`) already shipped in the Phase 7 prep PR. `FakeFailure::ProviderPanic` also added in prep. Phase 7 itself implements behavior against those shapes:

- `gr-session`:
  - `VirtualControllerManager::new` / `with_backends` / `create_session` / `close_session` bodies
  - `VirtualControllerSessionHandle::send_input` / `send_input_delta` / `subscribe_outputs` bodies
  - per-session input + reverse queues with the bounded / coalescing policies from the spec
  - session actor per active session; shared worker pool (tokio)
  - readiness-aware reverse event scheduling
  - per-session diagnostics counters populated using [`counter_keys`](RUST_IMPLEMENTATION_SPEC.md#counter-naming-convention)
- `gr-host-bridge`:
  - callback adapter (`CallbackSink` prep-shipped; Phase 7 wires it through the delivery worker)
  - bounded-channel adapter
  - stream/observable adapter
  - delivery worker decoupled from session actor (per [Reverse-event delivery threading](RUST_IMPLEMENTATION_SPEC.md#reverse-event-delivery-threading))
  - live `AudioStreamSink` / `AudioStreamSource` implementations for fake provider sessions that declare audio capability
- diagnostics: per-session and manager-wide telemetry snapshots; counters populated under the spec's canonical key set

### Iteration loop

- design pass: validate the actor + worker pool model against the high-session-count scaling claim
- contract tests:
  - session lifecycle state machine tests
  - queue coalescing tests (latest-state-wins, counter increments)
  - bounded reverse-event queue + drop policy tests
  - slow-consumer isolation tests (one slow callback never stalls another session)
  - re-entrancy: callbacks attempting to call back into the session handle must not deadlock (documented as undefined behavior, but tests confirm the typical patterns)
  - `session-scenario` fixtures drive end-to-end runs
- implementation: minimum runtime that satisfies the tests; prefer `tokio` per the spec recommendation
- demo wiring: `vgpd-demo simulate-session <scenario>` now spins up the full session engine; `vgpd-demo many-sessions <count>` exercises concurrent sessions against fake backends
- refactor: extract reusable session-state types
- gate-prep: assemble the multi-session stress scenario

### Testing tooling additions

- `kind: session-scenario` fully wired
- `vgpd-demo many-sessions` for scale demonstration
- `gr-cli simulate-session --concurrency <N>`
- diagnostics dumps as fixtures for snapshot tests

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] 100-session concurrent test passes on the Linux CI runner without exceeding the documented latency planning target
- [ ] `vgpd-demo phase-gate 7` exits 0

Manual portion:

- [ ] 1. `vgpd-demo simulate-session samples/scenarios/dualsense-coalesce.yaml` runs and shows coalesced frames in the diagnostics dump
- [ ] 2. `vgpd-demo many-sessions 32` spins up 32 concurrent fake sessions; diagnostics show no cross-session contention; one session can be killed without affecting others
- [ ] 3. `vgpd-demo simulate-session samples/scenarios/slow-consumer.yaml` keeps other sessions running while the slow callback backs up
- [ ] 4. Author a custom session-scenario fixture exercising a deliberate provider panic via `FakeFailure::ProviderPanic`; the manager isolates the failure and continues running other sessions
- [ ] 5. `vgpd-demo simulate-session samples/scenarios/dualsense-audio-mode.yaml` exercises the discrete audio command path; `audio_sink()` returns `None` for the fake (no PCM provider yet)

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 7 gate passed"`

## Phase 8: Linux `uinput` provider — compatibility tier (`gr-provider-linux-uinput`)

### Goal

First real Linux provider. Compatibility-tier emulation: host-visible Linux gamepad via `uinput`. Reverse path is EV_FF only (per the [uinput reverse-path note](../specs/ARCHITECTURE_SPEC.md#linux-uinput-provider)).

### Entry criteria

- Phase 7 gate signed off
- A developer host running Linux with permission to open `/dev/uinput` (most modern distros require either CAP_SYS_ADMIN or a udev rule)

### Deliverables

- `gr-provider-linux-uinput` crate (cfg-gated `target_os = "linux"`)
- `LinuxUinputBackendFactory` and `LinuxUinputBackendSession`
- evdev device creation, capability declaration from profile, button + axis + sync emission
- EV_FF effect upload receipt mapped to `OutputCommand::Rumble`
- unsafe code isolated to one module with documented invariants
- file descriptors wrapped in RAII

### Iteration loop

- design pass: validate `uinput`-specific data flow; map capability declarations to `UI_SET_*` ioctls
- contract tests:
  - against fake writer (no kernel): descriptor construction, ioctl sequencing, event batching
  - against real kernel (gated on Linux runner): device appears, capabilities query matches, events flow
- implementation: typed wrapper over `libc::ioctl` and `nix` where helpful; no `unsafe` outside the wrapper module
- demo wiring: `vgpd-demo run-uinput-smoke <profile>` creates a virtual pad and dumps its `/dev/input/event*` enumeration
- refactor: factor a small `LinuxKernelIoctl` shim that fakes can substitute for tests
- gate-prep: prepare a step-by-step manual checklist for plugging the device into common host software

Prep note:

- the `docs/phase-8-prep` branch lands the contract surface early: `LinuxUinputBackendFactory`, `LinuxUinputBackendSession`, the `LinuxKernelIoctl` boundary, `gr-cli run-uinput-smoke`, `vgpd-demo run-uinput-smoke`, and the first `support-report` skeleton
- real `/dev/uinput` I/O, capability ioctls, event writes, and EV_FF reads remain Phase 8 implementation work

### Testing tooling additions

- Tier B (privileged Linux) test runner per [HEADLESS_TEST_STRATEGY.md](../validation/HEADLESS_TEST_STRATEGY.md#tier-b-privileged-linux-runner)
- `gr-cli run-uinput-smoke` (records evidence into `support-report` output)
- `.github/workflows/provider-tier-b.yml` manual/nightly scaffold for privileged Linux provider validation

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] Linux-gated integration tests pass on the CI Linux matrix entry
- [ ] `vgpd-demo phase-gate 8` exits 0

Manual portion:

- [ ] 1. `vgpd-demo run-uinput-smoke generic-gamepad` creates a device; `evtest` (or `jstest`) finds it under `/dev/input/`
- [ ] 2. `evtest` shows the expected buttons and axes; press emitted events match
- [ ] 3. `vgpd-demo run-uinput-smoke xbox360` produces a device recognized as a controller by SDL (verify with `sdl2-test` or `jstest-gtk`)
- [ ] 4. Launch a native Linux SDL game or `jstest-gtk`, send inputs from `vgpd-demo` (use a scripted scenario fixture); inputs land in the game
- [ ] 5. Trigger an EV_FF rumble from `fftest` or a game; the session emits `OutputCommand::Rumble` (visible in demo verbose output)
- [ ] 6. Kill the demo; verify the device is removed cleanly (no zombie `event*` entries)

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 8 gate passed"`

## Phase 9: Linux `UHID` provider — identity-aware tier (`gr-provider-linux-uhid`)

### Goal

First identity-aware provider. Host software inspecting HID identity recognizes the virtual device. Reverse path covers output reports + feature reports + the full set of declared capability functions for one profile.

### Entry criteria

- Phase 8 gate signed off
- Real-hardware evidence available for at least one identity-aware target (descriptor + representative input + reverse reports per [DEVICE_SPEC_VALIDATION_PLAN.md](../validation/DEVICE_SPEC_VALIDATION_PLAN.md))

### Deliverables

- `gr-provider-linux-uhid` crate (cfg-gated)
- `LinuxUhidBackendFactory` and `LinuxUhidBackendSession`
- UHID device lifecycle, descriptor provisioning, HID input report write path, output and feature report receive paths
- one identity-aware target implemented end-to-end (recommend DualSense — most public evidence available)
- reverse translator integration produces normalized `OutputCommand`s for that target's declared output capabilities

### Iteration loop

- design pass: walk the chosen target's descriptor + report layout against captured fixtures
- contract tests:
  - against fake writer: descriptor validation, input report bytes, reverse report parsing
  - against real kernel (Linux runner): UHID device appears with the right identity metadata; hidraw can read descriptor and reports; output and feature reports reach the backend
- implementation: small unsafe surface for the UHID character device interface; one module
- demo wiring: `vgpd-demo run-uhid-smoke <profile>` brings up the device and prints what the host sees
- refactor: factor descriptor encoding helpers (most descriptors share grammar fragments)
- gate-prep: prepare a manual checklist driving the device against Steam / a real game

### Testing tooling additions

- `gr-cli run-uhid-smoke`, `gr-cli compare-real-device`
- Tier C (real-hardware) fixture replay against the chosen target's captured traces
- `support-report` output evolves to per-profile evidence per [HEADLESS_TEST_STRATEGY.md](../validation/HEADLESS_TEST_STRATEGY.md#support-evidence-report)

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] Linux-gated UHID integration tests pass
- [ ] `gr-cli compare-real-device` matches captured trace within documented tolerance
- [ ] `vgpd-demo phase-gate 9` exits 0

Manual portion:

- [ ] 1. `vgpd-demo run-uhid-smoke dualsense` brings up a HID device; `hidraw` enumeration shows DualSense vendor/product ids
- [ ] 2. `lsusb` (where the host expects USB) or `bluetoothctl` shows the expected device identity
- [ ] 3. SDL or `jstest-gtk` identifies the device as DualSense (correct gamepad mapping picked up automatically)
- [ ] 4. Launch a game that uses DualSense-specific features (e.g. one of the public Steam reference titles); confirm trigger-effect commands generate `OutputCommand::TriggerEffect`
- [ ] 5. Rumble from a game generates `OutputCommand::Rumble`
- [ ] 6. Steam (if installed) recognizes the controller in Steam Input
- [ ] 7. Author a custom session-scenario fixture exercising a Steam Input mode change; the reverse translator handles it
- [ ] 8. `support-report --profile dualsense` shows: descriptor evidence ✓, input reports ✓, output reports ✓, feature reports ✓, target software recognition ✓

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 9 gate passed"`

## Phase 10: Linux transport foundation (`gr-provider-linux-transport`)

### Goal

Build the transport-tier scaffolding: enumeration, control flow, packet state machines. No specific transport-faithful target yet; the goal is to prove the architecture admits transport providers without disturbing earlier layers.

### Entry criteria

- Phase 9 gate signed off

### Deliverables

- `gr-provider-linux-transport` crate (cfg-gated)
- generic transport backend factory and session traits sitting on top of `gr-backend-api`
- enumeration and protocol state-machine abstractions
- USB and Bluetooth packet models
- skeletal profile-family-specific transport translators (registered but not yet realizing real protocols)

### Iteration loop

- design pass: confirm transport contracts integrate cleanly with the existing planner + session engine (no upward leak of transport-specific types)
- contract tests:
  - transport state machine accepts captured enumeration traces (canned)
  - planner admissibility tests for transport-tier requests
  - reverse packet contract tests
- implementation: state machines + packet models without committing to any specific OS-level USB/BT gadget API (those land in Phase 11)
- demo wiring: `vgpd-demo plan-session --goal hardware-faithful` against a fake transport inventory plans successfully; `vgpd-demo replay-trace` on a transport-trace fixture exercises the state machine
- refactor: pull common bus-state representations into a shared type
- gate-prep: prepare a transport-trace fixture for the chosen Phase 11 target

### Testing tooling additions

- `kind: backend-trace` extended to USB + Bluetooth-shaped trace steps
- transport-state-machine snapshot tests
- `gr-cli replay-trace` handles transport traces

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] transport state machine round-trip tests pass on canned fixtures
- [ ] `vgpd-demo phase-gate 10` exits 0

Manual portion:

- [ ] 1. `vgpd-demo plan-session dualsense --goal hardware-faithful --inventory samples/inventories/linux-transport-stub.yaml` produces a plan with `selected_backend_family: LinuxTransportUsb` (or Bluetooth) and no realization yet
- [ ] 2. `vgpd-demo replay-trace crates/gr-provider-linux-transport/fixtures/dualsense-usb-enumeration.yaml` plays the captured enumeration steps through the state machine; final state matches the documented "ready" state
- [ ] 3. Author a custom transport-trace fixture omitting a mandatory startup step; replay reports the specific missing state transition
- [ ] 4. Confirm `gr-provider-linux-transport` is `cfg(target_os = "linux")`; `cargo check --target x86_64-pc-windows-msvc -p gr-planner` succeeds (planner stays portable)

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 10 gate passed"`

## Phase 11: First hardware-faithful target

### Goal

Land one real hardware-faithful profile end-to-end. Real enumeration, real packet handling, observed by a host that did not accept the lower-tier emulation. The target profile is the same one chosen in Phase 9 (recommend DualSense USB) for maximum reuse of evidence and fixtures.

### Entry criteria

- Phase 10 gate signed off
- Real-hardware traces available for the chosen target's connect, idle, active input, reverse command, disconnect (per [DEVICE_SPEC_VALIDATION_PLAN.md step 6](../validation/DEVICE_SPEC_VALIDATION_PLAN.md#step-6-capture-transport-behavior-only-when-required))

### Deliverables

- USB protocol state machine for the chosen target
- transport-level descriptor / control flow
- input + reverse packet handling
- timing-sensitive or handshake-sensitive logic where required
- end-to-end real-host validation evidence captured

### Iteration loop

- design pass: walk the real-device trace; identify mandatory state transitions, timing windows, handshake exchanges
- contract tests:
  - per-step trace replay against the state machine
  - timing-sensitive logic isolated and unit-tested
  - reverse packet handling per declared capability
- implementation: real protocol code; unsafe contained to one module; aggressive use of `#[track_caller]` on protocol-violating invariants
- demo wiring: `vgpd-demo run-transport-smoke <profile>` brings up the virtual device against the target's expected transport
- refactor: keep transport complexity inside the provider crate; nothing leaks upward
- gate-prep: produce a side-by-side trace comparison (real vs virtual)

### Testing tooling additions

- `gr-cli compare-real-device --layer transport`
- transport-trace recorder upgraded to capture timing intervals
- Tier C real-hardware comparison workflow per [HEADLESS_TEST_STRATEGY.md tier C](../validation/HEADLESS_TEST_STRATEGY.md#tier-c-real-hardware-capture-runner)

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo insta test --check` clean
- [ ] real-device comparison passes within documented tolerance
- [ ] `vgpd-demo phase-gate 11` exits 0

Manual portion:

- [ ] 1. `vgpd-demo run-transport-smoke dualsense` brings up the virtual device; the host enumerates it identically to a real DualSense (`lsusb -v` diff shows only allowed differences such as serial number)
- [ ] 2. A target host or game that rejected the UHID-tier emulation now accepts the transport-tier device
- [ ] 3. Reverse-path features (rumble, lighting, trigger effects) behave correctly under the real host
- [ ] 4. `support-report --profile dualsense --tier hardware-faithful` shows: transport enumeration ✓, control flow ✓, packet handling ✓, reverse packets ✓, real-host recognition ✓
- [ ] 5. Differences between real and virtual traces are documented (as `notes:` in the comparison report) and the user signs off that each difference is safe
- [ ] 6. Disconnect / reconnect cycle is clean (no orphan kernel resources)

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 11 gate passed"`

## Phase 12: Windows + macOS provider foundations

### Goal

Prove the Linux-first runtime can admit Windows and macOS providers without architectural rewrites. Both providers ship as inventory + diagnostics + deployment-requirement reporting only; no realization yet.

### Entry criteria

- Phase 11 gate signed off

### Deliverables

- `gr-provider-windows-hid` (cfg-gated `target_os = "windows"`): inventory entry, deployment-requirement modeling, `BackendSupportReport`s
- `gr-provider-macos-hid` (cfg-gated `target_os = "macos"`): inventory entry, entitlement / system-extension prerequisite modeling, `BackendSupportReport`s
- planner accepts, degrades, or rejects Windows / macOS requests explicitly
- documentation of what each platform's full implementation will require (linked from each provider crate's README)

### Iteration loop

- design pass: confirm `HostPlatform::Windows` and `HostPlatform::Macos` planner inventory entries integrate cleanly
- contract tests:
  - provider selection: a Windows-only inventory selects the Windows provider
  - degradation: a profile requesting transport-tier on macOS receives a degraded plan with reasons
  - deployment requirements surface in the plan
- implementation: skeletal provider crates with `can_realize` returning structured "not yet realized" reports
- demo wiring: `vgpd-demo plan-session ... --host-platform windows` and `--host-platform macos` succeed against synthetic inventories
- refactor: nothing platform-specific in core crates (verified by `cargo check` cross-builds)
- gate-prep: cross-build all core crates against Windows and macOS targets

### Testing tooling additions

- cross-build CI matrix entries verify the core crates and provider stubs compile on all three targets
- planner-inventory fixtures expanded with Windows and macOS shapes

### Exit gate

Automated portion:

- [ ] `cargo test --workspace --all-features` clean (Linux runner)
- [ ] cross-build: `cargo check --target x86_64-pc-windows-msvc --workspace --features provider-windows-hid` clean
- [ ] cross-build: `cargo check --target x86_64-apple-darwin --workspace --features provider-macos-hid` clean
- [ ] `cargo insta test --check` clean
- [ ] `vgpd-demo phase-gate 12` exits 0

Manual portion:

- [ ] 1. `vgpd-demo plan-session dualsense --goal identity-aware --host-platform windows --inventory samples/inventories/windows-hid-stub.yaml` plans the Windows provider with deployment-requirement annotations
- [ ] 2. Same against `--host-platform macos`; the plan surfaces entitlement / system-extension prerequisites
- [ ] 3. `vgpd-demo plan-session dualsense --goal hardware-faithful --host-platform macos --inventory samples/inventories/macos-hid-stub.yaml` degrades or rejects with explicit reasoning
- [ ] 4. Confirm no core crate references `windows`, `winapi`, `core-foundation`, etc. (provider details stay in their crates): `rg 'extern crate (windows|winapi|core_foundation|objc)' crates/{gr-core,gr-profiles,gr-config,gr-session-options,gr-runtime-model,gr-backend-api,gr-planner,gr-translators,gr-session,gr-host-bridge}` returns nothing
- [ ] 5. Each provider crate's README documents the realization roadmap

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase 12 gate passed"`

## After Phase 12

The library is "functional" by the project's working definition: architecture is ready for full device-emulation buildout, even though only one hardware-faithful target is implemented. The demo program graduates to its GUI / controller-visualizer phase per [demo/README.md](../../../demo/README.md).

Subsequent work — adding profiles, adding hardware-faithful targets, implementing the Windows and macOS providers in full — follows the same phase / gate model but is scheduled as separate v1.x or v2 efforts.

## Risk areas

### Backend complexity leaks into core types

Mitigation:

- keep `gr-backend-api` narrow
- forbid platform-specific dependencies in `gr-core`, `gr-profiles`, `gr-config`, `gr-session-options`, `gr-planner`
- gate 12's manual check #4 is a fast verifier

### Profiles become deployment policy

Mitigation:

- backend selection lives in `gr-planner`, never in `gr-profiles`
- planner tests against varying inventories
- snapshot tests catch silent shape drift

### HID abstraction becomes too generic

Mitigation:

- profile-family-specific translators required at gate 6
- descriptor / report compatibility tests
- no universal HID packet model

### Reverse path slips behind forward path

Mitigation:

- reverse translator interfaces required before any `identity-aware` claim
- reverse-path integration tests required before gate 9 passes
- `gr-cli capability-coverage` non-zero on missing reverse coverage

### Premature async complexity

Mitigation:

- core, profile, config, session-options, planner crates stay runtime-agnostic
- async lives in `gr-session` and provider crates only

### Per-session execution model does not scale

Mitigation:

- session isolation logical (one task per session), not thread-per-device
- shared workers / async scheduling
- bounded queues + latest-state coalescing
- the 100-session concurrent test at gate 7 is the canary

### Manual gates become rubber stamps

Mitigation:

- automated portion of each gate must remain meaningful (CI keeps `vgpd-demo phase-gate` honest)
- manual checklists are scoped to things that *only* a human can verify: ergonomics, real-hardware behavior, output readability
- gate sign-off commit is part of the PR description for the next phase

## Final guidance

The most important single rule for the buildout:

**keep profile definition, session-option compilation, planning, translation, backend realization, and host bridging as separate layers, and let every concrete device instance belong to one explicit session.**

The testing tooling, gates, and within-phase loop exist to defend that separation while the codebase grows.
