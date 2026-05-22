# Fidelity Guide

This document defines the three controller emulation fidelity tiers in human-understandable terms and describes how to create and validate each one.

For remote-first automation and real-device evidence capture, see [HEADLESS_TEST_STRATEGY.md](../validation/HEADLESS_TEST_STRATEGY.md) and [DEVICE_SPEC_VALIDATION_PLAN.md](../validation/DEVICE_SPEC_VALIDATION_PLAN.md).

## Tier names

Use these names externally in documentation and configuration.

| Human name | Internal level | What it means |
| --- | --- | --- |
| `compatibility` | `evdev` | Good Linux gamepad behavior without pretending to be the real controller at the HID or wire level |
| `identity-aware` | `hid` | The target looks like the expected controller to software that cares about HID identity and reverse feature flow |
| `hardware-faithful` | `transport` | The target behaves like the real hardware at the USB/Bluetooth protocol level |

## Support-claim rule

A tier is not supported until its validation requirements pass end to end for at least one target profile.

- `compatibility` support requires a host-visible usable virtual gamepad.
- `identity-aware` support requires HID descriptor/report identity and bidirectional output/feature report handling.
- `hardware-faithful` support requires transport-level enumeration, control-flow behavior, packet handling, and reverse packet handling over USB, Bluetooth, or another explicitly modeled bus.
- missing bidirectional behavior must cause rejection or explicit degradation; it must never be hidden behind a higher support claim.

## Internal modeling guidance

The fidelity tiers should not be treated as one identical execution model with more flags enabled.

- `compatibility` is usually a gameplay-input-centric model
- `identity-aware` is usually a descriptor/report model with reverse report and feature handling
- `hardware-faithful` is usually a transport-session model with enumeration, control flow, and packet/state handling

This matters because later features such as accessories, expansion ports, side channels, or vendor-specific reverse commands tend to belong naturally to the richer tier models rather than to the lowest common denominator.

## Tier 1: `compatibility`

### Goal

Make software see a usable controller with the expected gameplay inputs.

### Typical implementation

- create a Linux virtual input device
- expose the expected buttons and axes
- emit `evdev` events through `uinput`

### Best for

- SDL and native Linux games
- remapping tools
- simple controller virtualization

### What it does well

- low complexity
- broad compatibility
- easy debugging

### What it does not guarantee

- true controller identity
- HID descriptor matching
- vendor-specific features
- full reverse-path behavior
- extensible accessory or side-channel behavior beyond what the selected compatibility provider can expose

### Reverse-path note for Linux `uinput`

`uinput`'s only reverse channel is force-feedback (`EV_FF`) effect uploads received via `read` on the same fd. Rumble may be exposed through this channel, but lighting, trigger effects, audio, and feature reports are structurally unavailable at this tier. Reverse-path surface at `compatibility` is strictly smaller than at `identity-aware`.

### Creation procedure

1. define the target’s required gameplay functions
2. create a capability-minimal evdev device
3. map profile-specific input fields to evdev controls
4. emit correct ranges and synchronization events
5. add optional Linux force-feedback support if needed

### Validation procedure

Validate in three layers:

- static:
  confirm button and axis declarations match the profile
- behavioral:
  confirm games and tools see a usable gamepad
- experiential:
  confirm buttons, sticks, triggers, and d-pad feel correct in use

Suggested checks:

- does `/dev/input/event*` appear?
- do capability queries show the expected keys and axes?
- do SDL or test apps recognize a gamepad?
- do analog ranges and deadzones behave correctly?

### Reverse-engineering need

Usually low.

For this tier, open Linux input conventions and public controller mappings are often enough.

## Tier 2: `identity-aware`

### Goal

Make software recognize the target as the intended controller family, including reverse-path feature flow.

### Typical implementation

- create a virtual HID device
- provide the expected report descriptor
- emit HID input reports
- receive output and feature reports

### Best for

- Steam-family controller identity
- DualSense-like software integration
- controller features that depend on HID descriptors and output reports

### What it does well

- stronger device identity
- bidirectional feature handling
- better support for lighting, rumble, haptics, and special commands

### What it does not guarantee

- exact transport behavior
- full USB or Bluetooth parity
- vendor timing quirks
- complete support for transport-only accessory negotiation when the real device relies on bus-level behavior

### Creation procedure

There are two valid approaches.

#### Approach A: open-spec or public-behavior implementation

Use when:

- the target publishes descriptors or behavior
- Linux kernel drivers and open implementations are enough to reconstruct the behavior

Procedure:

