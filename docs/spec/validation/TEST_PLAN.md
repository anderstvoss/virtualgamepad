# Test Plan

This document defines a formal, language-agnostic test plan for the gamepad emulation library.

The purpose of the test plan is to ensure:

- the library behaves correctly as a reusable subsystem inside a larger program
- profile-specific input contracts are validated correctly
- fidelity-tier planning is correct
- capability definitions are accurate and complete
- reverse-path behavior is observable and validated
- backend integrations are verified at the appropriate level

Related validation strategy documents:

- [HEADLESS_TEST_STRATEGY.md](../validation/HEADLESS_TEST_STRATEGY.md)
- [DEVICE_SPEC_VALIDATION_PLAN.md](../validation/DEVICE_SPEC_VALIDATION_PLAN.md)

## Test strategy

The system should be tested in four layers:

1. unit tests
2. contract tests
3. integration tests
4. fidelity and host validation tests

Unit tests should cover deterministic logic.

Contract tests should verify module boundaries and schema consistency.

Integration tests should verify multi-module runtime behavior.

Fidelity and host validation tests should verify that claimed device behavior matches the requested fidelity tier.

Most development should be possible in headless remote environments. Real-hardware machines should primarily generate and refresh fixtures, validate provider-specific behavior, and compare virtual devices against physical-device evidence.

## Core testing principles

- test pure logic first
- test profile-specific input validation explicitly
- test capability definitions as first-class artifacts
- test degradation behavior explicitly
- test reverse-path outputs as seriously as forward input
- never treat a higher fidelity claim as valid without tier-specific validation

## Test inventory by component

## 1. Profile input contracts

### Scope

- per-profile input frame definitions
- per-profile delta definitions
- validation rules
- sequencing rules

### Unit tests

- accepts valid inputs for each implemented profile
- rejects or warns on invalid values according to policy
- preserves optional fields correctly
- applies timestamps correctly
- applies sequence numbers correctly
- accepts only legal delta fields for the chosen profile
- rejects unknown fields when policy requires it

### Edge-case tests

- maximum and minimum axis values
- zeroed input
- rapidly repeated updates
- absent optional capabilities such as pads or motion sensors
- valid frame followed by valid delta
- out-of-order sequence handling

## 2. Capability definitions

This area must be explicitly tested.

Capability definitions of the output devices are part of the library contract and should not be treated as documentation-only metadata.

### Scope

- profile-declared input capabilities
- profile-declared output capabilities
- supported input functions
- supported output functions
- required functions

### Unit tests

- every profile exposes a capability object
- every capability object includes both input and output sections
- every declared input capability has a stable name and category
- every declared output capability has a stable name and category
- every declared capability belongs to an allowed capability group
- no duplicate capability names exist within the same profile where uniqueness is required
- capability summaries match the underlying declarations

### Explicit output-device capability tests

These tests must exist for each target output device profile.

For each profile, verify:

- declared rumble channels are present when expected
- declared haptic outputs are present when expected
- declared lighting outputs are present when expected
- declared audio outputs are present when expected
- declared trigger effects are present when expected
- declared display or notification outputs are present when expected
- unsupported output categories are explicitly absent rather than ambiguous

### Capability accuracy tests

For each profile, validate that:

- required capabilities for the profile family are present
- capabilities that should not exist are not present
- capability properties such as channel count, modes, range, or optionality are correct

### Capability-to-function consistency tests

For each profile, validate that:

- each supported input function corresponds to a declared input capability
- each supported output function corresponds to a declared output capability
- each required function is backed by a declared capability
- no published required function points to a missing capability

## 3. Profile registry

### Scope

- profile lookup
- profile metadata
- fidelity support by goal

### Unit tests

- known profiles load successfully
- unknown profiles fail with structured errors
- identity metadata is returned correctly
- supported-goal metadata is returned correctly
- profile capability references remain intact
- every profile exposes an input contract

### Consistency tests

- every profile has at least one valid fidelity tier
- every declared translator family is syntactically valid
- every profile input contract is internally consistent

## 4. Configuration loader and validator

This is one of the highest-priority test areas.

