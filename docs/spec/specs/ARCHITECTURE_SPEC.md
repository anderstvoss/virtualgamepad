# Architecture Specification

This document defines the target architecture for `VirtualGamepad` as an embeddable controller-emulation subsystem.

It is intended to do three things:

- state the actual product goals and integration model
- identify the main architectural issues visible in the current repo
- define a complete build target for the production architecture

Related documents:

- [README.md](../../README.md)
- [IMPLEMENTATION_FRAMEWORK.md](../implementation/IMPLEMENTATION_FRAMEWORK.md)
- [CONFIGURATION_SPEC.md](../specs/CONFIGURATION_SPEC.md)
- [FIDELITY_GUIDE.md](../specs/FIDELITY_GUIDE.md)
- [TEST_PLAN.md](../validation/TEST_PLAN.md)
- [RUST_IMPLEMENTATION_PLAN.md](../implementation/RUST_IMPLEMENTATION_PLAN.md)

## Project goals

`VirtualGamepad` exists to let a larger host program create virtual controller identities appropriate to the selected target profile and expose them correctly to the host operating system.

The core product goals are:

- embed cleanly inside a larger application rather than act as a standalone daemon first
- require exact, profile-specific input contracts for each virtual device instance
- support multiple target controller families through profile data and per-family codecs rather than one-off logic
- support three fidelity tiers: `compatibility`, `identity-aware`, and `hardware-faithful`
- treat reverse-path behavior such as rumble, lighting, trigger effects, and mode commands as first-class architecture
- isolate platform- and transport-specific I/O behind replaceable backend adapters
- minimize end-to-end latency for production applications
- preserve throughput and responsiveness when many virtual devices are active concurrently
- remain stable enough for production systems that depend on long-lived virtual-device sessions
- stay gamepad-oriented by default while allowing adjacent non-gamepad device profiles when they fit the same session, planning, and reverse-path architecture
- make planning, degradation, and support gaps inspectable
- make the system testable without real kernel devices

## Linux-first multi-platform assumptions

For the production design, this project should be treated as a Linux-prioritized standalone library with an explicitly multi-platform architecture.

Linux remains the first concrete implementation target because it has the clearest immediate path for `uinput`, `UHID`, and transport-oriented experimentation. Windows and macOS are not first-delivery promises, but they must be first-class planning concerns in the architecture so the core runtime, planner, and module boundaries do not trap the project inside Linux-only assumptions.

The library is responsible for:

- creating and owning host-visible virtual controller instances through platform providers
- translating profile-specific controller input into those virtual devices
- receiving reverse-path events from those virtual devices
- returning normalized reverse commands to the embedding program
- scaling to many concurrent virtual devices without changing the public model

The embedding program is responsible for:

- producing exact controller input for the selected target profile
- selecting target profiles and fidelity goals
- consuming normalized reverse commands such as audio, rumble, LEDs, or trigger effects
- deciding whether reverse commands are bridged to real hardware, ignored, or used for app logic

This means the library should not try to own physical controller capture or UI policy, but it does need a cross-platform planning surface above platform-provider backends.

## Non-goals

This architecture does not require:

- a single implementation language
- a single runtime model
- transport spoofing in the first delivery phase
- exact proprietary report-descriptor contents in this document
- user-interface policy for the embedding host

This architecture also should not broaden into a universal arbitrary-device framework.

- profiles outside the gamepad family are admissible only when they fit the same host-visible device-session model, planner, and reverse-path contracts
- the library should not grow a second unrelated abstraction stack for keyboards, general HID automation, or arbitrary USB gadget emulation

## Planned integration model

The intended integration point is a host application that already owns the real input source and can produce controller input appropriate to the selected target profile.

The host is expected to:

- produce exact input snapshots or deltas for the selected target profile
- choose a target profile and requested fidelity
- receive reverse output commands from the emulated device session
- decide whether to bridge reverse outputs back to physical hardware, expose them to scripting, or ignore them

The router is expected to:

- validate session requests
- plan the minimum viable emulation layer
- instantiate a session against the chosen backend
- translate profile-specific controller input into backend frames
- receive backend-originated reverse messages
- expose telemetry, warnings, and support gaps

## Host-platform planning model

The architecture must distinguish three separate concerns that the current repo partly conflates:

- target controller identity:
  DualSense, Xbox 360, Steam Controller, generic gamepad, and similar profile families
