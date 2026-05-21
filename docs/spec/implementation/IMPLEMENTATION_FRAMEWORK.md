# Gamepad Emulation Framework

This document defines a language-agnostic implementation framework for a reusable gamepad emulation subsystem that lives inside a larger host program.

The framework assumes:

- the host wants to emulate one of several specific target controllers
- each active virtual device is created for one concrete target profile
- the host can provide exact input for that chosen profile
- some targets only need Linux input exposure
- some targets require HID identity and reverse report handling
- some targets require transport-level spoofing

The goal is to define a stable internal architecture that can be implemented in any language with strong module boundaries and host-friendly APIs.

## Design goals

- require exact per-profile input contracts instead of a library-wide unified control model
- isolate backend realization code from session, planning, and translation logic
- support both forward input flow and reverse software-to-controller flow
- let a host program create and manage multiple virtual device sessions
- express device identity separately from runtime backend choice
- keep high-level planning logic independent from low-level syscall code
- make testing possible without real kernel devices

## Scope

This framework covers:

- profile-specific input ingestion
- target profile selection
- capability definition
- emulation-level selection
- translation into backend frames
- reverse-path handling for rumble, LEDs, audio, and other outputs
- backend lifecycle and event routing

This framework does not prescribe:

- a specific implementation language
- a specific threading/runtime model
- a specific IPC mechanism
- a specific Linux FFI binding strategy
- exact HID report descriptors for proprietary devices
- a built-in universal remapping layer between unrelated controller families

Related documents:

- [ARCHITECTURE_SPEC.md](../specs/ARCHITECTURE_SPEC.md)
- [RUST_IMPLEMENTATION_PLAN.md](../implementation/RUST_IMPLEMENTATION_PLAN.md)
- [RUST_IMPLEMENTATION_SPEC.md](../implementation/RUST_IMPLEMENTATION_SPEC.md)
- [FIDELITY_GUIDE.md](../specs/FIDELITY_GUIDE.md)
- [CONFIGURATION_SPEC.md](../specs/CONFIGURATION_SPEC.md)

## Core idea

Treat controller emulation as coordination across three independent dimensions:

1. exact input for one chosen target profile
2. target device definition
3. backend realization layer

This avoids coupling a specific device profile directly to a specific syscall implementation while also avoiding a broad unified input abstraction that every real device must later escape.

## Top-level modules

The recommended framework is composed of the following modules.

### `ProfileInputBoundary`

Responsibility:

- receive snapshots or deltas for one selected target profile
- validate field presence, value ranges, and payload shape
- stamp timestamps and sequence ids

Consumes:

- host-generated profile-specific input

Produces:

- immutable `ProfileInputFrame`

Notes:

- this is a thin validation boundary, not a universal adapter
- this should not reinterpret one profile payload as another profile family

### `CapabilityRegistry`

Responsibility:

- describe what each target controller supports in both directions
- expose a structured query interface for host logic

Consumes:

- target profile definitions

Produces:

- `ControllerCapabilities`

Notes:

- this is where buttons, pads, motion sensors, audio endpoints, rumble channels, trigger effects, and lighting are declared
- this module answers "can this target do X?" independently of whether the current backend can realize it

### `ProfileRegistry`

Responsibility:

- store known device profiles
- expose profile lookup by id
- publish identity metadata and supported fidelity levels

Consumes:

- static or dynamic profile definitions

Produces:

- `ControllerProfile`

Notes:

- profiles should contain identity, capabilities, fidelity support, and translation metadata
- profiles should not contain live runtime state
- profiles should not pick concrete backend instances

### `EmulationPlanner`

Responsibility:

- choose the minimum viable realization layer for a requested target and goal
- validate whether available backends can satisfy the target

Consumes:

- profile
- goal
- available backend inventory
- host-platform and provider inventory

Produces:

- `EmulationPlan`

Notes:

- this is where you decide `evdev` versus `hid` versus `transport`
- this should produce degradation warnings when requested fidelity exceeds current backend support

### `ForwardTranslator`

Responsibility:

- convert profile-specific input into backend-specific outbound frames

Consumes:

- `ProfileInputFrame`
- `ControllerProfile`
- `EmulationPlan`

Produces:

- `BackendInputFrame`

Notes:

