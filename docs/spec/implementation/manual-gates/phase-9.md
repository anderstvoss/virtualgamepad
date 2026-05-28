# Phase 9 Manual Gate

This guide is the reviewer checklist for Phase 9
(`gr-provider-linux-uhid`). It covers the first identity-aware Linux
provider — host software inspecting HID identity must recognize the
virtual device — and the output/feature report reverse path for one
identity-aware target (the recommended target is DualSense). This gate
closes Phase 9 at the provider/runtime level. Profile-specific
host-software claims such as Steam Input recognition may remain pending
for a later supported validation system.

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
- Steam (optional, for the deferred validation queue).
- A public DualSense-aware reference title (optional, for the deferred
  validation queue).

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

## Check 4: Steam-shaped scenario evidence substitutes for unavailable host validation

Goal: confirm the reverse translator handles a representative
Steam-shaped mode-change/output report end to end without claiming that
the current machine has manually verified Steam Input behavior.

### Steps

1. Replay the built-in session-scenario fixture:

```bash
cargo run -p gr-cli -- run-scenario samples/scenarios/dualsense-steam-input-mode.yaml
```

2. Confirm:
   - the reverse translator emits the expected normalized outputs
   - the scenario exits 0
   - mark this as substitute evidence only; it is not a manual
     verification of Steam Input on the current host

## Check 5: support-report separates provider closure from deferred host claims

Goal: confirm per-profile evidence covers descriptor, input, output,
feature, Linux-host recognition, and explicit deferred host-software
validation status.

### Steps

1. Replay the built-in session-scenario fixture:

```bash
cargo run -p gr-cli -- support-report --profile dualsense
```

2. Confirm:
   - descriptor evidence ✓
   - input report evidence ✓
   - output report evidence ✓
   - feature report evidence ✓
   - linux-host recognition ✓
   - steam-input recognition is clearly marked pending/deferred when not
     validated on this machine
   - reference-title validation is clearly marked pending/deferred when
     not validated on this machine
   - this host is not treated as a manual validation source for those
     deferred Tier D claims

## Check 6: reference-title trigger effects are explicitly deferred

Goal: make it explicit that reference-title trigger-effect validation is
not manually verifiable on this host and must remain queued for a
supported system.

### Steps

1. Confirm the Phase 9 gate/docs do not require a public reference
   title to be available on this machine.
2. Confirm the deferred validation queue includes:
   - reference-title trigger effects
   - required environment
   - target profile
   - expected evidence artifact
3. Confirm:
   - provider-complete closure does not claim this check is complete
   - later support claims remain blocked on a supported validation host

## Check 7: Steam Input and host-software rumble remain explicitly deferred

Goal: make the missing Tier D checks explicit so later support claims
are blocked by a tracked queue rather than by forgotten assumptions.

### Steps

1. Record the following deferred checks for a supported validation
   system:
   - Steam Input recognition
   - Steam controller-layout mapping
   - host-originated rumble from target software
   - any Steam-specific mode or feature behavior found later
2. For each item, record:
   - required environment
   - target profile
   - expected evidence artifact
3. Confirm:
   - the queue distinguishes provider-complete from profile-claim-pending
   - no Phase 9 sign-off text claims full DualSense identity-aware
     validation until the deferred queue is cleared

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 9 provider-complete closure recorded"
```
