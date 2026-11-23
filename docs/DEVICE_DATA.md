# Controller device-data audit (Alpha 2 P10a)

**Scope.** This is a source-only audit of curated realizations; it is not a physical-device conformance finding. The audited default is synthetic. **Identity** must vary for independently created sessions and permit caller restoration where exposed; **inert factory data** (calibration, firmware and descriptive capability constants) does not need variation. Caller-supplied bytes are read at runtime and must never be added to either public repository.

A transport-only `physical_path` suffix is not a controller-reported identity. Only UHID has a `uniq` field in this set; uinput does not expose that field. UHID Bluetooth is a locally synthesized kernel HID device, not discoverable radio pairing.

## Emitted data by field and realization

| Family | Field / realization | Where exposed | Synthetic default; per-instance? | Caller override / restoration | File import |
| --- | --- | --- | --- | --- | --- |
| DualSense | Pairing identity, USB UHID | Feature `0x09`, `uniq` | Generated locally; **yes** | `DualSenseIdentity::from_bytes`, `DualSenseOptions::with_identity` | No direct loader; caller can restore bytes at runtime |
| DualSense | Capability constants, USB UHID | Feature `0x03` | Synthetic constant; **not required** | None | No |
| DualSense | IMU calibration, USB UHID | Feature `0x05` | Synthetic constant; **not required** | `DualSenseOptions::with_device_data` (P10b) | Yes, caller-runtime only |
| DualSense | Firmware/hardware version fields, USB UHID | Feature `0x20` | Synthetic constant; **not required** | `DualSenseOptions::with_device_data` (P10b) | Yes, caller-runtime only |
| DualShock 4 | Pairing identity, USB UHID | Features `0x12`/`0x81`, `uniq` | Generated locally; **yes** | `DualShock4Identity::from_bytes`, `DualShock4Options::with_identity` | No direct loader; caller can restore bytes at runtime |
| DualShock 4 | Calibration, USB UHID | Feature `0x02` | Synthetic constant; **not required** | `DualShock4Options::with_device_data` (P10b) | Yes, caller-runtime only |
| DualShock 4 | Firmware and hardware constants, USB UHID | Feature `0xA3` | Synthetic constant; **not required** | `DualShock4Options::with_device_data` (P10b) | Yes, caller-runtime only |
| Switch Pro | Controller identity, USB UHID | USB status `0x80/0x01`, subcommand `0x02`, `uniq` | Generated locally; **yes** | `SwitchProIdentity::from_bytes`, `SwitchProOptions::with_identity` | No direct loader; caller can restore bytes at runtime |
| Switch Pro | Factory IMU and stick calibration, USB UHID | SPI read subcommand `0x10` | Synthetic constant; **not required** | None; P10c | Planned P10c |
| Switch Pro | User calibration, colours and serial region, USB UHID | SPI read subcommand `0x10` | Unpopulated synthetic response; **not required** | None; P10c | Planned P10c |
| Xbox 360 | Device-reported serial, uinput | **Not emitted** (ordinary evdev keys/axes/FF only) | Not applicable | No identity API needed | No |
| Xbox 360 | Transport path, uinput | uinput `phys` label | Per-instance suffix; **yes** (not a reported serial) | No override; `Xbox360Options` empty | No |
| Xbox 360 | Standard gamepad presentation, USB UHID | UHID `uniq` and `phys` transport labels; no serial feature | Per-instance suffix; **yes** (transport label, not controller serial) | No override; `Xbox360Options` empty | No |
| Xbox Series | Bluetooth identity, Bluetooth UHID | `uniq` only; no serial feature in declared report map | Locally generated; **yes** | `XboxSeriesIdentity::from_bytes`, `XboxSeriesOptions::with_identity`, handle `identity()` | No direct loader; caller restores bytes at runtime |
| Xbox Series | Bluetooth feature-reported serial | **Not emitted** | Not applicable | No extra feature API | No |
| Xbox Series | Transport path, uinput | uinput `phys` label; **no `uniq`** | Per-instance suffix; **yes** (transport only) | Identity stored on handle but not emitted | No |
| Steam Controller 2026 | Wired serial, USB UHID | Native serial reply and `uniq` | Generated synthetic serial; **yes** | `SteamController2026Identity::from_serial`, `SteamController2026Options::with_identity` | No direct loader; caller supplies string at runtime |
| Steam Controller 2026 | Steam client attributes/settings | Native `0x83` compatibility profile | Synthetic/source-derived compatibility values; **not required** | `with_steam_client_compat` toggles profile, not arbitrary bytes | No |
| Wii Remote | Bluetooth identity, Bluetooth UHID | Kernel `uniq` in display order | Generated locally; **yes** | `WiiRemoteIdentity::from_bytes`, `WiiRemoteOptions::with_identity` | No direct loader; caller restores bytes at runtime |
| Wii Remote | EEPROM extent / custom calibration | Memory read/write and extension blocks | Empty by default; **not required** | `WiiEepromImage::import`, `WiiRemoteOptions::with_eeprom_image` | **Yes**, caller reads at runtime; demo Save/Load, never commit files |

