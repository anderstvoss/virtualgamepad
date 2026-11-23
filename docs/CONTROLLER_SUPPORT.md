# Controller support matrix

Current dispositions and known limitations are on the [status page](STATUS.md).
Emulated audio is opt-in WIP; see the [audio guide](CONTROLLER_AUDIO.md).

Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.

The **family-level** status of every compiled, usable realization remains WIP.
An individual per-function manifest cell can separately be `source-evidence`,
`virtual-live` or `physical-evidence` without promoting the entire family.
The inventory below distinguishes compiled WIP targets, undeclared targets,
deferred transport work, and declared targets that reject creation.

## Application-visible realization levels

The level describes what the application receives, **not an acceptance verdict**.
The realization IDs remain stable. Individual families declare only a subset
of these realizations; experimental `dummy_hcd` is refused by normal root
constructors. Bluetooth-bus UHID is local HID, not radio discovery or pairing.

| Realization ID | Level | Mechanism | Permission | Application-visible result |
| --- | --- | --- | --- | --- |
| `linux.uinput` | Generic | uinput | needs uinput access | Generic evdev gamepad; reduced protocol fidelity |
| `linux.uhid.usb` | Native HID | UHID | needs UHID access | Native HID reports, USB bus metadata; driver/SDL/Steam integration |
| `linux.uhid.bluetooth` | Native HID | UHID | needs UHID access | Native HID reports, Bluetooth bus metadata but no physical radio |
| `linux.usbip.usb-audio` | Real USB | USB/IP-VHCI | needs root broker | Composite virtual USB interfaces, including declared audio |
| `linux.dummy_hcd.usb-hid` | Real USB | dummy_hcd | needs root broker | Experimental USB gadget path; currently refused |

**Real radio** is a reserved level, with no declared realization.

## Per-controller support dispositions

| Controller | `linux.uinput` (Generic) | `linux.uhid.usb` (Native HID) | `linux.uhid.bluetooth` (Native HID) | `linux.usbip.usb-audio` (Real USB) | `linux.dummy_hcd.usb-hid` (Real USB) | Actual Bluetooth |
| --- | --- | --- | --- | --- | --- | --- |
| DualSense | WIP | WIP | WIP | WIP, opt-in | Declared, refused² | Deferred |
| DualShock 4 | WIP | WIP | WIP | WIP, opt-in | Declared, refused² | Deferred |
| Switch Pro | WIP | WIP | WIP | Undeclared | Declared, refused² | Deferred |
| Xbox 360, standard HID (not XInput) | WIP | WIP | Undeclared | WIP, opt-in | Declared, refused² | Deferred |
| Steam Controller 2026, wired | WIP (reduced) | WIP | Undeclared | Undeclared | Undeclared | Deferred |
| Xbox Series | WIP (reduced)⁴ | Undeclared | WIP⁴ | Undeclared | Undeclared | Deferred |
| Wii Remote | WIP³ | Undeclared (fixture only)³ | WIP³ | Undeclared | Undeclared | Deferred |

The Steam Puck/receiver is a **deferred separate receiver concept**, not a
compiled Steam Controller 2026 realization. For all seven families, the exact
target inventory is `acceptance/virtualgamepad.<family>.toml:4`.

¹ `linux.uhid.bluetooth` is a local UHID node with Bluetooth bus5 metadata,
available only where a compiled family manifest declares it. It provides no
radio, pairing or BlueZ. DualSense, DualShock 4 and Switch Pro have compiled source-backed Bluetooth-bus
UHID realizations ([DualSense](DUALSENSE_BLUETOOTH_UHID.md),
[DS4](dualshock4-bluetooth.md), [Switch Pro](SWITCH_PRO_BLUETOOTH.md));
the bounded P5b host observations do not establish physical equivalence.
Actual Bluetooth remains deferred to #164. USB personality over UHID is
virtual HID, not a USB bus device; dummy_hcd is separate.
The Xbox 360 HID profiles are standard HID, not USB XInput. DualShock 4's
Bluetooth-bus UHID realization is described in the [DS4 implementation note](dualshock4-bluetooth.md);
the DS4 table cell remains WIP pending acceptance. This table does not claim physical equivalence or full native function parity.
USB/IP composite HID/UAC2 and UHID/PipeWire audio are opt-in and declared
only for DualSense, DualShock 4 and Xbox 360.

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
- **WIP:** a compiled and constructible family target still under review; not
  a physical-equivalence verdict. Individual functions use finer manifest grades.
- **Undeclared:** no compiled production target for this family/realization.
- **Deferred:** future transport/receiver work, not a shipping realization.
- **Declared, refused:** present in the manifest, but creation rejects before
  opening resources. Not a usable target.
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
and its [per-function matrix](../acceptance/virtualgamepad.wii-remote.toml).
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

TODO(alpha2-final): P8 family support-matrix promotion after P5/P6/P7 and P10d.