- emulation fidelity:
  `compatibility`, `identity-aware`, or `hardware-faithful`
- host-platform realization:
  the concrete way the target becomes visible on Linux, Windows, or macOS

This means the planner is responsible for selecting both:

- a backend family:
  `evdev`, `hid`, or `transport`
- a concrete platform provider:
  for example `linux-uinput`, `linux-uhid`, `windows-vhf`, `macos-corehid`, or a future transport provider

The architecture should therefore be described as:

```text
Requested target profile
  + requested fidelity
  + host platform and provider inventory
  -> planner negotiation
  -> selected backend family
  -> selected platform provider
  -> per-session backend instance
```

Linux is the first provider family to implement thoroughly, but the architecture must preserve room for:

- Windows providers that may require driver-backed HID realization
- macOS providers that may require entitlements, virtual HID APIs, or DriverKit-based installation
- platform-specific degradation where a target is admissible on one OS and rejected or downgraded on another

## Architectural assessment

The repository already captures the right high-level idea: device profiles, fidelity tiers, translators, and pluggable emitters. That foundation is sound.

This repository no longer contains an implementation. The main architectural task now is to narrow the product shape before Rust code begins so the build does not inherit unnecessary abstraction. The biggest gaps are not missing code volume; they are missing boundaries around planning, session lifecycle, reverse flow, and exact device-specific input contracts.

## Key architectural issues

### 1. Planning is hard-coded into profiles instead of negotiated from runtime capability

The earlier prototype direction hard-coded backend choice into profile resolution instead of negotiating from runtime inventory and host policy.

Why this matters:

- planning cannot account for actual backend inventory
- there is no legal degradation path
- there is no separation between device requirements and deployment environment
- impossible plans fail too late or unclearly

Required fix:

- profiles should declare required capabilities and supported levels
- an `EmulationPlanner` should combine requested goal, host-platform inventory, backend inventory, and policy to create a session plan

### 2. Sessions do not exist yet; emitters are effectively singleton devices

The earlier prototype direction treated backend instances as effectively shared device objects instead of explicit per-session resources.

Why this matters:

- there is no concept of one active emulated device session versus another
- target switching is unsafe
- multi-device or concurrent sessions are not modeled
- backend teardown, re-open, and error recovery are undefined

Required fix:

- introduce `SessionManager` and per-session backend handles
- move device lifecycle out of shared emitter singletons

### 3. Reverse-path architecture is documented but not present in runtime contracts

The docs consistently call out reverse-path outputs, but the current runtime only does forward translation and `emit(frame)`. There is no reverse translator, no backend callback channel, and no host bridge for output commands.

Why this matters:

- `identity-aware` and `hardware-faithful` tiers are incomplete without reverse flow
- HID output reports, feature reports, rumble, LEDs, trigger effects, and mode switches have nowhere to go
- capability claims cannot be validated end to end

Required fix:

- define backend event input contracts
- add reverse translators per backend/protocol family
- add host-facing output dispatch interfaces

### 4. HID translation is generic, but HID behavior is profile-specific

The earlier prototype direction assumed one generic HID report shape across HID-capable profiles, which is not accurate for device families with materially different descriptors and report semantics.

Why this matters:

- descriptor identity and report encoding can drift apart
- profile capability data and translator behavior are not guaranteed to match
- adding more HID targets will amplify special-case logic

Required fix:

- make HID translation profile-family-specific
- define report schemas as profile data plus translator modules
- validate report compatibility against descriptor definitions

### 5. The architecture is still too broad because it keeps inventing a unified input model

The earlier direction tried to define one library-wide controller abstraction and then translate that abstraction into device-specific output. That is broader than the product needs and pushes complexity into the wrong place.

Why this matters:

- it creates an extra semantic layer that real devices still need to escape from
- it encourages approximate inputs instead of exact device contracts
- it makes device-specific features feel optional when they should be explicit
- it increases validation, mapping, and normalization complexity before any backend value is delivered

Required fix:

- remove the unified control input model from the production architecture
- make session creation bind one concrete target profile and one concrete input contract
- require every submitted frame or delta to already match that profile contract
- keep any optional adapters outside the core runtime, not inside the main session API

### 6. Configuration should stay narrow instead of becoming a semantic remapping engine

The repo has a strong configuration document, but the target runtime should not depend on a semantic mapping compiler in its core data path.

