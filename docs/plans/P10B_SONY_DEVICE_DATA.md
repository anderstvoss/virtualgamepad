# P10b Sony device-data implementation note

Prompt: `p10b-sony-device-data`; tracking: vg #365.

Scope: typed per-instance synthetic-by-default calibration and firmware feature data
for DualSense and DualShock 4; caller-validated import/export, USB/UHID and USB/IP
routing, application read-back, demo UI, tests, and public API inventory.

Privacy: no physical device data, captured blobs, serials, calibration contents,
or identifying addresses are stored in this repository. All tests use synthetic
fixtures; file import is strictly a runtime, caller-controlled operation.

The checked baseline is virtualgamepad `6c795401b4d4cc014a59087896e629af78044f62`
and controller-protocol-research `77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8`.
