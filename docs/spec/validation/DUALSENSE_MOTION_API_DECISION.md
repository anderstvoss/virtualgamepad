# DualSense motion-input API decision

Status: accepted

## Decision

DualSense motion is represented in `gr_core` as raw signed 16-bit, three-axis
samples in every full `DualSenseInput` frame:

- `motion.gyroscope.{x,y,z}`
- `motion.accelerometer.{x,y,z}`

Sparse deltas use `DualSenseMotionDelta` and `MotionAxesDelta`, so each axis
can be updated independently. Zero is the neutral sample. The frame timestamp
is the timestamp for both sensors; per-sensor timestamps are deliberately not
introduced because the DualSense input report carries one shared sensor time.

The values are intentionally raw report units. The library transports and
emulates controller data; it does not impose host-specific calibration,
gravity compensation, orientation, or SI-unit conversion.

## Realization

The DualSense HID and USB transport translators encode the raw samples into
the existing 64-byte USB input report payload:

- gyroscope: little-endian `i16` values at bytes `15..21`
- accelerometer: little-endian `i16` values at bytes `21..27`

The report ID remains out-of-band in `BackendFrame::HidInputReport`, so these
offsets are relative to the payload buffer, not a buffer that includes report
ID `0x01`. Bluetooth transport reuses the same payload before adding its
transport header.

The mapping is based on the Linux `hid-playstation` DualSense input-report
layout. Tests pin the byte-level mapping with positive and negative extrema.

## Scope and reconsideration

Motion is a typed input contract, not an inferred capability flag. Any new
profile with motion must add a profile-specific payload type and a tested
translator mapping; it must not reuse these DualSense fields.

This decision should be revisited only when adding calibrated/physical-unit
motion APIs, separate sensor timestamps, or an input provider whose report
format cannot carry the six raw samples. Those changes require a new explicit
contract rather than silently redefining the raw values above.