Why this matters:

- a general remapping engine pushes the design back toward a universal controller model
- target-specific bindings become harder to reason about than direct typed input
- host integration becomes broader and less explicit than the intended narrow Rust scope

Required fix:

- keep configuration focused on session options, fidelity policy, and provider selection
- make translators consume exact profile-shaped input structures rather than compiled semantic mappings
- if mapping helpers exist later, keep them outside the core device-session runtime

## Planned resolutions for Linux-first standalone-library use

The following design rules resolve the six main shortcomings while keeping Linux as the prioritized implementation target and Windows/macOS as planned architecture paths.

### Resolution 1: make device sessions first-class runtime objects

The library must expose explicit device-session APIs rather than a one-shot routing API.

Required design changes:

- replace one-shot routing as the primary model with `create_session`, `send_state`, `poll_events`, and `close_session`
- require every virtual controller instance to own exactly one backend session handle
- make session teardown idempotent and cheap
- support many active sessions under one manager

Required public model:

- `VirtualControllerManager`
- `VirtualControllerSession`
- `SessionPlan`
- `SessionDiagnostics`

### Resolution 2: move backend choice into runtime planning

Profile data must not choose a concrete backend implementation.

Required design changes:

- profiles may declare supported fidelity levels and descriptor metadata only
- planner must select the backend family and concrete platform provider from runtime inventory and policy
- planner must report degradation and rejection reasons explicitly
- planner output must be stable enough to cache per session

Required planner inputs:

- target profile
- requested fidelity
- host platform
- backend inventory
- platform-provider inventory
- strictness policy
- optional placement preferences

### Resolution 3: make reverse-path flow a mandatory library contract

Reverse-path support is part of the standalone library boundary, not an optional integration detail.

Required design changes:

- backend sessions must surface reverse events
- reverse translators must normalize them into library-level commands where possible and preserve profile-specific commands where needed
- the library must expose bounded queues or callback sinks for event delivery
- audio and other reverse events must carry session identity and timestamps

Required public model:

- `BackendReverseEvent`
- `ControllerOutputCommand`
- `ReverseEventSink`
- `SessionEventStream`

Required extensibility rule:

- reverse-path modeling must not assume every device command fits one small universal enum
- common functions such as rumble, LEDs, trigger effects, and audio may use normalized commands
- device-specific behavior such as accessory traffic, vendor channels, startup negotiation, or mode-specific commands must have a typed profile-specific escape path

### Resolution 4: split translators by profile family and transport family

Accurate emulation of specific physical controller types requires device-family-specific codecs.

Required design changes:

- generic evdev translation may remain shared where semantically valid
- HID translation must be split by profile family
- transport translation must be split by both profile family and transport type where required
- profile declarations must reference compatible translator families

Required validation:

- descriptor-to-translator compatibility tests
- capability-to-translator consistency tests
- reverse-command coverage tests per profile family

### Resolution 5: narrow the input contract to exact per-profile device input

The library should accept exact device-shaped input for the selected profile rather than owning a universal semantic controller model.

Required design changes:

- require session creation to bind one profile-specific input schema
- validate submitted input against that chosen schema only
- make translators operate directly on profile-specific input without an intermediate unified model
- move any optional normalization or adaptation helpers outside the core runtime crates

Hot-path rule:

- the per-frame path should perform shape validation, translation, and enqueue/write only
- it should not interpret cross-profile semantics, parse config, or perform repeated symbolic lookup

### Resolution 6: design explicitly for many concurrent virtual devices

The standalone library must treat scale as a first-class requirement rather than a future optimization.

Required design changes:

- separate control-plane work from data-plane work
- use per-session ownership with shared scheduling rather than shared mutable device objects
- use bounded queues for input and reverse events
- allow state coalescing when newer snapshots supersede older ones
- use preallocated frame buffers or reusable buffers where backend protocols allow
- keep telemetry aggregation off the hottest write path

Scalability rule:

- logical isolation should be per session
- execution resources do not need to be one dedicated OS thread per session

Recommended runtime model:

- one manager owning session registry and backend inventory
- one logical session actor per device
- a shared async runtime or worker pool for scheduling many sessions
- per-backend write serialization
- optional batching/coalescing for bursty input streams

### Resolution 7: use tier-specific internal runtime models

The three fidelity tiers should share one profile registry and one planner, but they should not be forced into one flattened execution model.

