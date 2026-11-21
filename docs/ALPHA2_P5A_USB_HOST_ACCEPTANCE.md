# Alpha 2 P5a USB host acceptance

Run on 2026-11-21. Tested revision `ed8065b4be2c266dd64e9cedba16e87889392a14` in a fresh detached worktree. Research pin `a67781e2412eceea9440d20dbb350ad5d3352c25`. Linux 7.3.0-1-mainline on x86-64, SDL 3.4.20. The run used synthetic controller defaults.

## Stock consumer matrix

Three fresh holders per realization. Node basenames are transient and grouped by session. The P4 harness only classifies the backend for SDL 3.2.0. The separate input sweep used an exact owned SDL device path to distinguish the Linux evdev and HIDAPI paths below.

| Family | Realization | Bound driver | P4 nodes by session | P4 diagnostic passed H/E |
| --- | --- | --- | --- | --- |
| DualSense | `linux.uinput` | — | event256, event31 / event256, event31 / event261, event262 | 1/1 of 3 |
| DualSense | `linux.uhid.usb` | playstation | hidraw13 / hidraw13 / hidraw13 | 0/0 of 3 |
| DualShock 4 | `linux.uinput` | — | event256, event31 / event256, event31 / event260, event261 | 1/1 of 3 |
| DualShock 4 | `linux.uhid.usb` | playstation | hidraw13 / hidraw13 / event260, event261, event264, hidraw14 | 0/0 of 3 |
| Switch Pro | `linux.uinput` | — | event31 / event31 / event260 | 3/3 of 3 |
| Switch Pro | `linux.uhid.usb` | nintendo | hidraw13 / event256, event31, hidraw13 / hidraw13 | 0/1 of 3 |
| Xbox 360 | `linux.uinput` | — | event31 / event31 / event29 | 3/3 of 3 |
| Xbox 360 | `linux.uhid.usb` | hid-generic | event31 / event261, hidraw14 / event29, hidraw13 | 1/3 of 3 |

The P4 diagnostic can fail its sensor check after SDL has selected a device. It also looks up the first associated node, which can appear before the gamepad event node or be a touch companion. The follow-up sweep waited for udev to mark a uniquely associated joystick event node. Each successful sweep delivered all four face-button press and release transitions and both endpoints on all four stick axes. The sweep sent no touch input.

| Family | Realization | HIDAPI enabled request, observed path and GUID | HIDAPI disabled request, observed path and GUID |
| --- | --- | --- | --- |
| DualSense | `linux.uinput` | evdev `030057564c050000e60c000011810000` (sweep passed) | evdev `030057564c050000e60c000011810000` (sweep passed) |
| DualSense | `linux.uhid.usb` | HIDAPI `030057564c050000e60c000000006800` (sweep passed) | No SDL gamepad open on selected node |
| DualShock 4 | `linux.uinput` | evdev `03009b514c050000c405000011810000` (sweep passed) | evdev `03009b514c050000c405000011810000` (sweep passed) |
| DualShock 4 | `linux.uhid.usb` | HIDAPI `03008fe54c050000c405000000006800` (sweep passed) | No SDL gamepad open on selected node |
| Switch Pro | `linux.uinput` | No SDL gamepad open on selected node | evdev `0300d71f7e0500000920000001000000` (sweep passed) |
| Switch Pro | `linux.uhid.usb` | HIDAPI `0300d71f7e0500000920000000006803` (sweep passed) | evdev `0300d71f7e0500000920000020820000` (sweep passed) |
| Xbox 360 | `linux.uinput` | evdev `03003aca5e0400008e02000001000000` (sweep passed) | evdev `03003aca5e0400008e02000001000000` (sweep passed) |
| Xbox 360 | `linux.uhid.usb` | No SDL gamepad open on selected node | evdev `030058765669727475616c2058626f00` (sweep passed) |

The operator ran the privileged P4 comparison from a normal terminal. The harness stopped on the first DualSense rebind attempt with a per-device sysfs write error, and the virtual holder closed. The alternate bindings and the Switch Pro neutral comparison under an alternate binding remain unmeasured. Xbox 360 USB UHID was already on `hid-generic`. The uinput realizations have no HID driver.

