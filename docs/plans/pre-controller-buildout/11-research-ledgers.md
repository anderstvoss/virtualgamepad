# Brief 11: protocol research ledgers for Wii, Steam and Xbox Series (#167)

Read `ground-rules.md` first, then `controller-protocol-research`'s `README.md` and `CONTRIBUTING.md`.

**Lane:** Research (research repo only; touches no virtualgamepad code). **Blocked by:** nothing beyond Anders's word that evidence work starts (it is background work by his guidance, so ask once in D1 whether this lane may run now). Each sub-brief is one research-repo PR.
**Needed by:** every controller package's source-evidence cells. Research repo `main` (`259ff98`) has no Wii or Xbox Series records and only 2 SC2 sources and a few Puck claims.

## 11a. Schema gaps first (from #167 comment 6163642205, still open)

`applicability.controller` and `applicability.transport` in `schemas/claim.schema.json` are free text.
- Add a `records/controller/` registry (id, display name, family, known VID/PID per transport) and validate `applicability.controller` against it.
- Make `transport` an enum (`usb`, `bluetooth`, `receiver`, `source_only`, `not_measured`), keeping the free-text `scope`. Migrate existing records.
- Add a consumer-evidence marker (kernel commit, driver, file:line) if the schema has no way to express it, per the rule on #167 comment 6173997992.
- Optional: `tools/status.py` printing each controller's category from its claims.

## 11b. Wii Remote L0 ledger (#158, plan comment 6173815463 section C and L0)

Sources: kernel `hid-wiimote-core.c`, `hid-wiimote-modules.c`, `hid-wiimote.h` at a pinned commit; Dolphin `WiimoteEmu` and `WiimoteReal/IOhidapi.cpp`; SDL HIDAPI Wii driver. Output: source records, claims for report IDs and modes, status/ACK flow, extension ID/init/encryption, MotionPlus pass-through, IR modes, LEDs/rumble/battery, memory map (treat `0x1700` EEPROM size as a hypothesis), and a consumer-evidence table of what `hid-wiimote` creates on evdev (the reference shape for the uinput realization). Restate facts with file:line; no GPL excerpts. Mark original Remote, early integrated Plus and `RVL-CNT-01-TR` distinctly.

## 11c. Steam Controller migration (#159)

Move the findings already posted on #159 into records: OpenPuck `29f4d1e` (1304/1305, 4 interfaces, report list), kernel `hid-steam` at `0fcb05b72344` (1302/1303/1304/1305, interface layout, reports, lizard-mode parameter) as consumer evidence, and the 1304 vs 1142 conflict. Coordinate with brief 03 P0 (it adds the kernel release pin and SDL findings); whoever lands second rebases.

## 11d. Xbox Series L0 inventory (#160, research comment 6163788857 and review 6173065010)

`xpad` evdev surface for `045e:0b12` at the pinned commit (26 capabilities, `ABS_Z`/`ABS_RZ` 0..1023, Share as `KEY_RECORD`, FF_RUMBLE zeroing trigger motors) as consumer evidence; SDL `SDL_hidapi_xboxone.c` rumble formats for Bluetooth and GIP; link the gamepadcapture evdev fixture accepted in research PR #6. Keep GIP and Bluetooth HID claims apart; never use Xbox 360 standard HID as a Series reference.

## Done when

Each PR passes `tools/publication_guard.py`, `tools/validate.py` and the tests, is merged, and the controller issue gets a one-line comment linking the records. Then bump virtualgamepad's `protocol-research` pin (as in brief 01b).
