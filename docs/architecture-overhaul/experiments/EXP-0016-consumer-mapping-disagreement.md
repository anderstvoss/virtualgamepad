# EXP-0016: consumer disagreement and Xbox HID button usages

## User-reported observations

Independent manual testing used Eden nightly's SDL input backend on ARM. Exact
Eden/Steam builds, SDL runtime, selected paths and mapping strings were not
provided, so these are user-reported observations, not an automated acceptance
matrix or a physical-controller comparison:

- DS4 and Switch Pro HID gyro work in Eden.
- Steam and Eden disagree on Sony/Switch axes. Steam shows DS4 right-stick
  movement as both triggers; DS4 trigger movement appears on right-stick Y.
  DualSense trigger movement produces no observed movement in that test.
- Steam shows Switch sticks about halfway down/right at demo neutral 0/0;
  Eden does not exhibit this offset.
- Xbox North/Y appears as X and West/X is not picked up in Eden. The reported
  evdev-code naming discrepancy must be distinguished from printed/spatial
  button identity.

These reports are useful consumer evidence. They do not establish a Steam bug or
justify changing Sony/Nintendo wire axes that Eden interprets correctly. DS4 has
no physical reference available. DualSense is unplugged and its previously
reported stick drift/broken triggers limit affected physical fidelity evidence.
Steam Controller development remains deferred until the overhaul lands.

## Confirmed Xbox defect and correction

The standard-HID descriptor declared consecutive Button usages 1–16, while its
encoder packs A/B/X/Y and seven auxiliary buttons into the first eleven bits.
Linux's generic Game Pad mapping uses `BTN_GAMEPAD + usage - 1`. Thus X became
BTN_C (306), and Y became BTN_X (307). Several auxiliaries were wrong too.
This follows the pinned [Linux v6.12 HID input mapping](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-input.c).

The descriptor now explicitly declares usages 1,2,4,5,7,8,11,12,13,14,15, followed
by five constant padding bits. Report length, encoder order and axes stay the
same. It exposes exactly the existing evdev profile's eleven keys, including the
legacy xpad X/Y assignment (307/308), as supported by the
[Linux v6.12 xpad implementation](https://github.com/torvalds/linux/blob/v6.12/drivers/input/joystick/xpad.c).
BTN_NORTH is an alias for legacy BTN_X; that numeric naming alone is not a reason
to swap the already validated evdev profile. This remains standard HID, not USB
XInput or an xpad-bound device. The compiled gadget profile shares this descriptor;
its live acceptance remains Gate G dependent.

A deterministic short-item decoder checks declared Linux keys against every
individual native control, repeated press/release and both HID framings. It also
rejects phantom capabilities and checks 72-bit input length and clear padding.
Existing individual spatial evdev tests remain unchanged.

## Acceptance scope

The existing individual-control apparatus now accepts an Xbox UHID condition.
It selects the exact process-owned HID device and its single event node, uses
SDL's Linux event backend, waits for consumer readiness before activation, and
checks all 26 cases followed by neutral releases across three creations. It
never injects touch or uses physical devices. Raw records stay outside Git.

Historical aggregate sweeps only established that all consumer controls could
activate; they could not detect permutations. EXP-0014 individual tests covered
evdev, not this UHID descriptor. Those results must not be read as proof that
this earlier HID mapping was correct. Eden/Steam retesting of the correction is
still required independently of the pinned SDL probe.

Live Xbox UHID acceptance passed on Linux 6.12.107 arm64 with pinned SDL
3.2.0: 156 individual-control/neutral observations across three creations, exactly
one selected device per observation and three successful cleanup records. All
observed event nodes were absent after the run. Both X and Y were observed in
their expected SDL positions. The Linux event backend was explicitly selected;
this is not Eden, Steam, HIDAPI, physical Xbox or gadget acceptance. No permissions,
modules, services or consumer configuration were changed.

## Next comparison

Record exact realization, demo revision, consumer build, SDL version/backend,
selected identity/path, mapping string and per-axis neutral/min/max separately
for each consumer. Compare Linux event codes/ranges with SDL observations on the
same owned device. If raw Linux and SDL agree while Steam differs, preserve that
negative result and investigate Steam mapping/calibration before altering the
protocol. Do not reset shared consumer settings or rebind drivers to force a pass.
No isolated DS4 touch environment is available; this work does not remove that
prerequisite or pass Gate E/G or the audio/Bluetooth gates.
