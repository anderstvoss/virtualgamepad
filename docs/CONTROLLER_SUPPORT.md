# Controller support matrix

Current dispositions and known limitations are on the [status page](STATUS.md).
Emulated audio is opt-in WIP; see the [audio guide](CONTROLLER_AUDIO.md).

Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.

All support entries remain **WIP**; bounded alpha acceptance does not promote
these cells. WIP includes implemented work under assessment and work not yet
started. It does not
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
See the historical [gate ledger](history/alpha-1/GATE_STATUS.md) and
[acceptance evidence](history/alpha-1/CORE_ACCEPTANCE.md) for those results.

## Per-controller function matrices

Create a separate function matrix for each controller during acceptance or its
follow-up issue. Rows name native functions; columns name realizations using the
same statuses. Cover applicable controls, motion, touch, outputs, identity,
accessories and lifecycle. Link each accepted cell to evidence and technical
limitations. Do not mark a whole controller accepted because one function works,
or require an absent native function merely because another controller has it.

Normal root constructors reject dummy_hcd before resource creation. The
[historical function review](history/alpha-1/CURRENT_CONTROLLER_REFINEMENT.md)
records the Alpha 1 per-function assessment.

Production Wii Remote remains positively gated on later maintainer authorization.
The test-only Wii-like topology experiment does not enable a controller package,
demo entry, Bluetooth personality, or any support cell.


## Audio evidence axes

| Path | Implemented | Deterministic | Host/live | Consumer | Physical matching |
| --- | --- | --- | --- | --- | --- |
| UHID + emulated PipeWire | Yes, opt-in | Queue/profile/policy/lifecycle | Selected short and recorded longer probes; mixed historical failures retained | Native continuity/duplex qualification unresolved; #112 is closed with incomplete acceptance | Unavailable (#116) |
| USB/IP HID/UAC2 | Yes, opt-in WIP | Framing/personality/IPC/ownership/rollback | Selected installed sample/lifecycle trials | Native continuity/full latency matrix deferred (#135/#145); seven scoped broker security/recovery phases passed (#115 closed) | Unavailable; UAC2 differs from reference UAC1 |

No support cell is promoted by compilation, issue closure or API signature freeze.