1. gather public descriptors, report formats, and public driver behavior
2. define a HID report model
3. implement forward input reports
4. implement reverse output and feature report parsing
5. expose host-visible normalized output commands
6. validate against real software expectations

#### Approach B: reverse-engineered implementation

Use when:

- public specs are incomplete
- software relies on private report structure or undocumented behavior

Procedure:

1. capture descriptors and report traffic from a real device
2. identify required input reports, output reports, and feature reports
3. determine which fields are mandatory versus cosmetic
4. replay observed traffic against test software
5. replace replay with structured implementation
6. refine until the target software behaves identically enough

### Validation procedure

Validate in four layers:

- descriptor validation
- report validation
- software-recognition validation
- reverse-command validation

Suggested checks:

- does the HID report descriptor parse cleanly?
- does the target software recognize the device family?
- are output commands such as rumble and LED changes received?
- do feature queries succeed where expected?
- are unknown or profile-specific reverse commands preserved as typed profile-specific events rather than dropped?

### Reverse-engineering guidance

Recommended sources:

- Linux kernel drivers
- public HID descriptors
- hidraw captures
- USB captures
- Bluetooth captures
- open-source community drivers

Recommended method:

1. observe a real device in a controlled session
2. capture idle behavior
3. capture one feature at a time
4. correlate each user action or software output with report deltas
5. build a field map
6. validate each field independently before combining them

### Profile-specific capability summaries (informational)

These tables are **non-normative reviewer aids**. The normative source for what a profile claims is the `ControllerProfile` struct in `gr-profiles` plus the per-profile capability snapshots produced by `vgpd-demo show-capabilities <profile_id>`. Tables here exist so manual gate checklists have a doc anchor to read against before Phase 2 lands those snapshots.

#### DualSense (`profile_id: dualsense`)

Input capabilities:

- face buttons: `cross`, `circle`, `square`, `triangle`
- shoulder buttons: `l1`, `r1`
- stick clicks: `l3`, `r3`
- system buttons: `create`, `options`, `ps`, `touchpad_click`
- d-pad: `up`, `down`, `left`, `right` (directional booleans)
- twin analog sticks: `left_x`, `left_y`, `right_x`, `right_y`
- analog triggers: `l2`, `r2`
- touchpad: two absolute multi-touch contacts (`contact_1`, `contact_2`) each with `active`, `x`, `y`
- motion: gyroscope (3-axis) + accelerometer (3-axis)
- microphone (PCM source, separate session sink)

Output capabilities:

- rumble (dual-rotor)
- adaptive trigger effects (per trigger)
- RGB lightbar
- player-indicator LEDs
- audio mode commands (discrete: mute, route, gain) via `OutputPayload::Audio`
- PCM speaker (separate session sink, not a discrete command)
- haptic feedback through the high-resolution motor path where the provider realizes it

Reverse-path note:

