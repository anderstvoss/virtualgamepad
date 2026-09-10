# Controller support matrix

All entries are **WIP** while the architecture overhaul is under review. WIP
includes implemented work under assessment and work not yet started. It does not
promise that every combination will ship. The later-controller rows are planned
future workflow exercises, not new controller APIs or current support.

| Controller | Linux uinput / evdev | USB personality over Linux UHID | USB HID through dummy_hcd | Bluetooth personality over UHID¹ | Actual Bluetooth¹ |
| --- | --- | --- | --- | --- | --- |
| DualSense | WIP | WIP | WIP | WIP | WIP |
| DualShock 4 | WIP | WIP | WIP | WIP | WIP |
| Switch Pro | WIP | WIP | WIP | WIP | WIP |
| Xbox 360 | WIP | WIP | WIP | WIP | WIP |
| Steam Controller (2026) + Puck | WIP | WIP | WIP | WIP | WIP |
| Xbox Series | WIP | WIP | WIP | WIP | WIP |
| Wii Remote | WIP | WIP | WIP | WIP | WIP |

¹ Gated concepts, not production realization IDs. USB personality over UHID is a
virtual HID presentation; dummy_hcd is a separate optional gadget realization.
The Xbox 360 HID profiles are standard HID, not USB XInput. This table does not
claim hardware fidelity, full function parity, audio or composite USB support.

## Status definitions and promotion

- **N/A:** explicitly not planned for this controller/realization, with a reason.
  Use this after scope review, for example for protocols outside a controller's
  intended generation. Missing hardware alone is not N/A.
- **WIP:** incomplete, unstarted, blocked or awaiting support review.
- **✓\*:** implemented and accepted for the documented scope without physical
  reference validation. Link deterministic and available consumer evidence and
  limitations; this is not physical equivalence.
- **✓:** accepted for the documented scope with linked physical reference
  validation. Record the device/profile, tested functions, consumer conditions
  and remaining limitations. Testing a virtual device alone does not qualify.

No cells are promoted yet, including those with successful Linux/SDL experiments.
See the [gate ledger](architecture-overhaul/GATE_STATUS.md) and
[acceptance evidence](architecture-overhaul/CORE_ACCEPTANCE.md) for those results.

## Per-controller function matrices

Create a separate function matrix for each controller during acceptance or its
follow-up issue. Rows name native functions; columns name realizations using the
same statuses. Cover applicable controls, motion, touch, outputs, identity,
accessories and lifecycle. Link each accepted cell to evidence and technical
limitations. Do not mark a whole controller accepted because one function works,
or require an absent native function merely because another controller has it.

The [landing and alpha plan](architecture-overhaul/LANDING_AND_ALPHA_PLAN.md)
defines the order: harden #106 and current controllers, review and land, prepare
initial alpha/versioning, then exercise new-controller issues and additions.

Production Wii Remote remains positively gated on later maintainer authorization.
The test-only Wii-like topology experiment does not enable a controller package,
demo entry, Bluetooth personality, or any support cell.
