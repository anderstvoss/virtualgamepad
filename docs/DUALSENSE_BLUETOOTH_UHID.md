# DualSense Bluetooth-bus UHID (p9a)

**Status:** WIP, source-backed protocol only. This is local `linux.uhid.bluetooth` with Linux UHID bus type 5, Sony 054c:0ce6 and a synthetic HID descriptor. It is not Bluetooth radio, BlueZ, pairing, security or reconnection ([#164](https://github.com/anderstvoss/virtualgamepad/issues/164)). No report herein is a captured reference-device report.

## Explicit creation and scope

Select `RealizationId::LINUX_UHID_BLUETOOTH` with `create_dualsense(CreationOptions::new(...), DualSenseOptions::default())` (or supply `DualSenseOptions::with_identity` using the existing restoration API). Bluetooth and USB targets are distinct and never substitute for each other. One session-owned identity drives the Bluetooth `uniq` value and feature reply. The `DualSenseInputs` and `DualSenseStatus` APIs, transactional edits, status, and retained typed output are common to the USB implementation.

The Bluetooth codec emits initial neutral simple input `0x01` (nine payload bytes) and switches to the 78-byte `0x31` enhanced input on accepted host feature/output activity. The enhanced report prefixes the 63-byte common input at wire byte 2, and signs with input CRC. Linux and SDL use distinct but compatible `0x31` host output prefixes; both are CRC-checked, and the shared 47-byte effect block is decoded to the existing typed output state, including LED and native 11-byte trigger payloads. Trigger output retention does **not** prove actuator or advanced-haptics fidelity.

Features `0x05` (calibration), `0x09` (pairing identity), and `0x20` (firmware) are served using the same existing synthetic common feature data as the USB codec, with Bluetooth-specific checksum trailers. This does not introduce a separate source of calibration or firmware data. Bad-CRC host output is rejected without changing retained output. No PCM, headset, controller speaker or audio-sidecar is available on this Bluetooth target.

## Evidence and tests

- [Pinned Linux input/output and CRC](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L244-L316)
- [Pinned Linux feature verification](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L799-L833)
- [Pinned SDL Bluetooth mode discovery](https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/hidapi/SDL_hidapi_ps5.c#L399-L416)
- [Source-only gap report](https://github.com/anderstvoss/controller-protocol-research/blob/40e43c5c1580f0791aa7fae9aa83661c128fc36c/docs/p9a-dualsense-bt-uhid.md)

**Evidence ceiling:** consumer source, synthetic deterministic tests. Exact physical Bluetooth descriptor, mode-transition timing, physical sensor rates, output actuation, real radio and pairing are not verified. Local Linux/SDL/Steam host runs follow in the P5b kalux qualification. **Not run** in this agent environment. All `acceptance/virtualgamepad.dualsense.toml` Bluetooth cells remain WIP; no support-matrix promotion.
