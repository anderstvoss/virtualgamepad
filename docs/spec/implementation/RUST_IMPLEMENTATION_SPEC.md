# Rust Implementation Specification

This document is the build-facing implementation specification for the Rust version of `VirtualGamepad`.

It refines the strategy in [RUST_IMPLEMENTATION_PLAN.md](../implementation/RUST_IMPLEMENTATION_PLAN.md) into concrete crate boundaries, type ownership, runtime contracts, scheduling behavior, and testing requirements.

It is written to resolve ambiguity before code is started.

This document is authoritative for the Rust build. Planning documents may describe sequencing, but crate ownership, public runtime types, backend contracts, translator contracts, and acceptance criteria should be updated to match this specification when wording diverges.

Related documents:

- [ARCHITECTURE_SPEC.md](../specs/ARCHITECTURE_SPEC.md)
- [RUST_IMPLEMENTATION_PLAN.md](../implementation/RUST_IMPLEMENTATION_PLAN.md)
- [CONFIGURATION_SPEC.md](../specs/CONFIGURATION_SPEC.md)
- [FIDELITY_GUIDE.md](../specs/FIDELITY_GUIDE.md)
- [TEST_PLAN.md](../validation/TEST_PLAN.md)

## Versioning and stability

The library is pre-1.0. Minor versions may break the public API. All workspace crates are versioned in lockstep and released together. Profile additions are additive; profile removals require a minor-version bump and a clear deprecation note in the changelog. Provider-crate additions are additive. The `#[non_exhaustive]` markers on `OutputFunctionRef`, `OutputPayload`, `ProfileInputPayload`, and similar reserve room for additive variant additions without bumping.

## Purpose

The Rust implementation is a Linux-prioritized standalone library with explicit cross-platform planning boundaries that:

- creates host-visible virtual controller instances through platform providers
- translates exact profile-specific controller input into those virtual devices
- returns normalized reverse-path commands to the embedding program
- scales to many concurrent virtual devices
- prioritizes minimal steady-state latency for production applications
- maintains performance under large active session counts
- remains stable under long-lived production workloads where virtual devices must not fail casually
- stays gamepad-oriented while permitting adjacent device profiles that fit the same session, planning, and reverse-path architecture

The embedding program is assumed to own:

- physical input capture
- controller input production
- profile and fidelity selection
- reverse-command consumption

Universality boundary:

- the implementation may support gamepad-adjacent device profiles when they can be expressed through the same profile, planner, backend, and reverse-command contracts
- the implementation must not broaden into a general arbitrary-device or arbitrary-USB framework

## Fidelity support claim rules

The implementation must not mark a fidelity tier as supported unless the tier's validation path passes end to end.

- `compatibility` means a host-visible usable gamepad, usually through Linux `uinput`/evdev. It does not claim HID, USB, Bluetooth, or physical-device identity.
- `identity-aware` means HID descriptor/report identity for the selected profile family plus reverse output and feature report handling.
- `hardware-faithful` means transport-level enumeration and control-flow behavior over USB, Bluetooth, or another explicitly modeled bus, including transport-level reverse packet handling.
- if `hardware-faithful` is requested and only UHID is available, the planner must reject or explicitly degrade to `identity-aware` according to policy.
- if `identity-aware` is requested but reverse output or feature report handling is unavailable, the planner must reject or explicitly degrade rather than silently claiming support.

## Review-driven decisions

This specification resolves the main issues identified in the Rust plan review.

### Decision 1: `SessionPlan` is not a `gr-core` type

`gr-core` must remain dependency-free internally. Because `SessionPlan` contains compiled session-option data and backend-selection results, it cannot live in `gr-core`.

Resolution:

- `gr-core` owns primitive domain types only
- a new crate, `gr-runtime-model`, owns `SessionRequest`, `SessionPlan`, `PreparedSession`, `ControllerOutputCommand`, and runtime diagnostics snapshots

### Decision 2: reverse-path delivery is a runtime primitive, not a later add-on

The session runtime must be able to deliver reverse commands before real UHID work is considered “usable”.

Resolution:

- reverse event sinks are part of `gr-runtime-model`
- `gr-session` consumes them from day one
- `gr-host-bridge` provides convenience adapters, not the only possible delivery mechanism

### Decision 3: the runtime must be event-driven enough to scale

A naive “poll every backend session in a loop” design is not acceptable for many concurrent devices.

Resolution:

- backend sessions expose non-blocking event readiness integration or bounded event draining
- the session runtime uses shared workers and event queues
- no design should require one dedicated thread per virtual controller

### Decision 4: translators need a prepared execution context

Passing a full `SessionPlan` to every translation call is too vague and risks hot-path branching.

Resolution:

- session startup produces a `PreparedTranslationContext`
- forward and reverse translators use that prepared context rather than ad hoc plan traversal

### Decision 5: fidelity tiers share a planner but not one flattened runtime model

Trying to force `compatibility`, `identity-aware`, and `hardware-faithful` into one identical internal execution model would either make lower tiers too heavy or make higher tiers too cramped for future device behavior.

Resolution:

- the planner still negotiates one public `SessionPlan`
- session preparation may produce tier-specific prepared state behind stable public handles
- `hardware-faithful` sessions may own richer transport state (and, in a future version, attached-function state) than `identity-aware` sessions
- `identity-aware` sessions may own richer report and feature state than `compatibility` sessions

### Decision 6: reverse commands need both normalized and profile-specific forms

Common reverse behaviors should be easy for hosts to consume, but the runtime must not lose information for uncommon or device-unique commands.

Resolution:

- `ControllerOutputCommand` remains the common host-facing container
- shared functions such as rumble, LEDs, trigger effects, and audio use normalized function and payload variants where feasible
- profile-specific commands must have typed escape hatches rather than being dropped or coerced into misleading generic enums

### Decision 7 (deferred): attached-function modeling

Attached-function modeling (expansion ports, accessory channels, side channels) is deferred from v1. Re-introduction is intended to be additive — `OutputFunctionRef`, `OutputPayload`, and `PreparedSessionModel` are all `#[non_exhaustive]` or extensible enums.

Isolation rule (still load-bearing for v1):

- transport channels, endpoint state, and handshake-sensitive state belong to `PreparedTransportSession` unless a lower tier explicitly realizes them
- lower-tier prepared session models must not carry dormant transport state merely for structural symmetry
- when attached-function support is re-introduced, its routing data is also confined to `PreparedTransportSession`

### Decision 8: production goals require explicit hot-path and failure-isolation rules

Performance, scalability, and stability should be treated as build-facing constraints rather than aspirational properties.

Resolution:

- hot-path translation and dispatch must avoid avoidable allocation, repeated lookup, and cross-session coordination
- the session runtime must isolate provider failures per session where possible
- diagnostics and background reporting must not block steady-state dispatch

Provider-reporting rule:

- backend support reporting must be capability-granular enough for the planner to reason about reverse-path coverage and feature-report coverage
- unknown capability coverage must be treated as unsupported for support-claim purposes

## Final crate layout

The implementation should use the following workspace layout instead of the slightly looser one in the plan.

```text
virtualgamepad/
  Cargo.toml
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

## Build configuration

Provider crates are platform-gated; core crates are not.

- `gr-provider-linux-uinput`, `gr-provider-linux-uhid`, `gr-provider-linux-transport` are `#[cfg(target_os = "linux")]` at the crate root; depending on them on non-Linux is a compile error by design
- `gr-provider-windows-hid` is `#[cfg(target_os = "windows")]`
- `gr-provider-macos-hid` is `#[cfg(target_os = "macos")]`
- workspace-level features control which provider crates are built: `provider-linux-uinput`, `provider-linux-uhid`, `provider-linux-transport`, `provider-windows-hid`, `provider-macos-hid`; defaults enable only providers matching the host target
- the core set — `gr-core`, `gr-profiles`, `gr-config`, `gr-session-options`, `gr-runtime-model`, `gr-backend-api`, `gr-planner`, `gr-translators`, `gr-session`, `gr-host-bridge` — must build cleanly on Linux, macOS, and Windows with no `cfg(target_os = …)` paths inside them
- `gr-testkit` and `gr-cli` follow the core-set rule