- examples: evdev event batches, HID input reports, USB/Bluetooth payloads
- translators should be stateless or session-prepared where possible
- translators should be split by device family where protocol shape requires it

### `ReverseTranslator`

Responsibility:

- convert software-driven controller outputs into framework-level effect commands

Consumes:

- backend-originating output reports or commands
- profile metadata

Produces:

- `ControllerOutputCommand`

Notes:

- examples: rumble requests, LED updates, audio mode changes, trigger effects
- this is essential for HID and transport targets

### `BackendAdapter`

Responsibility:

- realize the emulated device in the operating system or transport layer
- provide both write and receive hooks

Consumes:

- backend input frames
- lifecycle commands

Produces:

- backend events
- backend-originating reverse messages

Notes:

- there should be separate adapters for `uinput`, `UHID`, and transport-level implementations
- adapters should be the only place where syscalls, ioctls, file descriptors, or transport sockets are handled directly

### `SessionController`

Responsibility:

- manage one active emulation session
- connect validated profile input, plan, translators, and backend
- expose host-facing session controls

Consumes:

- host commands
- profile selection
- profile-specific input updates

Produces:

- device session lifecycle
- telemetry
- capability and support status

Notes:

- this is the best candidate for the module or class that a larger program instantiates directly

### `TelemetryAndDiagnostics`

Responsibility:

- expose counters, errors, support gaps, state transitions, and latency

Consumes:

- session events from all modules

Produces:

- structured logs
- metrics
- snapshots for debugging

## Primary public interfaces

The host program should interact with the framework through a small set of stable public interfaces.

### `GamepadEmulationManager`

Recommended responsibility:

- entry point owned by the host application
- stores registries and backend inventory
- creates and tears down sessions

Recommended host API:

```text
initialize(config)
registerProfile(profile)
registerBackend(adapter)
listProfiles()
getCapabilities(profileId)
createSession(sessionConfig)
shutdown()
```

### `GamepadEmulationSession`

Recommended responsibility:

- one active target-device realization
- accepts profile-specific input updates
- emits reverse-path output callbacks

Recommended host API:

```text
start()
stop()
pause()
resume()
sendInput(profileInputFrame)
sendInputDelta(profileInputDelta)
getStatus()
subscribeToOutputCommands(handler)
subscribeToDiagnostics(handler)
```

### `BackendAdapter`

Recommended backend contract:

```text
adapterId()
supportedLevels()
open(descriptor, capabilities, plan)
writeInput(frame)
pollReverseEvents()
setOutputListener(listener)
flush()
close()
```

### `TranslatorSet`

Recommended contract:

```text
translateForward(input, profile, plan) -> BackendInputFrame
translateReverse(backendMessage, profile, plan) -> ControllerOutputCommand[]
```

## Canonical data contracts

The framework should define language-neutral schemas for the following contracts.

## `ProfileInputFrame`

Purpose:

- exact, profile-scoped representation of host input for one chosen target device

Required properties:

- `profileId`
- `timestamp`
- `sequenceNumber`
- `payload`

Rules:

- `payload` must match the selected profile's concrete input contract
- unknown fields should be rejected or warned by policy
- value ranges should be validated against the chosen profile contract
- the framework should not normalize unrelated device families into one shared shape

## `ControllerCapabilities`

Purpose:

- describe the full feature surface of a target controller

Structure:

- `inputCapabilities`
- `outputCapabilities`

Recommended input groups:

- buttons
- axes
- pads
- motion sensors
- microphones
- speakers
- mode switches
- extra controls

Recommended output groups:

- rumble
- haptics
- lighting
- trigger effects
- audio output
- display
- notifications
- mode control

Each capability item should include:

- stable name
- category
- optional human label
- supported ranges or modes
- channel count if applicable
- transport/backend requirements if relevant
- optionality

## `ControllerProfile`

Purpose:

- define a target device

Recommended fields:

- `profileId`
- `displayName`
- `identity`
- `capabilities`
- `supportedGoals`
- `translationHints`
- `descriptorTemplates`
- `reverseCommandSupport`
- `inputContract`

### `identity`

Recommended fields:

- logical device family
- vendor id
- product id
- version
- bus hints
- HID descriptor reference
- transport protocol family

### `inputContract`

Recommended fields:

- required fields
- optional fields
- value ranges
- delta support rules
- validation policy hints

