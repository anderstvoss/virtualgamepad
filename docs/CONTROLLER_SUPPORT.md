# Controller support matrix

Current pre-alpha API refinement is tracked in
[the execution record](architecture-overhaul/ALPHA_API_REFINEMENT.md).
Emulated audio is implemented and opt-in; USB/IP remains WIP pending installed
security/recovery acceptance. Native-client continuity and matching fidelity remain
open. Demo defaults enable ALSA routing and WIP USB/IP; UHID/PipeWire
requires its feature. Root defaults remain empty. The expanded release gate is
not ready; see [remediation status](architecture-overhaul/ALPHA_REMEDIATION_STATUS.md).
Historical evidence below retains its original scope.

All entries are **WIP** during pre-alpha API/controller refinement. WIP
includes implemented work under assessment and work not yet started. It does not
promise that every combination will ship. The later-controller rows are planned
future workflow exercises, not new controller APIs or current support.

| Controller | Linux uinput / evdev | USB personality over Linux UHID | USB HID through dummy_hcd | Bluetooth personality over UHID¹ | Actual Bluetooth¹ |
| --- | --- | --- | --- | --- | --- |
| DualSense | WIP | WIP | Unavailable² | WIP | WIP |
| DualShock 4 | WIP | WIP | Unavailable² | WIP | WIP |
| Switch Pro | WIP | WIP | Unavailable² | WIP | WIP |
| Xbox 360 | WIP | WIP | Unavailable² | WIP | WIP |
| Steam Controller (2026) + Puck | WIP | WIP | WIP | WIP | WIP |
| Xbox Series | WIP | WIP | WIP | WIP | WIP |
| Wii Remote | WIP | WIP | WIP | WIP | WIP |

¹ Gated concepts, not production realization IDs. USB personality over UHID is a
virtual HID presentation; dummy_hcd is a separate optional gadget realization.
The Xbox 360 HID profiles are standard HID, not USB XInput. This table does not
claim hardware fidelity, full function parity, audio or composite USB support.

² The implemented Linux f_hid interface exposes report IDs without full GET_REPORT
metadata or an explicit negative reply. All creation entrypoints reject this
transport before host construction; permissions or module preparation cannot
enable it. Existing protocol prototypes remain historical/test evidence.

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

The [active pre-alpha ledger](architecture-overhaul/PRE_ALPHA_STATUS.md) requires
API stress, controller/demo refinement, hands-on feedback, final API review,
separate quality review and then a Git-based alpha. Normal root constructors
exclude gadget realization while Gate G remains unresolved; its implementation
and evidence stay available through experimental SPI. No WIP cell is promoted.
See the [current function review](CURRENT_CONTROLLER_REFINEMENT.md).

Production Wii Remote remains positively gated on later maintainer authorization.
The test-only Wii-like topology experiment does not enable a controller package,
demo entry, Bluetooth personality, or any support cell.


## Audio evidence axes

| Path | Implemented | Deterministic | Host/live | Consumer | Physical matching |
| --- | --- | --- | --- | --- | --- |
| UHID + emulated PipeWire | Yes, opt-in | Queue/profile/policy/lifecycle | Selected short and recorded longer probes; mixed historical failures retained | Native continuity/duplex qualification unresolved; #112 is closed with incomplete acceptance | Unavailable (#116) |
| USB/IP HID/UAC2 | Yes, opt-in WIP | Framing/personality/IPC/ownership/rollback | Selected installed sample/lifecycle trials | Native continuity/full latency matrix unresolved; installed security/recovery open (#115) | Unavailable; UAC2 differs from reference UAC1 |

No support cell is promoted by compilation, issue closure or API signature freeze.
The holistic review found defects; remediation and remaining live, security,
consumer and physical acceptance are the current release gate.
