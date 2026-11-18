# Typed rumble records — realization audit

Source review baseline: [virtualgamepad](https://github.com/anderstvoss/virtualgamepad/tree/324d250bb01a5ddbfc1dd189513d4c9160429ee1), [research](https://github.com/anderstvoss/controller-protocol-research/tree/6fc8a2d4d8a48a8f025d98ac8448e46c0f97ea7d). Source evidence only, no physical run. Optional observations are not the ADR-0024 accepted-command journal.

| Family | uinput | UHID USB | UHID Bluetooth | Existing typed view |
| --- | --- | --- | --- | --- |
| Xbox Series | FF_RUMBLE, two channels | Not supported | HID output 0x03, four channels | `host_outputs()` returns `XboxSeriesOutputState` with `RumblePlayback` or `XboxSeriesBluetoothRumble`; raw output observation has no rumble accessor. |
| DualShock 4 | FF_RUMBLE | HID output 0x05 | Not supported; 0x11 is research-only for this product | `DualShock4HidOutput::UsbOutput` carries motor bytes but does not gate them on the rumble validity bit; retained `DualShock4OutputState` does. |
| Xbox 360 | FF_RUMBLE | Standard HID input only; XUSB rumble report not admitted | Not supported | `Xbox360OutputState` retains evdev playback; native observation is raw, not an XUSB interpretation. |
| Wii Remote | FF_RUMBLE, collapsed to binary | Not supported | Shared bit 0 of declared HID outputs | `WiiRemoteOutputs::rumble` retains binary intent; raw observation has no typed accessor. |

USB/IP does not add an XUSB-native Xbox 360 path; DS4 USB worker is separately a USB presentation. No realization, FF input envelope, GIP or journal changes are implied. Proposed decoder tests: not run.