Required design changes:

- `compatibility` sessions may use a gameplay-input-centric realization model
- `identity-aware` sessions must use an explicit descriptor/report model with reverse report and feature report handling
- `hardware-faithful` sessions must use a transport-session model with enumeration, control flow, protocol state, and reverse packet handling
- richer tiers must be allowed to own additional state that lower tiers do not need, including feature negotiation, startup handshakes, timing state, and attached-function state
- the public host-facing API should remain stable even when the internal prepared session model differs by tier

Future-proofing rule:

- the architecture must allow attachable or nested device functions such as expansion ports, audio functions, touch surfaces, or other accessory endpoints without rewriting lower-tier session APIs
- attached-function support may be dormant in early implementations, but the runtime model must leave room for per-session subchannels, profile-specific reverse commands, and transport- or HID-level side functions

### Resolution 8: define a reverse-command normalization policy

The runtime needs a stable policy for deciding which reverse behaviors become shared semantic commands and which remain profile-specific.

Normalization rules:

- normalize a reverse command only when the behavior, payload meaning, and host-side handling are materially equivalent across multiple profile families
- keep a command profile-specific when its semantics depend on descriptor shape, transport state, attached functions, device modes, or vendor-specific interpretation
- allow a translator to emit both normalized and profile-specific commands from the same underlying event stream when that preserves ergonomics without hiding fidelity
- never coerce an unfamiliar device-specific command into a misleading generic semantic command just to fit the shared API

Promotion discipline:

- do not add a new normalized command for a single profile family
- do not add a new normalized command until at least two profile families share equivalent semantics and payload meaning
- every normalized-command addition must name the profile families it unifies and the host-side behavior it standardizes
- when equivalence is uncertain, prefer keeping the behavior profile-specific

Examples:

- rumble, basic lighting, trigger effects, and simple audio mode changes are good normalization candidates
- accessory traffic, expansion-port commands, startup feature negotiation, and vendor-defined side channels should remain profile-specific unless real cross-family equivalence is proven

### Resolution 9: define attached-function capability modeling

Profiles that expose side functions, accessory channels, or nested device functions need explicit capability declarations rather than hidden translator knowledge.

Required design changes:

- `ControllerProfile` must be able to declare attached functions and their capability groups
- attached functions must have stable identifiers, capability summaries, and routing metadata
- the planner must be able to report when an attached function is unsupported at the selected fidelity tier or provider
- reverse-command payloads and backend reverse events must be able to reference attached-function identifiers directly

Scope rule:

- attached-function modeling should be explicit enough to support future cases such as expansion ports, audio accessories, touch surfaces, or profile-family-specific side channels
- it should not require every profile to simulate subdevices when the profile family does not expose them

Profile schema example:

```yaml
profileId: "xbox-360"
family: "xbox"
supportedFidelityTiers:
  - "compatibility"
  - "identity-aware"
  - "hardware-faithful"
attachedFunctions:
  - attachedFunctionId: "expansion-port-1"
    family: "expansion-port"
    capabilitySummary:
      - "headset-audio"
      - "accessory-control"
    supportedFidelityTiers:
      - "hardware-faithful"
    routingHints:
      transportChannel: "expansion-port-1"
    reverseCommandSupport:
      - "ProfileSpecific"
      - "AttachedFunction"
    forwardInputSupport: []
```

Planner rule:

- if a profile declares attached functions, the planner must report support, degradation, or rejection for each required attached function rather than only for the parent profile

Transport-tier isolation rule:

- attached-function routing, transport channels, endpoint state, and handshake-sensitive behavior belong to `hardware-faithful` prepared-session state unless a lower tier explicitly realizes them
- `identity-aware` tiers may model report-level side functions only when those functions are truly expressed at the HID layer
- `compatibility` tiers must not inherit transport-session concepts merely to preserve internal symmetry

## Target architecture

The production architecture should be organized around explicit sessions, explicit plans, and explicit reverse-path handling.

```text
Host application
  -> Profile-Specific Input Producer
  -> Session Manager
       -> Profile Registry
       -> Capability Registry
       -> Session Options Compiler
       -> Emulation Planner
       -> Forward Translator
       -> Backend Session
       -> Reverse Translator
       -> Host Output Bridge
       -> Telemetry Sink
```

## Core design principles

