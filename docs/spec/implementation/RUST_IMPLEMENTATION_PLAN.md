# Rust Implementation Plan

This document defines the Rust build plan for implementing the target architecture described in [ARCHITECTURE_SPEC.md](../specs/ARCHITECTURE_SPEC.md).

It is intended to guide a real build-out from a greenfield workspace to a production-capable subsystem that can be embedded inside a larger host program.

This plan assumes:

- the host program can produce controller input appropriate to the selected target profile
- the library should ship primarily as an embeddable Rust crate set
- Linux is the first concrete platform target
- Windows and macOS must be accounted for in the architecture and planner even before their providers are implemented
- fidelity tiers will be implemented incrementally
- session isolation, planning correctness, and reverse-path handling matter more than early transport spoofing

Related documents:

- [ARCHITECTURE_SPEC.md](../specs/ARCHITECTURE_SPEC.md)
- [IMPLEMENTATION_FRAMEWORK.md](../implementation/IMPLEMENTATION_FRAMEWORK.md)
- [CONFIGURATION_SPEC.md](../specs/CONFIGURATION_SPEC.md)
- [FIDELITY_GUIDE.md](../specs/FIDELITY_GUIDE.md)
- [TEST_PLAN.md](../validation/TEST_PLAN.md)

## Authority and drift rule

[RUST_IMPLEMENTATION_SPEC.md](../implementation/RUST_IMPLEMENTATION_SPEC.md) is the authoritative build-facing specification for crate ownership, public runtime types, backend contracts, translator contracts, and acceptance criteria.

This plan defines sequencing. If this document conflicts with the implementation specification, update this plan to match the specification rather than broadening the runtime design.

## Implementation goals

- produce a Rust library workspace that directly reflects the architecture spec
- keep profile requirements separate from runtime backend selection
- make every emulated device instance session-scoped
- prepare profile-specific input validation before runtime translation
- support both forward input flow and reverse output flow
- keep platform-provider and transport I/O behind replaceable backend-session traits
- make degradation, unsupported capabilities, and diagnostics explicit
- enforce correctness through unit, contract, and integration testing before backend complexity grows

## Architecture alignment

The Rust implementation must align to these architectural decisions:

- planning is a runtime negotiation against backend inventory, not a static profile lookup
- planning includes host-platform and provider negotiation, not just backend-family negotiation
- backends are created per session, never shared as mutable singleton device instances
- HID and transport translators are profile-family-specific where required
- reverse-path output handling is a core runtime contract, not a later bolt-on
- the primary public API accepts device-focused controller input rather than requiring a unified control model
- telemetry and structured errors are part of the public integration surface

## Recommended workspace layout

The best long-term shape is a Cargo workspace with focused crates and clean dependency direction.

