# Controller device-data audit (Alpha 2 P10a)

**Scope.** This is a source-only audit of curated realizations; it is not a physical-device conformance finding. The audited default is synthetic. **Identity** must vary for independently created sessions and permit caller restoration where exposed; **inert factory data** (calibration, firmware and descriptive capability constants) does not need variation. Caller-supplied bytes are read at runtime and must never be added to either public repository.

A transport-only `physical_path` suffix is not a controller-reported identity. Only UHID has a `uniq` field in this set; uinput does not expose that field. UHID Bluetooth is a locally synthesized kernel HID device, not discoverable radio pairing.

## Emitted data by field and realization

| Family | Field / realization | Where exposed | Synthetic default; per-instance? | Caller override / restoration | File import |
| --- | --- | --- | --- | --- | --- |
| DualSense | Pairing identity, USB UHID | Feature `0x09`, `uniq` | Generated locally; **yes** | `DualSenseIdentity::from_bytes`, `DualSenseOptions::with_identity` | No direct loader; caller can restore bytes at runtime |
| DualSense | Capability constants, USB UHID | Feature `0x03` | Synthetic constant; **not required** | None | No |
| DualSense | IMU calibration, USB UHID | Feature `0x05` | Synthetic constant; **not required** | None; P10b | Planned P10b |
| DualSense | Firmware/hardware version fields, USB UHID | Feature `0x20` | Synthetic constant; **not required** | None; P10b | Planned P10b |
| DualShock 4 | Pairing identity, USB UHID | Features `0x12`/`0x81`, `uniq` | Generated locally; **yes** | `DualShock4Identity::from_bytes`, `DualShock4Options::with_identity` | No direct loader; caller can restore bytes at runtime |
| DualShock 4 | Calibration, USB UHID | Feature `0x02` | Synthetic constant; **not required** | None; P10b | Planned P10b |
| DualShock 4 | Firmware and hardware constants, USB UHID | Feature `0xA3` | Synthetic constant; **not required** | None; P10b | Planned P10b |
| Switch Pro | Controller identity, USB UHID | USB status `0x80/0x01`, subcommand `0x02`, `uniq` | Generated locally; **yes** | `SwitchProIdentity::from_bytes`, `SwitchProOptions::with_identity` | No direct loader; caller can restore bytes at runtime |
| Switch Pro | Factory IMU and stick calibration, USB UHID | SPI read subcommand `0x10` | Synthetic constant; **not required** | None; P10c | Planned P10c |
| Switch Pro | User calibration, colours and serial region, USB UHID | SPI read subcommand `0x10` | Unpopulated synthetic response; **not required** | None; P10c | Planned P10c |
| Xbox 360 | Device-reported serial, uinput | **Not emitted** (ordinary evdev keys/axes/FF only) | Not applicable | No identity API needed | No |
| Xbox 360 | Transport path, uinput | uinput `phys` label | Per-instance suffix; **yes** (not a reported serial) | No override; `Xbox360Options` empty | No |
| Xbox Series | Bluetooth identity, Bluetooth UHID | `uniq` only; no serial feature in declared report map | Locally generated; **yes** | `XboxSeriesIdentity::from_bytes`, `XboxSeriesOptions::with_identity`, handle `identity()` | No direct loader; caller restores bytes at runtime |
| Xbox Series | Bluetooth feature-reported serial | **Not emitted** | Not applicable | No extra feature API | No |
| Xbox Series | Transport path, uinput | uinput `phys` label; **no `uniq`** | Per-instance suffix; **yes** (transport only) | Identity stored on handle but not emitted | No |
| Steam Controller 2026 | Wired serial, USB UHID | Native serial reply and `uniq` | Generated synthetic serial; **yes** | `SteamController2026Identity::from_serial`, `SteamController2026Options::with_identity` | No direct loader; caller supplies string at runtime |
| Steam Controller 2026 | Steam client attributes/settings | Native `0x83` compatibility profile | Synthetic/source-derived compatibility values; **not required** | `with_steam_client_compat` toggles profile, not arbitrary bytes | No |
| Wii Remote | Bluetooth identity, Bluetooth UHID | `uniq` and device-facing identity | Generated locally; **yes** | `WiiRemoteIdentity::from_bytes`, `WiiRemoteOptions::with_identity` | No direct loader; caller restores bytes at runtime |
| Wii Remote | EEPROM extent / custom calibration | Memory read/write and extension blocks | Empty by default; **not required** | `WiiEepromImage::import`, `WiiRemoteOptions::with_eeprom_image` | **Yes**, caller reads at runtime; demo Save/Load, never commit files |

Families with uinput targets expose per-session path suffixes but **not** device calibration, firmware reports or identity features. A caller-restored identity on the controller handle does not imply uinput serial presentation. USB composite/dummy-HCD paths have separately owned protocol resources; this matrix is not evidence of physical identity replay on those targets.

## Conformance and outstanding work

`crates/gr-curated-controllers/tests/device_data.rs` covers synthetically generated and deterministic restored identities without hardware. Existing in-crate fake-provider/protocol tests exercise Sony feature response shape and transport labels. The audited default constant fields are intentional, not a failure of uniqueness. Missing **caller-provided** non-identity blob tests are explicitly ignored pending P10b (Sony) and P10c (Switch Pro); do not mistake ignored tests for validation. Full two-instance host-level report and restore coverage remains to be completed in those lanes.

The guard `scripts/check-private-device-data.py` inspects tracked/indexed Git blobs, never the working tree, and rejects private paths and likely dump shapes. The legacy Wii entry point delegates to it. Name/size/shape heuristics are defense in depth, **not** a proof that a renamed or transformed dump will be detected. Keep physical data outside the public repository regardless of scanner result.

## Pinned source basis

- [Xbox Series xpad identity and evdev evidence](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/xbox-series-xpad-identity-share.json) and [Bluetooth unknowns](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/xbox-series-bluetooth-device-gaps.json). Neither establishes an active vendor serial feature; [SDL's serial request is disabled](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/records/claim/xbox-series-sdl-disabled-serial-request.json).
- [Existing synthetic Wii EEPROM contract](https://github.com/anderstvoss/virtualgamepad/blob/6c795401b4d4cc014a59087896e629af78044f62/crates/gr-curated-controllers/src/wii_remote/memory.rs) and [original private-data guard](https://github.com/anderstvoss/virtualgamepad/blob/6c795401b4d4cc014a59087896e629af78044f62/scripts/check-private-wii-eeprom.py).