## Switch Pro request order

Each entry is `output report/command` in hexadecimal. Each `01/10` is one SPI-read request. The callback does not receive the device-to-host `0x21` acknowledgement or retain SPI contents.

- Session 1, 10 SPI reads. `80/02 80/03 80/02 80/04 01/02 01/10 01/10 01/10 01/10 01/10 01/10 01/40 01/03 01/48 01/30 01/38 80/01 80/01 80/01 80/02 80/03 80/02 80/04 01/10 01/10 01/10 01/10 01/48 80/04 01/30 01/40 01/38`
- Session 2, 14 SPI reads. `80/02 80/03 80/02 80/04 01/02 01/10 01/10 01/10 01/10 01/10 01/10 01/40 01/03 01/48 01/30 01/38 80/01 80/01 80/01 80/02 80/03 80/02 80/04 01/10 01/10 01/10 01/10 01/48 80/04 01/30 01/40 01/38 80/01 80/01 80/01 80/02 80/03 80/02 80/04 01/10 01/10 01/10 01/10 01/48 80/04 01/30`
- Session 3, 14 SPI reads. `80/02 80/03 80/02 80/04 01/02 01/10 01/10 01/10 01/10 01/10 01/10 01/40 01/03 01/48 01/30 01/38 80/01 80/01 80/01 80/02 80/03 80/02 80/04 01/10 01/10 01/10 01/10 01/48 80/04 01/30 80/01 80/01 80/01 80/02 80/03 80/02 80/04 01/10 01/10 01/10 01/10 01/48 80/04 01/30 01/40 01/38 80/01 80/01 80/01`

## Steam and Xbox Series

Steam client build 1794536281. The isolated automation created one owned pad at a time, used sequence-tagged commit and neutral commands, and verified removal and an empty USB/IP attach list. Face-button taps, each stick, and separate full-range trigger stimuli were captured. The Switch Pro exposes digital trigger buttons. Where Ping was available, one click was compared with typed retained outputs over 750 ms. Startup changes were kept separate.


| Family | Realization | Steam listing and test screen |
| --- | --- | --- |
| DualSense | `linux.uinput` | Listed. Four face buttons, both sticks and both individual triggers verified. Ping retained a pulse and stop. |
| DualSense | `linux.uhid.usb` | Absent from the isolated Steam list. In the earlier owner-observed attempt a tester and separate gyro screen were visible, with timing controllers present. UHID gyro availability remains inconclusive across the two attempts. |
| DualShock 4 | `linux.uinput` | Listed. Four face buttons, both sticks and both individual triggers verified. Ping retained a pulse and stop. |
| DualShock 4 | `linux.uhid.usb` | Absent from both the isolated owner check and automated Steam list. Tester unavailable. |
| Switch Pro | `linux.uinput` | Listed in isolation. Four face buttons, both sticks and the two digital trigger endpoints visible. The first left-trigger release frame still showed the press. A later neutral frame cleared it. Ping retained a pulse and stop. |
| Switch Pro | `linux.uhid.usb` | Absent from the isolated owner and automated Steam lists with the stock Nintendo driver. Steam neutral unavailable. |
| Xbox 360 | `linux.uinput` | Listed. Four face buttons and both sticks verified. Both individual trigger sweeps were visible in the smoke test. The repeat missed the first nonzero right-trigger step. Ping retained a pulse and stop. |
| Xbox 360 | `linux.uhid.usb` | Listed as Virtual Xbox 360 (Standard HID). Both sticks and both trigger sweeps verified. Three face buttons showed transitions. East was inconclusive. Ping unavailable. |

The isolated Steam run could not associate the USB UHID DualSense with a controller entry. That discovery failure is a candidate explanation for the missing gyro UI in the isolated run. The earlier owner-observed gyro screen was separate from the main tester. Report bytes were unchanged.

