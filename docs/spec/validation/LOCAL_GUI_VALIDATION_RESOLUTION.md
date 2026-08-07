# Local GUI live-provider validation resolution

## Purpose

The `vgpd-demo gui` command is a Linux-local development surface. Automated
tests validate its frame construction and session wiring, but a real controller
requires host-managed kernel permissions and, for transport fidelity, USB gadget
setup. This document resolves that boundary by defining the required manual
handoff rather than weakening the GUI or embedding external compliance tools.

For the durable scope decision behind this handoff, see
[Local provider scopes](LOCAL_PROVIDER_SCOPE_DECISION.md). This document is
only for the optional live-provider and lab paths, not a prerequisite for
general library use.

## Preconditions

- Run on Linux with a graphical desktop session.
- For evdev sessions, grant the tester access to `/dev/uinput`.
- For HID sessions, grant the tester access to `/dev/uhid`.
- For hardware-faithful DualSense USB transport, prepare a supported USB gadget
  controller, configfs, and an attached host that can enumerate the gadget.
- Do not run more than one hardware-faithful USB transport controller at once;
  the current provider does not guarantee concurrent gadget instances.

## Host setup resolution

### Standard development host: uinput and UHID

Use the repository-owned `udev` rules instead of running the GUI as root. On
the Linux development host, an administrator performs this one-time setup:

```bash
sudo ./samples/setup/install-linux-input-rules.sh
sudo usermod -aG input "$USER"
```

The user must then log out and back in (or start a new login session) before
running the GUI. Verify the resolved state before any live validation:

```bash
id -nG
ls -l /dev/uinput /dev/uhid
cargo run -p gr-cli -- run-uinput-smoke generic-gamepad
cargo run -p gr-cli -- run-uhid-smoke dualsense --bus usb
```

The acceptance condition is that the nodes exist, are group-owned by `input`,
have read/write group access, and the smoke commands report a created device.
If the nodes are missing, load the modules where the distribution supports it
(`sudo modprobe uinput` and `sudo modprobe uhid`) and rerun the setup script.
If permissions remain stale, follow the repository's retrigger instructions in
[`samples/setup/README.md`](../../../samples/setup/README.md); temporary
`chgrp`/`chmod` repairs are diagnostic only and must not be treated as the
permanent solution.

### Dedicated transport host: USB gadget

Hardware-faithful transport validation requires a distinct, prepared Linux
device with a peripheral-capable USB Device Controller. A normal desktop or
laptop USB host port is not sufficient. Before scheduling validation, the host
owner must confirm all of the following:

```bash
test -d /sys/kernel/config/usb_gadget
ls /sys/class/udc
test -e /dev/hidg0 || true
```

- `configfs` is mounted and at least one UDC is listed.
- The tester has delegated access to configure the gadget, or a designated
  operator runs only the transport session under the required privilege.
- A separate observing machine is connected to the peripheral-capable port and
  has `lsusb` available.
- The observer and gadget host have an agreed cleanup owner: the operator must
  stop the GUI, confirm the gadget directory is removed, and disconnect only
  after the virtual device is gone.

The complete transport checklist remains
[Phase 11's manual gate](../implementation/manual-gates/phase-11.md). If no
such host is available, record `pending-supported-host`; do not substitute a
host-only machine, run the GUI as root, or claim hardware-faithful validation.

### Ownership and retry policy

- The host owner supplies kernel modules, `udev` rules, group membership, and
  UDC/configfs access.
- The tester runs the smoke probes and GUI as their ordinary login user and
  records the selected plan, device identity, and command output.
- A missing node or permission failure returns the work to the host owner. A
  successful open with externally incorrect reports returns the work to the
  corresponding provider owner.

## Validation procedure

1. Start the GUI with `cargo run -p virtual_gamepad_demo -- gui`.
2. Create a compatibility-tier Generic Gamepad and exercise buttons, D-pad,
   sticks, and triggers. Confirm the GUI reports a running session and no send
   error.
3. Create an identity-aware DualSense session and exercise its buttons, touch
   contacts, sticks, and triggers. Confirm reverse commands, if emitted by the
   host, appear in the selected controller's log.
4. Create any additional concurrent evdev or UHID session and confirm their
   diagnostics and input state remain independent.
5. For a prepared USB gadget host, create exactly one hardware-faithful
   DualSense session and validate enumeration and emitted reports using the
   separately maintained external tools.
6. Record the host kernel version, provider selected by the session plan,
   requested/effective tier, device paths or gadget identity, and any failure
   message in the validation evidence for the corresponding provider.

## Acceptance and follow-up

- A provider open failure caused by permissions or unprepared gadget hardware is
  an environment prerequisite failure, not a GUI defect; retain the displayed
  error and remedy the host setup before retrying.
- A successful open followed by incorrect externally observed input or output
  is a provider/profile defect and must receive a focused provider issue and
  branch; keep the GUI branch unchanged unless its displayed state differs from
  the submitted frame or accepted session plan.
- Bluetooth transport remains out of scope until its provider offers live
  realization. The GUI must continue to show planner/open errors rather than
  presenting Bluetooth as a working local option.
