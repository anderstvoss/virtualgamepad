# Testing Tooling Specification

This document specifies the testing tooling that supports the [Rust implementation plan](RUST_IMPLEMENTATION_PLAN.md). The plan's phased buildout relies on a substantial, reusable test stack so that each phase can be validated automatically before its manual gate, and so users (developers and reviewers) can author **custom test cases** without writing Rust.

Tooling described here is part of the build target. It ships in the workspace alongside the runtime crates and is exercised in CI and at every phase gate.

Related documents:

- [RUST_IMPLEMENTATION_SPEC.md](RUST_IMPLEMENTATION_SPEC.md) — authoritative runtime contracts
- [RUST_IMPLEMENTATION_PLAN.md](RUST_IMPLEMENTATION_PLAN.md) — phased sequencing and gates that this tooling backs
- [TEST_PLAN.md](../validation/TEST_PLAN.md) — what gets tested and at what layer
- [HEADLESS_TEST_STRATEGY.md](../validation/HEADLESS_TEST_STRATEGY.md) — remote/headless validation tiers

## Goals

- make automated testing cheap so it grows with the codebase, not after
- make custom test case authoring possible for non-Rust contributors (reviewers, users, future docs)
- separate human-facing exploration tooling (`vgpd-demo`) from CI/automation tooling (`gr-cli`)
- keep the testing surface honest: a passing test suite must not be able to coexist with a missing support claim, a missing reverse path, or a silently degraded plan
- ship snapshot, property, fixture-replay, and trace-replay strategies as first-class — not as a stretch goal

## Non-goals

- replacing real-hardware validation; real devices remain the only source of authoritative descriptors and traces (see [DEVICE_SPEC_VALIDATION_PLAN.md](../validation/DEVICE_SPEC_VALIDATION_PLAN.md))
- a custom assertion framework — use stdlib + `assert_matches` + `insta` + `proptest` where they fit
- a heavyweight DSL or new fixture language — fixtures are YAML deserialized via `serde_yaml` into typed Rust structs

## Tool inventory

| Tool | Crate / location | Audience |
| --- | --- | --- |
| Fixture format and loaders | `gr-testkit` | All tests; custom test authors |
| Fake provider factories | `gr-testkit` | Crate integration tests |
| Assertion helpers | `gr-testkit` | All tests |
| Backend trace recorder / replayer | `gr-testkit` | Provider tests; gate runs |
| Snapshot tests | `insta` (workspace dev-dep), snapshots under each crate | All crates |
| Property tests | `proptest` (workspace dev-dep) | Domain crates with strong invariants |
| `gr-cli` | `gr-cli` crate (binary `gr-cli`) | CI, scripts, fixture validation |
| `vgpd-demo` | `demo/` crate (binary `vgpd-demo`) | Humans at phase gates and during exploration |
| Phase-gate runner | `vgpd-demo phase-gate <N>` subcommand backed by `gr-cli` | The user at each phase gate |
| Coverage report generator | `gr-cli capability-coverage` | CI artifact; phase gates |

## `gr-testkit` crate