Xbox Series P3. With `xpadneo` bound, SDL opened the joystick event node on the Bluetooth-bus UHID device before any edit. Both sticks read `[0, 0, 0, 0]` at open and throughout 3.2 seconds. This checks SDL-visible centre. SDL initializes axis state to zero, so this alone is insufficient to time the first delivered neutral input report.

## Timing

All batches used 50 cycles, `--sdl`, no budgets and HIDAPI disabled for timing. Each row gives the number of observed stages, followed by the warm p50 in milliseconds for API return, node appearance, consumer open, SDL readiness and node disappearance. SDL readiness measures probe completion. A missing consumer-open stage is logged even when SDL subsequently opens an individual gamepad. First-neutral timing was unavailable in xtask for these families.

| Family and realization | N | Slots | API / present / consumer / SDL / gone | Warm p50 in ms in the same order |
| --- | --- | ---: | --- | --- |
| DualSense `linux.uhid.usb` | 1 | 50 | 50 / 50 / 0 / 0 / 50 | 0.62 / 3.32 / — / — / 75.08 |
| DualSense `linux.uhid.usb` | 4 | 200 | 200 / 200 / 0 / 0 / 200 | 0.31 / 4.94 / — / — / 114.21 |
| DualSense `linux.uinput` | 1 | 50 | 50 / 50 / 0 / 50 / 50 | 0.56 / 1.56 / — / 583.03 / 32.01 |
| DualSense `linux.uinput` | 4 | 200 | 200 / 200 / 0 / 200 / 200 | 0.56 / 1.64 / — / 638.98 / 30.62 |
| DualSense `linux.usbip.usb-audio` | 1 | 50 | 0 / 0 / 0 / 0 / 0 | — / — / — / — / — |
| DualSense `linux.usbip.usb-audio` | 4 | 200 | 0 / 0 / 0 / 0 / 0 | — / — / — / — / — |
| DualShock 4 `linux.uhid.usb` | 1 | 50 | 50 / 50 / 0 / 0 / 50 | 0.41 / 2.86 / — / — / 52.04 |
| DualShock 4 `linux.uhid.usb` | 4 | 200 | 200 / 200 / 0 / 0 / 200 | 0.39 / 5.53 / — / — / 104.89 |
| DualShock 4 `linux.uinput` | 1 | 50 | 50 / 50 / 0 / 50 / 50 | 0.57 / 1.79 / — / 574.95 / 31.58 |
| DualShock 4 `linux.uinput` | 4 | 200 | 200 / 200 / 0 / 166 / 200 | 0.54 / 1.56 / — / 605.44 / 32.39 |
| DualShock 4 `linux.usbip.usb-audio` | 1 | 50 | 0 / 0 / 0 / 0 / 0 | — / — / — / — / — |
| DualShock 4 `linux.usbip.usb-audio` | 4 | 200 | 0 / 0 / 0 / 0 / 0 | — / — / — / — / — |
| Switch Pro `linux.uhid.usb` | 1 | 50 | 50 / 50 / 0 / 0 / 50 | 0.13 / 3.84 / — / — / 2089.65 |
| Switch Pro `linux.uhid.usb` | 4 | 200 | 200 / 200 / 1 / 0 / 200 | 0.11 / 3.58 / 26.54 / — / 44.43 |
| Switch Pro `linux.uinput` | 1 | 50 | 50 / 50 / 0 / 50 / 50 | 16.89 / 22.42 / — / 1486.73 / 22.71 |
| Switch Pro `linux.uinput` | 4 | 200 | 200 / 200 / 0 / 200 / 200 | 0.33 / 1.44 / — / 692.33 / 15.05 |
| Xbox 360 `linux.uhid.usb` | 1 | 50 | 50 / 50 / 0 / 49 / 50 | 0.08 / 2.66 / — / 493.31 / 16.53 |
| Xbox 360 `linux.uhid.usb` | 4 | 200 | 200 / 200 / 0 / 198 / 200 | 0.07 / 2.71 / — / 534.93 / 17.68 |
| Xbox 360 `linux.uinput` | 1 | 50 | 50 / 50 / 0 / 50 / 50 | 0.32 / 1.12 / — / 527.31 / 15.17 |
| Xbox 360 `linux.uinput` | 4 | 200 | 200 / 200 / 0 / 200 / 200 | 0.31 / 1.10 / — / 547.66 / 16.68 |
| Xbox 360 `linux.usbip.usb-audio` | 1 | 50 | 0 / 0 / 0 / 0 / 0 | — / — / — / — / — |
| Xbox 360 `linux.usbip.usb-audio` | 4 | 200 | 0 / 0 / 0 / 0 / 0 | — / — / — / — / — |