- separate policy from mechanism
- separate device identity from deployment environment
- separate semantic functions from backend encodings
- treat reverse output as mandatory architecture, not an extension
- keep planning inspectable and deterministic
- keep runtime sessions isolated
- prefer immutable plans and snapshots
- make unsupported features explicit

## Component model

### 1. `ProfileInputBoundary`

Responsibility:

- accept host snapshots or deltas for one already-selected target profile
- validate that the submitted data matches that profile's exact input contract
- stamp timestamp and sequence id
- expose immutable profile-specific input frames to the runtime

Input:

- host-provided profile-specific state

Output:

- validated profile-specific input frame

Rules:

- every active session has exactly one accepted input contract
- unknown fields are rejected or warned by policy
- optional fields are profile-defined, not globally inferred
- no cross-profile normalization layer exists inside the core runtime

### 2. `ProfileRegistry`

Responsibility:

- store target controller profiles
- expose identity metadata
- expose supported fidelity levels
- expose required semantic functions

Rules:

- profiles are static data
- profiles do not store live runtime session state
- profiles do not choose concrete backend instances

### 3. `CapabilityRegistry`

Responsibility:

- expose structured input and output capability metadata
- answer capability queries for host and planner use

Rules:

- capability declarations are part of the contract surface
- capability metadata must stay consistent with profile requirements and translators

### 4. `ConfigurationLoader`

Responsibility:

- parse host or file-backed session configuration
- validate schema and references
- produce `CompiledSessionConfig`

Consumes:

- configuration document
- target profile metadata

Produces:

- validated session options and validation policy

### 5. `SessionOptionsCompiler`

Responsibility:

- precompute session-local options needed by planner and runtime
- validate per-profile input policy
- precompute backend and provider hints

Produces:

- `CompiledSessionOptions`

### 6. `EmulationPlanner`

Responsibility:

- choose the best achievable fidelity level
- choose a backend family from actual runtime inventory
- choose a concrete host-platform provider from actual runtime inventory
- report degradation and unsupported features

Consumes:

- requested goal
- target profile
- host platform
- backend inventory
- platform-provider inventory
- configuration policy

Produces:

- `SessionPlan`

Planner output must include:

- requested goal
- requested fidelity name
- selected internal level
- selected backend family
- selected host platform
- selected provider id
- deployment requirements and install prerequisites
- degradation status
- unsupported capabilities
- warnings
- rationale

### 7. `TranslatorRegistry`

Responsibility:

- resolve the correct forward and reverse translator pair for a profile family and backend level

Rules:

- translators are selected by profile family plus level
- translators must not assume one universal HID or transport shape
- translators should stay as platform-neutral as possible above provider-specific framing details

### 8. `ForwardTranslator`

Responsibility:

- convert profile-specific input into backend frames

Output examples:

- evdev event batches
- HID input reports
- transport packets

### 9. `ReverseTranslator`

Responsibility:

- convert backend-originated output or feature traffic into normalized host commands

Output examples:

- rumble command
- LED update
- trigger-effect request
- audio route change
- mode-switch request

### 10. `BackendFactory`

Responsibility:

- create a backend session for the selected plan

Rules:

- backends are created per session
- factories may reject unsupported plans before session start

### 11. `BackendSession`

Responsibility:

- own one concrete device instance
- create, write, receive, flush, and close
- emit backend-originated messages

Required interface:

- `open(descriptor)`
- `send(frame)`
- `pollEvents()` or callback/event-stream equivalent
- `close()`
- `getDiagnostics()`

Additional rules:

- every backend session is bound to exactly one virtual controller session
- backend sessions must never be shared between active devices
- reverse events must identify their session source
- backend sessions used in production-sensitive environments must fail cleanly, expose diagnostics, and avoid corrupting unrelated sessions when one session encounters provider errors

### 12. `HostOutputBridge`

Responsibility:

- deliver normalized reverse commands back to the embedding host

Supported modes:

- callback
- channel/queue
- observable/event sink
- ignored with explicit policy

### 13. `SessionManager`

Responsibility:

- own session lifecycle
- create, start, stop, and switch sessions
- serialize or arbitrate target changes
- coordinate planner, translators, backend session, and telemetry
- schedule many concurrent sessions efficiently

Performance rules:

