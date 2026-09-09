# EXP-0015: compatibility sources and attached DualSense inspection

## Source review

OpenPuck revision `29f4d1e8b85c0a7c19e0c8b977a1a56a2c42ea6d` and HHD revision
`adbd50ff30614df886a6c76bd8855a64fde5d0b8` were inspected from their upstream Git
repositories. Corpus revision `e65c044d062ed833ad13641f54c826db16d176e4` was
reviewed, published remotely, then adopted. Source records pin file hashes and
locators; upstream code is not redistributed or executed by ordinary tests.

- [OpenPuck DS4 implementation](https://github.com/safijari/openpuck/blob/29f4d1e8b85c0a7c19e0c8b977a1a56a2c42ea6d/OpenPuck/mode_hidgyro.cpp)
  supplies input offsets, active touch layout and GET feature sizes. TinyUSB
  callback payload lengths exclude the report ID, unlike numbered UHID frames.
- [OpenPuck PlayStation helpers](https://github.com/safijari/openpuck/blob/29f4d1e8b85c0a7c19e0c8b977a1a56a2c42ea6d/OpenPuck/gamepad_util.cpp)
  supply contact packing and source mapping conventions.
- [HHD DualSense constants](https://github.com/hhd-dev/hhd/blob/adbd50ff30614df886a6c76bd8855a64fde5d0b8/src/hhd/controller/virtual/dualsense/const.py)
  describe two packed four-byte contacts and USB field placement.

New synthetic fixtures test a complete DS4 active input layout and a two-contact
packing example against our native encoders. They were assembled manually from
field facts, not captured from hardware or produced by the production encoders.
The contact example is within both adopted coordinate domains; it does not settle
controller-specific dimensions, sensor orientation or source normalization.

### Differences retained

OpenPuck emits zero touch samples with no active contacts. We retain the existing
explicit release sample and its regression; source compatibility is not proof that
a host consumes absent contact bytes. Its digital trigger threshold/source-button
policy is not our native trigger policy. HHD's rt/lt axis labels need interpretation
within its input-mapping layer before any trigger semantic adoption. Existing
nonzero-trigger regressions remain. OpenPuck cites Linux driver contracts for
feature sizes, so source agreement is not independent physical corroboration.
No DS4 hardware was available, and this source review does not promote its blocked
split evdev realization or close its physical-fidelity gate.

## Physical DualSense read-only evidence

The VM exposed one physical USB device with VID:PID `054c:0ce6` on kernel
`6.12.107+deb13-arm64`. Exact hidraw selection required a matching physical USB
ancestor; device-node major/minor was checked against sysfs after opening with
no-follow. Only this node was opened. No Xbox or Steam Controller appeared in
this VM's USB/input inventory at inspection time.

The user reports broken trigger modules and stick drift. Neither trigger response,
neutral stick accuracy nor adaptive-trigger actuation was tested or inferred.
No output reports, SET features, input grabs, driver rebinding, capture permission
changes or desktop/audio service changes were performed.

| Read-only observation | Result |
| --- | --- |
| HID descriptor | 273 bytes; raw descriptor and hash retained privately |
| Calibration GET `0x05` | 41 bytes including matching report ID |
| Pairing GET `0x09` | 20 bytes including matching report ID |
| Firmware GET `0x20` | 64 bytes including matching report ID |
| Passive input | 64 reports, each ID `0x01` and length 64 |
| Sensor timestamp field | 64 distinct, strictly increasing values |
| Cleanup | selected descriptor closed; no created nodes or child left running |

Feature queries used HIDIOCGFEATURE with the expected length, followed by bounded
nonblocking input reads (three-second maximum, 64 reports). The inspection process
had an outer 20-second timeout. Raw pairing/calibration contents and exact node
identity stay in private machine records, not corpus fixtures. This narrows the
physical B/P framing evidence only; it does not establish full consumer fidelity.

Read-only USB descriptors also expose one audio-control interface, two audio-stream
interfaces each with alternate settings 0 (no endpoints) and 1 (one endpoint), and
one HID interface. The audio OUT endpoint advertises address `0x01`, attributes 9,
maximum packet 392 and interval 4; audio IN advertises `0x82`, attributes 5,
maximum packet 196 and interval 4. HID endpoints are `0x84`/`0x03`, 64-byte maximum
packets and interval 6. These are observed descriptor fields, not inferred rates,
formats, audio operation or a Gate H pass. Audio class-specific format parsing and
sanitized provenance remain necessary before adopting a USB-audio profile.

## Program effect

Reuse the pinned layouts for routine regression coverage. Reserve further physical
experiments for unresolved calibration/descriptor differences, consumer behavior
and capabilities not settled by these sources. The damaged controller remains
useful for read-only protocol/topology work; repair or another reference is needed
for affected mechanical-fidelity claims. Source-backed DS4 work continues without
hardware. Gate G, isolated touch acceptance, physical outputs and Bluetooth remain
independently gated; none is promoted by this batch.