Across the 16 uinput and UHID timing batches, all 2,000 controllers returned from creation,
had an owned node appear, and had their owned nodes disappear. The all-node consumer-open
stage succeeded once. Its failures were 1,984 access denials and 15 cases in which a
selected node was missing at open. The exact-node SDL readiness stage succeeded 1,213
times. Of its 787 missing measurements, 753 had no owned node of the requested kind
available to the timing probe, and 34 failed during SDL enumeration or open. Timing
conditions differed from the focused stick sweep in node selection, desktop activity
and concurrent load. These values are observations without acceptance budgets.

Switch Pro does not declare USB/IP audio. The stock timing command failed to create USB/IP audio controllers because it did not supply the required Cargo feature and emulated audio options. Raw timing data stayed outside the worktree.

The opt-in retry used `cargo run --locked -p xtask --features virtualgamepad/audio-usbip -- acceptance` on tooling fix `31a5ca26a3b77068329a8b27b8ed1d9b010ce5d9`. It enables the existing Cargo feature and requests emulated audio for this target. The DualSense retries used that revision. DualShock 4 and Xbox 360 retries used `f3d43ec03af0ba90f555588b7d1f9a591257ff12`, which treats the absence of a creation-owned selector as unmeasured.

The worker did not supply a physical-path selector for the audio realizations. Therefore consumer open and SDL readiness could not be attributed to a creation even when the constructor succeeded. On the first retry revision the apparent removal counts compared an empty list against an empty list. The later revision reports the observation as unavailable. All 50 N1 creations succeeded per family. At N4,
150 of 200 creations per family succeeded and the fourth request in each cycle was
refused because no administrator-allowlisted VHCI port remained. The final USB/IP attach list was empty.

| Family and realization | N | Slots | API / present / consumer / SDL / gone | Warm p50 in ms in the same order |
| --- | --- | ---: | --- | --- |
| DualSense `linux.usbip.usb-audio` | 1 | 50 | 50 / 0 / 0 / 0 / 50 | 465.77 / — / — / — / 12.56 |
| DualSense `linux.usbip.usb-audio` | 4 | 200 | 150 / 0 / 0 / 0 / 150 | 406.16 / — / — / — / 10.56 |
| DualShock 4 `linux.usbip.usb-audio` | 1 | 50 | 50 / 0 / 0 / 0 / 0 | 437.67 / — / — / — / — |
| DualShock 4 `linux.usbip.usb-audio` | 4 | 200 | 150 / 0 / 0 / 0 / 0 | 422.22 / — / — / — / — |
| Xbox 360 `linux.usbip.usb-audio` | 1 | 50 | 50 / 0 / 0 / 0 / 0 | 382.30 / — / — / — / — |
| Xbox 360 `linux.usbip.usb-audio` | 4 | 200 | 150 / 0 / 0 / 0 / 0 | 376.71 / — / — / — / — |

## Physical DualSense LEDs

On one owner-observed USB DualSense with the stock `playstation` driver bound, RGB channels were red, green and blue in order. Setup 0x02 extinguished the bar. The five centred player patterns appeared as expected. The observer could not distinguish 0x02 from 0x08 or 0x01 from 0x10 by eye. The player LEDs visibly faded in with bit 0x20 clear and came on immediately with it set. Mute values 0, 1 and 2 appeared off, solid on repeat, and pulsing. Brightness values 0 through 2 made no visible difference. LED release left the lightbar cyan and switched the player and mute LEDs off. The owner confirmed that the blue lightbar, the single centre player LED, and mute LED off returned after restoration.