- the manager must support a large session registry without linear scans on every input frame
- session lookup by id must be constant-time or equivalent
- background diagnostics collection must not block frame dispatch
- steady-state dispatch must avoid avoidable allocation, repeated symbolic lookup, and cross-session lock contention on the hot path
- latency-sensitive writes must not wait on unrelated reverse-event consumers or diagnostics aggregation

Stability rules:

- one failing provider session must not destabilize the rest of the manager
- session recovery and teardown must be explicit and observable
- degraded operation must be reported rather than silently changing behavior under load

Provider support-report rule:

- backend providers must report support at the level of concrete capabilities, reverse-path coverage, and attached-function routing, not only at the level of coarse backend family
- provider support reports must distinguish forward input support, reverse output support, feature report support, attached-function support, and timing- or handshake-sensitive support where applicable
- planners must treat unknown provider capability state as insufficient for a support claim, not as optimistic success

### 14. `TelemetrySink`

Responsibility:

- collect counters, timings, warnings, and errors
- expose plan summaries and support gaps

Required telemetry:

- session create/close
- backend open failures
- translation failures
- degraded plan selections
- dropped reverse events
- input validation errors

## Canonical data contracts

### `ProfileInputFrame`

Required fields:

- `timestamp`
- `sequence`
- `profileId`
- `payload`

Constraints:

- `payload` must conform to the chosen profile-specific input contract
- unknown fields are rejected or logged by policy
- value ranges are defined by the selected profile contract, not by a global schema

### `ControllerProfile`

Required sections:

- identity metadata
- transport hints
- capabilities
- attached functions
- supported fidelity levels
- required semantic input functions
- supported semantic output functions
- backend descriptor metadata per level
- provider compatibility hints where known

Profile rule:

- a profile may describe what is needed for a level, but not the specific runtime backend instance to use
- a profile may declare gamepad-adjacent non-gamepad functions when they belong to the same host-visible device identity and reverse-path model

### Attached-function capability model

Each attached function declaration should include:

- `attachedFunctionId`
- `family`
- `capabilitySummary`
- `supportedFidelityTiers`
- `routingHints`
- `reverseCommandSupport`
- `forwardInputSupport` where applicable

Rules:

- attached functions must be optional at the profile level unless the device identity requires them
- unsupported attached functions must appear in plan diagnostics rather than disappearing silently
- attached functions must not force the main profile input contract to become a universal schema
- attached functions must not introduce transport-specific state into lower-tier profile input contracts

### `SessionRequest`

Required fields:

- `profileId`
- `goal`
- `config`

Optional fields:

- host platform preference
- backend preference
- provider preference
- strictness policy
- host session metadata

### `SessionPlan`

Required fields:

- `sessionId`
- `profileId`
- `requestedGoal`
- `requestedFidelityTier`
- `selectedLevel`
- `selectedBackend`
- `selectedHostPlatform`
- `selectedProvider`
- `compiledSessionOptions`
- `enabledCapabilities`
- `unsupportedCapabilities`
- `deploymentRequirements`
- `warnings`
- `rationale`

### `BackendFrame`

Tagged union:

- `EvdevFrame`
- `HidInputReport`
- `TransportPacket`

Each frame must include:

- `sessionId`
- `profileId`
- `level`
- `sequence`

### `ControllerOutputCommand`

Required fields:

- `sessionId`
- `profileId`
- `commandType`
- `function`
- `payload`
- `timestamp`

Required interpretation rules:

- commands should use normalized `function` values for common cross-device behaviors where that improves host ergonomics
- commands must also be able to represent profile-specific or transport-specific behavior that does not map cleanly to a shared function enum
- command payloads must be able to reference attached functions or accessory channels when the profile family supports them

### Reverse-command normalization policy

The shared host-facing command surface should follow these rules:

- expose a small stable normalized surface for common commands
- expose profile-specific command families explicitly rather than growing the normalized surface for every niche behavior
- preserve enough information for a host to bridge a profile-specific command back to real hardware when needed
- let hosts ignore profile-specific commands by policy without misinterpreting them as successful normalized handling
- require a short written rationale when a new normalized command is added to the shared surface

## Runtime flows

### Session creation flow

1. Host submits `SessionRequest`.
2. `ConfigurationLoader` validates config.
3. `SessionOptionsCompiler` prepares runtime session options.
4. `EmulationPlanner` selects the best legal plan from runtime inventory.
5. `BackendFactory` creates a per-session backend.
6. `TranslatorRegistry` resolves translators for the selected profile family and level.
7. `SessionManager` returns a `SessionHandle` plus plan summary.

