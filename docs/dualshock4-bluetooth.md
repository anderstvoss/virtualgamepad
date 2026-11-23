# DualShock 4 Bluetooth-bus UHID realization (p9b)

**Status:** local synthetic UHID with bus metadata 5; no radio, pairing, BlueZ or physical Bluetooth connection. Source evidence only; host runs follow in local kalux P5b (**not run**). The support-matrix cell stays **WIP**.

### Realization levels

| Realization ID | Level | Mechanism | Permission |
| --- | --- | --- | --- |
| `linux.uhid.bluetooth` | Native HID | UHID | needs UHID access |

## Exact selection and semantic state

`create_dualshock4` requires explicit `LINUX_UHID_BLUETOOTH` to create the Bluetooth-bus node. USB and Bluetooth cannot silently substitute for each other. Both expose `DualShock4Inputs`, `set_inputs`, `edit_inputs`, `status()` and native button, axis, motion and two-contact touch semantics. Supplying `DualShock4Options::with_identity` sets the Bluetooth UHID `uniq` according to the instance identity, while a new Bluetooth session allocates its own identity. No private identity is logged, committed or synthesized in documentation.

## Native Bluetooth report contract (synthetic, not physical proof)

| Report | Length including ID | Role | Source-based boundary |
| --- | ---: | --- | --- |
| Input 0x01 | 10 | Minimal, sticks and buttons before enhancement | [Linux DS4 decoder](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L2389-L2454) |
| Input 0x11 | 78 | Enhanced, common buttons/IMU/battery, four touch-history slots, CRC32 seeded with input HIDP header | [Linux report layouts](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L468-L505) |
| Output 0x11 | 78 | Flags gate right/left rumble, RGB, blink; invalid CRC rejected for emulated writes | [Linux output worker](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L2277-L2386) |
| Feature 0x02 / 0x05 / 0xA3 | 37 / 41 / 49 | SDL's initial calibration read; BT calibration with feature CRC and optional firmware info | [Linux BT calibration](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L1938-L2033) |

The compiled descriptor declares these report sizes but is deliberately synthetic. The 0x11 full report starts only after a supported calibration read or a valid effects output, matching the feature-switch consumer path in [pinned SDL](https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/hidapi/SDL_hidapi_ps4.c#L462-L647). The exact retail descriptor, mode-transition timing, frame cadence, checksum acceptance and firmware-specific feature bytes remain unknown. The virtual adapter does not copy any captured private data.

## Restrictions and evidence ceiling

- **Bluetooth device fidelity:** local bus-type metadata is not wireless discovery, pairing or a retail descriptor. Do not report a physical Bluetooth connection or fidelity acceptance.
- **Bluetooth audio:** UHID has neither speaker nor headset PCM endpoints; cannot carry native audio. Audio is addressed separately for other realizations.
- **Host and device observation:** compile-time/golden synthetic CRC and report tests are not Linux, SDL or physical captures; local P5b qualification and eventual owner-approved hardware comparison are required.

The per-function [DS4 acceptance manifest](../acceptance/virtualgamepad.dualshock4.toml) enumerates the new target with WIP statuses, consumer claim IDs and audio limitations. [Research draft #159](https://github.com/anderstvoss/controller-protocol-research/pull/159) adds `ds4-bt-consumer-input-envelope`, `ds4-bt-consumer-output-effects`, `ds4-bt-consumer-calibration-open`, and `ds4-bt-consumer-identity-uniq`. Because that draft is not merged, the research pin remains unchanged and must be updated in a separate follow-up after merge.
