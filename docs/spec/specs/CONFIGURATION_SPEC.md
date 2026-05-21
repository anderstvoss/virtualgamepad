# Configuration Specification

This document defines the recommended configuration format for describing:

- which target controller is being emulated
- which fidelity tier is requested
- which runtime and provider preferences apply to the session
- how reverse-path outputs are surfaced back to the host
- which validation and delivery policies should be enforced

## Document format

Configuration documents are YAML. `gr-config` parses them via `serde_yaml`. JSON is accepted as a strict subset for tools that prefer it (since YAML is a superset of JSON, the same parser handles both).

The most important rule is:

configuration must stay session-oriented and narrow.

That means configuration should describe:

- what device is being created
- what fidelity is requested
- what runtime policy applies
- how reverse outputs are handled

It should not try to be a universal semantic remapping language between unrelated controller families.

## Design goals

- make session configuration human-readable
- make policy choices diff-friendly
- support audit and validation tooling
- keep profile selection separate from backend realization
- keep input validation explicit without inventing a unified controller schema

## Configuration layers

The library should treat configuration as four layers.

### `session`

Declares:

- session id if the host wants to provide one
- target profile id
- requested fidelity tier
- host platform preference
- backend family preference
- provider preference

### `input`

Declares:

- whether full frames, deltas, or both are accepted
- validation strictness for the selected profile contract
- unknown-field policy
- range-handling policy

### `outputHandling`

Declares:

- how reverse output commands are surfaced
- whether they are ignored, logged, forwarded, or bridged
- backpressure policy for reverse outputs

### `validation`

Declares:

- strictness rules
- required reverse-output handling policy if applicable
- unsupported-capability policy

## Human-readable fidelity tier names

Use these names in configuration rather than raw internal layer labels where possible.

- `compatibility`:
  basic host-visible controller behavior, usually backed first by Linux `evdev`
- `identity-aware`:
  controller-specific HID identity and reverse command behavior, usually backed by `hid`
- `hardware-faithful`:
  transport-level impersonation of the real device, usually backed by `transport`

Internal mapping:

- `compatibility` -> `evdev`
- `identity-aware` -> `hid`
- `hardware-faithful` -> `transport`

## Recommended configuration shape

```yaml
session:
  profileId: "dualsense"
  fidelityTier: "identity-aware"
  hostPlatformPreference: "linux"
  backendPreference: "hid"
  providerPreference: "linux-uhid"

input:
  acceptedUpdateKinds:
    - "frame"
    - "delta"
  rejectUnknownFields: true
  rejectOutOfRangeValues: true
  allowMissingOptionalFields: true

outputHandling:
  mode: "callback"
  callbackNamespace: "virtualGamepad"
  backpressurePolicy: "drop-oldest"
  logDroppedOutputs: true
  bridgeCapabilities:
    - "leftRumble"
    - "rightRumble"
    - "playerLightBar"

validation:
  requireSupportedProfile: true
  rejectUnsupportedFidelity: true
  rejectUnsupportedProviderPreference: true
  unsupportedCapabilityPolicy: "report"
```

## Profile-input contract

Every target profile should publish its own input contract.

The configuration layer does not redefine that contract. It only declares how strictly the runtime should enforce it.

A profile input contract should define:

- required fields
- optional fields
- accepted delta fields
- value ranges
- enum domains where applicable
- profile-specific invariants

Examples:

- DualSense may define analog sticks, analog triggers, a center touchpad, motion sensors, and mute button fields
- Xbox 360 may define analog sticks, analog triggers, d-pad buttons, and guide button fields
- Steam Controller-style targets may define pads, paddles, and mode-related fields

## Input validation policy

Configuration should control validation behavior for the chosen profile contract.

Recommended input-policy fields:

- `acceptedUpdateKinds`
- `rejectUnknownFields`
- `rejectOutOfRangeValues`
- `coerceIntegerLikeValues`
- `allowMissingOptionalFields`
- `requireMonotonicSequence`

Recommended meanings:

- `rejectUnknownFields`:
  reject fields not declared by the profile input contract
- `rejectOutOfRangeValues`:
  reject values outside the declared profile ranges
- `coerceIntegerLikeValues`:
  allow lossless coercion where the runtime considers it safe
- `requireMonotonicSequence`:
  reject or warn on non-monotonic sequence ids

## Reverse output handling rules

Reverse output configuration should describe what the host wants to do with software-driven controller outputs.

Possible handling modes:

- callback
- channel
- log only
- pass-through to physical device
- ignore

Possible backpressure policies:

- `drop-newest`
- `drop-oldest`
- `block-producer`

Optional reverse-output handling fields:

- `bridgeCapabilities`
- `callbackNamespace`
- `stateFieldPrefix`
- `logDroppedOutputs`
- `maxQueueDepth`

## Validation procedure

Configuration validation should happen in four passes.

### Pass 1: schema validation

Validate:

- document structure
- required top-level sections
- field types
- enum values

### Pass 2: profile validation

Validate:

- requested profile exists
- requested profile publishes an input contract
- requested profile publishes the capability metadata needed by output handling

### Pass 3: session-planning validation

Validate:

- requested fidelity tier is legal for the profile
- provider and backend preferences are structurally valid
- impossible combinations are reported clearly

### Pass 4: runtime-policy validation

Validate:

- input-policy fields are internally consistent
- output-handling mode is fully specified
- unsupported-capability policy is valid

### Unknown config fields

- unknown top-level sections are rejected
- unknown fields inside known sections are warned by default and rejected when `validation.rejectUnknownConfigFields: true`
- the default is warn so additive spec changes do not break existing configs; hosts that want strict forward-compatibility opt in

## Unsupported-capability policy

Some requested capabilities will not be realizable in every environment.

Configuration should make the host policy explicit.

Recommended values:

- `reject`
- `report`
- `ignore`

Recommended behavior:

- `reject`:
  fail session creation when required capabilities are unavailable
- `report`:
  allow degraded session creation but expose warnings and unsupported-capability details
- `ignore`:
  allow startup with minimal reporting, useful mainly for controlled internal testing

## Open questions to make explicit in config

Some behaviors should never be left implicit.

Configurations should explicitly answer:

- should unknown profile input fields be rejected or warned?
- should out-of-range values be rejected or clamped?
- are reverse outputs ignored, logged, bridged, or dispatched to callbacks?
- should unsupported capabilities fail startup or only produce degradation reports?

## Provider preference rule

Provider preferences are hints. The planner may reject a request when the preferred provider is unavailable, but it must never bypass capability validation to honor a preference. Strict failure when a preferred provider is missing is expressed via `validation.rejectUnsupportedProviderPreference: true`; otherwise the planner is free to fall back to an admissible provider and record the substitution in the session plan.

## Final recommendation

Treat the configuration file as a session-policy document, not a universal controller remapping document.

If a human can read the config and answer:

- what device is being created
- what fidelity is being asked for
- how strict input validation should be
- how reverse outputs are handled

then the configuration format is doing its job.
