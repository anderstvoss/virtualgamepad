# Phase 2 Manual Gate

This guide expands the manual portion of Phase 2 into concrete,
repeatable reviewer steps.

Use it only after the automated Phase 2 checks are green.

Related docs:

- [Rust Implementation Plan](../RUST_IMPLEMENTATION_PLAN.md)
- [Fidelity Guide](../../specs/FIDELITY_GUIDE.md)

## Before you start

1. Make sure you are on the Phase 2 work branch.
2. Run the automated gate first:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 2
```

3. If the automated checks fail, stop here and fix them before
   continuing.

## Check 1: built-in profile list

Goal: confirm the shipped profile set and public-facing names are stable.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- list-profiles
```

2. Confirm the output includes exactly these profile ids:
   - `generic-gamepad`
   - `xbox360`
   - `dualsense`
   - `steam-controller`
3. Confirm the display names are:
   - `Generic gamepad`
   - `Xbox 360`
   - `DualSense`
   - `Steam Controller`
4. Confirm the ordering is stable across repeated runs.

### What to record

- Any spelling drift between ids, display names, and docs
- Any ordering instability

## Check 2: DualSense capability surface

Goal: confirm DualSense exposes the expected Phase 2 capability claims.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- show-capabilities dualsense
```

2. Confirm the output lists these input capabilities:
   - touch surface
   - accelerometer
   - gyroscope
3. In the stick capability entries, confirm the YAML makes the shared
   axis assumption explicit rather than implicit:
   - `left-stick` names both `sticks.left_x` and `sticks.left_y`
   - `right-stick` names both `sticks.right_x` and `sticks.right_y`
   - the shown stick range clearly states it applies to both axes in
     each stick pair
4. Confirm the output lists these output capabilities:
   - rumble
   - haptics
   - lighting
   - player-indicators
   - trigger-effect
   - audio
5. Cross-check the output against the DualSense reviewer table in
   [`FIDELITY_GUIDE.md`](../../specs/FIDELITY_GUIDE.md#dualsense-profile_id-dualsense).

### What to record

- Any missing DualSense-specific capability
- Any capability that looks mislabeled or too implementation-specific

## Check 3: Xbox 360 isolation

Goal: confirm Xbox 360 exposes only Xbox 360-appropriate capabilities.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- show-capabilities xbox360
```

2. Confirm the input side includes sticks, triggers, d-pad, face
   buttons, shoulders, stick clicks, and system buttons.
3. Confirm the output side includes:
   - `rumble`
   - `lighting`
   - `player-indicators`
4. Confirm the output does **not** include:
   - `trigger-effect`
   - `audio`
   - `haptics`

### What to record

- Any DualSense-specific capability leaking into Xbox 360
- Any expected Xbox 360 capability that is missing

## Check 4: ad-hoc profile rejection

Goal: confirm the registry rejects incomplete profiles with field-specific errors.

### Steps

1. Run the targeted test:

```bash
cargo test -p gr-profiles invalid_profiles_fail_with_field_specific_errors
```

2. Confirm the command reports **6 passed** (one case per required
   field covered by the parametrized `rstest`) and the remaining
   `gr-profiles` tests as **filtered out**. This is expected because the
   command intentionally runs one targeted, table-driven test rather
   than the full crate suite.
3. Confirm each passing case asserts that the failure is tied to one
   concrete missing field (`display_name`, `supported_fidelity`,
   `input_contract.required_fields`, `capabilities.input`,
   `identity.vendor_id`, `identity.product_id`), not a generic "invalid
   profile" message.

### What to record

- Whether the rejection message points to a specific missing field

## Check 5: snapshot readability

Goal: confirm the capability snapshots are useful review artifacts.

### Steps

1. Run:

```bash
cargo insta test --check
```

2. Open `crates/gr-profiles/src/snapshots/`.
3. Confirm the snapshots are easy to read by eye and that each built-in
   profile has a capability dump snapshot.
4. If a snapshot feels too dense to review comfortably, record that as a
   follow-up readability issue; YAML remains the gate artifact for now,
   but future reviewer tooling could include a collapsible JSON-oriented
   viewer without changing the default format.

### What to record

- Any snapshot that is noisy, confusing, or missing expected capability information