```text
virtualgamepad/
  Cargo.toml
  crates/
    gr-core/
    gr-profiles/
    gr-config/
    gr-session-options/
    gr-runtime-model/
    gr-planner/
    gr-backend-api/
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

## Crate responsibilities

### `gr-core`

Owns:

- core ids and metadata types
- profile-specific input payload ids and shared input metadata
- semantic input and output function enums
- fidelity-tier and backend-level enums
- shared error and diagnostics types

Must not own:

- Linux-specific code
- compiled session options
- session plans
- backend descriptors
- concrete profiles
- concrete backend implementations

### `gr-profiles`

Owns:

- built-in target controller profiles
- identity metadata
- capability declarations
- profile-family metadata
- supported fidelity levels
- required semantic input and output functions
- backend descriptor metadata per supported level

Important rule:

- profiles describe what a target needs and supports
- profiles do not choose the actual runtime backend instance

### `gr-config`

Owns:

- config-file schema
- serde parsing
- schema validation
- normalization into strongly typed config structures

Produces:

- `SessionConfig`
- validation reports

### `gr-session-options`

Owns:

- session-option validation
- input-policy compilation
- provider and delivery option compilation
- optional out-of-core helper adapters

Produces:

- `CompiledSessionOptions`

Important rule:

- `gr-session-options` compiles session policy only: input validation policy, provider hints, reverse delivery policy, and backpressure policy
- translators consume exact profile-shaped input and prepared translation context, not compiled semantic mappings
- optional mapping or adaptation helpers may exist later only outside the core runtime path, and their output must already match the selected profile input contract

### `gr-planner`

Owns:

- backend inventory modeling
- host-platform inventory modeling
- fidelity negotiation
- backend-family selection
- provider selection
- degradation analysis
- unsupported-capability analysis
- final `SessionPlan` generation

Consumes:

- target profile
- compiled session options
- runtime backend inventory
- host policy

Produces:

- `SessionPlan`

### `gr-runtime-model`

Owns:

- `SessionRequest`
- `SessionPlan`
- `PreparedSession`
- `PreparedTranslationContext`
- `ControllerOutputCommand`
- session status and diagnostics snapshots
- reverse delivery and backpressure policy types

Important rule:

- runtime model types are cross-cutting contracts and must not live in `gr-core`
- these types may depend on primitive domain types and compiled session-option data

### `gr-backend-api`

Owns:

- backend family enums and capability traits
- host-platform enums and provider identifiers
- per-session backend-factory traits
- per-session backend-session traits
- backend descriptor types
- backend frame enums
- backend-originated reverse event enums
- backend diagnostics types

This crate is the key boundary that prevents session logic from leaking into backends and backend details from polluting core logic.

### `gr-translators`

Owns:

- forward translator traits
- reverse translator traits
- translator registry
- profile-family-specific translator implementations

Recommended module split:

- `evdev/`
- `hid/dualsense.rs`
- `hid/steam_controller.rs`
- `transport/xbox360_usb.rs`
- `transport/dualsense_usb.rs`
- `transport/dualsense_bluetooth.rs`

Important rule:

- do not implement one universal HID translator for all HID-capable profiles

### `gr-session`

Owns:

- session manager
- session lifecycle state machine
- session creation and teardown
- state update processing
- backend session ownership
- reverse event dispatch
- telemetry coordination

Important rule:

- every backend instance belongs to exactly one active session
- logical session isolation does not imply one dedicated OS thread per session

### `gr-host-bridge`

Owns:

- host-facing manager API
- callback and channel adapters
- reverse-output delivery interface
- optional FFI-safe wrappers if the host is not Rust
- bounded reverse-event delivery policy

### `gr-provider-linux-uinput`

Owns:

- Linux `uinput` backend factory
- Linux `uinput` backend session
- evdev device creation and write path
- optional force-feedback support hooks

### `gr-provider-linux-uhid`

Owns:

- Linux `UHID` backend factory
- Linux `UHID` backend session
- descriptor provisioning
- HID input report write path
- output and feature report receive path

### `gr-provider-linux-transport`

Owns:

- future transport backend factories and sessions
- USB and Bluetooth transport state machines
- packet encoding and decoding
- enumeration and protocol behavior

This crate may remain skeletal until the earlier session, planning, and reverse-path contracts are stable.

### `gr-provider-windows-hid`

Owns:

- Windows-oriented virtual HID provider interfaces and sessions
- provider capability declarations and deployment checks
- driver-backed realization glue when implemented

Important rule:

- this crate may begin as inventory, diagnostics, and support reporting before full device realization exists

### `gr-provider-macos-hid`

Owns:

- macOS-oriented HID provider interfaces and sessions
- provider capability declarations and deployment checks
- entitlement and install-prerequisite modeling when implemented

Important rule:

- this crate may begin as inventory, diagnostics, and support reporting before full device realization exists

### `gr-testkit`

Owns:

- fake backend inventory
- fake backend factories and sessions
- profile-input fixture builders
- config fixtures
- profile fixtures
- reverse-event fixtures
- validation helpers for contract tests

### `gr-cli`

Owns:

- developer diagnostics and validation commands

Recommended commands:

- `validate-config`
- `list-profiles`
- `show-capabilities`
- `plan-session`
- `simulate-session`
- `dry-run-route`

## Dependency direction rules

- `gr-core` depends on no internal crates
- `gr-profiles` depends on `gr-core`
- `gr-config` depends on `gr-core`
- `gr-session-options` depends on `gr-core`, `gr-config`, and `gr-profiles`
- `gr-runtime-model` depends on `gr-core` and `gr-session-options`
- `gr-backend-api` depends on `gr-core` and `gr-runtime-model`
- `gr-planner` depends on `gr-core`, `gr-profiles`, `gr-session-options`, `gr-runtime-model`, and `gr-backend-api`
- `gr-translators` depends on `gr-core`, `gr-profiles`, `gr-runtime-model`, and `gr-backend-api`
- `gr-session` depends on `gr-core`, `gr-runtime-model`, `gr-backend-api`, `gr-planner`, and `gr-translators`
- `gr-host-bridge` depends on `gr-runtime-model` and `gr-session`
- concrete backend crates depend on `gr-core`, `gr-runtime-model`, and `gr-backend-api`, but not on `gr-session`
- `gr-testkit` may depend on all runtime crates as needed for testing
- `gr-cli` depends on public crates only, never on private crate internals

## Recommended core Rust patterns

### Strong enums

Use enums instead of strings internally for:

- semantic input functions
- semantic output functions
- capability categories
- fidelity tiers
- backend levels
- backend families
- reverse command types
- session states

### Newtypes

Use newtypes for:

- `ProfileId`
- `SessionId`
- `BackendId`
- `VendorId`
- `ProductId`
- `SequenceId`

### Immutable value types

Prefer immutable state snapshots, immutable compiled session options, immutable plans, and immutable outbound frame values.

### Builder patterns

Use builders for:

- profile-input fixtures
- controller profiles
- compiled session requests
- backend descriptors
- session plans

### Structured errors

Use a central error model with:

- domain-specific enums
- `thiserror`
- optional machine-readable error codes
- rich context for logging and host diagnostics

### Traits

Use traits for:

- backend factories
- backend sessions
- forward translators
- reverse translators
- telemetry sinks
- host output sinks

## Recommended dependencies by phase

### Early dependencies

- `serde`
- `serde_yaml`
- `serde_json`
- `thiserror`
- `indexmap`
- `smallvec`
- `bitflags`

### Mid-phase dependencies

- `tracing`
- `tracing-subscriber`
- `parking_lot`
- `crossbeam` if queue patterns need it
- `tokio` only if async session execution proves worthwhile

### Linux provider dependencies

- `nix`
- `libc`
- direct ioctl bindings where required

Optional candidates:

- `evdev`
- `udev`

Use optional wrappers only if they fit the backend-session abstraction cleanly.

### Planned Windows provider dependencies

- keep optional until the Windows provider is active
- prefer a narrow provider boundary over leaking Windows SDK types upward
- model driver-install and availability checks before implementing frame send paths

### Planned macOS provider dependencies

- keep optional until the macOS provider is active
- prefer a narrow provider boundary over leaking Apple framework types upward
- model entitlement and install checks before implementing frame send paths

### Testing dependencies

- `rstest`
- `insta`
- `proptest`
- `assert_matches`

## Canonical Rust contracts

### `ProfileInputFrame`

Required fields:

- `timestamp`
- `sequence`
- `profile_id`
- `payload`

Rules:

- every frame is tied to one selected target profile
- the payload must match that profile's concrete input contract
- validation happens against the chosen profile contract before translation

### `SessionRequest`

Required fields:

- `profile_id`
- `goal`
- `session_config`

Optional fields:

- host platform preference
- backend preference
- provider preference
- host metadata

Strictness lives inside `session_config.validation`, not on the request.

### `SessionPlan`

`SessionPlan` is defined authoritatively in [RUST_IMPLEMENTATION_SPEC.md](../implementation/RUST_IMPLEMENTATION_SPEC.md#sessionplan). The field list below is reproduced for reference; the spec is the source of truth.

Required fields:

- `session_id`
- `profile_id`
- `requested_goal`
- `requested_fidelity_tier`
- `selected_level`
- `target_host_platform`
- `selected_backend_family`
- `selected_provider_id`
- `selected_translator_family`
- `capability_result`
- `degradation`
- `warnings`
- `deployment_requirements`
- `backend_open_context`
- `session_options`

### `BackendFactory`

Recommended responsibility:

- create a per-session backend session from a backend realization request and open context

Recommended shape:

```rust
trait BackendFactory {
    fn backend_id(&self) -> BackendId;
    fn family(&self) -> BackendFamily;
    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport;
    fn open_session(&self, context: &BackendOpenContext) -> Result<Box<dyn BackendSession>, BackendError>;
}
```

### `BackendSession`

Recommended responsibility:

- own one concrete emulated device instance
- send frames
- surface reverse events
- close deterministically

Performance rules:

- backend sessions may keep reusable frame buffers or protocol scratch buffers
- backend sessions must not be shared across active devices
- reverse-event receive paths must be bounded

Recommended shape:

```rust
trait BackendSession {
    fn session_id(&self) -> SessionId;
    fn open(&mut self) -> Result<(), BackendError>;
    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError>;
    fn drain_reverse_events(&mut self, out: &mut dyn Extend<BackendReverseEvent>) -> Result<(), BackendError>;
    fn readiness(&self) -> EventReadiness;
    fn diagnostics(&self) -> BackendDiagnostics;
    fn close(&mut self) -> Result<(), BackendError>;
}
```

`send` and `drain_reverse_events` are non-blocking; see the backend blocking contract in [RUST_IMPLEMENTATION_SPEC.md](../implementation/RUST_IMPLEMENTATION_SPEC.md#backend-blocking-contract).

### `ForwardTranslator`

Recommended shape:

```rust
trait ForwardTranslator {
    fn translate(
        &self,
        input: &ProfileInputFrame,
        ctx: &PreparedTranslationContext,
        out: &mut TranslationScratch,
    ) -> Result<BackendFrame, TranslationError>;
}
```

### `ReverseTranslator`

Recommended shape:

```rust
trait ReverseTranslator {
    fn translate_reverse(
        &self,
        event: &BackendReverseEvent,
        ctx: &PreparedTranslationContext,
        out: &mut SmallVec<[ControllerOutputCommand; 4]>,
    ) -> Result<(), TranslationError>;
}
```

## Phase 0: Workspace bootstrap

### Goal

Create the Rust workspace and baseline development infrastructure.

### Deliverables

- workspace `Cargo.toml`
- initial crate skeletons
- linting and formatting setup
- CI shell with placeholder jobs
- baseline Rust development notes in the repo

### Tasks

1. Create the Cargo workspace.
2. Add `rustfmt` and `clippy` expectations.
3. Add CI commands for:
   - `cargo fmt --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
