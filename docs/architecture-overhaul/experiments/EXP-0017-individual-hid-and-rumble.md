# EXP-0017: individual HID controls and Switch output framing

## Source review and deterministic correction

Switch report 0x10 was accepting eight payload bytes, although it contains one
packet counter and two four-byte motor words. The minimum is now nine bytes;
SET completion tests cover every length 0–64, exact success/error status, repeated
service and terminal cleanup. `SwitchProOutputEvent::rumble()` exposes the counter
and separate encoded left/right words from both rumble-bearing report classes.
It does not claim a physically validated amplitude/frequency decoder.

Independent pinned examples agree on these field boundaries:

- OpenPuck `29f4d1e8b85c0a7c19e0c8b977a1a56a2c42ea6d`,
  `mode_switch_pro.cpp::jcRumble`: at least nine bytes, motors at offsets 1 and 5.
- SDL 3.2.0 `535d80badefc83c5c527ec5748f2a20d6a9310fe`,
  `SDL_hidapi_switch.c`: common output counter followed by two rumble words.

OpenPuck's compressed decoder maintains per-band state and reduces its result to
SC2 single-motor magnitude; it is not evidence that copying that reduction gives
faithful Switch HD rumble. Pinned HHD `adbd50ff30614df886a6c76bd8855a64fde5d0b8`
provides no independent Switch HID rumble decoder. Retain encoded motor data and
this fidelity limit. Existing corpus pins remain unchanged and published.

## Consumer apparatus

Each family uses three sequential creations with application IDs 7, 7 and 65543.
Every creation runs 26 isolated controls/endpoints, each followed by neutral:
156 exact observations per family. The harness selects one process-owned device,
services its protocol while SDL runs, removes it, closes again and verifies the
owned node disappears. Sony/Switch select HIDAPI and exact hidraw paths; Xbox
selects Linux event input. No touch injection or physical device is used.

A clean private SDL 3.2.0 build uses the revision above, Release/shared,
console-only, X11/Wayland/audio/camera/tests disabled. Audio was disabled after the
old SDL source failed against installed PipeWire headers. This input acceptance
build is not a product dependency or audio acceptance. Library paths are scoped
to each invocation. Source, configuration, build failures and raw records remain
in the user's private cache. No system library or service was changed.

The first Sony attempts failed D-pad Up after 24 passing observations each. The
probe announced readiness immediately after open, before polling neutral input.
Pinned SDL Sony drivers compare reports against a zero-initialized previous
button byte, which also encodes Up. Waiting for a serviced neutral phase before
applying the isolated transition fixes this apparatus sequence without changing
controller encoding. The probe now polls for at least 100 ms with neutral state
before readiness, and records SDL revision, raw joystick counts and final hat.
A pure regression checks the readiness time boundary and non-neutral rejection;
Sony wire regressions independently assert each cardinal hat and neutral release.
This does not prove held-input-at-consumer-open behavior for every consumer.

## Acceptance scope

Final per-family results are recorded in the gate ledger. Initial failures and
cleanup records are retained privately; they are not silently replaced by passes.
The earlier cached-library combined run is unsuitable for cross-family assessment:
its Sony mapping omitted D-pad entries and the first panic poisoned the shared
live-test lock. Subsequent runs use a clean library and separate test processes.

This measures isolated standard controls and endpoints. Motion, touch and output
acceptance retain the separate EXP-0008/0009 evidence; no new touch pass is claimed.
Steam/Eden build/backend/mapping comparisons remain pending. DS4/Switch physical
fidelity, compressed-rumble mechanics, DS4 split association and Gate G remain
separate limitations. Every support cell stays WIP.