`gr-testkit` is named in [RUST_IMPLEMENTATION_SPEC.md](RUST_IMPLEMENTATION_SPEC.md#gr-testkit). This section specifies its full surface area at v1.

### Module layout

```text
gr-testkit/
  src/
    lib.rs
    fixtures/
      mod.rs
      input_frame.rs
      reverse_event.rs
      plan_snapshot.rs
      backend_trace.rs
      session_scenario.rs
      schema.rs        // Common envelope + kind discriminator
    builders/
      mod.rs
      input.rs         // ProfileInputFrame builders, profile-typed
      reverse.rs       // BackendReverseEvent builders
      profile.rs       // ControllerProfile builders for ad-hoc test profiles
      plan.rs          // SessionRequest / planner-input builders
      config.rs        // SessionConfig builders
    fakes/
      mod.rs
      backend_factory.rs
      backend_session.rs
      failure.rs       // Failure-injection helpers
    assertions/
      mod.rs
      plan.rs          // assert_plan_matches, assert_degraded_to, ...
      session.rs       // assert_input_delivered, assert_reverse_received, ...
      capability.rs    // assert_capability_present, ...
    recorder/
      mod.rs
      capture.rs       // Wraps a real BackendSession to record traces
      replay.rs        // Plays a BackendTrace into a translator/session
    harness/
      mod.rs           // SessionHarness — drives end-to-end scenarios
    proptest_strategies/
      mod.rs           // Strategy<T> implementations for domain types
```

### Public API surface

The `gr-testkit` public API targets test ergonomics: short imports, fluent builders, no required boilerplate to spin up a session against a fake backend.

```rust
// Builders return concrete domain types and are profile-typed where it matters.
pub mod builders {
    pub fn dualsense_input() -> DualSenseInputBuilder;
    pub fn xbox360_input() -> Xbox360InputBuilder;
    pub fn steam_controller_input() -> SteamControllerInputBuilder;
    pub fn generic_gamepad_input() -> GenericGamepadInputBuilder;
    pub fn reverse_event() -> BackendReverseEventBuilder;
    pub fn session_request(profile: ProfileId) -> SessionRequestBuilder;
    pub fn session_config() -> SessionConfigBuilder;
    pub fn ad_hoc_profile(id: &str) -> ControllerProfileBuilder;
}

pub mod fakes {
    pub fn backend_factory() -> FakeBackendFactoryBuilder;
    pub fn failing_backend(kind: FakeFailure) -> Arc<dyn BackendFactory>;
}

pub mod assertions {
    // Plan assertions
    pub fn assert_plan_matches(plan: &SessionPlan, expected: PlanExpectation);
    pub fn assert_degraded_to(plan: &SessionPlan, tier: FidelityTier);
    pub fn assert_rejected(result: Result<SessionPlan, PlanningError>, reason: PlanRejectReason);

    // Session assertions
    pub fn assert_input_delivered(harness: &SessionHarness, frame: &ProfileInputFrame);
    pub fn assert_reverse_received(
        harness: &SessionHarness,
        expected: &ControllerOutputCommand,
    );
    pub fn assert_coalesced(harness: &SessionHarness, expected_coalesces: u32);

    // Capability assertions
    pub fn assert_capability_present(profile: &ControllerProfile, function: SemanticOutputFunction);
    pub fn assert_no_undeclared_outputs(profile: &ControllerProfile, translator: &dyn ReverseTranslator);
}

pub mod harness {
    pub struct SessionHarness { /* fake-backend-backed runtime */ }

    impl SessionHarness {
        pub fn new(request: SessionRequest) -> Result<Self, HarnessError>;
        pub fn with_fake(backend: Arc<dyn BackendFactory>) -> Self;
        pub fn send(&self, frame: ProfileInputFrame) -> Result<(), SessionSendError>;
        pub fn inject_reverse(&self, event: BackendReverseEvent);
        pub fn drain_commands(&self) -> Vec<ControllerOutputCommand>;
        pub fn diagnostics(&self) -> SessionDiagnosticsSnapshot;
        pub fn run_scenario(&self, scenario: SessionScenario) -> ScenarioOutcome;
    }
}

pub mod recorder {
    pub struct TraceRecorder<B: BackendSession> { /* ... */ }
    pub fn record<B: BackendSession>(inner: B) -> TraceRecorder<B>;
    pub fn replay(trace: BackendTrace) -> ReplayBackend;
}
```

### Fake backend factory

The fake backend supports:

- arbitrary `BackendSupportReport` shapes via builder configuration
- success and failure paths for `open`, `send`, `drain_reverse_events`, and `close`
- injected reverse events with timing control
- per-session capture of every `BackendFrame` written, accessible from assertions
- configurable `EventReadiness` returns to exercise the readiness-aware scheduler

```rust
let factory = fakes::backend_factory()
    .family(BackendFamily::LinuxUhid)
    .platform(HostPlatform::Linux)
    .declares_reverse_output(SemanticOutputFunction::Rumble)
    .declares_reverse_output(SemanticOutputFunction::Lighting)
    .declares_feature_reports(true)
    .open_succeeds()
    .send_succeeds()
    .reverse_events_from_iter(canned_reports.into_iter())
    .build();
```

### Failure injection

A small enum exhausts the failure modes the runtime must tolerate. Tests use it to assert isolation and recovery behavior.

```rust
pub enum FakeFailure {
    OpenRefused(BackendError),
    SendWouldBlock,                 // Returns BackendError::WouldBlock once, then succeeds
    SendPermanentlyFails(BackendError),
    DrainParseError,
    CloseFails,
    EventReadinessFlapping,         // Alternates Readable / NoReverseEvents
    ProviderPanic,                  // Simulates provider task panic; session must isolate
}
```

### Assertion helpers — example use

```rust
#[test]
fn dualsense_uhid_plan_degrades_when_transport_unavailable() {
    let factory = gr_testkit::fakes::backend_factory()
        .family(BackendFamily::LinuxUhid)
        .declares_reverse_output(SemanticOutputFunction::Rumble)
        .build();

    let request = gr_testkit::builders::session_request(ProfileId::DualSense)
        .goal(EmulationGoal::HardwareFaithful)
        .build();

    let plan = gr_planner::plan(&request, &[factory]).expect("plan");

    gr_testkit::assertions::assert_degraded_to(&plan, FidelityTier::IdentityAware);
    insta::assert_yaml_snapshot!("dualsense_uhid_degraded_plan", &plan);
}
```

## Fixture format

Fixtures are YAML documents with a shared envelope and a `kind` discriminator. They are parsed by `serde_yaml` into typed Rust enums under `gr_testkit::fixtures`. Round-trip is supported via `serde` derives.

### Common envelope

Every fixture document includes:

```yaml
fixture: virtualgamepad/v1            # Schema discriminator; bump on breaking change
kind: input-frame                     # One of the kinds below
id: dualsense-buttons-cross-down      # Stable id used as test name / snapshot key
profile_id: dualsense                 # Required when applicable
notes: |                              # Optional free-text rationale, displayed in failures
  Cross button held; all other inputs at neutral.
payload: { ... }                      # Kind-specific payload
```

### `kind: input-frame`

A single profile input frame.

```yaml
fixture: virtualgamepad/v1
kind: input-frame
id: dualsense-buttons-cross-down
profile_id: dualsense
payload:
  timestamp: 0
  sequence: 0
  profile: dualsense
  fields:
    buttons:
      face:
        cross: true
        circle: false
        triangle: false
        square: false
      shoulders:
        l1: false
        r1: false
      stick_clicks:
        l3: false
        r3: false
      system:
        create: false
        options: false
        ps: false
        touchpad_click: false
    dpad:
      up: false
      down: false
      left: false
      right: false
    sticks:
      left_x: 0
      left_y: 0
      right_x: 0
      right_y: 0
    triggers:
      l2: 0
      r2: 0
    touchpad:
      contact_1:
        active: false
        x: 0
        y: 0
      contact_2:
        active: false
        x: 0
        y: 0
```

Loaders validate the `fields` payload against the profile's published input contract. Unknown fields fail loading; out-of-range values fail loading.

### `kind: reverse-event`

A single backend reverse event.

```yaml
fixture: virtualgamepad/v1
kind: reverse-event
id: dualsense-rumble-low
profile_id: dualsense
payload:
  source: hid-output-report
  report_id: 0x05
  bytes: [0x05, 0xff, 0x00, 0x10, 0x10, ...]
```

Loaders carry raw `bytes` through unmodified so reverse translators can be exercised against exact wire data. Decoded fields are optional and used for assertion targets.

### `kind: plan-snapshot`

A golden plan output for snapshot regression.

```yaml
fixture: virtualgamepad/v1
kind: plan-snapshot
id: dualsense-hardware-faithful-uhid-degrades
profile_id: dualsense
payload:
  request:
    goal: HardwareFaithful
    backend_preference: Hid
  expected_plan:
    selected_level: Hid
    target_host_platform: Linux
    selected_backend_family: LinuxUhid
    degradation:
      from: HardwareFaithful
      to: IdentityAware
      reasons: [TransportNotRealizable]
    unsupported_capabilities: [TriggerEffect, Audio]
```

Plan snapshots are also used by `insta` for the canonical text snapshot — the YAML fixture stores the *expectation* in a human-author-friendly format that the test compares against the live plan.

### `kind: backend-trace`

A captured or hand-authored sequence of backend interactions.

```yaml
fixture: virtualgamepad/v1
kind: backend-trace
id: dualsense-usb-enumeration
profile_id: dualsense
notes: Captured from a real DualSense via hid-recorder on 2026-04-12, kernel 6.8.
payload:
  steps:
    - direction: outbound
      kind: hid-input-report
      report_id: 0x01
      bytes: [...]
    - direction: inbound
      kind: hid-output-report
      report_id: 0x05
      bytes: [...]
    - direction: inbound
      kind: hid-feature-report
      report_id: 0xa3
      bytes: [...]
```

Backend traces feed the replay path. The recorder writes them. Custom-authored traces let a user define a precise scenario for any backend session, including provider-failure interleavings.

### `kind: session-scenario`

A scripted scenario for the session engine.

```yaml
fixture: virtualgamepad/v1
kind: session-scenario
id: dualsense-coalesces-stale-input
profile_id: dualsense
payload:
  config:
    fidelity_tier: identity-aware
    backend_preference: hid
  steps:
    - send_input: { ref: dualsense-buttons-cross-down }
    - send_input: { ref: dualsense-buttons-circle-down }
    - send_input: { ref: dualsense-buttons-triangle-down }
    - assert_coalesced: { at_least: 1 }
    - assert_last_written: { ref: dualsense-buttons-triangle-down }
    - inject_reverse: { ref: dualsense-rumble-low }
    - assert_reverse_received:
        function: Rumble
        payload_matches: { left_intensity: 0xff }
```

Scenarios reference other fixtures by `id` (via `ref`), keeping individual fixture files small and reusable.

### Validation rules

- the loader rejects unknown top-level keys
- the loader rejects unknown payload kinds
- profile-shaped payloads are validated against the active `ProfileInputContract`
- fixture ids must be unique within a directory; loader scans report duplicates
- a fixture directory is loaded recursively; failures include the offending file path and `id`

### Custom test authoring workflow

1. Author a YAML file under `tests/fixtures/` (per-crate) or `crates/gr-testkit/fixtures/community/` (cross-cutting).
2. Validate with `gr-cli validate-fixture <path>`. The CLI prints decoded structure on success and a targeted error on failure (path + line + reason).
3. Wire it into a test:
   ```rust
   #[test]
   fn user_authored_xbox_neutral_held() {
       let fixture = gr_testkit::fixtures::load("tests/fixtures/xbox_neutral_held.yaml");
       fixture.run_in_default_harness();
   }
   ```
4. For scenarios, the `run_in_default_harness` helper spins up a fake-backend session matching the scenario's `config`, plays the steps, and surfaces failures with the fixture's `notes` attached.

The fixture format is `#[non_exhaustive]` from day one. New kinds are additive.

## CLI surfaces

Two binaries, two audiences, one shared library backbone.

### `gr-cli` — internal / CI / automation

Owned by the `gr-cli` crate. Optimized for scripts, CI jobs, and the phase-gate runner's automated portion.

Commands:

| Command | Purpose |
| --- | --- |
| `gr-cli validate-config <path>` | Parse + validate a session config; structured error output |
| `gr-cli validate-fixture <path>` | Validate a fixture file; print decoded structure |
| `gr-cli list-profiles` | List built-in profile ids and family classifications |
| `gr-cli show-capabilities <profile>` | Dump capability summary in YAML or JSON |
| `gr-cli plan-session <profile> --goal <tier>` | Run planner against a synthetic inventory; emit `SessionPlan` YAML |
| `gr-cli simulate-session <scenario>` | Run a `session-scenario` fixture against a fake backend; emit per-step trace |
| `gr-cli replay-trace <path>` | Replay a `backend-trace` fixture through a translator pair; emit decoded outputs |
| `gr-cli capability-coverage [--profile <id>]` | Cross-check declared capabilities against translator coverage; exit non-zero on gap |
| `gr-cli support-report [--profile <id>]` | Generate the support-claim evidence report described in [HEADLESS_TEST_STRATEGY.md](../validation/HEADLESS_TEST_STRATEGY.md#support-evidence-report) |
| `gr-cli phase-gate <N> --auto` | Run the deterministic portion of Phase N's gate; exit 0 / non-zero |

Output formats: `--format yaml` (default) and `--format json`. All commands accept `--profile <id>` filters where applicable.

### `vgpd-demo` — humans / phase gates

Owned by the `demo/` crate. Optimized for a developer running commands at a phase gate. Where commands overlap with `gr-cli`, `vgpd-demo` invokes the same backing functions but prettier-prints output, prompts for confirmation, and bundles the manual checklist.

Subcommands grow phase by phase per [the implementation plan](RUST_IMPLEMENTATION_PLAN.md). The phase-gate command:

```text
$ vgpd-demo phase-gate 3
Phase 3: Configuration + session options
==========================================
Automated checks:
  ✓ cargo test --workspace --all-features
  ✓ gr-cli validate-fixture crates/gr-config/fixtures/*.yaml
  ✓ insta review (clean working tree)

Manual checklist:
  [ ] 1. Run `vgpd-demo validate-config samples/dualsense-identity.yaml` — confirm it accepts
  [ ] 2. Run `vgpd-demo validate-config samples/broken-mode.yaml`     — confirm it rejects with a useful error
  [ ] 3. Author a custom config under tests/fixtures/ and validate it — confirms authoring workflow
  [ ] 4. Inspect `target/.../config_snapshots/` — confirm structure is intelligible

When complete, sign off with:
  git commit --allow-empty -m "chore(phase-gate): Phase 3 gate passed"
```

The phase-gate runner does not enforce gate completion mechanically; it documents what "done" means and produces an automatable subset.

## Snapshot testing with `insta`

`insta` is the workspace's snapshot library. Snapshots live next to the test file (default `insta` layout) and are reviewed via `cargo insta review`.

Snapshot-worthy outputs:

- `SessionPlan` instances per profile × fidelity × inventory permutation
- `DegradationReport` shapes
- Descriptor templates (compact hex + decoded form)
- Capability summaries per profile
- Diagnostics snapshots after a canned scenario run
- `support-report` outputs (per [HEADLESS_TEST_STRATEGY.md](../validation/HEADLESS_TEST_STRATEGY.md))

Rules:

- snapshot only stable, human-readable output (no addresses, no timestamps, no random ids)
- changes require explicit `cargo insta review` and a paired commit; CI fails on stale snapshots
- snapshot names include the profile id and the scenario id (e.g. `dualsense__plan_degraded_uhid.snap`)

## Property testing with `proptest`

`proptest` is the workspace's property-testing library. Domain types provide `Strategy<T>` implementations under `gr_testkit::proptest_strategies` so any crate's tests can build on them.

Required properties:

- **Input payload roundtrip**: any well-formed `ProfileInputPayload` serialized and deserialized via `serde_yaml` is bit-equal
- **Sequence monotonicity**: a session emits backend frames in the same order as input sequence ids (with documented coalescing exceptions)
- **Capability negotiation completeness**: for any profile × backend support permutation, the planner produces either a valid plan or a structured rejection — never an inconsistent plan
- **Reverse-translator coverage**: a reverse translator never emits an `OutputFunctionRef::Semantic(_)` for a function the profile did not declare as supported
- **Fixture roundtrip**: every kind of fixture loads to a typed value and re-serializes to a byte-equal YAML document

Strategy ownership lives in `gr-testkit`; consumers import them.

## Backend trace recorder / replayer

Two halves of the same tool:

- **Recorder** wraps any `BackendSession` and emits a `backend-trace` fixture on the side as the session runs. Useful for: capturing real-device behavior in Tier B/C runs, generating regression fixtures from successful flows, and producing snapshots of provider failure modes.
- **Replayer** loads a `backend-trace` fixture and surfaces it as a `BackendSession` to the rest of the runtime. Useful for: deterministic translator tests, planner integration tests against canned reverse events, and reproducing field-reported failures.

Both share a stable on-disk format; a trace captured today must replay against the runtime a year later, even if intervening backend trait additions occur (`#[non_exhaustive]`).

## Manual gate format

Each phase in [RUST_IMPLEMENTATION_PLAN.md](RUST_IMPLEMENTATION_PLAN.md) ends with an **Exit gate** section. Format:

```markdown
### Exit gate

Run `vgpd-demo phase-gate <N>` and complete the checklist.

Automated portion (must pass before manual review):

- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `cargo insta test --check` clean (no stale snapshots)
- [ ] `gr-cli phase-gate <N> --auto` exits 0

Manual portion:

- [ ] N. <action> (verifies <invariant>)
- [ ] N+1. <action> (verifies <invariant>)
...

Sign-off: `git commit --allow-empty -m "chore(phase-gate): Phase N gate passed"`
```

Sign-off is documentary: it produces a git marker on the branch that downstream tooling and PR reviewers can verify. Phase N+1's PR description references the gate commit's sha.

The user owns the manual checklist. Agents and CI cannot mark items complete on the user's behalf.

## CI integration

CI runs the full automated portion of every phase gate the workspace claims to satisfy. The MSRV, clippy, test, deny, and audit jobs already exist; new jobs added in support of testing tooling:

- **fixture-validation** — runs `gr-cli validate-fixture` over every `*.yaml` under `tests/fixtures/` and `crates/*/fixtures/`
- **insta-check** — runs `cargo insta test --check` to reject stale snapshots
- **capability-coverage** — runs `gr-cli capability-coverage`; non-zero exit fails the job

These run on the same Linux matrix entry; macOS and Windows runners are unchanged (they continue to validate cross-platform build and core test coverage).

## Performance / hot-path tooling

Not the focus of v1, but reserved:

- a `criterion`-based bench harness for hot-path microbenchmarks under `crates/gr-session/benches/`
- a `vgpd-demo benchmark` subcommand for ad-hoc throughput checks under fake backends
- annotations on hot-path tests so they can be excluded from default `cargo test` and run via `cargo test -- --ignored hot`

These tools land alongside the latency target in [RUST_IMPLEMENTATION_SPEC.md performance acceptance targets](RUST_IMPLEMENTATION_SPEC.md#performance-acceptance-targets) when Phase 6+ work begins exercising them.

## File layout summary

```text
crates/
  gr-testkit/
    src/                  # described above
    fixtures/community/   # cross-cutting fixtures
  gr-cli/
    src/
      bin/gr-cli.rs
      commands/
demo/
  src/
    main.rs               # vgpd-demo entry
    phase_gate.rs         # phase-gate driver
tests/
  fixtures/               # workspace-level user-authored fixtures
```

Each runtime crate may carry its own `fixtures/` directory for tightly-scoped scenarios; `gr-testkit` provides the loader that walks both per-crate and workspace-level fixture roots.

## Versioning rules

- the fixture envelope's `fixture: virtualgamepad/v1` is the on-disk schema version; bumping it requires migrating or rejecting v0 documents
- `gr-testkit`'s public API follows the workspace's pre-1.0 versioning rule (minor versions may break)
- snapshot files (`*.snap`) are not versioned independently; review them on every change
- `backend-trace` fixtures carry a `notes` field that should record kernel / firmware / tool versions used at capture (see [DEVICE_SPEC_VALIDATION_PLAN.md real-hardware capture checklist](../validation/DEVICE_SPEC_VALIDATION_PLAN.md#real-hardware-capture-checklist))

## Acceptance criteria for the tooling itself

This spec is satisfied when:

- `gr-testkit` exposes the public API surface listed above
- the fixture format is implemented with all five kinds, loadable by `gr-cli validate-fixture`, and round-trippable
- `gr-cli` exposes the command set listed above
- `vgpd-demo phase-gate <N>` runs the deterministic portion and prints the manual checklist for Phase N
- snapshot and property tests are wired in at least one crate per the matrix in [TEST_PLAN.md](../validation/TEST_PLAN.md)
- backend trace record + replay roundtrips for at least one Linux provider
- CI jobs `fixture-validation`, `insta-check`, and `capability-coverage` are green on `main`