### Scope

- configuration parsing
- session-policy validation
- profile validation
- output-handling validation
- fidelity-tier validation

### Unit tests

- valid configuration parses successfully
- invalid top-level schema is rejected
- unknown profile ids are rejected
- unsupported fidelity tiers are rejected
- unsupported provider preferences are rejected when configured strictly
- invalid output-handling blocks are rejected
- invalid backpressure policy values are rejected
- incompatible validation policies are rejected

### Policy tests

These tests should explicitly verify policies such as:

- reject unknown profile input fields
- reject out-of-range values
- accept only `frame` updates
- accept both `frame` and `delta` updates
- report unsupported capabilities without rejecting startup
- reject startup when unsupported-capability policy is strict

### Completeness tests

- every required config field for a session is present
- callback output mode requires callback configuration
- channel output mode requires queue or delivery configuration
- unsupported optional provider preferences produce warnings instead of hard failures when configured that way

## 5. Emulation planner

### Scope

- fidelity-level selection
- backend selection
- degradation reporting

### Unit tests

- selects `compatibility` for evdev-only targets when appropriate
- selects `identity-aware` when HID identity is required
- selects `hardware-faithful` when transport fidelity is required
- honors backend preference when legal
- rejects impossible plans when no supported backend exists
- rejects or degrades `identity-aware` plans when reverse output or feature report handling is missing
- rejects or degrades `hardware-faithful` plans when transport enumeration, control flow, or reverse packet handling is missing
- computes enabled capabilities correctly
- computes degraded capabilities correctly
- computes unsupported capabilities correctly

### Degradation tests

- request `hardware-faithful` with only HID support available
- request `hardware-faithful` with only UHID support available
- request `identity-aware` with missing reverse output support
- request `compatibility` for a profile that has richer optional outputs

Each case should assert:

- selected level
- warning set
- unsupported capabilities
- whether session creation is allowed

## 6. Forward translators

### Scope

- profile-input to backend-frame translation

### Unit tests

- button encoding is correct
- axis encoding is correct
- stick scaling is correct
- trigger scaling is correct
- inversion logic is correct where supported
- d-pad encoding is correct
- pad or touch encoding is correct
- motion encoding is correct
- neutral input produces a neutral frame
- frame ordering and synchronization are correct

### Per-target input tests

For each target profile, test representative inputs for:

- primary face buttons
- d-pad directions
- both sticks
- triggers
- guide, start, and back style buttons
- any target-specific controls such as pad click, mute, paddles, or record key

## 7. Reverse translators

### Scope

- backend reverse messages to normalized output commands

### Unit tests

- rumble report becomes normalized rumble command
- LED report becomes normalized lighting command
- trigger effect report becomes normalized trigger-effect command
- audio-mode or speaker command becomes normalized audio output command
- unsupported backend reports are handled predictably
- malformed backend reports do not corrupt session state

### Output capability conformance tests

For each profile with output capabilities, verify:

- reverse translator only emits commands for declared output capabilities
- translator does not emit commands for capabilities the profile does not declare
- normalized command names match published output function names

For each `identity-aware` or `hardware-faithful` profile, verify:

- a reverse translator is assigned before the tier can be claimed
- output and feature report fixtures translate into declared output capabilities
- missing reverse translator coverage fails the support-claim tests

## 8. Backend adapter contracts

### Scope

- backend lifecycle and frame handoff

### Contract tests with fakes or mocks

- backend opens with expected descriptor and capability context
- backend accepts frames of the right type
- backend rejects frames of the wrong fidelity level
- backend emits reverse events to the registered listener
- backend closes cleanly
- repeated close behavior is safe according to contract

### Per-backend contract coverage

#### Compatibility backend

- accepts evdev frames
- exposes expected capability declaration hooks
- optionally exposes force-feedback support

#### Identity-aware backend

- accepts HID input reports
- provides output report and feature report hooks
- exposes identity metadata and descriptor hooks
- fails support-claim tests if output or feature report handling is stubbed

#### Hardware-faithful backend

