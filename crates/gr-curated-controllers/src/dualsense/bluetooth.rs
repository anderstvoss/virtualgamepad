//! Consumer-compatible DualSense Bluetooth-bus UHID framing.
//!
//! Bluetooth-bus UHID is a local HID node, not radio transport or pairing.
//! Format evidence: pinned Linux hid-playstation and SDL HIDAPI; physical
//! descriptor identity, real-device behavior and host acceptance are unknown.
//! https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L76-L95
//! https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/hidapi/SDL_hidapi_ps5.c#L1431-L1475