The Xbox 360 USB UHID presentation uses the project-standard gamepad descriptor rather than native Xbox identity; its `uniq` is not a controller-reported serial. Families with uinput targets expose per-session path suffixes but **not** device calibration, firmware reports or identity features. A caller-restored identity on the controller handle does not imply uinput serial presentation. USB composite/dummy-HCD paths have separately owned protocol resources; this matrix is not evidence of physical identity replay on those targets.

## Conformance and outstanding work

`crates/gr-curated-controllers/tests/device_data.rs` covers synthetically generated and deterministic restored identities without hardware. Existing in-crate fake-provider/protocol tests exercise Sony feature response shape and transport labels. The audited default constant fields are intentional, not a failure of uniqueness. P10b adds unignored pure synthetic tests for caller-selected Sony feature reports, round-trip restoration and typed USB/IP refusal. Mainline audit tests for non-identity blobs may remain ignored until lanes are integrated. P10c (Switch Pro) and full two-instance host-level report and restore coverage remain outstanding; do not mistake ignored tests for validation.

The guard `scripts/check-private-device-data.py` inspects tracked/indexed Git blobs, never the working tree, and rejects private paths and likely dump shapes. The legacy Wii entry point delegates to it. Name/size/shape heuristics are defense in depth, **not** a proof that a renamed or transformed dump will be detected. Keep physical data outside the public repository regardless of scanner result.

## Pinned source basis

- [Xbox Series xpad identity and evdev evidence](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/xbox-series-xpad-identity-share.json) and [Bluetooth unknowns](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/xbox-series-bluetooth-device-gaps.json). Neither establishes an active vendor serial feature; [SDL's serial request is disabled](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/xbox-series-sdl-disabled-serial-request.json).
- [Existing synthetic Wii EEPROM contract](https://github.com/anderstvoss/virtualgamepad/blob/6c795401b4d4cc014a59087896e629af78044f62/crates/gr-curated-controllers/src/wii_remote/memory.rs) and [original private-data guard](https://github.com/anderstvoss/virtualgamepad/blob/6c795401b4d4cc014a59087896e629af78044f62/scripts/check-private-wii-eeprom.py).

## Sony feature device data (P10b)

P10b augments the `DualSense` and `DualShock4` families with typed
**per-instance** USB feature data. No physical dump is shipped or needed.
Defaults are synthetic, deterministic and byte-identical to the prior
compiled calibration and firmware responses.

| Family | Calibration report | Firmware report | Pairing source |
| --- | --- | --- | --- |
| DualSense | USB 0x05, 41 bytes | USB 0x20, 64 bytes | `DualSenseIdentity` |
| DualShock 4 | USB 0x02, 37 bytes | USB 0xA3, 49 bytes | `DualShock4Identity` |

### Application API

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
**Known gap — `linux.usbip.usb-audio`:** the audio/USB/IP worker still
serves only the existing synthetic feature data. Supplying even a
validated `DualSenseDeviceData` or `DualShock4DeviceData` value
explicitly through `with_device_data(...)` returns
`ControllerError::Unsupported` **before opening the worker**. To retain
synthetic USB/IP behavior, omit `with_device_data` entirely. This
avoids changing the installed worker's wire protocol; a compatible
broker/audio-workstream design must close the gap separately.
No caller-selected report bytes are sent in worker frames or executable
arguments in this PR. Supplying a pair cannot partially update a live
controller. The current Sony Bluetooth-bus realization is not on
the checked `main`; this API accepts **numbered USB reports only**.
Bluetooth forms (including CRC-bearing variants) are rejected; no
claim of Bluetooth feature-report support is made here.

### Caller-selected file format

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

### Validation limits

Pure synthetic test vectors exercise USB GET defaults versus pinned
compiled fixture construction, supplied GET responses, invalid report
rejection, round-trip serialization, re-creation from a retained
value, and typed USB/IP refusal without opening a worker. No host, hardware capture, Bluetooth, or USB/IP timing run is
asserted; actual host protocol acceptance remains unqualified.

Related tracking: vg #154, #155, #162, #365.