### Forward input flow

1. Host submits a profile-specific input snapshot or delta.
2. `ProfileInputBoundary` validates it against the selected profile contract.
3. `ForwardTranslator` converts it into a backend frame.
4. `BackendSession` writes the frame.
5. Telemetry records sequence, latency, and failures.

Optimization rule:

- after session creation, this flow should use preselected translators, compiled session options, and reusable buffers only

Latency rule:

- the forward input flow should minimize work between accepted host input and backend write, and should avoid control-plane recomputation during steady-state dispatch

### Reverse output flow

1. `BackendSession` receives output report, feature request, or transport event.
2. `ReverseTranslator` converts it into `ControllerOutputCommand`, using normalized forms where possible and profile-specific forms where required.
3. `HostOutputBridge` dispatches it according to policy.
4. Telemetry records delivery or drop behavior.

Delivery rule:

- reverse events must use bounded delivery paths so one slow consumer does not stall unrelated virtual devices

### Target switch flow

1. Host requests new profile or fidelity.
2. `SessionManager` pauses writes for the active session.
3. Old backend session closes cleanly.
4. New session plan is created.
5. New backend session opens.
6. State resumes with the latest valid profile-specific snapshot.

## Fidelity and degradation rules

The planner must treat fidelity as a negotiated outcome, not a static lookup.

Rules:

- if the requested tier is available, use it
- if not available and degradation policy allows fallback, choose the highest safe lower tier
- report all dropped identity or output features explicitly
- report host-platform limitations explicitly even when fidelity is preserved only partially
- if degradation would invalidate required host behavior, fail session creation

Example:

- DualSense requested as `hardware-faithful` with only `UHID` available
- planner may degrade to `identity-aware`
- plan must state that transport parity, advanced enumeration, and any transport-only features are unavailable

## Platform-provider architecture

The architecture must describe providers by both family and host platform.

Linux is the primary implementation target:

- `linux-uinput` for `compatibility`
- `linux-uhid` for `identity-aware`
- future Linux transport providers for `hardware-faithful`

Windows and macOS are planned provider families:

- Windows HID realization providers, likely driver-backed for true virtual-device exposure
- macOS HID realization providers, potentially split between user-space virtual HID paths and DriverKit-backed paths
- future transport providers only where the host OS makes them practical and supportable

### Linux `uinput` provider

Responsibilities:

- create evdev-visible virtual devices
- emit `EV_KEY`, `EV_ABS`, `EV_SYN`
- optionally support Linux force-feedback handling

Limits:

- no native HID identity
- reverse-path is limited compared to HID or transport

### Linux `UHID` provider

Responsibilities:

- create HID-visible virtual devices
- expose report descriptors
- send input reports
- receive output and feature reports

Limits:

- does not guarantee USB/Bluetooth transport parity

### Linux transport provider

Responsibilities:

- emulate USB or Bluetooth protocol behavior
- own enumeration and packet state
- provide packet send and receive

Limits:

- highest complexity
- should be introduced only after evdev and HID contracts are stable

### Windows planned providers

Responsibilities:

- expose virtual controller identity through Windows-supported realization paths
- model driver-backed deployment requirements explicitly
- preserve the same session, planner, and reverse-path contracts as Linux providers

Architecture rules:

- do not assume Windows can reuse Linux `evdev` or `UHID` abstractions directly
- keep provider-specific deployment requirements visible in `SessionPlan`
- allow planner rejection when the required provider is not installed or not permitted

### macOS planned providers

Responsibilities:

- expose virtual HID devices through supported macOS realization paths
- model entitlements, install requirements, and system-extension constraints explicitly
- preserve the same session and reverse-path architecture as Linux providers

Architecture rules:

- do not assume a single macOS provider shape will cover both lightweight virtual HID and deeper driver-backed cases
- treat deployment prerequisites as part of planning, diagnostics, and support reporting
- allow platform-specific fidelity ceilings where transport-faithful behavior is not realistic

## Translation architecture

Translation must be split by level and by profile family.

### Evdev translators

May be relatively generic, but still profile-aware for:

- button inventory
- axis ranges
- optional controls
- force-feedback support

### HID translators

Must be profile-family-specific.

Recommended shape:

- `DualSenseHidTranslator`
- `SteamControllerHidTranslator`
- future `VendorXHidTranslator`