## `EmulationPlan`

Purpose:

- concrete runtime decision for a given session

Recommended fields:

- `profileId`
- `goal`
- `selectedLevel`
- `selectedBackendId`
- `forwardTranslatorId`
- `reverseTranslatorId`
- `enabledCapabilitySet`
- `degradedCapabilitySet`
- `warnings`

This contract is especially valuable because the host can inspect whether the current environment can faithfully realize the requested target.

## `BackendInputFrame`

Purpose:

- backend-specific outbound payload

Examples:

- `evdev` event batch
- HID input report
- USB packet set
- Bluetooth packet set

Recommended common fields:

- `level`
- `profileId`
- `sequenceNumber`
- `timestamp`
- `payload`

## `ControllerOutputCommand`

Purpose:

- normalized reverse-path command from software to the virtual controller abstraction

Examples:

- set rumble motors
- set RGB light
- play audio sample
- configure adaptive trigger
- switch Steam desktop mode

Recommended common fields:

- `commandType`
- `targetCapability`
- `parameters`
- `duration`
- `priority`
- `source`

## Lifecycle model

The framework should support a predictable session lifecycle.

### Initialization

1. host creates `GamepadEmulationManager`
2. manager loads profiles
3. manager loads available backends
4. manager runs backend capability discovery

### Session creation

1. host requests a session for a target profile and goal
2. planner builds an `EmulationPlan`
3. session creates backend instance
4. backend is opened with descriptor and capability context
5. reverse listeners are attached
6. session transitions to active

### Runtime loop

1. host submits `ProfileInputFrame`
2. session validates it against the selected profile contract
3. translator emits `BackendInputFrame`
4. backend writes the frame
5. backend polls or receives reverse messages
6. reverse translator turns those into `ControllerOutputCommand`
7. host callback handles the command or logs it

### Reconfiguration

Supported runtime operations:

- target profile switch
- goal switch
- backend switch
- capability downgrade or upgrade
- output-command policy change

When switching target or backend, the framework should prefer a controlled session rebuild instead of in-place mutation unless the backend explicitly supports dynamic reconfiguration.

### Shutdown

1. stop accepting new input
2. flush pending writes if needed
3. detach reverse listeners
4. close backend
5. mark session closed

## Backend taxonomy

The framework should model backend adapters by capability, not by platform label alone.

### `EvdevVirtualPadBackend`

Use when:

- the target only needs Linux-visible gamepad input

Must support:

- virtual device creation
- event capability declaration
- batched event write

May support:

- Linux force feedback

Does not usually support:

- rich HID reverse commands
- transport identity

### `HidVirtualDeviceBackend`

Use when:

- the target needs HID identity or reverse report flow

Must support:

- report descriptor provisioning
- input report submission
- output report receipt
- feature report exchange if needed

May support:

- hidraw-visible identity
- bidirectional command channels for lighting, haptics, and audio

### `TransportSpoofBackend`

Use when:

- exact USB, Bluetooth, or controller-protocol identity is required

Must support:

- enumeration semantics
- device descriptors or equivalent
- transport-specific packet exchange

May support:

- wireless pairing behavior
- transport-specific audio paths
- vendor protocol timing quirks

## Reverse-path architecture

Forward input emulation is only half of the system. A larger host program will often want to observe or relay software-driven controller outputs.

Examples:

- game requests rumble
- Steam sends haptic pulse commands
- software changes LED color
- host wants to expose trigger effect changes in UI

Recommended reverse-path flow:

```text
backend reverse event
  -> reverse translator
  -> normalized controller output command
  -> host callback or output dispatcher
```

### `OutputCommandDispatcher`

Responsibility:

- distribute normalized output commands to host subscribers

Possible subscribers:

- logging
- UI preview
- physical passthrough layer
- analytics
- test harness

### `PhysicalFeedbackBridge`

Optional responsibility:

- map reverse commands from the virtual target back to a real physical controller

Example:

- virtual DualSense receives rumble command
- host forwards that to the real source controller if it supports rumble

## Capability negotiation

Not every backend can realize every profile capability. The framework should expose negotiation results explicitly.

Recommended process:

1. profile declares theoretical capabilities
2. backend declares realizable capabilities
3. planner computes:
   - enabled capabilities
   - degraded capabilities
   - unsupported capabilities
