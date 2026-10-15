# Brief 13: Wii Remote package (#158), checkpoint C5

Read `ground-rules.md` first, then #158's realization proposal (comment 6173815463), its consumer list (6174005221) and its realization table (6174032146). This brief carries their decisions forward and supersedes their "Bluetooth-bus UHID deferred" line (superseded on 2026-10-15; D7 settles it).

**Lane:** Controller. **Checkpoint:** C5, branch `buildout/c5-wii-remote`.
**Blocked by:** checkpoint C1 merged (registry); **D6** (Wii production gate on #158); **D7** (first realizations). 13c also needs checkpoint C2 (live topology and persistent resources). 13d needs a host (D9) for its live objective; without one, land the code and leave the live cell untested.
**Not blocked by:** D2. IR reaches uinput as axes; the typed point-set kind (10c) is used only if D2 has landed.

## Goal

Playable Wii Remote with every hardware feature a game needs: buttons, accelerometer, IR, Nunchuk, Classic/Classic Pro, MotionPlus (alone and in pass-through), LEDs, rumble and battery. It ships in two realizations, per D7's recommendation:

- `linux.uinput` (playable, reduced). Its evdev shape follows what `hid-wiimote` creates, so evdev consumers see the same devices a real Remote produces.
- `linux.uhid.bluetooth` (playable, full). A UHID node on bus 5, which is the only path that binds `hid-wiimote`. This is not real Bluetooth: there's no radio, pairing or BlueZ (#164).

Out of scope: the speaker (a later, separately gated realization), real Bluetooth, USB-bus UHID (a fixture-only experiment per D7), Balance Board, Wii U Pro, Guitar/Drums/Turntable (later, using the same extension seam), and Dolphin Real Wii Remote passthrough.

## Evidence rule for autonomous work

Every byte the package sends comes from a record in `controller-protocol-research` (ledger 11b: kernel `hid-wiimote-*` at a pinned commit, Dolphin `WiimoteEmu`, SDL). Cite the record ID in a comment next to each table or constant.

- If a fact you need has no record: add the source record and claim first (research PR), then use it.
- If no source settles it: leave that feature unimplemented, mark its acceptance cell "No evidence", and list it under Judgement calls. Never invent bytes, timings or calibration.
- Kernel behaviour is consumer evidence. It tells you what the host expects, and never promotes a device-side status on its own.

## Sub-steps (one PR each into `buildout/c5-wii-remote`)

**13a. Family skeleton and native state.** Register `wii-remote` in the registry with `linux.uinput` and `linux.uhid.bluetooth`. Add typed native state (core buttons, accel, IR points, extension slot, LEDs, rumble, battery) and the identity variants (original Remote, integrated Plus, `RVL-CNT-01-TR`), kept apart as the ledger requires.
Objective: registry tests pass with the new family and no other family's test changed. The new family needs only its module plus one registry line, as ADR-0018 promises.

**13b. Report codec.** Output reports from the host: rumble/LED `0x11`, report mode `0x12`, IR enable `0x13`/`0x1a`, status request `0x15`, memory write `0x16` and read `0x17`. Input reports to the host: status `0x20`, read data `0x21`, ACK `0x22`, data modes `0x30` to `0x3f`.
Objective: table-driven tests encode every data mode from typed state and decode every output report, using synthetic fixtures recorded in the research repo. A status request always gets a status report. Each write gets an ACK with the documented result code. A read outside the modelled memory map returns the documented error.

**13c. Extensions and memory.** Nunchuk, Classic/Classic Pro and MotionPlus with pass-through, using live topology (step 8) for attach and detach. Implement the unencrypted init (`0x55` to `0xa400f0`, `0x00` to `0xa400fb`), the extension ID at `0xa400fa`, and the MotionPlus activation and pass-through modes. Put the EEPROM and calibration in persistent resources (step 9), treating the `0x1700` size as a hypothesis until a record settles it.
Objective: topology tests cover attach, detach, swapping one extension for another, MotionPlus activation with and without a downstream extension, and the status report's extension bit on each change. A persistent-resource test shows an imported EEPROM read back through `0x17`.

**13d. Realizations.**
- uinput: one evdev device per `hid-wiimote` interface (core, accel, IR, MotionPlus, and each extension), with names, codes and ranges taken from the ledger's consumer table. Extension devices appear and disappear with topology.
- `linux.uhid.bluetooth`: the Nintendo identity on bus 5 with a descriptor from the ledger.
- Objective, deterministic (CI): a golden-shape test generates the expected evdev capability set from the ledger's `hid-wiimote` consumer records, so it isn't hand-copied, and compares the uinput realization against it with a fake provider.
- Objective, live (host, D9): the acceptance runner (step 7) shows `hid-wiimote` bound to the UHID node (sysfs `driver` is `wiimote`), its input devices match the same golden shape, an extension attach produces the extension's device, LEDs and rumble set through the kernel reach the native state, and the battery appears under `power_supply`. Record this as a research-repo experiment.

**13e. Acceptance and timing.** `acceptance/wii-remote.toml` cells per feature and realization. #166 creation and removal timing for 1 and N controllers on the host.
Objective: every cell is filled with a test ID or an honest status (`untested`, `No evidence`, or `unavailable in this realization`, with the reason). SDL's HIDAPI Wii driver and Dolphin stay separate rows; mark them untested unless the run includes them.

## Checkpoint C5 PR

It follows IMPLEMENTATION_PLAN.md 1.6. **Needs your attention** lists every feature left at "No evidence" and every deviation from `hid-wiimote`'s shape in the uinput realization (document them rather than widening the type, per the project's controller guidance). Post one line on #158 linking the C5 PR.