4. Create placeholder crates for:
   - `gr-core`
   - `gr-profiles`
   - `gr-config`
   - `gr-session-options`
   - `gr-planner`
   - `gr-backend-api`
   - `gr-translators`
   - `gr-session`
   - `gr-host-bridge`
   - `gr-testkit`
5. Establish crate dependency direction rules.

### Exit criteria

- workspace builds
- empty test suite passes
- lint and formatting checks pass

## Phase 1: Core domain model

### Goal

Implement the canonical value types used by every other crate.

### Deliverables

- `ProfileInputFrame`
- typed input and output semantic function enums
- capability types
- profile identity structs
- fidelity-tier enums with human-readable labels
- backend-family and backend-level enums
- shared error and diagnostic types

### Tasks

1. Define normalized button and axis types.
2. Define touch and motion types.
3. Define `SequenceId`.
4. Define fidelity tiers:
   - `Compatibility`
   - `IdentityAware`
   - `HardwareFaithful`
5. Define backend levels:
   - `Evdev`
   - `Hid`
   - `Transport`
6. Define backend families:
   - `LinuxUinput`
   - `LinuxUhid`
   - `TransportUsb`
   - `TransportBluetooth`
7. Define shared structured error categories.
8. Implement per-profile input validation policy.

### Tests

