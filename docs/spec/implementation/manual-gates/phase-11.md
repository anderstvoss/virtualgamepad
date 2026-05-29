# Phase 11 Manual Gate

This guide is the reviewer checklist for Phase 11
(`gr-provider-linux-transport`, first hardware-faithful target). Phase 11
turns the Phase 10 transport foundation into a real, live Linux USB
gadget: real enumeration, real packet handling, and reverse-path features
observed by a host that did **not** accept the lower (UHID-identity) tier.
The target profile is the same one chosen in Phase 9 — DualSense USB — for
maximum reuse of evidence and fixtures.

Unlike Phase 10 (state-machine and planner contracts only, no `/dev/*`
prerequisites), Phase 11 realizes a real transport device and therefore
carries genuine hardware prerequisites. The live device path is implemented
end-to-end — `configfs` gadget enumeration **and** `/dev/hidgN` report I/O
(input write + non-blocking reverse read) — so the checks below are
*validation*, not implementation gates: they confirm the implemented path
against a real host. They cannot be manually verified without a supported
host and a real DualSense, so any that can't be exercised here remain in a
`pending-supported-host` deferred-validation queue — the same vocabulary
Phase 9 uses for its deferred Tier D claims. `pending-supported-host` is
**not** `prerequisite-pending`: the latter is a temporary marker that the
implementation PR clears, whereas `pending-supported-host` is a real Tier D
deferral that closes only on a supported validation system.

Host prerequisites:

- A Linux host with USB **device/peripheral mode** available — a working
  USB Device Controller (UDC) exposed through `configfs`
  (`/sys/kernel/config/usb_gadget`). Most desktop/laptop x86 hosts have
  host-only USB controllers and cannot satisfy this; a board with a
  dual-role or peripheral-capable port (or a Bluetooth path) is required.
- The runner user must have permission to configure the gadget (typically
  `root` or an equivalent udev/configfs grant).
- `lsusb` (with `-v`), and ideally a second machine to act as the
  observing host for Checks 1–2.
- A real DualSense controller and its captured reference traces
  (connect / idle / active input / reverse command / disconnect) per
  [DEVICE_SPEC_VALIDATION_PLAN.md step 6](../validation/DEVICE_SPEC_VALIDATION_PLAN.md).
- A target host or game known to reject the UHID-tier emulation (for
  Check 2), and DualSense-aware software exercising rumble / lighting /
  trigger effects (for Check 3).

Phase 11 target support (single source of truth: `SUPPORTED_PROFILE_BUS_PAIRS`
in `crates/gr-provider-linux-transport/src/lib.rs`):

- DualSense on USB (the Phase 11 realization target)

DualSense-on-Bluetooth and Xbox 360-on-USB remain plannable transport
targets but are not part of the Phase 11 realization scope; reviewers
should not treat their deferral as a Phase 11 bug.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 11
```

This runs the automated portion (workspace tests + snapshot check +
fixture-backed transport comparison). The live `lsusb` and target-host
checks below remain manual and gated on the hardware prerequisites
above.

## Check 1: virtual device enumerates identically to a real DualSense

Goal: confirm the transport-tier device brings up under a real host and
enumerates with the same descriptors as a genuine DualSense, modulo
allowed differences (e.g. serial number).

### Steps

1. Bring up the virtual device against the target's expected transport:

```bash
cargo run -p virtual_gamepad_demo -- run-transport-smoke dualsense
```

2. On the observing host, capture and diff the verbose descriptor dump:

```bash
lsusb -v -d 054c:0ce6
```

3. Confirm:
   - the command exits 0 and the gadget enumerates
   - the `lsusb -v` diff against a real DualSense shows only allowed
     differences (such as serial number)
   - if no peripheral-mode host / real controller is available, mark this
     check `pending-supported-host` and record it in the deferred queue

## Check 2: a UHID-rejecting host accepts the transport-tier device

Goal: demonstrate the value of the hardware-faithful tier — a host or
game that refused the Phase 9 UHID-identity device now accepts the
transport-tier device.

### Steps

1. With the device from Check 1 up, exercise the target host/game that
   previously rejected the UHID-tier emulation.

2. Confirm:
   - the target now recognizes and accepts the device
   - record which host/game and which rejection it previously exhibited
   - mark `pending-supported-host` if the rejecting target is unavailable
     on this machine

## Check 3: reverse-path features behave correctly under the real host

Goal: confirm rumble, lighting, and trigger effects driven by real host
software behave correctly through the reverse packet path.

### Steps

1. With the device up, drive reverse-path features from DualSense-aware
   host software (rumble, lightbar, adaptive-trigger effects).

2. Confirm:
   - each reverse-path feature produces the expected device behavior
   - reverse packets are handled per the declared profile capability
   - mark `pending-supported-host` for any feature not exercisable here

## Check 4: support-report shows hardware-faithful evidence

Goal: confirm the per-profile support report distinguishes the
hardware-faithful tier and surfaces each evidence axis.

### Steps

1. Render the hardware-faithful support report:

```bash
cargo run -p gr-cli -- support-report --profile dualsense --tier hardware-faithful
```

2. Confirm the report shows:
   - transport enumeration ✓
   - control flow ✓
   - packet handling ✓
   - reverse packets ✓
   - real-host recognition ✓
   - any axis not validated on this host is clearly marked
     `pending-supported-host`, not silently claimed

## Check 5: real-vs-virtual trace differences are documented and signed off

Goal: make every divergence between the real-device capture and the
virtual device explicit and reviewed, rather than assumed safe.

### Steps

1. Produce a side-by-side comparison of the real and virtual transport
   traces:

```bash
cargo run -p gr-cli -- compare-real-device --profile dualsense --layer transport
```

2. Confirm:
   - the comparison passes within the documented tolerance
   - each difference is recorded as a `notes:` entry in the comparison
     report
   - the reviewer signs off that each documented difference is safe
   - if real captures are unavailable, mark `pending-supported-host`

## Check 6: disconnect / reconnect cycle is clean

Goal: confirm tearing down and re-bringing-up the device leaves no orphan
kernel resources.

### Steps

1. Bring the device up, then tear it down, then bring it up again:

```bash
cargo run -p virtual_gamepad_demo -- run-transport-smoke dualsense
# stop the session, then re-run
cargo run -p virtual_gamepad_demo -- run-transport-smoke dualsense
```

2. Confirm:
   - the gadget tears down cleanly (no leftover `configfs` gadget dirs,
     no bound-UDC leak)
   - the second bring-up succeeds without manual cleanup
   - mark `pending-supported-host` if no peripheral-mode host is available

## Sign-off

When all automated checks and all *verifiable* manual checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 11 gate passed"
```

If the hardware-dependent checks (1, 2, 3, 5, 6) cannot be exercised on
the available host, they remain in a `pending-supported-host`
deferred-validation queue and Phase 11 closes as a provider-complete
deferral rather than a full hardware-faithful validation:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 11 provider-complete closure recorded"
```

For each deferred check, record: required environment (peripheral-mode
host + real DualSense), target profile (DualSense USB), and the expected
evidence artifact (`lsusb -v` diff, host-acceptance note, reverse-feature
observation, or `compare-real-device` report). Provider-complete closure
does not authorize a "DualSense hardware-faithful validation complete"
claim until the deferred queue clears on a supported system.