4. session exposes the result to the host

Examples:

- DualSense profile requests audio and adaptive triggers
- UHID backend supports HID reports but not controller audio
- plan marks audio as unsupported and trigger effects as enabled

This is better than silently dropping features.

## Error model

The framework should use structured errors with domain-specific categories.

Recommended categories:

- invalid profile input
- unknown profile
- unsupported emulation goal
- backend unavailable
- backend open failure
- write failure
- reverse translation failure
- unsupported capability
- degraded session

Each error should ideally include:

- category
- stable code
- human-readable message
- session id if applicable
- profile id if applicable
- backend id if applicable
- recoverability hint

## Concurrency model

The framework should avoid hiding concurrency assumptions.

Recommended approach:

- single writer per session
- ordered input updates
- reverse events processed on a separate queue if needed
- immutable or copy-on-write input and frame objects

Good implementation patterns:

- actor per session
- single-threaded event loop per session
- queue feeding a backend worker
- synchronous shell around an async backend

Avoid:

- sharing mutable backend state across sessions
- interleaving writes from multiple sessions into one device adapter
- coupling reverse event handling to the main render loop without buffering

## Test strategy

A good framework design is testable in three layers.

### Pure logic tests

Test:

- capability registry
- profile lookup
- plan selection
- forward translation
- reverse translation

No kernel or device access required.

### Adapter contract tests

Test:

- backend open and close lifecycle
- frame serialization
- reverse event delivery

Use fake adapters or loopback harnesses.

### End-to-end integration tests

Test:

- session startup
- input routing
- reverse output callbacks
- downgrade behavior when backend support is missing

Run these only where Linux device interfaces are available.

## Recommended host embedding pattern

For a larger program, the most practical architecture is:

```text
Host application
  -> GamepadEmulationManager
      -> ProfileRegistry
      -> CapabilityRegistry
      -> BackendRegistry
      -> SessionFactory
  -> active GamepadEmulationSession
      -> EmulationPlanner
      -> ForwardTranslator
      -> ReverseTranslator
      -> BackendAdapter
      -> OutputCommandDispatcher
```

### Why this shape works well

- the host only needs to keep one high-level manager reference
- individual sessions are isolated and restartable
- target selection is runtime-configurable
- test doubles can be inserted at every boundary

## Recommended implementation classes or modules

If your language supports classes, the following class set is a strong default:

- `GamepadEmulationManager`
- `GamepadEmulationSession`
- `ProfileRegistry`
- `CapabilityRegistry`
- `BackendRegistry`
- `EmulationPlanner`
- `ForwardTranslator`
- `ReverseTranslator`
- `OutputCommandDispatcher`
- `TelemetrySink`

If your language prefers modules and interfaces:

- `manager`
- `session`
- `profiles`
- `capabilities`
- `planning`
- `translators.forward`
- `translators.reverse`
- `backends.evdev`
- `backends.hid`
- `backends.transport`
- `diagnostics`

## Minimal viable implementation

If you want to build this incrementally, implement in this order:

1. `ProfileInputFrame` contract
2. `ControllerCapabilities`
3. `ControllerProfile`
4. `ProfileRegistry`
5. `EmulationPlanner`
6. `ForwardTranslator` for `evdev`
7. `EvdevVirtualPadBackend`
8. `GamepadEmulationSession`
9. `ReverseTranslator` plus output command model
10. `HidVirtualDeviceBackend`
11. `TransportSpoofBackend`

This sequence gives usable value early while preserving the long-term architecture.

## Decision guidance by target type

Use `evdev` when:

- the consumer only needs a Linux gamepad
- identity does not matter
- reverse features are optional or minimal

Use `hid` when:

- software inspects HID identity
- output reports matter
- controller-specific features need to round-trip

Use `transport` when:

- exact USB or Bluetooth identity matters
- a platform expects a specific on-wire protocol
- controller-specific pairing or enumeration behavior is part of the target

## Final recommendation

The most important architectural rule is:

keep **device definition**, **translation logic**, and **backend realization** as separate modules.

That separation is what lets this framework remain language-agnostic, embeddable in a larger application, and expandable from a simple virtual evdev pad into a full HID or transport-level controller emulation system without broadening the host-facing API into a universal controller abstraction.
