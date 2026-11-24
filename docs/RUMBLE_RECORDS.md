# Typed rumble records — realization audit

The original source review was pinned to [virtualgamepad `324d250`](https://github.com/anderstvoss/virtualgamepad/tree/324d250bb01a5ddbfc1dd189513d4c9160429ee1) and [research `6fc8a2d`](https://github.com/anderstvoss/controller-protocol-research/tree/6fc8a2d4d8a48a8f025d98ac8448e46c0f97ea7d). The typed decode accessors below are now compiled on `main`. Deterministic records and scoped host observations do not assert physical motor response. **Optional output events and retained snapshots are not the accepted-command VG-2 journal.**

| Family | Relevant realizations | Current root typed view |
| --- | --- | --- |
| DualSense | uinput FF; USB/BT UHID | Native trigger report/effect types, including `DualSenseTriggerEffectUpdate`; use `decode_dualsense_trigger_effect` for supported payloads. BT actuation and physical fidelity remain unverified. |
| DualShock 4 | uinput FF; USB output 0x05 and BT output 0x11 | `DualShock4OutputEvent::rumble()` returns `DualShock4Rumble` for recognized reports or playback. The BT payload requires its valid flags and CRC. |
| Switch Pro | uinput FF; USB/BT UHID motor words | `SwitchProOutputEvent::rumble()` and `SwitchProRumble::decode` describe supported encoded words, not physically measured HD-rumble amplitude/frequency. |
| Xbox 360 | uinput FF; standard HID USB input | `Xbox360OutputEvent::rumble()` exposes conventional playback; `Xbox360Rumble::decode_xusb` is **source-only**, not a report admitted by this USB HID realization. |
| Wii Remote | uinput FF; BT-bus UHID shared rumble bit | `WiiRemoteOutputEvent::rumble()` returns a binary-intent `WiiRemoteRumble`. No speaker PCM playback follows from this output. |
| Xbox Series | uinput FF; BT-bus UHID output 0x03 | `XboxSeriesOutputEvent::rumble()` returns `XboxSeriesRumble` (conventional two-motor playback or native four-channel command). |

USB/IP does not add an XUSB-native Xbox 360 path; DS4 USB worker is separately a USB presentation. No realization, FF input envelope, GIP or journal changes are implied. Decoder tests are deterministic synthetic fixtures; no physical run.


## Native fields and mappings

- Xbox Series Bluetooth 0x03: ID-excluded eight bytes, mask bits 0/1/2/3 select weak/strong/right-trigger/left-trigger; the four raw magnitudes are 0–100, duration/delay are 10 ms ticks, loop count is raw. The record retains even unselected magnitude bytes; consumers must consult the mask. The same pure validator is used by report admission and decoding. FF playback uses integer division by 655 (0–65535 → 0–100, consistent with the existing product regression); no trigger/mask/timing bytes are invented. Claims: `xbox-series-sdl-bluetooth-rumble`, `xbox-cgap-descriptor-example`, `xbox-series-xpad-trigger-rumble-limitation`; supplemental `xbox-series-rumble-native-fields` ([research PR #155](https://github.com/anderstvoss/controller-protocol-research/pull/155)). [Descriptor](https://github.com/atar-axis/xpadneo/blob/56b538d951300bf804f6631738603c5eb14c04f0/docs/descriptors/xbxs.md#L123-L165), [consumer layout](https://github.com/atar-axis/xpadneo/blob/56b538d951300bf804f6631738603c5eb14c04f0/hid-xpadneo/src/xpadneo/xpadneo.h#L71-L105).
- DS4 USB 0x05 (31-byte ID-excluded payload) and Bluetooth 0x11 (77-byte ID-excluded payload) share motor fields and two validity bytes; bit 0 of the first flag enables both 0–255 motor fields. Clear bit yields no update, even for nonzero raw bytes. BT requires HID and CRC flags and CRC32; parser only, not a new realization. FF maps each magnitude to its high byte, as an explicit quantization policy. Claims: `ds4-l0-consumer-rumble-lightbar-output`, `ds4-l0-device-usb05-output`, `ds4-l0-device-bt11-output`; supplemental `ds4-rumble-native-fields` (research PR #155). [Kernel layout](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L507-L539), [consumer flag/CRC convention](https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/hidapi/SDL_hidapi_ps4.c#L944-L970).
- Xbox 360: source-only complete wired XUSB eight-byte rumble parser; current standard-HID output event is not an XUSB report. FF maps each magnitude to its high byte (0–255), as SDL's wired consumer does. Claims: `x360-l0-consumer-sdl360-rumble-led`, `x360-l0-device-xusb-output-ff-led`. [SDL](https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/hidapi/SDL_hidapi_xbox360.c#L199-L215).
- Wii: bit 0 of the first payload byte on every declared output; parsing verifies ID and length, not other stateful admission rules. In FF playback any nonzero original magnitude means on; no byte quantization before the binary decision. 0x18 still discards speaker audio. Claims: `wii-gap-wiibrew-rumble-bit-retention`, `wii-c5-report-layouts`, `wii-kernel-outputs-battery`.

All FF records are derived only from playback commands: zero repetitions stops both motors, regardless of original magnitude. Upload/erase and lifecycle events are not rumble playback. Raw bytes, report IDs and original FF effects (including delay, length and repetitions) stay on the original event alongside the decoded record. The record is a pure view, not a new journal or a physical actuation assertion.

## Acceptance

Existing acceptance grades are unchanged. Decode sub-checks name deterministic tests and claims in the existing rumble cells. The two supplemental research claims are cited in descriptions rather than in the manifests' machine-resolved evidence arrays until the corpus PR lands and a separate reviewed pin update adopts it. Native XUSB and DS4 Bluetooth tests are source fixtures, not accepted VG host operations.

TODO(alpha2-final): G2 DualSense Bluetooth output decode and VG-2 journal verification.