## Crate ownership matrix

### `gr-core`

Owns:

- identifiers and newtypes
- fidelity and backend enums
- profile-specific input ids and shared input metadata types
- semantic input/output function enums
- capability category enums
- shared error enums
- shared time/sequence metadata

Must not own:

- compiled session options
- session plans
- backend descriptors
- profile instances

### `gr-profiles`

Owns:

- built-in profile definitions
- capability declarations
- the `CapabilityRegistry` query API over the registered profile set
- supported function sets
- required function sets
- descriptor metadata templates
- profile-family classification

Profile extension rule:

- v1 ships a closed built-in registry. A public `ProfileRegistry::register_external` API is a v2 concern and is intentionally not designed yet. Hosts that need custom profiles before v2 fork `gr-profiles`.

### Profile identifier convention

`ProfileId` is a `gr-core` newtype around a lowercase ASCII string. Built-in profiles ship with stable, deliberately-chosen identifiers:

| Profile          | `ProfileId`         | Rust type tag       | `display_name`      |
| ---------------- | ------------------- | ------------------- | ------------------- |
| Xbox 360         | `xbox360`           | `Xbox360`           | `Xbox 360`          |
| DualSense        | `dualsense`         | `DualSense`         | `DualSense`         |
| Steam Controller | `steam-controller`  | `SteamController`   | `Steam Controller`  |
| Generic gamepad  | `generic-gamepad`   | `GenericGamepad`    | `Generic gamepad`   |

Format rules:

- lowercase ASCII; the alphabet is `[a-z0-9-]`
- kebab-case when the name has multiple distinct word components (`steam-controller`, `generic-gamepad`)
- collapse to one token when the identifier reads naturally as a compact brand+model (`xbox360`, `dualsense`)
- new identifiers introduced after v1 follow the same rule; the working assumption is kebab-case unless the model token is itself compact and unambiguous

Stability rules:

- a shipped `profile_id` is permanent; aliases are not designed for v1
- renaming a profile is a breaking change and requires a new identifier alongside removal of the old one in a major-version bump
- additions to the built-in set are append-only

Name distinctions:

- `display_name: &'static str` — the human label for the device *family*, e.g. `"Xbox 360"`. Used in `show-capabilities`, snapshots, config validation messages, and any docs surface. Lives on `ControllerProfile`.
- per-session `name: String` — the per-instance label the host application assigns to a live session, e.g. `"Xbox 360 Controller 1"`. Lives on the session handle, not on the profile. Out of scope for `gr-profiles`; flagged here so `display_name` is not conflated with it.

### `gr-config`

Owns:

- user-facing config schema
- parsing
- validation reports
- config normalization

Produces:

- `SessionConfig`
- `ConfigValidationReport`

### `gr-session-options`

Owns:

- session-option validation
- input validation policy compilation
- provider hint compilation
- reverse delivery policy compilation
- backpressure policy compilation

Ownership rule:

- this crate **compiles** policy values into `CompiledSessionOptions`; the type definitions for `ReverseEventDeliveryPolicy` and `BackpressurePolicy` live in `gr-runtime-model`

Produces:

- `CompiledSessionOptions`

Rules:

- `gr-session-options` does not compile semantic input mappings
- optional mapping or adaptation helpers may exist later outside the core runtime path
- any helper output must already match the selected profile input contract before it reaches `gr-session`
- `gr-session-options` must not become a second extensibility mechanism for transport-side behavior or profile-specific accessory routing

### `gr-runtime-model`

Owns the **type definitions** for cross-cutting runtime contracts:

- `SessionRequest`
- `SessionPlan`
- `PreparedSession`
- `PreparedTranslationContext`
- `ControllerOutputCommand`
- prepared reverse-command payload types
- `SessionStatusSnapshot`
- `SessionDiagnosticsSnapshot`
- `ReverseEventDeliveryPolicy` (type definition only; compilation lives in `gr-session-options`)
- `BackpressurePolicy` (type definition only; compilation lives in `gr-session-options`)

Reason:

- these types are cross-cutting runtime contracts
- they depend on both primitive domain types and compiled execution data

### `gr-backend-api`

Owns:

- backend capability declarations
- host-platform and provider-identification types
- backend inventory types
- backend factory traits
- backend session traits
- backend frame types
- backend reverse event types
- backend descriptor/open context types
- backend diagnostics types
- event-readiness abstraction

### `gr-planner`

Owns:

- fidelity negotiation
- backend-family selection
- host-platform and provider selection
- degradation logic
- unsupported-capability analysis
- creation of `SessionPlan`

Consumes:

- `SessionRequest`
- `CompiledSessionOptions`
- profile metadata
- backend inventory

### `gr-translators`

Owns:

- translator traits
- translator registry
- family-specific translators
- reverse translators
- descriptor compatibility checks

Consumes:

- `PreparedTranslationContext`
- backend frames and reverse events

### `gr-session`

Owns:

- manager
- session registry
- session lifecycle state machine
- scheduling
- state ingestion
- reverse event handling
- telemetry hooks

### `gr-host-bridge`

Owns:

- callback adapters
- bounded channel adapters
- blocking facade if needed
- FFI-safe bridging if needed later

### Concrete backend crates

`gr-provider-linux-uinput`, `gr-provider-linux-uhid`, `gr-provider-linux-transport`, `gr-provider-windows-hid`, and `gr-provider-macos-hid` each own:

- one or more `BackendFactory` implementations
- concrete `BackendSession` implementations
- provider-specific wire logic and deployment checks
- isolated unsafe code

### `gr-testkit`

Owns:

- fake backend factories
- fake backend sessions
- queue/backpressure test fixtures
- profile-input builders
- profile fixtures
- integration harnesses

### `gr-cli`

Owns:

- diagnostics commands
- session simulation
- config validation
- planning inspection

## Dependency rules

- `gr-core` depends on no internal crates
- `gr-profiles` depends on `gr-core`
- `gr-config` depends on `gr-core`
- `gr-session-options` depends on `gr-core`, `gr-config`, and `gr-profiles`
- `gr-runtime-model` depends on `gr-core` and `gr-session-options` because `SessionPlan` carries `Arc<CompiledSessionOptions>`. Hosts that want runtime types without configuration types depend on `gr-runtime-model` directly; the config-only types are not re-exported from it.
- `gr-backend-api` depends on `gr-core` and `gr-runtime-model`
- `gr-planner` depends on `gr-core`, `gr-profiles`, `gr-session-options`, `gr-runtime-model`, and `gr-backend-api`
- `gr-translators` depends on `gr-core`, `gr-profiles`, `gr-runtime-model`, and `gr-backend-api`
- `gr-session` depends on `gr-core`, `gr-runtime-model`, `gr-backend-api`, `gr-planner`, and `gr-translators`
- `gr-host-bridge` depends on `gr-runtime-model` and `gr-session`
- concrete backend crates depend on `gr-core`, `gr-runtime-model`, and `gr-backend-api`
- `gr-testkit` may depend on all runtime crates
- `gr-cli` depends only on public runtime crates

## Canonical runtime types

### `ControllerProfile`

```rust
pub struct ControllerProfile {
    pub profile_id: ProfileId,
    pub display_name: &'static str,
    pub profile_family: ProfileFamily,
    pub identity: ProfileIdentity,
    pub capabilities: ControllerCapabilities,
    pub supported_fidelity: &'static [FidelityTier],
    pub input_contract: ProfileInputContract,
    pub descriptor_templates: &'static [DescriptorTemplate],
    pub reverse_command_support: ReverseCommandSupport,
}
```

Accompanying types (defined in `gr-profiles`):