- default state tests
- bounds and normalization tests
- sequence and timestamp behavior tests
- serde round-trip tests if serde is used

### Exit criteria

- all core types compile
- profile-input policy is explicit
- no Linux-specific code exists in this phase

## Phase 2: Profile and capability system

### Goal

Implement built-in profiles and capability declarations as typed Rust data.

### Deliverables

- `ControllerProfile`
- `ControllerCapabilities`
- built-in profile registry
- required and supported function lists
- backend descriptor metadata per level

### Tasks

1. Implement capability item types for input and output.
2. Define capability groups:
   - buttons
   - axes
   - pads
   - motion sensors
   - microphones
   - speakers
   - rumble
   - haptics
   - lighting
   - audio
   - trigger effects
   - display
   - force feedback
   - misc
3. Add built-in profiles for:
   - generic Linux gamepad
   - Xbox 360
   - DualSense
   - Steam Controller 2026
4. Publish:
   - supported input functions
   - supported output functions
   - required input functions
   - required output functions
   - supported fidelity levels
   - descriptor metadata per level

### Important rule

- profiles may describe transport or HID requirements
- profiles may not hard-code one concrete runtime backend choice

### Tests

This phase must implement the explicit output capability tests described in [TEST_PLAN.md](../validation/TEST_PLAN.md).