- accepts transport packet objects
- exposes enumeration and control-flow hooks
- provides reverse transport message hooks
- fails support-claim tests if enumeration, control flow, or reverse packet handling is absent

## 9. Session controller

### Scope

- runtime orchestration of one active emulation session

### Integration tests

- start opens backend successfully
- stop closes backend successfully
- pause and resume work if supported
- input updates flow through translator and backend in order
- reverse events are received and dispatched
- target profile switch triggers controlled reconfiguration
- fidelity-tier switch triggers replanning or session rebuild

### Failure-path tests

- backend open failure leaves session in safe state
- frame write failure is surfaced correctly
- reverse translator failure does not corrupt the session
- unsupported configuration prevents session start

## 10. Manager and registries

### Scope

- top-level host-facing subsystem behavior

### Integration tests

- manager initializes registries correctly
- manager lists profiles correctly
- manager returns capability information correctly
- manager creates sessions correctly
- manager shuts down active sessions correctly

### Multi-session tests

If multi-session support exists:

- sessions remain isolated
- one session failure does not corrupt another
- backend resources are not shared unsafely

## 11. Diagnostics and telemetry

### Scope

- structured errors
- warnings
- counters
- latency or state-transition metrics

### Unit tests

- errors carry category and stable code
- degradation warnings are emitted correctly
- metrics increment correctly on normal flow
- metrics increment correctly on failure flow

## 12. Fidelity-tier validation tests

These are not pure unit tests, but they are required before claiming tier support.

## `compatibility` validation

Verify:

- expected Linux-visible controls exist
- games and test tools recognize a usable controller
- analog ranges are correct
- validation does not claim HID, USB, Bluetooth, or physical-device identity

## `identity-aware` validation

Verify:

- target software recognizes the intended controller family
- HID descriptors parse correctly
- HID input reports match the profile family descriptor
- reverse output commands are received
- feature reports are handled for the implemented profile subset
- declared output capabilities actually work at least at the implemented subset
- missing reverse output or feature report handling prevents claiming the tier

## `hardware-faithful` validation

Verify:

- enumeration works as expected
- protocol-specific control flow works
- required transport-level identity checks pass
- transport packets are encoded and decoded correctly for the implemented profile subset
- reverse-path behavior works under the real host environment
- missing enumeration, control-flow, or reverse packet handling prevents claiming the tier

## Test matrix by priority

### Priority 0

These tests should exist before the library is considered usable.

- profile-input validation
- capability definition tests
- explicit output-device capability tests
- profile lookup
- configuration validation
- planner tests
- forward translator tests
- reverse translator tests
- session start, update, and stop tests

### Priority 1

- backend contract tests
- degradation behavior tests
- diagnostics and telemetry tests
- manager integration tests

### Priority 2

- multi-session tests
- performance and sustained-update tests
- recovery tests for malformed reverse events
- hot-reconfiguration edge-case tests

## Acceptance criteria

The library should not be considered complete unless:

- all public interfaces have success-path tests
- all validation rules have failure-path tests
- all profile input contracts have representative tests
- all declared output-device capabilities are explicitly tested
- every profile’s required capabilities and required functions are verified
- degraded-fidelity behavior is surfaced explicitly in tests
- reverse-path behavior is covered for every profile that declares output capabilities
- `identity-aware` support claims fail without reverse translator coverage and output/feature report handling
- `hardware-faithful` support claims fail without transport enumeration, control-flow, and reverse packet validation
- descriptor-to-translator contract tests exist for every HID or transport profile family
- documentation review checks prevent core-runtime specs from reintroducing a universal controller or remapping model

## Recommended test artifacts

The implementation should maintain reusable test fixtures for:

- valid profile input frames
- invalid profile input frames
- sample profile definitions
- sample capability definitions
- valid configuration files
- invalid configuration files
- fake backend adapters
- representative reverse output reports

## Final recommendation

Treat capability definitions, especially output-device capability definitions, as executable contract data.

They should be tested with the same seriousness as translators and planners, because the correctness of rumble, lighting, haptics, audio, trigger effects, and other reverse-path features depends on those declarations being accurate, complete, and internally consistent.
