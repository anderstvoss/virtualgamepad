# DualSense motion-input API decision

Status: accepted

`DualSenseInput` exposes raw signed 16-bit gyroscope and accelerometer vectors.
The concrete handle accepts a complete `DualSenseMotion` value through
`set_motion` or an atomic typed-state edit. Motion is native controller state,
not a universal capability or a string-addressed field.

The 64-byte USB input payload encodes gyroscope little-endian values at bytes
`15..21` and accelerometer values at `21..27`; report ID `0x01` remains outside
that payload for UHID. The library does not silently calibrate, rotate, apply
gravity compensation, or convert raw report units to SI units.

A future controller with motion defines its own typed state and report mapping.
Shared physical-unit types may be introduced only when their calibration and
meaning are truly equivalent across controllers.