Required tests:

- per-profile capability presence tests
- per-profile output capability correctness tests
- per-profile capability-to-function consistency tests
- duplicate-capability prevention tests

### Exit criteria

- capability registry answers deterministic queries
- every built-in profile has explicit tests
- output-device capabilities are covered explicitly

## Phase 3: Configuration system

### Goal

Implement configuration parsing and validation as the source of session intent.

### Deliverables

- session config parsing
- validation reports
- normalized config structures

### Tasks

1. Model config file types with serde.
2. Implement fidelity-tier parsing from:
   - `compatibility`
   - `identity-aware`
   - `hardware-faithful`
3. Implement session-policy validation against declared profile input contracts.
4. Implement target semantic function validation against profile data.
5. Implement output-handling validation for callback, channel, log-only, pass-through, and ignore modes.
6. Validate reverse-output delivery and backpressure policy.
7. Normalize parsed config into strongly typed internal config.
8. Add optional schema export if useful.

### Tests

- valid config fixtures
- invalid schema fixtures
- unsupported function fixtures
- invalid output-handling fixtures
- unsupported provider and fidelity fixtures

### Exit criteria

- config validation is deterministic
- session policy semantics are test-covered
- normalized config is ready for session-option compilation

## Phase 4: Session options compiler

### Goal

Compile session configuration into validated session options before runtime session start without introducing a universal remapping layer.

### Deliverables

- input validation policy
- provider hints
- reverse delivery policy
- backpressure policy
- compiled session options

### Tasks

1. Compile input validation settings for the selected profile contract.
2. Compile provider and host-platform hints for planner use.
3. Compile reverse output delivery mode and queue policy.
4. Validate unsupported-capability policy.
5. Expose `CompiledSessionOptions` as immutable runtime data.
6. Keep optional mapping, transform, or adaptation helpers outside the core device-session runtime.
7. Require any helper output to be exact profile-shaped input before `send_input` or `send_input_delta`.

### Tests

- direct session-policy validation tests
- input validation policy tests
- provider-hint tests
- reverse delivery policy tests
- backpressure policy tests
- helper-boundary tests proving core translators do not depend on semantic mappings

### Exit criteria

- compiled session options are deterministic
- translators consume exact profile-shaped input and prepared context only
- the per-frame path is ready to operate without config parsing, semantic remapping, or repeated capability lookup

## Phase 5: Planner and negotiation

### Goal

Implement runtime negotiation across profile requirements, compiled session options, and available backend inventory.

### Deliverables

- `BackendInventory`
- `SessionPlan`
- degradation analysis
- unsupported-capability analysis

### Tasks

1. Implement planner input model:
   - target profile
   - requested fidelity tier
   - compiled session options
   - backend inventory
   - host policy
2. Implement selected backend-level logic.
3. Implement backend-family selection from actual inventory.
4. Implement capability negotiation.
5. Implement degradation and warning generation.
6. Implement session-admissibility decision.
7. Ensure planner output is inspectable and serializable for diagnostics.
8. Ensure planner output is cacheable and session-local after creation.

### Tests

- per-profile fidelity plan tests
- degraded plan tests
- impossible plan rejection tests
- enabled capability set tests
- unsupported capability set tests
- backend-family selection tests

### Exit criteria

- planner behavior matches [FIDELITY_GUIDE.md](../specs/FIDELITY_GUIDE.md)
- profile requirements are cleanly separated from deployment environment
- degraded states are explicit and test-covered

## Phase 6: Backend API and fake backends

### Goal

Define backend contracts and create fake per-session backends for most integration testing.

### Deliverables

- backend-factory traits
- backend-session traits
- frame enums
- reverse-event enums
- fake backend inventory
- fake backend factories and sessions

### Tasks

1. Define backend factory trait.
2. Define backend session trait.
3. Define backend-open descriptor and diagnostics types.
4. Define frame enums:
   - evdev frame
   - HID frame
   - transport frame
