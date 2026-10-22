# Wii Remote C5 package

The compiled supporting package `gr-curated-controllers::wii_remote` provides
`linux.uinput` and `linux.uhid.bluetooth`. The ordinary-root API is unchanged.
This is a source-backed compatibility implementation under assessment, with the
per-feature dispositions in
[`acceptance/virtualgamepad.wii-remote.toml`](../acceptance/virtualgamepad.wii-remote.toml).
It is not physical-reference or whole-controller acceptance.

## Native ownership and use

Use the supporting `create_wii_remote(target, session_id)` constructor for an
Original Remote, or `create_wii_remote_with_image` for an explicit revision and
validated `WiiEepromImage`. Original and TR identities are distinct. Early
integrated Plus and undeclared targets reject; there is no realization fallback.
The ordinary `virtualgamepad::RealizationId` is the same underlying target type.
Applications using the supporting package must pin it with the rest of this
workspace; it is outside the ordinary-root compatibility snapshot.

The handle exposes native buttons, ten-bit accelerometer inputs, four optional
IR points, Nunchuk/Classic inputs, and fourteen-bit MotionPlus inputs. `attach`
changes the compiled live subset; `current_topology` and `topology_generation`
report it. Same-kind input edits preserve initialization. Extension replacement
emits a disconnect/reconnect status sequence so the kernel re-probes its ID.
Inserting or removing MotionPlus while a downstream extension stays attached
also emits both edges, because it resets that extension's initialization.
Host Stop retains attachment edits without queuing notices or arming deadlines;
Start reports the current status once, without replaying stopped edits.
Native uinput companions prepare before installation; a failed prepare leaves
the old attachment intact. Required host requests are handled before optional
observations. Call `service` while inputs are idle, and recompute
`next_service_in` after edits, commits and service. No measured firmware cadence
is invented.

`host_outputs` returns retained accepted intent, independent of observation loss.
On uinput only conventional rumble has an output channel; the two conventional
magnitudes collapse to the one native binary motor. Four LED bits are meaningful
on the Bluetooth-bus HID path. These values do not prove physical output delivery.

EEPROM is an opaque caller-imported value beginning at zero. Its caller-chosen
extent is a model boundary, capped by the source-backed sixteen-bit address
window; it is not a retail-size or factory-calibration claim. The HID host can
read/write this image, with exact negative replies for invalid spans. The image
has sticky resource dirtiness separate from input dirtiness. After explicit or
error close, choose `KeepSnapshot` or `Discard`; the first choice wins. Drop,
consumer close and host lifecycle events do not export anything. The library
chooses no file path and claims no filesystem durability. Uinput has no addressed
memory channel and rejects a nonempty import.

The demo creates Wii handles and provides native controls, extension selection
and typed IR editing. Advanced raw battery and EEPROM configuration use the
supporting API. Interactive desktop/game acceptance remains untested.

## Realization limits

- Uinput follows the six `hid-wiimote` interface names and capability sets: core,
  accelerometer, IR, Nunchuk, Classic and MotionPlus. The provider advertises
  USB bus3 and cannot express the kernel consumer's nonzero fuzz. Minimum,
  maximum and flat values match the extracted source table. Decoder values can
  exceed the advertised ABS bounds; they are preserved, including the missing
  IR1023/1023 sentinel. There is no explicit evdev point-visibility/blob field.
- Wii LED-class and `power_supply` endpoints belong to the HID kernel driver;
  plain uinput cannot provide them. Its target surface states this limitation.
- Bluetooth-bus UHID uses Nintendo identity and an explicitly synthetic
  vendor-page functional descriptor with source-backed report sizes. It is not
  a retail descriptor. Bus5 is local kernel metadata: no radio, pairing or BlueZ.
- Classic Pro wire identity, early Plus discrimination, encrypted extension
  workflows, factory calibration/default registers, retail EEPROM size and
  visible extended/full IR blob metadata remain **No evidence**. Basic point
  coordinates and no-point data shapes are implemented; unsupported visible
  blobs reject rather than silently disappear.
- Speaker support remains a later separately gated realization under D7 and
  ADR-0023. USB-bus UHID is a fixture-only experiment, outside this production
  package. Actual Bluetooth remains deferred to #164.

## Bounded host evidence

On Linux6.12.113+deb13-arm64 aarch64, the ignored host probe at
`ccb98dc1a4fd384a20bf0c7015fd0115c006798e` observed the exact owned UHID object bound
to `wiimote`. All six interface names and key/axis masks matched the independently
extracted golden. Nunchuk attachment, Nunchuk-to-Classic replacement and standalone
MotionPlus appeared. A `power_supply` endpoint existed. Kernel evdev rumble
upload/play/stop/erase succeeded and reached retained native intent. The owned
object disappeared after idempotent close.

All four LED brightness writes failed with EACCES. Live numeric absinfo ranges,
fuzz/resolution, sensor delivery, IR tracking, calibrated motion, SDL's HIDAPI Wii
driver and Dolphin were not measured. Kernel initialization setting an LED is
not a successful user LED-control test. These missing cells remain explicit.

The source and virtual-host records are in
[research PR17](https://github.com/anderstvoss/controller-protocol-research/pull/17):
`wii-c5-bt-host-binding`, `wii-c5-evdev-native-values` and the four
`wii-c5-*-host-run` experiments. The acceptance manifest pins their reviewed
research revision. No fixture is a physical capture.

## Creation and removal timing

Ten cycles were recorded at N1 and N4 for each realization, using the same
hashed executable built from tracked-clean `ccb98dc`, for100 total samples.
The first cycle is labelled cold; subsequent cycles warm. N sessions coexist
before removal. All operation error lists were empty, owned nodes disappeared,
and fd/thread/input/hidraw counts returned to baseline for all four runs.
Consumer-open permission failures, the missing first-valid-neutral oracle and
unrun SDL discovery remain in the records.

| Realization | N | Warm samples | API return p50 ms | Close return p50 ms | Close return max ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| uinput | 1 | 9 | 0.463 | 133.010 | 157.876 |
| uinput | 4 | 36 | 1.478 | 140.902 | 185.018 |
| Bluetooth-bus UHID | 1 | 9 | 0.152 | 8277.520 | 9364.405 |
| Bluetooth-bus UHID | 4 | 36 | 0.243 | 4130.361 | 8300.427 |

These are historical distributions for the pinned implementation, without a
latency budget or a final-head performance claim. The later native-value parity
fix changes emitted values; the measurements are not silently relabelled. Early
kernel initialization can make UHID removal take seconds. Peer servicing and
initialization state differ between N1 and N4, so these numbers do not establish
parallel throughput or a general performance advantage. The longer functional
probe services initialization before close; it has a different timing scope.

Reproduce the structural checks and bounded probe with:

```bash
cargo test --locked -p gr-curated-controllers wii_remote
cargo test --locked -p gr-curated-controllers --test acceptance_manifests
cargo test --locked -p gr-curated-controllers --lib bluetooth_host_binding_extensions_outputs_and_battery_probe -- --ignored --nocapture
cargo run --locked -p xtask -- acceptance --controller virtualgamepad.wii-remote --realization linux.uinput --cycles 10 --concurrency 1 --out /tmp/wii-uinput-n1.json
```

Use `--concurrency 4` for N4 and `--realization linux.uhid.bluetooth` for the
Bluetooth-bus case. The host probe requires appropriate UHID/event permissions
and a C compiler; it never changes permissions or module policy. Generic timing
runs currently commit the Wii neutral state; native press/release behavior has
separate deterministic regressions and is not a first-neutral delivery oracle.
