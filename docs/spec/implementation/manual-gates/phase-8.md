# Phase 8 Manual Gate

This guide is the reviewer checklist for Phase 8
(`gr-provider-linux-uinput`). It covers the first real Linux provider,
host-visible evdev compatibility, and the EV_FF rumble reverse path.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 8
```

## Check 1: generic gamepad device visibility

Goal: confirm the generic-gamepad smoke path creates a host-visible
Linux input device.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- run-uinput-smoke generic-gamepad
```

2. Confirm:
   - the command exits 0
   - the report identifies a created device
   - `evtest` or `jstest` finds the device under `/dev/input/`

## Check 2: buttons and axes match expected events

Goal: confirm the created evdev device exposes the expected controls
and emitted presses land as matching host events.

### Steps

1. Run `evtest` against the created device.
2. Trigger representative inputs through the smoke flow.
3. Confirm:
   - expected buttons are present
   - expected axes are present
   - observed press and axis events match what the smoke flow emitted

## Check 3: SDL recognizes the Xbox-style device

Goal: confirm the Xbox 360 compatibility path produces a controller SDL
recognizes as a gamepad.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- run-uinput-smoke xbox360
```

2. Confirm:
   - the command exits 0
   - the created device is recognized by SDL via `sdl2-test` or
     `jstest-gtk`

## Check 4: scripted inputs reach host software

Goal: confirm a real Linux host consumer receives the forward input
path end to end.

### Steps

1. Launch a native Linux SDL game or `jstest-gtk`.
2. Use the scripted input flow exposed by the smoke/demo surface.
3. Confirm representative stick, trigger, and button inputs land in the
   host software.

## Check 5: EV_FF rumble surfaces as runtime output

Goal: confirm a host-triggered rumble request reaches the runtime as
`OutputCommand::Rumble`.

### Steps

1. Trigger rumble with `fftest` or a game against the created device.
2. Confirm the demo's verbose output shows the reverse path surfacing as
   `OutputCommand::Rumble`.

## Check 6: teardown removes the device cleanly

Goal: confirm stopping the demo tears down the `uinput` device without
leaving zombie `event*` nodes behind.

### Steps

1. Kill or stop the demo after the smoke run.
2. Confirm:
   - the device disappears from `/dev/input/`
   - no zombie `event*` entry remains on subsequent runs

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 8 gate passed"
```
