# Sony feature device data (P10b)

P10b augments the `DualSense` and `DualShock4` families with typed
**per-instance** USB feature data. No physical dump is shipped or needed.
Defaults are synthetic, deterministic and byte-identical to the prior
compiled calibration and firmware responses.

| Family | Calibration report | Firmware report | Pairing source |
| --- | --- | --- | --- |
| DualSense | USB 0x05, 41 bytes | USB 0x20, 64 bytes | `DualSenseIdentity` |
| DualShock 4 | USB 0x02, 37 bytes | USB 0xA3, 49 bytes | `DualShock4Identity` |

## Application API

`DualSenseDeviceData::default()` and `DualShock4DeviceData::default()`
produce synthetic report pairs. To override, call
`DeviceData::from_feature_reports(calibration, firmware)`, or build a
copy with `with_feature_report(report_id, bytes)`; both reject invalid
report IDs and exact lengths and reject zero denominators used by motion
calibration. Construction is atomic and opens no provider resources.
`feature_report(report_id)` permits byte-for-byte read-back of the
validated USB report. Supply the result through
`DualSenseOptions::with_device_data(data)` or
`DualShock4Options::with_device_data(data)`, then call the ordinary
`create_dualsense` or `create_dualshock4` constructor.
`controller.device_data()` returns the value retained by that
instance even after closure.

The separate creation identity is always the source of address-bearing
feature reports. **Address-bearing feature reports are never accepted
in a device-data value**; callers must select `with_identity(...)`
on the family options to restore their independent identity.

USB/UHID accepts the selected immutable feature reports directly.
USB/IP carries their validated pair on the private worker control
channel and requires an acknowledgment *before* the worker starts
servicing USB requests. The privileged broker's executable arguments
carry no report bytes. Supplying a pair cannot partially update a live
controller. The current Sony Bluetooth-bus realization is not on
the checked `main`; this API accepts **numbered USB reports only**.
Bluetooth forms (including CRC-bearing variants) are rejected; no
claim of Bluetooth feature-report support is made here.

## Caller-selected file format

`load_file(path)` and `save_file(path)` use a bounded UTF-8 format:

1. Exact header `vg-sony-device-data-v1`.
2. One line with two hex digits identifying the calibration report,
   one ASCII space, and the contiguous lowercase hex encoding of the
   complete numbered USB report.
3. One line with the firmware report ID and complete numbered report
   in the same format.
4. Exactly one terminating line feed, no comments, no additional lines,
   no address report or identity entry.

The importer checks version, family, exact report order/length and
report IDs, then revalidates motion denominators before returning
a value. Wrong-family files and partial files are rejected without
mutating existing creation choices. Export preserves the supplied
calibration and firmware exactly. File extensions are conventionally
`.sonydata` and **are private runtime artifacts, never repository
fixtures**. Choose a path outside any public working tree. The demo's
advanced creation options expose Load, Save and Use synthetic defaults
for each Sony family and persist the validated choice across same-family
recreation. Saving defaults writes only the synthetic pair.

## Validation limits

Pure synthetic test vectors exercise USB GET defaults versus pinned
compiled fixture construction, supplied GET responses, invalid report
rejection, round-trip serialization, and re-creation from a retained
value. No host, hardware capture, Bluetooth, or USB/IP timing run is
asserted; actual host protocol acceptance remains unqualified.

Related tracking: vg #154, #155, #162, #365.
