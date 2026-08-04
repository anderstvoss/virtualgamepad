# Local GUI live-provider validation resolution

## Purpose

The `vgpd-demo gui` command is a Linux-local development surface. Automated
tests validate its frame construction and session wiring, but a real controller
requires host-managed kernel permissions and, for transport fidelity, USB gadget
setup. This document resolves that boundary by defining the required manual
handoff rather than weakening the GUI or embedding external compliance tools.

## Preconditions

- Run on Linux with a graphical desktop session.
- For evdev sessions, grant the tester access to `/dev/uinput`.
- For HID sessions, grant the tester access to `/dev/uhid`.
- For hardware-faithful DualSense USB transport, prepare a supported USB gadget
  controller, configfs, and an attached host that can enumerate the gadget.
- Do not run more than one hardware-faithful USB transport controller at once;
  the current provider does not guarantee concurrent gadget instances.

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
