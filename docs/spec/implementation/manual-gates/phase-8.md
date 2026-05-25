# Phase 8 Manual Gate

This guide is the reviewer checklist for Phase 8
(`gr-provider-linux-uinput`). It covers the first real Linux provider,
host-visible evdev compatibility, and the EV_FF rumble reverse path.

Current limitation: `run-uinput-smoke` is a one-shot probe command. It
creates the device, prints a report, and then exits immediately, which
tears the device down. That means the host-inspection portion of the
gate is not yet feasible with the current demo surface alone. Treat the
steps below as:

- feasible today for report-based preflight verification
- blocked for live `evtest` / `jstest` / SDL inspection until a
  persistent or interactive Phase 8 demo surface lands

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
cargo run -p virtual_gamepad_demo -- run-uinput-smoke generic-gamepad
```

2. Confirm:
   - the command exits 0
   - the report identifies a created device
   - the report includes the expected button and axis capability summary
   - note that `evtest` / `jstest` attachment is blocked by immediate
     teardown after the command exits

## Check 2: buttons and axes match expected events

Goal: confirm the created evdev device exposes the expected controls
and emitted presses land as matching host events.

### Steps

1. Compare the smoke report capability summary against the expected
   `generic-gamepad` controls.
2. Record that live host verification is blocked with the current
   one-shot command shape.
3. Do not mark this check complete until a persistent or interactive
   demo session exists.

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
   - the report identifies a created device
   - the report declares `EV_FF` / `FF_RUMBLE`
   - note that SDL / `jstest-gtk` verification is blocked by immediate
     teardown after the command exits

## Check 4: scripted inputs reach host software

Goal: confirm a real Linux host consumer receives the forward input
path end to end.

### Steps

1. Record that this check is blocked with the current one-shot demo
   surface.
2. Defer execution until a follow-up command can keep the device alive
   and inject scripted inputs while the reviewer observes host software.

## Check 5: EV_FF rumble surfaces as runtime output

Goal: confirm a host-triggered rumble request reaches the runtime as
`OutputCommand::Rumble`.

### Steps

1. Use the smoke report to confirm the Xbox-style profile declares
   `EV_FF` and `FF_RUMBLE`.
2. Record that live `fftest` / game-driven rumble verification is
   blocked until a persistent demo command exists.

## Check 6: teardown removes the device cleanly

Goal: confirm stopping the demo tears down the `uinput` device without
leaving zombie `event*` nodes behind.

### Steps

1. Observe that `run-uinput-smoke` exits immediately after reporting.
2. Confirm:
   - the command does not leave a long-lived device behind
   - a persistent session is still required for meaningful manual host
     inspection

## Sign-off

Do not sign off this gate as fully passed until the blocked live-host
checks above can actually be executed with a persistent or interactive
demo surface.

When that follow-up exists and all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 8 gate passed"
```