5. Define backend reverse event enums.
6. Implement fake backend factories with:
   - support-report behavior
   - session creation
   - controllable failure injection
7. Implement fake backend sessions with:
   - frame capture
   - injected reverse events
   - session id tracking
   - close semantics
   - bounded queue behavior
   - coalescing policy tests where applicable

### Tests

- fake backend open/close tests
- wrong-frame rejection tests
- reverse-event injection tests
- session isolation tests
- failure-path tests
- bounded queue behavior tests

### Exit criteria

- session crate can be developed against fake backends only
- backend instances are clearly session-scoped in tests

## Phase 7: Translator system

### Goal

Implement forward and reverse translators with the correct profile-family boundaries.

### Deliverables

- translator registry
- generic evdev translator path
- profile-family-specific HID translators
- reverse translators for HID and transport-capable paths

### Tasks

1. Define forward and reverse translator traits.
2. Implement evdev translation path using exact profile-shaped input.
3. Implement `DualSenseHidTranslator`.
4. Implement `SteamControllerHidTranslator`.
5. Implement corresponding reverse translators for:
   - rumble
   - LEDs
   - trigger effects
   - mode commands
6. Add translator compatibility checks against profile descriptor metadata.

### Important rule

- do not treat HID reports as one generic 64-byte format for all profiles
- treat descriptor/report compatibility as a contract, not an implementation detail

### Tests

- neutral-state frame tests
- per-target profile-input translation tests
- descriptor/report compatibility tests
- reverse translation tests

### Exit criteria

- HID behavior is profile-family-specific where required
- reverse translators exist for identity-aware targets

## Phase 8: Session engine

### Goal

Implement the host-facing session runtime around session-scoped backends and compiled plans.

### Deliverables

- `GamepadEmulationManager`
- `GamepadEmulationSession`
- session lifecycle state machine
- reverse-output dispatch
- telemetry sink abstraction

### Tasks

1. Implement manager initialization with backend inventory.
2. Implement profile and capability queries.
3. Implement session creation from config plus planner output.
4. Implement per-session backend ownership.
5. Implement state update API.
6. Implement reverse event polling or dispatch loop.
7. Implement lifecycle transitions:
   - created
   - starting
   - active
   - paused
   - stopping
   - stopped
   - failed
8. Implement session switch and rebuild flow.
9. Implement telemetry sink abstraction.

### Concurrency recommendation

Use an actor-like session model.

Recommended shape:

- one logical session actor per active session
- ordered inbound state update queue
- separate reverse-event loop or polling cycle
- shared runtime workers rather than one dedicated OS thread per session by default
- state coalescing when a newer input snapshot supersedes an older queued snapshot

If async is adopted:

- keep async localized to session and backend layers
- isolate backend-specific blocking calls carefully

If synchronous first:

- keep the public API sync
- hide optional async behind feature flags later

### Tests

- start/update/stop integration tests
- reconfiguration tests
- session switch tests
- reverse event dispatch tests
- failure recovery tests

### Exit criteria

- full end-to-end flow works with fake backends
- every active backend instance belongs to exactly one session
- the scheduling model is compatible with many concurrent virtual devices

## Phase 9: Linux `uinput` compatibility provider

### Goal

Deliver the first real Linux backend for `compatibility` fidelity.

### Deliverables

- `gr-provider-linux-uinput`
- `LinuxUinputBackendFactory`
- `LinuxUinputBackendSession`

### Tasks

1. Implement Linux `uinput` device creation.
2. Implement capability declaration from profile data.
3. Implement evdev event emission:
   - buttons
   - axes
   - sync reports
4. Implement device identity metadata:
   - name
   - bus type
   - vendor id
   - product id
5. Add optional force-feedback support hooks even if initially partial.
6. Add developer diagnostics for emitted events.
7. Add reusable event-buffer strategy where practical.

### Rust implementation notes

- keep all unsafe code tightly isolated
- wrap file descriptors in RAII types
- keep kernel interaction in one module tree

### Tests

- descriptor construction tests
- fake writer tests for event batches
- Linux integration tests gated behind environment checks

### Exit criteria

- `compatibility` tier works for the generic gamepad and Xbox-style layout
- this tier is documented as host-visible gamepad behavior, not physical-device identity

