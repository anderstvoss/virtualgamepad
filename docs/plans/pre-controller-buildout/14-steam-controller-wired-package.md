# Brief 14: Steam Controller 2026, wired (#159), checkpoint C6

Read `ground-rules.md` first, then brief 03 and #159's consumer findings (`hid-steam`, SDL, Steam client).

**Lane:** Controller. **Checkpoint:** C6, branch `buildout/c6-steam-wired`.
**Blocked by:** checkpoint C1 merged; **step 16's result** saying `linux.uhid.usb` is accepted by `hid-steam` or SDL; ledger 11c merged.
- If step 16 says USB/IP is needed instead, stop this brief. Post an "Agent question (step 19)" on #176 asking for a USB/IP plan (#168), and move on.
- Capacitive and pressure inputs need the typed kinds from 10c (D2). Without D2, ship the rest and leave those cells "unavailable in this realization", listed under Judgement calls.

## Goal

Playable wired Steam Controller 2026 (`28de:1302`, one unified HID interface) for the consumers #159 names:

- `linux.uhid.usb` (playable, full) for `hid-steam`, SDL HIDAPI and the Steam client;
- `linux.uinput` (playable, reduced), shaped like what `hid-steam` creates.

Not in scope: the Puck (`28de:1304`), the Steam Machine receiver (`1305`), Bluetooth (`1303`), and lizard-mode emulation beyond what the consumers require.
- **Why the Puck is out:** it needs receivers (C4), possibly USB/IP (#168), and a resolution of the interface-count conflict (OpenPuck says 4, the kernel says 7).
- **Instead:** write a one-page Puck feasibility note in the C6 PR stating which of those three items is still open.

## Evidence rule for autonomous work

There is no physical capture yet. Everything comes from source:
- `hid-steam` at the first release with the IBEX ID (v7.3-rc1 or later, pinned in step 3);
- the SDL Triton driver at the pinned commit (`6defbc0…` in step 3);
- OpenPuck where it applies to the controller rather than the Puck.

This is consumer evidence: implement what these consumers send and expect, and cite the record next to each constant.
- If the consumers disagree with each other: implement what both accept, or, failing that, what `hid-steam` expects. Record the disagreement as a research-repo claim and list it under Judgement calls.
- If no consumer source covers a behaviour: leave it unimplemented and mark the cell "No evidence".

## Sub-steps (one PR each into `buildout/c6-steam-wired`)

**14a. Family, state and descriptor.** Register `steam-controller-2026` with both realizations. Add typed state for buttons, sticks, trackpads (position, touch and click), triggers, IMU and grip buttons. Use the report descriptor the ledger records.
Objective: registry tests pass and no other family's test changed. A descriptor test checks the bytes against the research fixture's hash.

**14b. Reports and feature exchange.** The input report the consumers decode, plus the feature or output reports `hid-steam` and SDL send on open: settings, lizard mode off, and haptics. Feature reads return the documented replies.
Objective: table tests, one per consumer-sent request, with the expected reply from the fixtures. A test shows every report the consumers send on open is handled, with no unhandled-request fallthrough.

**14c. Realizations.**
- `linux.uhid.usb` with the `1302` identity.
- uinput shaped like `hid-steam`'s evdev devices.
- Objective, deterministic (CI): a golden-shape test built from the `hid-steam` consumer records.
- Objective, live (host on kernel v7.3-rc1 or later, D9): the acceptance runner shows `hid-steam` bound (sysfs `driver`), evdev devices matching the golden shape, and the SDL3 gamepad probe (`scripts/run-sdl3-gamepad-probe.sh` with HIDAPI hints) opening it as a Steam controller and receiving one input change per control.

**14d. Acceptance and timing.** `acceptance/steam-controller-2026.toml` plus #166 timing for 1 and N controllers.
The Steam client row stays **manual**: list the exact steps for Anders in the C6 PR and leave the cell untested until he runs them.

## Checkpoint C6 PR

It follows IMPLEMENTATION_PLAN.md 1.6. **Needs your attention** lists:
- every place the two consumers disagreed;
- every "No evidence" cell;
- the manual Steam client check;
- the Puck feasibility note.

Post one line on #159 linking the C6 PR.