```rust
#[non_exhaustive]
pub enum ProfileFamily {
    GenericGamepad,
    Xbox360,
    DualSense,
    SteamController,
}

#[non_exhaustive]
pub struct ProfileIdentity {
    pub vendor_id: VendorId,
    pub product_id: ProductId,
    pub version: Option<u16>,
    pub transport_hints: &'static [TransportHint],
}

pub struct ControllerCapabilities {
    pub input: &'static [CapabilityItem],
    pub output: &'static [CapabilityItem],
}

pub struct CapabilityItem {
    pub category: CapabilityCategory,
    pub semantic: SemanticRef,
    pub optionality: Optionality,
    pub range: Option<ValueRange>,
}

pub struct ProfileInputContract {
    pub required_fields: &'static [InputFieldRef],
    pub optional_fields: &'static [InputFieldRef],
    pub ranges: &'static [InputFieldRange],
    pub delta_support: DeltaSupportRule,
}

pub struct DescriptorTemplate {
    pub fidelity: FidelityTier,
    pub descriptor: DescriptorBytes, // placeholder until real bytes ship
}

#[non_exhaustive]
pub struct ReverseCommandSupport {
    pub supported: &'static [OutputFunctionRef],
}
```

Rules:

- all `ControllerProfile` fields are `'static` so the v1 closed registry can hold profiles in `static` data without `Arc`
- `descriptor_templates` is the only field with realization-tier specificity; everything else is profile-intrinsic
- `#[non_exhaustive]` on `ProfileFamily`, `ProfileIdentity`, and `ReverseCommandSupport` so additive growth (new families, new identity fields, new normalized output functions) is non-breaking
- the profile set is append-only; renaming a `profile_id` is a breaking change (see [profile identifier convention](#profile-identifier-convention))
- `ProfileInputContract` is the planner-facing source of truth for what an inbound frame is allowed to look like; it lives next to the payload shape so the planner can validate frames before they reach translators
- `CapabilityItem::semantic` references `SemanticInputFunction` / `SemanticOutputFunction` from `gr-core` (input vs output disambiguated by the containing slice)
- a profile that declares an output capability under `capabilities.output` must also list the corresponding `OutputFunctionRef` under `reverse_command_support.supported`; the contract test in `gr-profiles` enforces this

### `ProfileInputFrame`

```rust
pub struct ProfileInputFrame {
    pub profile_id: ProfileId,
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    pub payload: ProfileInputPayload,
}
```

Rules:

- every frame is tied to exactly one target profile
- `payload` must match that profile's concrete input contract
- the runtime never re-interprets one variant as another; translators are dispatched off the discriminant only

### `ProfileInputPayload`

```rust
#[non_exhaustive]
pub enum ProfileInputPayload {
    GenericGamepad(GenericGamepadInput),
    Xbox360(Xbox360Input),
    DualSense(DualSenseInput),
    SteamController(SteamControllerInput),
}
```

Rules:

- closed enum over built-in profile families; per-variant payload structs preserve profile-specific shape
- `#[non_exhaustive]` so additive profile additions are non-breaking
- chosen for static dispatch, zero heap on the hot path, and uniform storage in `VirtualControllerManager`'s session registry
- out-of-tree profile registration is intentionally not designed yet; see the `gr-profiles` profile-extension note

### `ProfileInputDelta`

```rust
pub struct ProfileInputDelta {
    pub profile_id: ProfileId,
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    pub payload: ProfileInputDeltaPayload,
}
```

Rules:

- deltas are only valid within the chosen profile contract
- hosts may use full frames only if delta handling is unnecessary

### `SessionRequest`

```rust
pub struct SessionRequest {
    pub session_id: SessionId,
    pub profile_id: ProfileId,
    pub goal: EmulationGoal,
    pub requested_fidelity_tier: FidelityTier,
    pub host_platform_preference: Option<HostPlatform>,
    pub backend_preference: Option<BackendLevel>,
    pub provider_preference: Option<ProviderId>,
    pub host_metadata: SessionHostMetadata,
}
```

Rules:

- this is the only input accepted by session creation
- no ad hoc planner-only side channels
- `session_id` is assigned by the manager before the planner is invoked; the planner mirrors it onto `SessionPlan.session_id` and `SessionPlan.backend_open_context.session_id`. The planner does not invent ids.
- host-platform and provider preferences are hints, never permission to bypass planner validation
- strictness is sourced exclusively from `config.validation`; there is no separate `strictness` field on `SessionRequest`

### `CompiledSessionOptions`

```rust
pub struct CompiledSessionOptions {
    pub input_validation_policy: InputValidationPolicy,
    pub provider_hints: ProviderHints,
    pub delivery_policy: ReverseEventDeliveryPolicy,
    pub backpressure_policy: BackpressurePolicy,
}
```

Rules:

- immutable after creation
- shareable by reference within one session

### `SessionPlan`

```rust
pub struct SessionPlan {
    pub session_id: SessionId,
    pub profile_id: ProfileId,
    pub requested_goal: EmulationGoal,
    pub requested_fidelity_tier: FidelityTier,
    pub selected_level: BackendLevel,
    pub target_host_platform: HostPlatform,
    pub selected_backend_family: BackendFamily,
    pub selected_provider_id: ProviderId,
    pub selected_translator_family: TranslatorFamily,
    pub capability_result: CapabilityNegotiationResult,
    pub degradation: DegradationReport,
    pub warnings: Vec<PlannerWarning>,
    pub deployment_requirements: DeploymentRequirements,
    pub backend_open_context: BackendOpenContext,
    pub session_options: SessionOptionsSnapshot,
}
```

`BackendOpenContext` is defined in `gr-runtime-model` (re-exported by `gr-backend-api`) so `SessionPlan` can own it without `gr-runtime-model` taking a dependency on `gr-backend-api`.

`session_options` carries the serializable snapshot rather than the live `Arc<CompiledSessionOptions>`. The live shareable form is held by the session actor at runtime; the snapshot on the plan is for diagnostics, `plan-snapshot` fixtures, and trace replay.

Rules:

- created once during session startup
- not mutated on the hot path
- safe to snapshot for diagnostics
- must make deployment prerequisites inspectable before any provider is opened
- for `identity-aware` and `hardware-faithful`, must identify the selected backend provider, forward translator, reverse translator, enabled output capabilities, unsupported output capabilities, and degradation status
- must reject or explicitly degrade plans where requested bidirectional behavior cannot be realized; rejection uses [`PlanRejection`](#planrejection)

### `DegradationReason`

```rust
#[non_exhaustive]
pub enum DegradationReason {
    TransportNotRealizable {
        requested_backend_level: BackendLevel,
        available_backend_levels: Vec<BackendLevel>,
        reason: String,
    },
    ReversePathUnavailable,
    BackendDoesNotSupportFidelity { requested: FidelityTier, available: FidelityTier },
    ProviderHintIgnored { preferred: ProviderId, reason: String },
    BackendLevelHintIgnored { preferred: BackendLevel, reason: String },
    UnsupportedOutputCapability { function: SemanticOutputFunction, reason: String },
}
```

Rules:

- canonical reason set for `DegradationReport.reasons`; the planner emits these instead of free-text strings so consumers can match on `kind`
- variants are `#[non_exhaustive]`; new degradation reasons may be added without a breaking change
- free-form `reason: String` fields inside variants are reserved for context that does not fit a canonical code (e.g. "linux-uhid does not support transport-tier descriptors yet")

### `PlanRejection`

```rust
pub struct PlanRejection {
    pub profile_id: ProfileId,
    pub requested_goal: EmulationGoal,
    pub requested_fidelity_tier: FidelityTier,
    pub reasons: Vec<PlanRejectionReason>,
    pub considered_backends: Vec<BackendId>,
}

#[non_exhaustive]
pub enum PlanRejectionReason {
    NoBackendSupportsProfile {
        requested_backend_level: BackendLevel,
        available_backends: Vec<BackendId>,
        reason: String,
    },
    NoBackendSupportsFidelity { requested: FidelityTier },
    NoBackendSupportsHost { requested: HostPlatform },
    BidirectionalSupportRequired { missing: Vec<SemanticOutputFunction> },
}
```

Rules:

- returned from `gr-planner::plan_session` as the `Err` variant when no [`SessionPlan`] can be produced at any tier
- `considered_backends` records which inventory entries the planner queried; useful for diagnosing why a rejection occurred
- a `PlanRejection` and a `SessionPlan` are mutually exclusive for a given input

### Planner output contract

The planner's signature is:

```rust
fn plan_session(
    request: &SessionRequest,
    session_options: &CompiledSessionOptions,
    inventory: &[BackendInventoryEntry],
    factories: &[Arc<dyn BackendFactory>],
) -> Result<SessionPlan, PlanRejection>;
```

Rules:

- a successful `Ok(plan)` may still carry degradation; degradation is **not** rejection
- rejection is reserved for cases where no plan is possible at any tier given the inventory and hints
- a plan and a rejection are mutually exclusive for a given input
- the planner does not own factories; `factories` is a borrowed view so the manager retains ownership

### Planner hint policy

Hints on `SessionRequest` (`provider_preference`, `backend_preference`, `host_platform_preference`) shape selection but never bypass validation. Per-hint behavior:

- **`provider_preference`**: if the named provider is absent from the inventory or its `can_realize` returns `SupportLevel::None`, the planner falls through to default selection and records `DegradationReason::ProviderHintIgnored { preferred, reason }`. The hint never causes rejection on its own.
- **`backend_preference`**: if no factory matches the preferred `BackendLevel`, falls through to default selection and records `DegradationReason::BackendLevelHintIgnored { preferred: BackendLevel, reason }`.
- **`host_platform_preference`**: must match exactly. A mismatch is a **rejection** (`PlanRejectionReason::NoBackendSupportsHost`), not a degradation — host platform is the binding constraint that gates which factories are even considered.

Tie-breaking when multiple backends satisfy the same family at the same level:

1. backend matching `provider_preference` (if any)
2. higher `BackendLevel` (Transport > Hid > Evdev — closer to hardware-faithful)
3. first in inventory order

### `PreparedTranslationContext`

```rust
pub struct PreparedTranslationContext {
    pub session_id: Option<SessionId>,
    pub profile_family: Option<String>,
    pub host_platform: Option<HostPlatform>,
    pub backend_family: Option<BackendFamily>,
    pub provider_id: Option<ProviderId>,
    pub level: Option<BackendLevel>,
    pub session_options: Option<SessionOptionsSnapshot>,
    /// Live reference into `gr-profiles` static data; `#[serde(skip)]`
    /// because `&'static` references are not deserializable.
    pub descriptor_template: Option<&'static gr_profiles::DescriptorTemplate>,
    pub translation_constants: TranslationConstants,
}
```

Purpose:

- remove repeated lookup work from the per-frame path
- let translators operate with concrete prepared data, including the live descriptor bytes via `descriptor_template`
- let reverse translators route both normalized and profile-specific commands without repeated registry lookup

The Optional fields are an artifact of the Phase 3 skeleton stage; Phase 6's `prepared_translation_context(plan, registry)` (see [Translator contracts](#translator-contracts)) populates them all and Phase 7's session runtime consumes the result by reference for the lifetime of the session.

### Tier-specific prepared session state

`PreparedSession` may internally contain one of several tier-specific prepared models as long as the public session handle remains stable.

```rust
#[non_exhaustive]
pub enum PreparedSessionModel {
    Compatibility(PreparedCompatibilitySession),
    IdentityAware(PreparedIdentitySession),
    HardwareFaithful(PreparedTransportSession),
}
```

Rules:

- `PreparedCompatibilitySession` should stay minimal and focused on gameplay-input realization
- `PreparedIdentitySession` should own descriptor/report state, feature negotiation state where needed, and reverse report routing data
- `PreparedTransportSession` should own transport state machines, control-flow state, and timing-sensitive state where needed; this is also where attached-function routing data will live when re-introduced
- later tiers may add state that lower tiers do not carry, but lower tiers must not be forced to simulate transport concerns they do not realize
- session preparation should front-load as much lookup and capability routing work as practical so steady-state dispatch remains low-latency

### `ControllerOutputCommand`

```rust
pub struct ControllerOutputCommand {
    pub session_id: SessionId,
    pub profile_id: ProfileId,
    pub timestamp: Timestamp,
    pub command_type: OutputCommandType,
    pub function: OutputFunctionRef,
    pub payload: OutputPayload,
}
```

Rules:

- all reverse-path outputs return through this host-facing type
- every command carries session id, profile id, timestamp, command type, target function reference, and typed payload
- audio events use typed payload variants rather than unstructured blobs when feasible
- common reverse-path outputs should use normalized function references and payloads
- profile-specific commands must be representable without pretending they are one of the normalized common functions
- v1 payloads do not carry attached-function references; this is intentionally deferred

```rust
#[non_exhaustive]
pub enum OutputFunctionRef {
    Semantic(SemanticOutputFunction),
    ProfileSpecific(ProfileSpecificOutputFunctionId),
}
```

```rust
#[non_exhaustive]
pub enum OutputPayload {
    Rumble(RumblePayload),
    Lighting(LightingPayload),
    TriggerEffect(TriggerEffectPayload),
    Audio(AudioCommand),
    FeatureRequest(FeatureRequestPayload),
    ProfileSpecific(ProfileSpecificOutputPayload),
}
```

`#[non_exhaustive]` reserves room for additive future variants — most notably the deferred `AttachedFunction` variants — without requiring a breaking change.

Normalization policy rules:

- add a new normalized semantic output only when at least two profile families share materially equivalent behavior and host-side meaning
- otherwise prefer `ProfileSpecific` payloads over expanding the normalized enum surface
- profile-specific payloads must still be typed enough for routing, logging, replay, and bridge integration
- adding a new normalized semantic output should require a short rationale naming the profile families and host-side meaning being unified

### Audio stream contract

`OutputPayload::Audio` carries discrete audio events only — mode changes, mute toggle, route selection, gain change. It does not carry PCM frames.

Continuous audio (controller speaker output, controller microphone input) flows over a separate per-session sink:

```rust
impl VirtualControllerSessionHandle {
    pub fn audio_sink(&self) -> Option<AudioStreamSink>;
    pub fn audio_source(&self) -> Option<AudioStreamSource>;
}
```

Rules:

- `audio_sink` returns `Some` only for profiles that declare a speaker capability **and** for sessions whose selected provider can realize PCM output at the chosen fidelity tier; otherwise `None`
- `audio_source` returns `Some` only for profiles that declare a microphone capability with realizable provider support; otherwise `None`
- the `identity-aware` tier may claim audio mode commands (`OutputPayload::Audio`) without claiming PCM stream support; declaring PCM stream support requires that the provider actually realizes it
- the discrete-command path and the stream path are independent and have independent backpressure policies
- absent audio capabilities the methods return `None` so the host does not branch on backend support out of band

## Backend API contracts

### Common type vocabulary

The trait definitions in this section depend on the data types defined below. All types live in `gr-backend-api`, derive `Debug + Clone + PartialEq + Eq + Serialize + Deserialize` unless otherwise noted, and are `#[non_exhaustive]` where listed so additive variants do not break providers.

### `BackendError`

```rust
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendError {
    WouldBlock,
    OpenFailed { reason: String },
    WriteFailed { reason: String },
    CloseFailed { reason: String },
    ReverseEventParseFailed { reason: String },
    SessionClosed,
    Unsupported { reason: String },
}
```

Rules:

- `WouldBlock` is the only variant the session runtime treats as recoverable on the steady-state hot path; the session must re-arm via `readiness()` before retrying
- variants must map cleanly onto the machine-readable causes enumerated in [Error taxonomy](#error-taxonomy); new variants require updating that list
- providers must not coerce structurally distinct failures into the same variant

### `BackendFrame`

```rust
#[non_exhaustive]
pub enum BackendFrame {
    HidInputReport { report_id: Option<u8>, bytes: Vec<u8> },
    HidFeatureReport { report_id: u8, bytes: Vec<u8> },
    TransportPacket { endpoint_id: u8, bytes: Vec<u8> },
    EvdevEvents { events: Vec<EvdevEvent> },
}

pub struct EvdevEvent {
    pub event_type: u16,  // serde-renamed to `type` in fixtures
    pub code: u16,
    pub value: i32,
}
```

Rules:

- variants correspond one-to-one with `BackendLevel` (Evdev, Hid, Transport); a translator emits the variant matching the selected level
- `HidInputReport::report_id = None` represents the single-report HID convention
- providers must reject `BackendFrame` variants outside their advertised level (returning `BackendError::Unsupported`) rather than silently coercing

### `BackendReverseEvent`

```rust
pub struct BackendReverseEvent {
    pub session_id: SessionId,
    pub profile_id: Option<ProfileId>,
    pub timestamp: Timestamp,
    pub sequence: SequenceId,
    pub kind: BackendReverseEventKind,
    pub target: Option<BackendReverseTarget>,
    pub payload: BackendReversePayload,
}

#[non_exhaustive]
pub enum BackendReverseEventKind {
    HidOutputReport, HidFeatureReport, TransportPacket, EvdevEvent,
}

#[non_exhaustive]
pub enum BackendReverseTarget {
    SemanticOutput(SemanticOutputFunction),
    ProfileSpecificOutput(ProfileSpecificOutputFunctionId),
    ReportId(u8),
    EndpointId(u8),
}

#[non_exhaustive]
pub enum BackendReversePayload {
    Hid { report_id: Option<u8>, bytes: Vec<u8> },
    Transport { endpoint_id: u8, bytes: Vec<u8> },
    Evdev { events: Vec<EvdevEvent> },
}
```

Backend reverse events represent host-to-device traffic observed at the selected emulation layer.

Examples:

- HID output reports
- HID feature reports
- transport packets
- rumble commands
- lighting commands
- trigger-effect commands
- audio and mode commands

Rules:

- every event must carry session id, profile id where known, timestamp, sequence id (for diagnostic ordering), event kind, target function or capability where known, and typed payload
- backend reverse events are untrusted until reverse-translated and validated against declared output capabilities
- backend reverse events must be able to identify profile-specific channels, report ids, or endpoints when the selected tier exposes them
- providers should preserve raw discriminators such as report ids, endpoint ids, or transport channel ids when those are needed for stable reverse translation

### `BackendDiagnostics`

```rust
pub struct BackendDiagnostics {
    pub backend_id: BackendId,
    pub family: BackendFamily,
    pub state: BackendState,
    pub frames_sent: u64,
    pub reverse_events_drained: u64,
    pub write_failures: u64,
    pub last_error: Option<String>,
    pub vendor_counters: BTreeMap<String, u64>,
}

pub enum BackendState { NotOpen, Open, Closed, Failed }
```

Rules:

- snapshot-friendly: `BackendSession::diagnostics()` returns by value and must not allocate on the hot path beyond cloning `last_error` and `vendor_counters`
- counters align with the per-session telemetry surface in [Telemetry requirements](#telemetry-requirements)
- `vendor_counters` is the escape hatch for provider-specific metrics that do not warrant a normalized counter

### `BackendOpenContext`

```rust
pub struct BackendOpenContext {
    pub session_id: SessionId,
    pub profile_id: ProfileId,
    pub fidelity_tier: FidelityTier,
    pub backend_level: BackendLevel,
    pub host_platform: HostPlatform,
}
```

Rules:

- intentionally lean: descriptor templates, compiled session options, and translator context reach the backend via sibling fields on `SessionPlan` (`session_options`, `backend_open_context`) or via the provider's own configuration — `BackendOpenContext` carries only the identity a backend needs to open
- `gr-backend-api` must not gain a dependency on `gr-session-options` or `gr-translators` through this type
- a backend that opens successfully must record the context for its own diagnostics

### `BackendRealizationRequest`

```rust
pub struct BackendRealizationRequest {
    pub profile_id: ProfileId,
    pub requested_goal: EmulationGoal,
    pub requested_fidelity_tier: FidelityTier,
    pub host_platform: HostPlatform,
    pub required_output_functions: Vec<SemanticOutputFunction>,
}
```

Rules:

- intentionally smaller than a full `SessionPlan`; the planner should not need translator internals to ask support questions
- `required_output_functions` lists semantic outputs the host has indicated are needed; the support report explains how the backend covers them. The planner populates this with every `SemanticOutput` capability the profile declares — host-level opt-out (selectively disabling capabilities) is a future concern. At plan time the planner asks about all declared outputs so the support report is complete.

### `BackendSupportReport`

```rust
pub struct BackendSupportReport {
    pub forward_support: SupportLevel,
    pub reverse_support: SupportLevel,
    pub supported_output_functions: Vec<SemanticOutputFunction>,
    pub unsupported_output_functions: Vec<UnsupportedOutputFunction>,
    pub notes: Vec<String>,
}

pub enum SupportLevel { Full, Partial, None }

pub struct UnsupportedOutputFunction {
    pub function: SemanticOutputFunction,
    pub reason: String,
}
```

Rules:

- forward and reverse support are reported independently; a generic supported/unsupported boolean would hide implementation gaps
- `unsupported_output_functions[].reason` must be human-readable and machine-stable enough for the planner to log
- `notes` is free-form provider commentary (e.g. "requires kernel 5.14+", "RGB requires firmware ≥ 0.7")

### `BackendInventoryEntry`

```rust
pub struct BackendInventoryEntry {
    pub backend_id: BackendId,
    pub family: BackendFamily,
    pub level: BackendLevel,
    pub host_platform: HostPlatform,
    pub supported_fidelity_tiers: Vec<FidelityTier>,
    pub notes: Vec<String>,
}
```

Rules:

- the manager builds its inventory from these entries at composition time
- entries are static for the lifetime of a `BackendFactory` instance; dynamic capability negotiation belongs in `can_realize` / `BackendSupportReport`

### `BackendFactory`

```rust
pub trait BackendFactory: Send + Sync {
    fn backend_id(&self) -> BackendId;
    fn family(&self) -> BackendFamily;
    fn inventory_entry(&self) -> BackendInventoryEntry;
    fn can_realize(&self, request: &BackendRealizationRequest) -> BackendSupportReport;
    fn open_session(
        &self,
        context: &BackendOpenContext,
    ) -> Result<Box<dyn BackendSession>, BackendError>;
}
```

Notes:

- `BackendRealizationRequest` is intentionally smaller than full `SessionPlan`
- planner should not need translator internals to ask support questions
- support reporting should not collapse partial reverse-path support into a generic supported/unsupported boolean when that would hide implementation gaps

### `BackendSession`

```rust
pub trait BackendSession: Send {
    fn session_id(&self) -> SessionId;
    fn open(&mut self) -> Result<(), BackendError>;
    fn send(&mut self, frame: BackendFrame) -> Result<(), BackendError>;
    fn drain_reverse_events(
        &mut self,
        out: &mut dyn BackendReverseEventSink,
    ) -> Result<(), BackendError>;
    fn readiness(&self) -> EventReadiness;
    fn diagnostics(&self) -> BackendDiagnostics;
    fn close(&mut self) -> Result<(), BackendError>;
}

pub trait BackendReverseEventSink {
    fn push(&mut self, event: BackendReverseEvent);
}

impl<T> BackendReverseEventSink for T
where
    T: Extend<BackendReverseEvent>,
{
    fn push(&mut self, event: BackendReverseEvent) {
        self.extend(std::iter::once(event));
    }
}
```

Why `drain_reverse_events` instead of `try_recv_reverse_event`:

- reduces repeated call overhead
- allows bounded batched draining
- works better with shared schedulers

Why `&mut dyn BackendReverseEventSink` instead of `&mut dyn Extend<BackendReverseEvent>` or a concrete `SmallVec`:

- backends do not dictate the container choice or stack-buffer size
- `Extend` is not dyn-compatible (it has a generic method); a one-method sink trait is, so the session runtime can supply a reusable per-session collector behind a `&mut dyn` reference
- a blanket impl over any `Extend<BackendReverseEvent>` means `Vec`, `SmallVec`, and bespoke collectors all satisfy the sink without ceremony at the call site

### Backend blocking contract

`send` and `drain_reverse_events` must be non-blocking. The session runtime never wraps backend calls in `spawn_blocking` and a backend that blocks is a contract violation.

Provider requirements:

- file descriptors must be opened `O_NONBLOCK` (or the platform equivalent); on Windows providers must use overlapped I/O or equivalent non-blocking primitives
- when a call would block, the backend returns `BackendError::WouldBlock` and the session re-arms via `readiness()` before retrying
- `open` and `close` are allowed to perform short bounded blocking work (device creation, teardown) because they are control-plane operations, not steady-state dispatch

v1 deliberately ships a sync trait only. An `async` variant of `BackendSession` may be added later as an additive trait if a provider proves it cannot meet the non-blocking contract; until then, all providers must.

### `EventReadiness`

`EventReadiness` must remain cross-platform. The handle variant is split so `gr-backend-api` itself builds on all targets:

```rust
pub enum EventReadiness {
    AlwaysPoll,
    NoReverseEvents,
    Readable(ReadinessHandle),
    UserEventToken(u64),
}

#[cfg(unix)]
pub struct ReadinessHandle(pub std::os::fd::RawFd);

#[cfg(windows)]
pub struct ReadinessHandle(pub std::os::windows::io::RawHandle);
```

Rules:

- the session runtime is responsible for cfg-gated readiness integration (for example `mio` on unix, IOCP on Windows)
- `gr-backend-api` must compile on Linux, macOS, and Windows; no platform-specific dependency may leak in through this type
- providers that cannot expose a readiness primitive return `AlwaysPoll` or `NoReverseEvents`
- `EventReadiness` and `ReadinessHandle` deliberately do **not** derive `Serialize` / `Deserialize`: a raw FD or `HANDLE` is a runtime resource handle, not fixture content, and round-tripping one through YAML would be a category error
- a `#[cfg(not(any(unix, windows)))]` `ReadinessHandle(u64)` fallback exists so the crate continues to build on exotic targets used for documentation builds and tooling

This allows the session runtime to avoid naive N-session polling where possible.

## Translator contracts

### `ForwardTranslator`

```rust
pub trait ForwardTranslator: Send + Sync {
    fn family(&self) -> TranslatorFamily;
    fn translate(
        &self,
        input: &ProfileInputFrame,
        ctx: &PreparedTranslationContext,
        out: &mut TranslationScratch,
    ) -> Result<BackendFrame, TranslationError>;
}
```

Notes:

- `TranslationScratch` is a reusable per-session scratch area
- translators should avoid allocation during steady-state translation

### `ReverseTranslator`

```rust
pub trait ReverseTranslator: Send + Sync {
    fn family(&self) -> TranslatorFamily;
    fn translate_reverse(
        &self,
        event: &BackendReverseEvent,
        ctx: &PreparedTranslationContext,
        out: &mut SmallVec<[ControllerOutputCommand; 4]>,
    ) -> Result<(), TranslationError>;
}
```

Notes:

- reverse translation is batched and allocation-aware
- reverse translators may emit a mix of normalized commands and profile-specific commands in the same batch

### `TranslationError` and `TranslationScratch`

```rust
#[non_exhaustive]
pub enum TranslationError {
    NoTranslatorRegistered { family: TranslatorFamily, level: BackendLevel },
    InvalidInput { reason: String },
    InvalidReverseEvent { reason: String },
    DescriptorUnavailable,
    DescriptorViolation { reason: String },
}

pub struct TranslationScratch {
    pub bytes: Vec<u8>,
}
```

Rules:

- both types live in `gr-translators`
- `TranslationError` is `#[non_exhaustive]` so additive variants do not require an API bump
- `TranslationScratch::clear()` empties the buffer without releasing its allocation, so the session actor's per-call reset is allocation-free in steady state
- `NoTranslatorRegistered` is the only variant the planner can encounter during context construction; the rest are emitted by `translate()` / `translate_reverse()` on the hot path

### `TranslatorRegistry`

```rust
pub struct TranslatorRegistry { /* zero-sized facade */ }

impl TranslatorRegistry {
    pub const fn new() -> Self;
    pub fn forward(&self, family: TranslatorFamily, level: BackendLevel)
        -> Option<&'static dyn ForwardTranslator>;
    pub fn reverse(&self, family: TranslatorFamily)
        -> Option<&'static dyn ReverseTranslator>;
}
```

Rules:

- closed v1: mirrors the `gr-profiles::CapabilityRegistry` pattern. The registry is a zero-sized facade over `&'static` data populated at compile time with the per-family translator implementations
- lookup is `(family, level)` for forward translators because the same family may have distinct evdev / HID / transport translators (e.g. `DualSense` family at `Hid` level uses `DualSenseUsbHidTranslator`, at `Evdev` level uses a generic evdev translator)
- reverse translators are keyed on family alone because reverse events arrive already classified by `BackendReverseTarget` (semantic vs. report-id vs. endpoint) and the translator dispatches internally
- no runtime registration; plugin-style extension is intentionally deferred

### `PreparedTranslationContext` construction

```rust
pub fn prepared_translation_context(
    plan: &SessionPlan,
    registry: &TranslatorRegistry,
) -> Result<PreparedTranslationContext, TranslationError>;
```

Rules:

- canonical construction path; lives in `gr-translators`
- the manager (Phase 7+) calls this once at session startup; the result is shared by reference within the session for the session's lifetime
- populates `descriptor_template` from the `gr-profiles` static descriptor data matching the plan's selected profile + fidelity tier
- returns `TranslationError::NoTranslatorRegistered` when the plan names a `(family, level)` pair the closed registry does not cover
- returns `TranslationError::DescriptorUnavailable` when the plan's fidelity tier has no descriptor template in the profile's `descriptor_templates` list

### Translator semantics

Translators are **descriptor-driven**. Per-profile byte and bit mappings are defined by the live `gr_profiles::DescriptorTemplate` referenced from `PreparedTranslationContext.descriptor_template`, not duplicated in this document. To change a mapping, update the descriptor; translator code follows the descriptor's layout.

This document therefore does not enumerate every "Xbox button A → HID byte N bit M" mapping. Such mappings are implementation details verified by translator round-trip tests against the descriptor.

### `capability-coverage` translator gap detection

Beyond the Phase 2 registry self-consistency rules, `gr-cli capability-coverage` (post-Phase-6) must additionally verify, for every built-in profile:

- every declared `OutputFunctionRef::Semantic` has a `ReverseTranslator` registered for the profile's family
- every profile that declares an `Hid` or `Transport` level descriptor template has both a `ForwardTranslator` for `(family, level)` and a `ReverseTranslator` for `family`
- the registered forward translator's family matches the descriptor template's family (no cross-family mismatches)
- the registered reverse translator's family matches the same

These checks turn the deferred "translator-coverage gap detection" from Phase 2 into hard rules. The Phase 6 PR implements the assertions; this section is the spec they must match.

## Session runtime model

### Top-level types

```rust
pub struct VirtualControllerManager { /* ... */ }
pub struct VirtualControllerSessionHandle { /* ... */ }
```

### Manager responsibilities

- hold backend inventory
- own session registry
- create and destroy sessions
- route host input to session actors
- expose diagnostics snapshots
- maintain constant-time or equivalent session lookup under large active session counts
- isolate one session's provider failure from unrelated active sessions where possible

### Session responsibilities

- own one `BackendSession`
- own one prepared translation context
- own one tier-specific prepared session model
- own one forward translator
- own one reverse translator
- own one bounded input queue
- own one bounded output queue or sink adapter
- own one session-local telemetry accumulator
- keep steady-state input-to-write latency low by avoiding control-plane work on the hot path

### Provider registration

Providers are registered explicitly by the host program. There is no automatic discovery, plugin loader, or `inventory`-style registry in v1.

- each `gr-provider-*` crate exposes a public `factory()` constructor returning an `Arc<dyn BackendFactory>`
- the host composes the desired set and passes them via `VirtualControllerManager::with_backends`
- `VirtualControllerManager::new` returns a manager with an empty backend inventory; callers can register lazily later if needed

This keeps platform-specific dependencies out of the manager's own dependency graph and makes provider composition fully explicit in the host's `Cargo.toml`.

### Reverse-event delivery threading

`SessionOutputSubscription` callbacks and channel sinks are filled by a delivery worker that is decoupled from the session actor.

- callback subscriptions: the callback runs on the delivery worker, never on the session actor and never on the caller's thread
- channel subscriptions: the delivery worker pushes into the channel; the consumer's thread is the consumer's concern
- the bounded reverse-event queue between the session actor and the delivery worker is what guarantees slow-consumer isolation; the session actor never waits on the delivery worker
- callbacks must not call back into the originating `VirtualControllerSessionHandle` synchronously; doing so risks deadlock and is documented as undefined behavior at the API level (this is a contract, not enforced at compile time)

## Queue and backpressure policy

### Input queue

Each session must have a bounded input queue.

Required default policy:

- queue capacity defaults to a small bounded value such as `4` or `8`
- if a new state arrives while the queue is full, coalesce to “latest state wins”
- preserve a counter of coalesced frames
- preserve the latest sequence id for observability

Why:

- controller state is temporal and supersedable
- stale frames are less useful than latest state

### Reverse-event queue

Each session must have a bounded reverse-event delivery path.

Required default policy:

- queue capacity defaults to a bounded size such as `32`
- overflow policy is configurable:
  - `DropNewest`
  - `DropOldest`
  - `BlockProducer` only where explicitly enabled
- dropped-event counters are mandatory diagnostics
- reverse-event delivery should preserve low latency for active write paths even under reverse-command bursts

Why:

- a slow consumer must not stall unrelated controller sessions by default

## Scheduling model

### Non-goals

The runtime must not require:

- one OS thread per device
- busy-polling all sessions at fixed frequency
- per-frame allocation on the hot path

### Required behavior

- one logical actor per session
- shared worker pool or async runtime
- per-session ordered input processing
- per-session serialized backend writes
- readiness-based reverse-event scheduling when available
- explicit separation between control-plane mutation and steady-state data-plane dispatch

### Recommended implementation

Preferred first implementation:

- `tokio` runtime in `gr-session`
- one task per logical session
- bounded `mpsc` for input delivery
- bounded `mpsc` or sink adapter for output delivery
- optional `mio` or async-fd integration for readiness-aware backends

Alternative acceptable implementation:

- one shared thread pool
- explicit work-stealing or dispatch queue
- no public async API required

## Session lifecycle

### Startup sequence

1. Host submits `SessionRequest`.
2. `gr-config` validates config.
3. Optional internal adapter preparation runs if enabled.
4. `gr-planner` produces `SessionPlan`.
5. `gr-translators` resolves translator family and builds `PreparedTranslationContext`.
6. `gr-session` requests backend creation from selected factory.
7. `BackendSession::open()` succeeds.
8. Session actor registers with manager and transitions to `Active`.

### Steady-state input sequence

1. Host submits profile-specific input accepted by the selected session contract.
2. State is assigned a `SequenceId`.
3. Session input queue coalesces if needed.
4. Session actor loads latest state.
5. Forward translator writes into reusable scratch.
6. Backend session sends the frame.
7. Telemetry counters update locally.

### Reverse-event sequence

1. Backend becomes ready or is polled according to `EventReadiness`.
2. Session actor drains reverse events into a small stack buffer.
3. Reverse translator normalizes them into `ControllerOutputCommand`.
4. Commands are delivered to the configured sink.
5. Delivery successes, drops, and latency stats are recorded.

### Shutdown sequence

1. Manager marks session as `Stopping`.
2. Input queue stops accepting new state.
3. Pending work drains according to policy.
4. Backend session closes.
5. Final diagnostics snapshot is persisted or exposed.
6. Session registry entry is removed or archived.

## Public Rust API shape

The public API should optimize for direct library use rather than over-abstraction.

### Manager construction

```rust
impl VirtualControllerManager {
    pub fn new(config: ManagerConfig) -> Self;
    pub fn with_backends(config: ManagerConfig, backends: Vec<Arc<dyn BackendFactory>>) -> Result<Self, ManagerError>;
}
```

### `ManagerConfig`

```rust
#[non_exhaustive]
pub struct ManagerConfig {
    pub session_input_queue_depth: usize,    // default 8
    pub session_reverse_queue_depth: usize,  // default 32
    pub worker_pool_size: Option<usize>,     // None = tokio runtime default
}
```

Rules:

- defaults match the input + reverse queue depth defaults in [Queue and backpressure policy](#queue-and-backpressure-policy)
- `worker_pool_size: None` defers to the tokio runtime default (typically `num_cpus`); explicit values pin the shared worker pool
- `#[non_exhaustive]` so later phases can add fields (e.g. delivery-worker tuning, telemetry toggles) without breaking host crates

### `ManagerError` and `SessionError`

```rust
#[non_exhaustive]
pub enum ManagerError {
    NoBackendsRegistered,
    PlanRejected(PlanRejection),
    BackendOpenFailed { backend_id: BackendId, source: BackendError },
    TranslatorContextFailed(TranslationError),
    SessionAlreadyActive { session_id: SessionId },
}

#[non_exhaustive]
pub enum SessionError {
    SessionClosed,
    SubscriptionClosed,
    AudioNotAvailable,
}

#[non_exhaustive]
pub enum SessionSendError {
    QueueFull,
    SessionClosed,
    InvalidInput { reason: String },
}
```

Rules:

- all three are `#[non_exhaustive]`; new variants land additively
- `ManagerError::BackendOpenFailed.source` carries the underlying `BackendError` via `#[source]` so error chains traverse cleanly
- `ManagerError::TranslatorContextFailed` implements `From<TranslationError>` so the manager's `create_session` body can use `?`
- `SessionSendError::QueueFull` is distinct from `SessionClosed`: the caller should back off and retry on `QueueFull`, treat `SessionClosed` as terminal

### Session lifecycle

```rust
impl VirtualControllerManager {
    pub fn create_session(&self, request: SessionRequest) -> Result<VirtualControllerSessionHandle, ManagerError>;
    pub fn close_session(&self, session_id: SessionId) -> Result<(), ManagerError>;
    pub fn session_status(&self, session_id: SessionId) -> Option<SessionStatusSnapshot>;
    pub fn diagnostics(&self, session_id: SessionId) -> Option<SessionDiagnosticsSnapshot>;
}
```

### Session input

```rust
impl VirtualControllerSessionHandle {
    pub fn send_input(&self, input: ProfileInputFrame) -> Result<(), SessionSendError>;
    pub fn send_input_delta(&self, delta: ProfileInputDelta) -> Result<(), SessionSendError>;
}
```

### Reverse output

```rust
pub trait OutputSink: Send {
    fn deliver(&mut self, command: ControllerOutputCommand);
}

pub struct SessionOutputSubscription { /* opaque handle */ }

impl SessionOutputSubscription {
    pub fn unsubscribe(self);
}

impl VirtualControllerSessionHandle {
    pub fn subscribe_outputs(
        &self,
        sink: Box<dyn OutputSink>,
    ) -> Result<SessionOutputSubscription, SessionError>;
}
```

Rules:

- the host provides the sink (callback adapter, channel adapter, stream adapter, or custom), the manager returns an opaque subscription handle
- `OutputSink::deliver` is invoked on the dedicated delivery worker — never on the session actor or the host's submission thread — so a slow sink in one session cannot stall the actor or other sessions
- `gr-host-bridge` provides convenience adapters: `CallbackSink<F>` (closure), channel + stream adapters land alongside the Phase 7 delivery worker
- dropping the subscription handle, or calling `unsubscribe`, detaches the sink on the next delivery-worker tick
- a sink that panics is contained within the delivery worker; the session continues, the panic is recorded in `SessionDiagnosticsSnapshot.last_error`, and the subscription is detached

### Audio stream surface

```rust
impl VirtualControllerSessionHandle {
    pub fn audio_sink(&self) -> Option<Box<dyn AudioStreamSink>>;
    pub fn audio_source(&self) -> Option<Box<dyn AudioStreamSource>>;
}

pub trait AudioStreamSink: Send {
    fn push_samples(&mut self, samples: &[i16]) -> Result<usize, AudioStreamError>;
    fn sample_rate_hz(&self) -> u32;
    fn channels(&self) -> u8;
}

pub trait AudioStreamSource: Send {
    fn pull_samples(&mut self, out: &mut [i16]) -> Result<usize, AudioStreamError>;
    fn sample_rate_hz(&self) -> u32;
    fn channels(&self) -> u8;
}

#[non_exhaustive]
pub enum AudioStreamError { Closed, Backpressure }
```

Rules:

- both methods return `None` for profile + provider combinations that do not realize PCM output / input at the chosen tier (see [Audio stream contract](#audio-stream-contract))
- traits live in `gr-host-bridge` so the audio surface is reusable by future host integrations
- `push_samples` / `pull_samples` return the count actually transferred; short returns signal backpressure without requiring the caller to interpret an enum
- `AudioStreamError::Backpressure` is recoverable — caller should back off; `Closed` is terminal — caller should drop the handle

## Concrete translator families

### Evdev

- `GenericEvdevTranslator`
- `XboxStyleEvdevTranslator`
- `DualSenseEvdevTranslator` only if identity-specific evdev shaping differs materially
- `SteamControllerEvdevTranslator` only if needed

### HID

- `DualSenseUsbHidTranslator`
- `SteamControllerHidTranslator`

### Reverse HID

- `DualSenseHidReverseTranslator`
- `SteamControllerReverseTranslator`

### Transport

- `Xbox360UsbTransportTranslator`
- `DualSenseUsbTransportTranslator`
- `DualSenseBluetoothTransportTranslator`

## Descriptor compatibility contract

For every HID- or transport-capable profile family:

- a descriptor template must exist
- a forward translator family must be assigned
- declared forward input capabilities and reverse/output capabilities must exist
- a reverse translator family must be assigned for every `identity-aware` or `hardware-faithful` support claim
- contract tests must assert that the translator and descriptor pair are consistent

Examples of required assertions:

- field widths match descriptor expectations
- report ids align
- optional feature reports are declared consistently
- reverse output functions map to declared profile capabilities
- reverse translators do not emit commands for undeclared output capabilities

## Error taxonomy

### Core categories

- `ConfigError`
- `SessionOptionsError`
- `PlanningError`
- `TranslationError`
- `BackendError`
- `SessionError`
- `DeliveryError`

### Required machine-readable causes

- unknown profile
- unsupported fidelity
- unsupported backend
- invalid profile input field
- missing required profile input field
- incompatible descriptor/translator pair
- queue overflow
- backend open failure
- backend write failure
- backend would-block (`BackendError::WouldBlock`; session must re-arm via `readiness()`)
- reverse event parse failure
- session closed

## Telemetry requirements

Each active session must expose:

- frames received
- frames coalesced
- frames written
- write failures
- reverse events received
- reverse commands emitted
- reverse commands dropped
- average and p95 translation latency
- queue depth high-water marks
- backend diagnostics snapshot

### Counter naming convention

Per-session counters surface through `SessionDiagnosticsSnapshot.counters: BTreeMap<String, u64>` ([`gr_runtime_model`](#gr-runtime-model)). The canonical key names — pinned in [`gr_session::counter_keys`](#gr-session) — are:

| Key                              | Meaning                                                                |
|----------------------------------|------------------------------------------------------------------------|
| `frames.received`                | Host-submitted input frames accepted into the session queue            |
| `frames.coalesced`               | Frames dropped because a fresher state arrived (latest-state-wins)     |
| `frames.written`                 | Frames successfully written to the backend session                     |
| `write_failures`                 | Backend `send` calls returning a non-`WouldBlock` error                |
| `reverse_events.received`        | Reverse events drained from the backend                                |
| `reverse_events.emitted`         | `ControllerOutputCommand` values delivered to the session's sink       |
| `reverse_events.dropped`         | Reverse events dropped per the configured `BackpressurePolicy`         |
| `queue_depth.input.hwm`          | High-water mark of input queue depth                                   |
| `queue_depth.reverse.hwm`        | High-water mark of reverse-event queue depth                           |
| `translation.latency_p95_us`     | p95 forward-translation latency in microseconds (recent steady state)  |

Rules:

- the key set is the canonical surface; additive keys may appear, but renames are breaking and need a deprecation cycle
- units are part of the key suffix where ambiguous (`_us` for microseconds, `_hwm` for high-water marks)
- counters are monotonic u64 for the session's lifetime; high-water marks are non-decreasing within the session

Manager-wide telemetry must expose:

- active session count
- backend family counts
- aggregate drop/coalesce counts
- planner degradation counts

## Test requirements by crate

### `gr-core`

- normalization bounds tests
- serde round-trip tests
- sequence ordering tests

### `gr-profiles`

- capability presence tests
- required/supported function consistency tests
- descriptor-template presence tests

### `gr-config`

- valid and invalid config fixtures
- schema-export tests if supported

### `gr-session-options`

- session-option validation tests
- delivery-policy tests
- provider-hint tests
- optional helper-adapter tests when enabled

### `gr-runtime-model`

- serialization tests for diagnostics and plans
- delivery-policy tests

### `gr-backend-api`

- trait conformance fakes
- event readiness abstraction tests

### `gr-planner`

- backend selection tests
- degradation tests
- impossible-plan rejection tests

### `gr-translators`

- neutral-state tests
- per-profile input translation tests
- descriptor/report contract tests
- reverse translation tests

### `gr-session`

- queue coalescing tests
- lifecycle tests
- slow-consumer isolation tests
- high-session-count smoke tests with fake backends

### Concrete backend crates

- descriptor/open-context tests
- fake writer/reader tests
- Linux-gated integration tests

## Implementation order

The recommended order is:

1. `gr-core`
2. `gr-profiles`
3. `gr-config`
4. `gr-session-options`
5. `gr-runtime-model`
6. `gr-backend-api`
7. `gr-planner`
8. `gr-testkit` fake backends
9. `gr-translators`
10. `gr-session`
11. `gr-host-bridge`
12. `gr-provider-linux-uinput`
13. `gr-provider-linux-uhid`
14. `gr-provider-linux-transport`
15. `gr-provider-windows-hid`
16. `gr-provider-macos-hid`
17. `gr-cli`

## Performance acceptance targets

Steady-state input-to-backend-write latency target: p99 < 2 ms with 16 concurrent active sessions on Linux with `uinput`/`UHID` providers, measured against the fake-backend baseline before real hardware. This is a planning target; acceptance numbers will be refined after Milestone 2 measurements expose real cost. The hot-path rules and queue-bound rules in this document are what the implementation uses to hit it.

## Acceptance criteria

The Rust implementation is ready for real use when all of the following are true:

- crate boundaries compile without dependency cycles or ownership ambiguity
- session creation uses validated config, compiled session options, and planner output only
- `uinput` sessions work end to end with bounded queues and diagnostics
- at least one `UHID` profile family works with reverse events delivered to the host
- the runtime handles many concurrent fake sessions without requiring one thread per device
- descriptor/translator contract tests pass for all implemented profile families
- no `identity-aware` support claim exists without reverse output and feature report handling
- no `hardware-faithful` support claim exists without transport enumeration, control-flow, and reverse packet validation

## Final rule

The implementation must optimize for explicit ownership and prepared execution contexts.

If a runtime code path still needs to “figure out what to do” on every frame, the preparation phase is incomplete.
