# Controller support matrix

Current dispositions and known limitations are on the [status page](STATUS.md).
Emulated audio is opt-in WIP; see the [audio guide](CONTROLLER_AUDIO.md).

Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.

All compiled family realizations remain **WIP**; bounded alpha acceptance does
not promote any cell. The table distinguishes declared targets from undeclared
or deferred targets, and declared targets that refuse creation. A WIP support
category does not promise that every native function or future target will ship.

| Controller | Linux uinput / evdev | USB personality over Linux UHID | USB HID through dummy_hcd | Bluetooth personality over UHID¹ | USB/IP HID/audio (opt-in) | Actual Bluetooth¹ |
| --- | --- | --- | --- | --- | --- | --- |
| DualSense | WIP | WIP | Declared, refused² | Undeclared | WIP | Deferred |
| DualShock 4 | WIP | WIP | Declared, refused² | Undeclared | WIP | Deferred |
| Switch Pro | WIP | WIP | Declared, refused² | Undeclared | Undeclared | Deferred |
| Xbox 360 | WIP | WIP | Declared, refused² | Undeclared | WIP | Deferred |
| Steam Controller (2026), wired | WIP (reduced) | WIP | Undeclared | Undeclared | Undeclared | Deferred |
| Xbox Series | WIP (reduced)⁴ | Undeclared | Undeclared | WIP⁴ | Undeclared | Deferred |
| Wii Remote | WIP³ | Undeclared (fixture only)³ | Undeclared³ | WIP³ | Undeclared | Deferred |

Deferred receiver concept (not a compiled controller family):

| Receiver | Production realization | Disposition |
| --- | --- | --- |
| Steam Puck / receiver | Undeclared | Deferred; separate receiver lifecycle work (#159, #165), not part of wired Steam Controller 2026 |


¹ `linux.uhid.bluetooth` is a local UHID node with Bluetooth bus5 metadata,
available only where a compiled family manifest declares it. It provides no
radio, pairing or BlueZ. Actual Bluetooth remains deferred to #164. USB
personality over UHID is a virtual HID presentation; dummy_hcd is separate.
The Xbox 360 HID profiles are standard HID, not USB XInput. This table does not
claim hardware fidelity or full function parity. USB/IP composite HID/UAC2
and UHID/PipeWire audio are opt-in WIP only for DualSense, DualShock 4 and
Xbox 360; see the audio guide for scope.

² The implemented Linux f_hid interface exposes report IDs without full GET_REPORT
metadata or an explicit negative reply. All creation entrypoints reject this
transport before host construction; permissions or module preparation cannot
enable it. Existing protocol prototypes remain historical/test evidence.

## Application input API

Every realized family exposes native `<Family>Inputs`, borrowed `inputs()`,
copyable `<Family>Status` through `status()`, and transactional `set_inputs`,
`edit_inputs`, targeted setters and `neutralize()`. This naming does not change
any support classification below. See [the final migration table](ALPHA_API_MIGRATION.md#alpha1--alpha2-final-consumer-table)
and [ADR-0025](adr/ADR-0025-input-model.md).

## Status definitions and promotion

- **N/A:** explicitly not planned for this controller/realization, with a reason.
  Use this after scope review, for example for protocols outside a controller's
  intended generation. Missing hardware alone is not N/A.
- **WIP:** incomplete, unstarted, blocked or awaiting support review. In the
  realization inventory above, WIP denotes a declared, implemented path,
  not full function acceptance.
- **Undeclared:** no compiled production realization for this family/target.
- **Deferred:** future work, not a shipping realization.
- **Declared, refused:** listed in the compiled definition but creation is
  explicitly rejected; not a usable realization.
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

³ D6/D7 authorize the Wii supporting package on uinput and Bluetooth-bus UHID.
USB-bus UHID remains a fixture-only experiment, dummy_hcd is not declared, and
actual Bluetooth is deferred. See the [Wii package and qualification](WII_REMOTE.md)
and its [feature matrix](../acceptance/virtualgamepad.wii-remote.toml).
Source-backed code and bounded virtual-host observations do not promote missing
physical, SDL HIDAPI or Dolphin acceptance cells.


⁴ Xbox Series has [xpad-style uinput evidence](XBOX_SERIES.md#c9-linux-and-sdl-acceptance)
and a [per-function manifest](../acceptance/virtualgamepad.xbox-series.toml).
The [Bluetooth UHID checkpoint](XBOX_SERIES_CHECKPOINT.md#integrated-bluetooth-uhid-acceptance) records
Linux and SDL evidence. Actual Bluetooth is #164. Wired GIP stays with #168.

## Audio evidence axes

| Path | Implemented | Deterministic | Host/live | Consumer | Physical matching |
| --- | --- | --- | --- | --- | --- |
| UHID + emulated PipeWire | Yes, opt-in | Queue/profile/policy/lifecycle | Selected short and recorded longer probes; mixed historical failures retained | Native continuity/duplex qualification unresolved; #112 is closed with incomplete acceptance | Unavailable (#116) |
| USB/IP HID/UAC2 | Yes, opt-in WIP | Framing/personality/IPC/ownership/rollback | Selected installed sample/lifecycle trials | Native continuity/full latency matrix deferred (#135/#145); seven scoped broker security/recovery phases passed (#115 closed) | Unavailable; UAC2 differs from reference UAC1 |

No support cell is promoted by compilation, issue closure or API signature freeze.
