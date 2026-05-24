# Phase 6 Manual Gate

This guide is the step-by-step reviewer checklist for Phase 6
(`gr-translators`). It covers translator behavior, descriptor-backed HID
report shaping, reverse-event decoding, the reviewer-facing replay
surface, and translator-coverage gap detection in
`gr-cli capability-coverage`.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 6
```

## Check 1: DualSense buttons round-trip

Goal: confirm the DualSense forward translator encodes profile input
into a HID input report whose decoded summary matches the input
frame.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- replay-trace crates/gr-translators/fixtures/dualsense-buttons-roundtrip.yaml
```

2. Confirm the replay output shows:
   - the raw HID input report bytes
   - a decoded DualSense summary
   - dpad / face button state matching the input frame

### What to record

- Any decoded field that doesn't match the input frame
- Any byte position in the report that looks unexpected

## Check 2: DualSense rumble decode from host

Goal: confirm the DualSense reverse translator decodes a host rumble
request into the expected `OutputCommand::Rumble` plus any companion
lighting / player-indicator / trigger-effect commands.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- replay-trace crates/gr-translators/fixtures/dualsense-rumble-from-host.yaml
```

2. Confirm the replay output decodes the inbound host report into:
   - `Rumble(RumblePayload { ... })` with non-zero strong/weak values
   - any populated lighting / player-indicator / trigger-effect /
     audio commands that the test fixture exercises

### What to record

- Any command the decode emits that the fixture doesn't expect
- Any expected command the decode silently dropped

## Check 3: Xbox 360 evdev round-trip

Goal: confirm the Xbox-style evdev forward translator emits the
correct evdev key/abs events for the canonical xbox360 input frame.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- replay-trace crates/gr-translators/fixtures/xbox360-evdev-roundtrip.yaml
```

2. Confirm the replay output shows the expected evdev event summary —
   button presses, dpad axes, and analog stick values that match the
   input frame.

### What to record

- Any evdev code or value that diverges from the input frame
- Any missing event you'd expect for the input state

## Check 4: Steam Controller lighting decode

Goal: confirm the Steam Controller reverse translator decodes a host
lighting command into `OutputCommand::Lighting` with sensible RGB
payload, alongside the forward HID summary.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- replay-trace crates/gr-translators/fixtures/steam-controller-lighting.yaml
```

2. Confirm the replay output shows:
   - a decoded Steam Controller HID input summary on the outbound step
   - a decoded `Lighting(LightingPayload { ... })` on the inbound step
     with non-zero RGB values

### What to record

- Any decoded RGB triplet that doesn't match the fixture
- Any output command the decode emits that wasn't in the fixture

## Check 5: capability-coverage translator-gap detection

Goal: confirm `gr-cli capability-coverage` reports zero gaps once every
profile family has its expected forward + reverse translator wired and
the descriptor templates are real.

### Steps

1. Run:

```bash
cargo run -p gr-cli -- capability-coverage
```

2. Confirm:
   - exit code is 0
   - output contains `gaps: []`

### What to record

- Any gap entry that prints (none expected for the built-in profile set)
- Any cross-family translator mismatch surfaced by the coverage check

## Check 6: snapshot readability

Goal: confirm stored translator-adjacent snapshots read like
intentional user-facing output rather than debug dumps.

### Steps

1. Review `crates/gr-cli/src/snapshots/` (replay-trace snapshots) and
   `crates/gr-profiles/src/snapshots/` (descriptor + capability
   snapshots).
2. Confirm:
   - replay-trace outputs render decoded commands in a human-readable
     form (semantic function names, typed payload fields)
   - descriptor bytes look plausible for the device family (correct
     report IDs, sensible report sizes)

### What to record

- Any snapshot whose format would confuse a reviewer reading it cold

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 6 gate passed"
```