## Phase 10: Linux `UHID` identity-aware provider

### Goal

Deliver `identity-aware` fidelity for targets that need HID identity and reverse report handling.

### Deliverables

- `gr-provider-linux-uhid`
- `LinuxUhidBackendFactory`
- `LinuxUhidBackendSession`
- reverse event receive path

### Tasks

1. Implement UHID device lifecycle.
2. Implement descriptor provisioning API.
3. Implement HID input report writer.
4. Implement reverse output report receiver.
5. Implement feature report handling.
6. Integrate reverse translators for:
   - rumble
   - LEDs
   - trigger effects
   - mode commands
7. Add telemetry for descriptor, input-report, and reverse-event failures.
8. Add bounded reverse-event queue handling and backpressure policy.

### Scope guard

This phase should focus on descriptor correctness, input reports, and reverse-path structure before trying to emulate every advanced feature at full richness.

### Tests

- descriptor validation tests
- HID input report translation tests
- reverse output command translation tests
- integration tests with fake UHID shims where useful

### Exit criteria

- one real identity-aware target works end to end
- HID descriptor and report identity are validated for the implemented profile
- reverse output commands are observed and normalized correctly
- output and feature report handling is present before claiming `identity-aware` support

## Phase 11: Host output bridge

### Goal

Make reverse-path features first-class and host-usable.

### Deliverables

- typed controller output commands
- callback and channel bridges
- optional physical-controller forwarding abstraction

### Tasks

1. Define typed controller output commands.
2. Implement callback sink support.
3. Implement channel or queue sink support.
4. Add optional bridge abstraction for forwarding reverse commands to a real physical controller.
5. Implement policy for ignored or unsupported reverse commands.
6. Implement bounded delivery behavior so one slow consumer cannot stall all sessions.

### Tests

- callback delivery tests
- channel-delivery tests
- unsupported reverse command policy tests
- slow-consumer isolation tests

### Exit criteria

- reverse-path handling is no longer backend-private
- host applications can consume output commands deterministically

## Phase 12: Linux transport provider foundation

### Goal

Introduce the minimum architecture needed for `hardware-faithful` targets without destabilizing earlier layers.

Transport work must not begin until sessions, planning, exact profile input contracts, and reverse-path contracts are stable in fake, `uinput`, and `UHID` paths.

### Deliverables

- transport backend trait implementations
- enumeration and session state-machine interfaces
- packet models
- skeletal profile-family-specific transport translators

### Tasks

1. Define transport backend session shapes.
2. Define enumeration and protocol state-machine traits.
3. Define transport packet models.
4. Implement early packet send and reverse packet receive paths.
5. Add skeletal profile-family-specific transport translators for:
   - Xbox 360 USB
   - DualSense USB
   - DualSense Bluetooth

### Tests

- packet model tests
- transport planner admissibility tests
- reverse packet contract tests

### Exit criteria

- transport is architecturally integrated even if not yet fully faithful

## Phase 13: Linux hardware-faithful transport implementations

### Goal

Implement real transport behavior only after the earlier contracts are proven.

### Deliverables

- concrete USB and Bluetooth protocol state machines
- descriptor and enumeration handling
- transport-level input and reverse output packet handling

### Tasks

1. Capture and model real enumeration behavior.
2. Implement packet encoding and decoding.
3. Implement transport timing and handshake-sensitive logic where required.
4. Implement reverse packet handling and translation.
5. Validate target behavior against real hosts where possible.

### Tests

- enumeration tests
- descriptor tests
- packet tests
- timing-sensitive validation where practical

### Exit criteria

- at least one `hardware-faithful` target passes real transport validation
- USB, Bluetooth, or another modeled bus has validated enumeration and control-flow behavior
- reverse packet handling is translated into typed host-visible output commands

## Milestones

### Milestone 1: Spec/runtime model with fake backend contracts

Success means:

- full planning and session-option system with fake backends
- explicit session lifecycle
- fake backend contracts for session lifecycle, planner output, and reverse-event delivery
- reverse-path translation working in simulation

### Milestone 2: Real Linux compatibility backend

Success means:

- `uinput` backend works for `compatibility`
- host software sees a usable Linux virtual gamepad
- this milestone does not claim physical-device identity
- session and diagnostics model remains unchanged from fake backend testing

### Milestone 3: First real identity-aware target

