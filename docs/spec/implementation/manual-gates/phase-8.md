# Phase 8 Manual Gate

This guide is the reviewer checklist for Phase 8
(`gr-provider-linux-uinput`). It covers the first real Linux provider,
host-visible evdev compatibility, and the EV_FF rumble reverse path.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 8
```

## Check 1: generic gamepad device visibility

Goal: confirm the generic-gamepad smoke path can create a Linux input
device and reports the expected capability surface.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- run-uinput-smoke generic-gamepad --interactive
```

2. Confirm:
   - the command exits 0
   - the report identifies a created device
   - the report includes the expected button and axis capability summary
   - the interactive banner tells you how to stop the session
   - the device stays alive long enough to attach host inspection tools

## Check 2: buttons and axes match expected events

Goal: confirm the created evdev device exposes the expected controls
and emitted presses land as matching host events.

### Steps

1. In another terminal, attach `evtest` or `jstest` to the created
   device node.
2. Run:

```bash
cargo run -p virtual_gamepad_demo -- run-uinput-smoke generic-gamepad --interactive --script exercise
```

3. Confirm:
   - the device reports the expected `generic-gamepad` buttons and axes
   - the scripted loop produces visible button, dpad, stick, and trigger
     activity
   - stopping the demo with Enter or Ctrl-C tears the session down cleanly

## Check 3: SDL recognizes the Xbox-style device

Goal: confirm the Xbox 360 compatibility path produces a controller SDL
recognizes as a gamepad.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- run-uinput-smoke xbox360 --interactive
```

2. Confirm:
   - the command exits 0
   - the report identifies a created device
   - the report declares `EV_FF` / `FF_RUMBLE`
   - SDL, `jstest-gtk`, or another host consumer recognizes the device
     while the session remains open

## Check 4: scripted inputs reach host software

Goal: confirm a real Linux host consumer receives the forward input
path end to end.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- run-uinput-smoke xbox360 --interactive --script exercise
```

2. Confirm:
   - host software receives the scripted inputs while the session stays
     open
   - the interactive status banner matches the observed profile and
     device node
   - the demo exits only when you press Enter or send Ctrl-C

## Check 5: EV_FF rumble surfaces as runtime output

Goal: confirm a host-triggered rumble request reaches the runtime as
`OutputCommand::Rumble`.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- run-uinput-smoke xbox360 --interactive
```

2. In another terminal, trigger rumble with `fftest` or compatible host
   software.
3. Confirm:
   - the profile declares `EV_FF` and `FF_RUMBLE`
   - the demo prints live rumble output lines with strong/weak values
   - the session remains healthy after the reverse-path event

## Check 6: teardown removes the device cleanly

Goal: confirm stopping the demo tears down the `uinput` device without
leaving zombie `event*` nodes behind.

### Steps

1. Start either interactive smoke command above.
2. Stop it with Enter or Ctrl-C.
3. Confirm:
   - the session prints the shutdown summary
   - the `uinput` device disappears after teardown
   - restarting the command creates a fresh device successfully

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 8 gate passed"
```