- declaring `audio mode commands` at `identity-aware` does not by itself declare PCM speaker / microphone realization; those are gated separately on provider support per [audio stream contract](../implementation/RUST_IMPLEMENTATION_SPEC.md#audio-stream-contract).

#### Xbox 360 (`profile_id: xbox360`)

Input capabilities:

- face buttons: `a`, `b`, `x`, `y`
- shoulder buttons: `lb`, `rb`
- stick clicks: `ls`, `rs`
- system buttons: `start`, `back`, `guide`
- d-pad: `up`, `down`, `left`, `right`
- twin analog sticks: `left_x`, `left_y`, `right_x`, `right_y`
- analog triggers: `lt`, `rt`

Output capabilities:

- rumble (dual-rotor force feedback)
- ring lighting control
- player-indicator LEDs

Notes:

- no touchpad, no motion sensors, no PCM audio, no lightbar
- a Phase 2+ manual gate that lists DualSense-specific outputs against Xbox 360 indicates capability leakage

## Tier 3: `hardware-faithful`

### Goal

Behave like the real hardware on the transport, not just inside the OS input stack.

### Typical implementation

- emulate USB or Bluetooth enumeration
- provide transport-level descriptors and endpoints
- implement transport packet behavior
- support protocol-specific timing and state transitions
- (v2) route attached-function or accessory traffic where the real device exposes it; deferred from v1

### Best for

- true console or platform spoofing
- exact Xbox-style transport identity
- environments that expect real enumeration behavior

### What it does well

- closest possible identity match
- best chance of fooling software that inspects the wire protocol
- best room for future support of expansion ports, accessories, and profile-specific reverse channels

### What it costs

- highest engineering cost
- hardest validation
- most reverse-engineering effort

### Creation procedure

1. capture real enumeration and control flow
2. identify required descriptors, endpoints, and state transitions
3. implement a transport state machine
4. implement input and output packet handling
5. implement timing-sensitive or handshake-sensitive behavior
6. validate against real target software or hardware hosts

### Validation procedure

Validate in five layers:

- enumeration validation
- descriptor validation
- packet validation
- timing validation
- user-experience validation

Suggested checks:

- does the host enumerate the device as expected?
- are descriptors byte-accurate where required?
- do control requests or equivalent protocol requests succeed?
- do applications that rejected lower tiers now accept the device?
- do reverse-path features behave correctly?
- (v2 only) if the profile declares attached-function or accessory channels, do those commands reach the correct per-device channel? — not exercised in v1

### Reverse-engineering guidance

Recommended method:

1. capture real-device traffic during connect, idle, active play, and disconnect
2. isolate mandatory startup exchanges
3. isolate feature-specific exchanges
4. test replay against the target host
5. replace replay with generated protocol logic
6. compare traces until differences are understood

## Validation matrix by tier

| Validation area | Compatibility | Identity-aware | Hardware-faithful |
| --- | --- | --- | --- |
| Linux input capabilities | required | required | required |
| HID descriptor correctness | optional | required | required |
| Output report handling | optional | required | required |
| Feature report handling | optional | usually required | required |
| USB/Bluetooth enumeration parity | not required | not required | required |
| Packet/timing parity | not required | partial | required |

## Degradation policy

The library should never silently pretend a higher tier is complete when it is not.

Recommended behavior:

- if `hardware-faithful` is requested but only HID parity is implemented, report degradation
- if `hardware-faithful` is requested but only UHID is available, degrade to `identity-aware` or reject according to policy
- if `identity-aware` is requested but output or feature reports are stubbed, degrade or reject according to policy
- if `compatibility` is requested, it is acceptable to drop unsupported reverse features as long as this is explicit

### Canonical planner cases

These five cases are the charter that `gr-planner` tests and the Phase 5 manual gate are expected to back. Each maps a `(profile, goal, inventory)` triple to the expected planner outcome.

| Case | Profile | Goal | Inventory | Outcome |
|---|---|---|---|---|
| 1 | `dualsense` | `identity-aware` | [linux-uhid-only](../../../samples/inventories/linux-uhid-only.yaml) | `Ok(plan)` with `selected_backend_family: LinuxUhid`, `selected_level: Hid`, `degradation.degraded == false` |
| 2 | `dualsense` | `hardware-faithful` | [linux-uhid-only](../../../samples/inventories/linux-uhid-only.yaml) | `Ok(plan)` with `requested_fidelity_tier: IdentityAware`, `degradation.degraded == true`, `reasons == [TransportNotRealizable]` |
| 3 | `dualsense` | `hardware-faithful` | empty inventory | `Err(PlanRejection { reasons: [NoBackendSupportsProfile], .. })` |
| 4 | `xbox360` | `compatibility` | [linux-uinput-only](../../../samples/inventories/linux-uinput-only.yaml) | `Ok(plan)` with `selected_backend_family: LinuxUinput`, `selected_level: Evdev`, no degradation |
| 5 | `dualsense` | `identity-aware` | [linux-uinput-only](../../../samples/inventories/linux-uinput-only.yaml) | `Ok(plan)` with `requested_fidelity_tier: Compatibility`, `degradation.degraded == true`, `reasons == [ReversePathUnavailable]` (uinput cannot carry HID output reports) |

`DegradationReason` and `PlanRejectionReason` are typed enums in `gr-runtime-model`; the planner emits them directly so tests can match on `kind` rather than string contents.

Cases 1-4 back the [Phase 5 manual gate items 1-4](../implementation/RUST_IMPLEMENTATION_PLAN.md#phase-5-planner-gr-planner). Case 5 is the canonical tie-breaking example (`dualsense` is a HID-tier profile but the inventory only exposes the lower evdev tier).

## Open-source implementation guidance

Use open-source implementations when:

- the target behavior is already documented
- the target’s public behavior is sufficient for your use case
- the legal and maintenance burden is lower than reverse-engineering from scratch

Good uses:

- Linux input behavior replication
- Linux HID behavior replication
- public descriptors and public controller mappings

Be more cautious when:

- copying undocumented proprietary protocol behavior
- assuming open-source behavior is complete
- relying on a single third-party implementation without validation

## Final recommendation

Start every target at the lowest tier that satisfies the host requirement.

- choose `compatibility` for Linux gamepad usability
- choose `identity-aware` when software must recognize the controller family
- choose `hardware-faithful` only when the wire identity actually matters

That keeps scope under control and makes validation much more honest.