### Transport translators

Must be fully profile-family-specific and usually transport-specific.

Recommended shape:

- `Xbox360UsbTransportTranslator`
- `DualSenseUsbTransportTranslator`
- `DualSenseBluetoothTransportTranslator`

## Error model

Errors should be structured and machine-readable.

Required categories:

- invalid profile input
- invalid configuration
- unknown profile
- unsupported fidelity request
- planner no-solution
- backend-open failure
- translation failure
- reverse-translation failure
- session-closed or wrong-session write

## Concurrency and lifecycle rules

- every active device instance belongs to exactly one session
- backend writes must be serialized per session unless a backend explicitly supports batching
- reverse events must carry session identity
- state updates may be coalesced, but sequence gaps must be observable
- session closure must be idempotent
- session scheduling should use shared workers or async tasks, not require one dedicated OS thread per device
- telemetry and diagnostics must be decoupled from the write-critical path

## Performance and scaling model

The target library should scale by keeping session state isolated while sharing execution resources efficiently.

### Control plane

Owns:

- session creation
- planner execution
- backend selection
- diagnostics snapshots
- configuration validation

Properties:

- latency-sensitive but low-frequency
- may allocate more freely than the data path

### Data plane

Owns:

- state ingestion for active sessions
- frame translation
- backend writes
- reverse-event normalization and dispatch

Properties:

- high-frequency
- should avoid repeated allocation where practical
- should avoid symbolic lookups after session startup

### Recommended optimizations

- prevalidated per-profile input codecs or typed builders
- reusable frame buffers per session
- bounded MPSC queues for state updates and reverse events
- latest-state coalescing for bursty producers
- batched telemetry emission
- per-session counters stored locally and periodically flushed

### Explicit non-requirement

The library does not need lock-free complexity everywhere.

It does need:

- predictable ownership
- bounded queues
- per-session isolation
- no shared mutable backend handles across devices

## Observability requirements

The host should be able to inspect:

- active session plan
- enabled and degraded capabilities
- backend type and state
- latest input sequence
- latest reverse command
- last error
- latency counters

## Security and trust boundaries

- host input is untrusted until validated
- configuration is untrusted until validated
- backend-originated output reports are untrusted until reverse-translated and range-checked
- transport spoofing code should be isolated because it is the most privileged and protocol-sensitive layer
- platform-provider code may carry OS-specific privilege, driver, or entitlement requirements and must expose those requirements clearly to the planner and host

## Recommended repository/module mapping

Near-term repo structure should evolve toward:

- `src/core/`
  profile input types, shared errors, ids, enums
- `src/profiles/`
  built-in profiles and capability definitions
- `src/config/`
  configuration parsing and validation
- `src/session_options/`
  session-option validation and compilation
- `src/planner/`
  fidelity and backend planning
- `src/session/`
  session manager and host-facing handles
- `src/translators/`
  forward and reverse translators, split by profile family and level
- `src/backends/`
  backend API plus concrete implementations
- `src/telemetry/`
  metrics and diagnostics

## Phased build order

### Phase 1

- formalize per-profile input validation
- introduce `SessionPlan`
- separate planner from profile registry
- make backends session-scoped

### Phase 2

- implement configuration loader and session-options compiler
- move translators onto exact profile input contracts
- add telemetry and structured errors

### Phase 3

- add reverse translator contracts
- implement `UHID` reverse-path handling
- add host output bridge

### Phase 4

- split HID translators by profile family
- validate descriptor/report compatibility
- add contract tests for capability-to-translator consistency

### Phase 5

- introduce transport backend state machines
- add transport-specific session planning
- validate `hardware-faithful` flows against real targets

## Acceptance criteria for the target architecture

The architecture is complete when all of the following are true:

- the host creates explicit sessions rather than calling one-shot routing
- planning is based on runtime backend inventory
- every session has a clear degradation summary
- reverse-path output commands work end to end for HID-capable targets
- translators are profile-family-specific where required
- configuration drives session options and policy
- tests can validate all of the above without requiring real hardware for core logic

## Current repo status summary

Today this repository is best understood as:

- a strong conceptual architecture and documentation set
- a narrowed Rust-first design target with explicit per-device input contracts
- not yet the complete production integration architecture

That is a good place to be, as long as the next implementation steps lock in sessions, planners, exact profile input contracts, and reverse flow before backend complexity grows.