Success means:

- one HID target works through `UHID`
- descriptor and input-report validation pass
- output and feature reports are observed and normalized into host-visible commands

### Milestone 4: First hardware-faithful target

Success means:

- transport path works with the same session and planning architecture
- transport enumeration and control-flow validation pass for one target
- reverse transport packets are handled and normalized
- transport complexity remains isolated from core crates

## Planned post-Linux platform phases

These phases are architectural commitments and planned implementation paths, not immediate delivery promises.

### Phase 14: Windows provider foundation

Goal:

- prove that the Linux-first runtime can admit a Windows provider without architectural rewrites

Deliverables:

- `gr-provider-windows-hid`
- `HostPlatform::Windows` planner inventory entries
- provider support-report contracts for deployment prerequisites
- planner diagnostics for unavailable or uninstalled Windows providers

Tasks:

1. Define Windows provider inventory and diagnostics types.
2. Model driver-backed install requirements in `BackendSupportReport` and `SessionPlan`.
3. Add provider-selection tests proving Windows can be negotiated without changing translator or session contracts.
4. Keep realization code skeletal until the provider boundary is validated.

Exit criteria:

- planner can accept, degrade, or reject Windows requests explicitly
- no Linux-specific assumptions remain in runtime-model or planner crates

### Phase 15: macOS provider foundation

Goal:

- prove that the Linux-first runtime can admit macOS providers with explicit entitlement and install constraints

Deliverables:

- `gr-provider-macos-hid`
- `HostPlatform::Macos` planner inventory entries
- provider support-report contracts for entitlement and install prerequisites
- planner diagnostics for unsupported or unavailable macOS realizations

Tasks:

1. Define macOS provider inventory and diagnostics types.
2. Model entitlement, system-extension, or similar prerequisites in `BackendSupportReport` and `SessionPlan`.
3. Add provider-selection tests proving macOS can be negotiated without changing translator or session contracts.
4. Keep realization code skeletal until the provider boundary is validated.

Exit criteria:

- planner can accept, degrade, or reject macOS requests explicitly
- deployment prerequisites are visible without leaking platform APIs into core crates

## Risk areas

## Risk: backend complexity leaks into core types

Mitigation:

- keep `gr-backend-api` narrow
- forbid Linux-specific dependencies in `gr-core`, `gr-profiles`, `gr-config`, `gr-session-options`, and `gr-planner`
- review backend-related API additions carefully

## Risk: profiles become deployment policy

Mitigation:

- keep actual backend selection inside `gr-planner`
- let profiles describe requirements and supported levels only
- test planner behavior against varying backend inventories

## Risk: HID abstraction becomes too generic

Mitigation:

- keep profile-family-specific HID translators
- require descriptor/report compatibility tests
- avoid one universal HID packet model when the devices diverge materially

## Risk: reverse path slips behind forward path

Mitigation:

- require reverse translator interfaces before claiming `identity-aware` support
- require reverse-path integration tests before claiming support
- reject or explicitly degrade `identity-aware` plans when output or feature report handling is missing

## Risk: premature async complexity

Mitigation:

- keep core, profile, config, session-options, and planning crates runtime-agnostic
- introduce async only in session and backend layers if it proves useful

## Risk: per-session execution model does not scale

Mitigation:

- keep per-session isolation logical rather than thread-per-device
- use shared workers or async scheduling by default
- compile session options before activation
- use bounded queues and latest-state coalescing
- measure queue depth, drop/coalesce counts, and end-to-end latency

## Recommended implementation order

1. `gr-core`
2. `gr-profiles`
3. `gr-config`
4. `gr-session-options`
5. `gr-runtime-model`
6. `gr-backend-api`
7. `gr-planner`
8. fake backends in `gr-testkit`
9. `gr-translators`
10. `gr-session`
11. `gr-host-bridge`
12. `gr-provider-linux-uinput`
13. `gr-provider-linux-uhid`
14. host output bridge integration
15. `gr-provider-linux-transport`
16. `gr-provider-windows-hid`
17. `gr-provider-macos-hid`

## Final guidance

If there is a single rule to preserve while implementing this in Rust, it is this:

keep profile definition, session-option compilation, planning, translation, and backend realization as separate layers, and make every concrete device instance belong to an explicit session.
