# Phase 9 Manual Gate

This guide is the reviewer checklist for Phase 9
(`gr-provider-linux-uhid`). It covers the first identity-aware Linux
provider — host software inspecting HID identity must recognize the
virtual device — and the output/feature report reverse path for one
identity-aware target (the recommended target is DualSense).

Host prerequisites:

- A Linux host with `/dev/uhid` available and the runner user in the
  `input` group (or equivalent udev rule granting `/dev/uhid` access).
- If the host is not already prepared, install the sample rule with
  `sudo ./samples/setup/install-linux-input-rules.sh` and verify
  `/dev/uhid` is group-owned by `input` before starting the gate.
- `hidraw` enumeration tools (`lsusb`, `bluetoothctl`, or `hid-tools`).
- SDL or `jstest-gtk` installed for gamepad-mapping recognition.
- A real DualSense controller for capture comparison, **or** the
  captured DualSense fixture set referenced from
  `DEVICE_SPEC_VALIDATION_PLAN.md`.
- Steam (optional, for Check 6 — Steam Input recognition).
- A public DualSense-aware reference title (Check 4 — trigger effects).

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 9
```

## Check 1: DualSense device identity is visible to `hidraw`

Goal: confirm the UHID device announces the expected DualSense vendor
and product identifiers and that `hidraw` enumerates it.

### Steps

1. Run the USB identity surface:

```bash
cargo run -p virtual_gamepad_demo -- run-uhid-smoke dualsense --interactive --bus usb
```

2. Confirm:
   - the command exits 0
   - the report identifies a created UHID device with a `/dev/hidraw*`
     node and the USB DualSense vendor/product id pairing (`0x054c`
     / `0x0ce6`)
   - `ls /dev/hidraw*` shows the new node while the session is open

3. Repeat for the Bluetooth identity surface:

```bash
cargo run -p virtual_gamepad_demo -- run-uhid-smoke dualsense --interactive --bus bluetooth
```

4. Confirm:
   - the command exits 0
   - the report identifies the Bluetooth DualSense identity surface
     (`0x054c` / `0x0df2`)

## Check 2: Linux HID and input identity surfaces match DualSense

Goal: confirm Linux reports the expected DualSense identity for the
virtual `UHID` device through `hidraw` and `input` enumeration. `UHID`
does not create a real USB or Bluetooth transport device, so
transport-layer tools such as `lsusb` and `bluetoothctl devices` are
not authoritative evidence here.

### Steps

1. With the USB smoke session from Check 1 still running, note the
   reported `hidraw` node and run:

```bash
udevadm info -q property -n /dev/hidrawN
for js in /sys/class/input/js*; do echo "$js: $(cat "$js/device/name")"; done
```

2. Repeat for the Bluetooth smoke session.

3. Confirm:
   - `udevadm` reports a `DEVPATH` under
     `/devices/virtual/misc/uhid/...`
   - the `hidraw` path encodes the expected DualSense vendor/product
     pair for the chosen bus surface
   - Linux exposes an input node named
     `Sony Interactive Entertainment DualSense Wireless Controller`
   - Bluetooth mode may still appear under the same Linux `UHID`
     subsystem path; the evidence is the product id and controller name,
     not a real `bluetoothctl` transport entry

## Check 3: SDL identifies the device as DualSense

Goal: confirm SDL-based host software auto-binds a DualSense gamepad
mapping rather than treating the device as a generic gamepad.

### Steps

1. With either smoke session from Check 1 still running, use the
   reported `js_nodes` list or run:

```bash
for js in /sys/class/input/js*; do echo "$js: $(cat "$js/device/name")"; done
```

2. Identify the node named exactly:

```text
Sony Interactive Entertainment DualSense Wireless Controller
```

Ignore sibling nodes such as
`Sony Interactive Entertainment DualSense Wireless Controller Motion Sensors`.

3. Launch SDL's `controllermap`, `jstest-gtk`, or `jstest` against that
   joystick node.
2. Confirm:
   - the host software identifies the device as DualSense
   - the canonical DualSense control layout (sticks, dpad, triggers,
     face buttons, touchpad button) is picked up automatically

## Check 4: trigger effects round-trip as `OutputCommand::TriggerEffect`

Goal: confirm DualSense-specific trigger-effect commands sent by a
real game reach the runtime as normalized `OutputCommand::TriggerEffect`
values.

### Steps

1. Launch a public DualSense-aware reference title (any title that
   exercises adaptive triggers).
2. Trigger an in-game scenario known to fire an adaptive-trigger effect.
3. Confirm:
   - the demo prints live `OutputCommand::TriggerEffect` lines with the
     expected effect kind and per-trigger parameters
   - the reverse-event sequence numbers advance monotonically

## Check 5: rumble round-trips as `OutputCommand::Rumble`

Goal: confirm host rumble requests reach the runtime as normalized
`OutputCommand::Rumble`.

### Steps

1. With either smoke session from Check 1 still running, trigger rumble
   from the host (an in-game rumble scene, `fftest`, or equivalent
   tooling that targets the DualSense HID surface).
2. Confirm:
   - the demo prints live rumble output lines with strong/weak values
   - the session remains healthy after the reverse-path event

## Check 6: Steam Input recognizes the controller

Goal: confirm Steam Input picks up the virtual device as a DualSense
controller (skip if Steam is not installed).

### Steps

1. Launch Steam with one of the interactive smoke sessions still running.
2. Open Steam → Settings → Controller.
3. Confirm:
   - Steam Input lists the virtual device as a DualSense controller
   - the controller settings page shows the canonical DualSense control
     layout

## Check 7: a Steam Input mode-change scenario round-trips

Goal: confirm the reverse translator handles a real Steam Input mode
change end to end.

### Steps

1. Replay the built-in session-scenario fixture:

```bash
cargo run -p gr-cli -- run-scenario samples/scenarios/dualsense-steam-input-mode.yaml
```

2. Confirm:
   - the reverse translator emits the expected normalized outputs
   - the scenario exits 0

## Check 8: support-report evidence is complete for DualSense

Goal: confirm per-profile evidence covers descriptor, input, output,
feature, and target-software recognition.

### Steps

1. Run:

```bash
cargo run -p gr-cli -- support-report --profile dualsense
```

2. Confirm the report shows ticks for:
   - descriptor evidence ✓
   - input report evidence ✓
   - output report evidence ✓
   - feature report evidence ✓
   - target software recognition ✓

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 9 gate passed"
```
