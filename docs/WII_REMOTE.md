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
observations. Wait on `readiness()` even while inputs are idle, also watching
writability when `wants_write()` is true. Call `service` on readiness or deadline;
use periodic service only for `Readiness::Poll`. The descriptor is borrowed until
close. `dropped_output_events()` counts optional observation loss, without changing
retained intent. Recompute
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

The supporting API also accepts immutable caller-imported extension calibration.
Use `WiiExtensionCalibration::import` with optional 32-byte MotionPlus and
16-byte Nunchuk/Classic blocks, then
`WiiEepromImage::with_extension_calibration` before creating the handle through
`create_wii_remote_with_image`. Import validates the MotionPlus CRC32 and the
two additive checksum bytes on Nunchuk/Classic. It does not repair bytes, infer
factory values, or certify physical calibration. The snapshot retains these
blocks alongside EEPROM; `bytes()` still returns only EEPROM bytes. No imports
are selected by default. Missing blocks retain the exact error 7 response.

Reads at extension offset `0x20` use the addressed device. An inactive MP answers
at A6; an active MP answers at A4 in modes `04`, `05` and `07`. Inactive MP
permits an initialized downstream Nunchuk/Classic to answer at A4. This follows
`wii-gap-dolphin-emulated-motionplus-modes-passthrough` and the existing identity
routing. Imported MP data is returned in at most 16-byte chunks; a request
straddling any block fails whole before a prefix is sent. No downstream copy at
`0x40`, Nunchuk/Classic calibration mirror at `0x30`, encrypted workflow or Classic Pro resource
is inferred. Host writes cannot mutate calibration. Attachment changes reset
register initialization but preserve the caller's imported resource.

Layouts and checksums cite `wii-gap-dolphin-motionplus-crc32-calibration`,
`wii-gap-dolphin-nunchuk-calibration-layout`,
`wii-gap-dolphin-classic-calibration-layout` and
`wii-gap-dolphin-calibration-additive-checksums`. Uinput rejects calibration
imports before opening a provider because it has no addressed memory channel.
The ordinary-root API remains unchanged; `scripts/check-alpha-api.py` checks a
separate Wii supporting-API snapshot for these additions. Consumer acceptance
and physical calibration remain separate gates.

Speaker control is acknowledged without producing audio. Reports `0x14` and
`0x19` retain enable/mute intent, apply the shared rumble bit, and send the
standard `0x22` error 0 ACK only when bit 1 requests it. The enable flag appears
in status; status requests preserve both controls. Writes through `0x16` to
A20000–A20009 store bounded control-register bytes and receive an ACK. Reads
retain error 7: the cited register records establish writes but do not decide
the read contract here. Out-of-block and straddling writes keep error 7 without
storing a prefix. These controls do not create an audio stream or audio endpoint.

For `0x18`, the fixed report is admitted only to preserve its shared rumble bit.
The audio area is discarded, including oversized encoded lengths, without an
ACK, decoder, queue, mixer, PipeWire sink or playback. Transport success is not
speaker-audio acceptance. This is an explicit deviation while playback remains
gated by D7 and audio phase A3. No A3 work is included. Uinput has no HID output
report or addressed register channel; its existing native rumble is unchanged.
Evidence: `wii-gap-wiibrew-speaker-control-reports`,
`audio-wii-dolphin-control-and-stream-reports`,
`wii-gap-wiibrew-speaker-init-config-registers`, and
`audio-wii-dolphin-speaker-register-model`. The speaker-enabled status flag
`0x04` cites `wii-gap-wiibrew-status-20-resend`; it retains accepted enable
intent and does not establish audio playback. No public API is added.

The demo creates Wii handles and provides native controls, extension selection
and typed IR editing. Advanced raw battery and EEPROM configuration use the
supporting API. Detached editors can use `WiiRemoteState::set_attachment` to
project a queued transition before same-frame native edits. This changes only
the snapshot; live nodes still require `WiiRemoteController::attach`. Wii
association and provider diagnostics are explicitly unavailable in the demo
ordinary-root lab view. Interactive desktop/game acceptance remains untested.

## Realization limits

- Uinput follows the six `hid-wiimote` interface names and capability sets: core,
  accelerometer, IR, Nunchuk, Classic and MotionPlus. The provider advertises
  USB bus 3 and cannot express the kernel consumer's nonzero fuzz. Minimum,
  maximum and flat values match the extracted source table. Decoder values can
  exceed the advertised ABS bounds; they are preserved, including the missing
  IR 1023/1023 sentinel. There is no explicit evdev point-visibility/blob field.
- Wii LED-class and `power_supply` endpoints belong to the HID kernel driver;
  plain uinput cannot provide them. Its target surface states this limitation.
- Bluetooth-bus UHID uses Nintendo identity and an authored interim synthetic
  Generic Desktop/Game Pad collection with vendor-page fields and source-backed
  report sizes (W2 option B; retail-evidence successor #238). It is not a retail
  descriptor. Bus 5 is local kernel metadata: no radio, pairing or BlueZ.
- Classic Pro wire identity, early Plus discrimination, encrypted extension
  workflows, factory calibration/default registers, retail EEPROM size and
  visible extended/full IR blob metadata remain **No evidence**. Basic point
  coordinates and no-point data shapes are implemented; unsupported visible
  blobs reject rather than silently disappear.
- Speaker support remains a later separately gated realization under D7 and
  ADR-0023. USB-bus UHID is a fixture-only experiment, outside this production
  package. Actual Bluetooth remains deferred to #164.

## Bounded host evidence

On Linux 6.12.113+deb13-arm64 aarch64, the ignored host probe at
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
[research PR 17](https://github.com/anderstvoss/controller-protocol-research/pull/17):
`wii-c5-bt-host-binding`, `wii-c5-evdev-native-values` and the four
`wii-c5-*-host-run` experiments. The acceptance manifest pins their reviewed
research revision. No fixture is a physical capture.

The C5 review fixes were checked on the same Linux 6.12 host at tracked-clean
`1b431cd5eabdf55c5c84091311c15f8105a25c87`. The unchanged ignored probe passed
once, including kernel rumble and owned-node cleanup; all four LED writes remained
permission-denied. A separate temporary supporting-API harness observed Nunchuk,
then both Nunchuk and MotionPlus after insertion, then Nunchuk without MotionPlus
after removal, and disappearance of the owned HID object on close. This bounded
interface observation does not establish sensor or application acceptance. No
research record or historical timing sample was changed.

## W1 bounded follow-up observations

[Part A PR 235](https://github.com/anderstvoss/virtualgamepad/pull/235) is the
implementation tested at clean `50161b913d0f82950c292465625e9cdd8854daf2`.
[Research PR 22](https://github.com/anderstvoss/controller-protocol-research/pull/22)
retains five original `wii-w1-*` experiments, structured fixtures, executable and
source hashes, and the original bounded probes under `tools/wii_w1`. The current
manifest pins research main `732ca43c397f710e1632d825dbc03ccaf5637ad9`, which
retains the W1 records through merged research PR 22. The earlier C5 paragraphs and timing table
remain historical observations of their own exact revisions.

On the same Linux 6.12.113+deb13-arm64 aarch64 host, the W1 probes opened all six
owned interface roles for each realization. Live minimum, maximum, flat and
resolution matched the source table; kernel fuzz was 2 for accelerometer/IR/
Nunchuk, 1 for Classic and 4 for MotionPlus. Uinput reported fuzz 0, its documented
provider limitation. Selected explicit inputs delivered these consumer values:

| Native input | Delivered event values |
| --- | --- |
| Accelerometer 600/400/512 | 88/-112/0 |
| IR point 123/234; other points absent | 123/234; missing 1023/1023 |
| Nunchuk stick 150/100; accel 600/400/512 | 22/-28; 88/-112/0 |
| Classic left 50/20, right 20/10, triggers 4/9 | 18/-12, 8/-12, 18/8 in source event order |
| MotionPlus 9000/8000/8192, all slow | 0/-1728/7272 in source event order |

Explicit neutral transitions restored the expected native values, including
accelerometer-512 and IR missing 1023/1023 outside the advertised limits. This is
selected-value virtual delivery, not physical calibration, a full-domain sensor
test or a first-valid-neutral creation oracle. Unchanged core buttons have no
evdev edge, and this probe opens nodes after initialization. Each helper's fd
count returned 4→4, owned nodes disappeared, and close remained idempotent.

The strengthened ignored probe asserted raw battery 192→kernel capacity 75,
rumble upload/play/stop/erase and retained native intent, replacement of Nunchuk
by Classic, and owned-HID disappearance. All four LED brightness writes still
returned EACCES 13; initialization LED intent is not user LED-control acceptance.

SDL's installed Debian 3.2.10+ds-1 runtime was selected explicitly, avoiding the
separate local 3.5.0 installation. Both runs set HIDAPI=1, HIDAPI_WII=1,
WII_PLAYER_LED=1 and ALLOW_BACKGROUND_EVENTS=1. With normal controller-only HID
filtering the synthetic vendor-page collection was absent; setting
HIDAPI_ENUMERATE_ONLY_CONTROLLERS=0 exposed it, but owned hidraw O_RDWR returned
EACCES 13. SDL instead opened the owned core through evdev:11 buttons, 0 axes,
0 hats and no accelerometer/gyro sensors. Rumble reached retained intent through
that fallback. RGB LED failed; a successful player-index setter is not proof of
LED delivery. The core stayed connected across extension edits, but this is no
Wii HIDAPI extension or sensor acceptance.

The existing Dolphin Flatpak 2609a (commit
`a22e93a81ac2898f9a4cd69bbcbaa7b8de52262f7b6f5cc4eb18211a3bd25d7e`)
ran offscreen with a fresh in-sandbox profile, Real slot 1, continuous scanning and
controller-interface connection enabled. The exact owned hidraw existed inside
its sandbox but could not be opened. No internal backend discovery/connection,
report modes, memory reads, extension initialization or MotionPlus activation
was established. No ROM/emulation session was used; bounded timeout124 is
termination, not application acceptance. Uinput has no HID endpoint for either
of these HID-specific consumer cells.

These historical runs kept `wiimote` bound and made no root, installation, udev,
unbind, permission or module-policy change. Anders subsequently confirmed the
five temporary per-node grants authorized by
[the owner decision on 176](https://github.com/anderstvoss/virtualgamepad/issues/176#issuecomment-6347519179).
The later granted reruns are separate new records in
[research PR 28](https://github.com/anderstvoss/controller-protocol-research/pull/28),
not retained by this pin. The manifest preserves the historical blocked results
and missing neutral oracle; it does not claim that grants are still pending.
Research pin adoption does not adopt independent `wii-gap-*` protocol behavior:
Classic Pro, early Plus, descriptor and other existing refusals remain unchanged.

## Creation and removal timing

Ten cycles were recorded at N1 and N4 for each realization, using the same
hashed executable built from tracked-clean `ccb98dc`, for 100 total samples.
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

## Part C documentary protocol adoption

Research main `732ca43c397f710e1632d825dbc03ccaf5637ad9` separates documentary
protocol descriptions from physical evidence. The following behaviours cite
its records; none establishes retail fidelity or a firmware report clock.

- Unsolicited extension-status notifications pause data until a valid host
  report 0x12 selects a mode again. ACKs and memory replies continue while
  paused. Solicited status 0x15 does not pause data
  (`wii-gap-wiibrew-status-20-resend`, `wii-gap-wiibrew-data-report-powerup-default`).
- MotionPlus deactivation 0x55 at 0xA400F0 ACKs and publishes unplug/current-port
  statuses atomically. Initialized downstream state survives, data pauses, and
  inactive 0xA600FA retains the previous 04/05/07 plus 05 suffix
  (`wii-gap-wiibrew-motionplus-deactivation`). No downstream means no plug flag.
- EEPROM reads ignore the high address byte and intersect the caller image
  with documented host window[0, 0x1700). Out-of-window or straddling reads error;
  the full imported image remains available for explicit terminal resolution.
  Writes retain caller-image bounds and Dolphin-qualified 0x22 errors; the
  documentary 0x21 error disagreement and physical write protection remain open
  (`wii-gap-wiibrew-eeprom-address-mirroring`,
  `wii-gap-dolphin-out-of-range-write-ack`, `wii-memory-size-hypothesis`).
- Classic uses existing format 01. Writes 01/02/03 to 0xA400FE explicitly return
  error 7 rather than accepting an unimplemented format
  (`wii-gap-wiibrew-extension-data-format-selector`). MotionPlus pass-through
  field moves and alternating records retain existing encoding and now have
  independent decoders (`wii-gap-wiibrew-motionplus-passthrough-field-moves`).
- Acceleration fragments remain restricted to acceleration-bearing modes,
  including the interleaved Z fragments. Existing IR ACKs and opaque camera
  setup do not accept visible extended/full IR or invent init delays
  (`wii-gap-wiibrew-data-report-powerup-default`,
  `wii-gap-wiibrew-ir-initialization-uncertainty`).

Encrypted workflows and Classic Pro stay refused. EEPROM images remain
opaque, length-validated caller imports, with no invented checksums or per-device
factory values (`wii-gap-extension-calibration-checksum-unknown`,
`wii-gap-classic-pro-full-id-physical-unknown`). The subsequent W2 owner decision on #158 authorizes only the interim synthetic
Game Pad collection described below; the retail descriptor remains unknown.
Permissions, consumer discovery and timing remain separate acceptance questions;
the earlier C5/W1 measurements are never relabelled as Part C measurements.

## W2 interim descriptor and Part C follow-up

Owner option B on #158 comment 6354044408 authorizes an independently authored
synthetic Generic Desktop/Game Pad Application collection. Its byte fields stay
on vendor page 0xFF00 with the existing Variable flags. All report directions,
IDs and payload lengths were unchanged at W2; speaker IDs 14/18/19 were absent.
The historical `wii-c5-functional-descriptor` fixture still records C5 and is
used to verify the historical report body. W8 later appends source-sized speaker
control reports and data handling for rumble only, without playback. No rnconrad candidate bytes were
copied or used as a template. Retail-evidence replacement is tracked in#238;
this interim descriptor makes no retail authenticity claim.

Unsupported active MotionPlus A4FE selectors now use the same atomic deactivation
path as 55 at A400F0: ACK, unplug/current-port statuses, data pause and preserved
inactive suffix/downstream initialization. This is the explicit software policy
chosen by the W2 handoff, not newly authenticated retail selector behavior.
The regression covers every unsupported byte for all three active modes, plus
queue admission and the no-downstream case.

The manifest adopts `wii-w1-granted-led-host-run` at retained research main
`16df95a3f8353a08cd12b81a24e15d67cdb4a6bf`. All four brightness writes succeeded
and reached retained intent on the granted Part A virtual device. This supports
bounded software LED intent, not physical light delivery or whole-controller
acceptance. Earlier blocked W1 and C5 records remain historical.

## W3 diagnosis and bounded evidence adoption

Research main `c380e6b5dc8d2fa0289deb5711a8858541a7ae03` retains W2 and the
WiiBrew subpage research. The manifest separates default HID enumeration/open
and initial Wii-enabled HIDAPI claim from sustained SDL acceptance, which stays
WIP after the observed disconnect. This changes evidence dispositions, not the
interim descriptor or protocol model.

The unchanged-main W3 trace observes unsolicited Nunchuk status, prompt host
report 0x12 reselection and resumed DATA. Repeating the bounded sequence with
explicit application commits still disconnects the old HIDAPI joystick; the
actual-default evdev control remains connected. Pinned SDL 3.2.10 source
`877399b2b2cf21e67554ed9046410f268ce1d1b2`, HandleStatus 1350–1384, requests a
mode reset and marks extension changes disconnected to reinitialize. Its
UpdateDevice 1511–1576 separately implements an inactivity heuristic. The trace
and control support the explicit host reinitialization explanation rather than
a DATA-pause deadlock. Reopening/sustained HIDAPI acceptance is not established.
No firmware clock, workaround or conflicting protocol behavior is invented.

The trace includes host reports from the bound kernel driver, SDL and the
explicit owned-node MotionPlus deactivation probe; individual driver ownership
of every host packet is not asserted. Report timestamps describe successful
virtual UHID syscall boundaries, not physical delivery or latency acceptance.
New W3 research records retain filtered report sequences and diagnostic limits;
raw application logs and identifying metadata stay private.

The historical W2 N1 input-class count 26→29 is attributable to an unrelated
USB-parent gamepad's input/event/joystick class entries: its retained udev
initialization time falls within that run. It has non-UHID ancestry and does not
match the virtual controller's association. Historical N1 remains blocked as
directed; no harness ordering change or replacement timing run is made.

The new Nunchuk six-byte and Classic format 01 documentary claims corroborate
existing packing only. Formats 02/03 stay refused; Classic Pro digital-trigger
documentation does not lift its identity refusal. Calibration/checksum and
Classic Pro physical-identity unknowns remain unknown. #238 retains the retail
descriptor work; Dolphin, radio, speaker and Balance Board are outside W3.

## W4 fixed-extension SDL scope

W4 separates a held raw-path HIDAPI claim with fixed topology from full sensor
and hotplug acceptance. On unchanged main `abb94e2` and SDL 3.2.10+ds-1, five fresh
SDL processes each observed 60 seconds with none, Nunchuk, Classic format 01,
MotionPlus, or MotionPlus+Nunchuk attached before open. All held their claims
without disconnected polls/removal events. Selected buttons/applicable axes and
core acceleration changed; Classic acceleration is not advertised by SDL.
Player-index 1/2 requests reached retained LED bits despite SDL's capability flag
being false. This is software intent, not physical light delivery.

Both MotionPlus profiles advertised/enabled gyro but polled values stayed zero.
Their full coverage experiments remain blocked; the fixed-claim cell supports
claim-held only. No complete button/axis mapping, secondary Nunchuk acceleration,
calibration, sensor rate, firmware cadence, game or physical acceptance follows.
The application explicitly drives inputs/commits; no production model changes.

The non-MotionPlus hotplug cell is `not-supported` for SDL 3.2.10, a pinned
consumer restriction: extension status removes the joystick, while a retained
raw handle prevents the unchanged still-enumerated device from being re-added.
This is a known source-supported consumer restriction.
It is separate from the overall SDL WIP cell and device realization refusals.

MotionPlus-active hotplug is not presumed safe: although status takes a different
branch, SDL `HandleButtonData` sets the same disconnect flag on downstream
presence changes in MotionPlus packets, and `HandleResponse` can do so on sensor
availability changes. Optional live active-hotplug was not run. The new consumer
claims cite 3.2.10 source at `877399b`; separately inspected release 3.4.18 at
`829a65d` and main at `8adc0f9` retain the relevant path. Neither newer runtime was run.
Full SDL restart or actual node removal is outside the no-reattach claim.

Research records use `wii-w4-*` and preserve all historical/cloud claims. No
protocol-model or descriptor change, gyro workaround, upstream report, physical
or later-controller work was introduced. The corpus stores the explicit
`evidence_role: consumer` marker in existing `applicability.scope`, preserving its
v1 schema. W4 adoption pinned merged research
`b4df2a3da3e35fdb517dd16908b0665d7c44f0a5` (research #33).

## W5 gyro consumer boundary

W5 runs on unchanged main `b23cb41` with the W4 Linux 6.12.113 / SDL 3.2.10 tuple.
Before SDL, two 20-second kernel-only MotionPlus evdev holders produce acknowledged
map 04/05 writes and changing ABS_RX/RY/RZ at the exact expected driven extrema.
Captured active packets decode both distinct drive vectors with valid gyro bits,
all slow flags and alternating Nunchuk pass-through. A new pure Rust regression
also covers both vectors and all eight slow-flag combinations; no runtime change.
The `kernel-motionplus-gyro` cell records this bounded kernel-consumer delivery.

Fresh SDL holder/control arms produce zero gyro events; the explicit kernel
holder is not a working SDL workaround. SDL-origin extension resets are observed
around initialization/open. Linux's SDL activation path omits both restoration
and cached-mode assignment. Kernel mapping/gyro can recover while SDL still
produces no gyro, and disconnects must remain explicit. Public API boundaries and
readbacks are not direct private InitDevice/cached-mode instrumentation; shared
reader/reset timing prevents attributing every zero uniquely to cached mode.
The `sdl-hidapi-wii-gyro` cell is consumer `not-supported` only for this recorded
configuration, citing the new W5 claim and runs. No universal SDL/physical claim.

The runtime requires creation-node access; the lab's temporary hidraw/LED grants
serve consumer access and output testing. The provider does not change host
permissions. No persistent setup, upstream SDL report, descriptor, radio, speaker,
Steam or later-controller work is included. H2 newer SDL is not run. Prior
research records and W4 acceptance boundaries are preserved.

Anders also requested a direct Dolphin sanity check. Two 45-second no-ROM
Flatpak 2609a runs with the correct WiimoteControllerInterface setting can open
the owned raw node, but direct backend connection/gyro reception and decoded
values remain unestablished. This is blocked, not measured zero gyro. Dolphin's
direct Wii HIDAPI path bypasses SDL; its SDL gamepad path does use SDL sensors.
No package, unbind, game, persistent profile or physical acceptance was added.

W5 pins merged research #35 at
`70606d91c124eaa40930a75d845286b66604c018`. Historical records and
previous generated fixture bytes are unchanged; only the fixture manifest pin changes.

## W7 independent consumer results

W7 ran on unchanged Wii runtime at main `840db277` after the demo W6 merge, on
Linux 6.12.113+deb13-arm64. The consumer survey supplied hypotheses, not passes.

- **Cemu ARM64 AppImage 29f2f1f: native transport works; gyro unestablished.**
  The initial PATH/Flatpak lookup missed this non-PATH AppImage; its absence
  inference is corrected by the additive follow-up. Four 35-second MP/MP+Nunchuk
  by holder/no-holder profiles, repeated with byte capture, exchange raw status,
  EEPROM, register and input reports. No-holder Cemu writes A600F0=55/A600FE=04;
  holder runs show Cemu A400 resets alongside kernel 04/05 maps. MP-only selects 33;
  Nunchuk also selects/receives 37. All arms receive EEPROM 0016/8 error 8 because
  the default caller image is empty and no image was imported. Calibration and
  decoded application gyro remain unestablished; raw 37 delivery is not a gyro
  oracle. No Cemu gyro acceptance cell is added. See the
  [AppImage follow-up](../protocol-research/docs/wii-w7-cemu-appimage.md).
- **xwiishow/libxwiimote 2-4.3: bounded gyro/normalization pass.** The exact owned
  wiimote HID enumerates; MP+Nunchuk mode 05 and both driven gyro triples produce
  changing events. Static offsets 10/20/30 subtract exactly per axis. Nunchuk is
  detected but its requested interface bit is not opened by this version; no
  Nunchuk library-event or calibrated-rate claim. The new
  `xwiimote-motionplus-gyro` cell is bounded to UHID; uinput lacks its required HID
  parent/wiimote-driver discovery contract.
- **evtest 1.35: repeatability control passed.** The owned MP-only node reports
  changing ABS_RX/RY/RZ with mode 04 over 12 seconds, supplementing the existing
  kernel gyro evidence without rewriting W5 packet validation.
- **RetroArch 1.20.0 Debian udev: generic controls pass on both realizations.** An
  original no-content input diagnostic sees changing core and Nunchuk controls
  over 600 frames with explicit lab bindings. Accel/gyro activation returns false
  on all 8 ports; held FDs at 4/8 seconds are core and Nunchuk, no sensor sibling.
  This installed version has no udev joypad sensor callbacks, unlike the newer
  surveyed RXYZ fallback. The new `retroarch-udev-generic` cells accept only those
  generic controls, not complete printed-label mappings or gyro. Both frontend
  processes reach their 25-second bounds after the diagnostic shutdown request.
- **Dolphin 2609a Flatpak: direct scanner checkpoint blocked.** Separate sandbox
  raw open and enumeration/open trace-positive-control checks pass. Both 35-second
  no-ROM MP/MP+Nunchuk Dolphin traces contain no owned enumeration or I/O. Direct
  scanner enumeration is the first unestablished checkpoint; status write,
  Connect, ControllerInterface AddDevice and gyro remain unestablished. Dynamic
  hooks do not identify an internal activation/filter failure. No new Dolphin
  acceptance cell or inference of measured zero gyro.

Anders installed the requested packages and confirmed the five temporary grants
supplied locally. The agent performed no grants, sudo, unbind, persistent policy
change or game-content injection. The initial Dolphin pilot overlapped child
lifetimes across profiles and cannot support independent comparison. A fresh,
human-granted repeat verified exact-profile child cleanup between both 35-second arms (two TERM, no KILL). Both corrected traces
stayed empty; separate raw-access and trace controls passed again. All owned
consumers and sessions closed idempotently; HID, raw, LED and original uinput nodes
were verified removed. Raw reports, paths, identities, terminal output and private
profiles are not public fixtures. No Wii model/codec/descriptor/API/dependency
change or physical/calibration/radio/whole-controller promotion follows. The
existing SDL gyro acceptance cell is unchanged.

Initial W7 evidence merged in research [#41](https://github.com/anderstvoss/controller-protocol-research/pull/41)
at `b4b68006837540e128901226cc275e4727c97eb8`. The corpus provides five scoped
`wii-w7-*-host-run` experiments, reduced fixtures, source/tool hashes and original
probes; [W7 details](../protocol-research/docs/wii-w7-consumer-runs.md).

The Cemu repeat used fresh app profiles on one reused UHID, with no cold-device
isolation claim. All eight app instances and external holders closed between
arms, then owned HID/raw/LED nodes were removed. No uinput device was created in
this repeat. Temporary grants were supplied locally and confirmed by Anders;
process-only raw-input/radio guards made no persistent host policy change.

W7 Cemu follow-up pins merged research
[#42](https://github.com/anderstvoss/controller-protocol-research/pull/42) at
`8cad518c2d3961dbd332e9d6d94e975c9ca4f94c`. The Wii acceptance
header is updated alongside its adopted corpus revision. The protocol-research
checker now resolves every acceptance evidence ID at that manifest's declared
revision and requires the revision to be within the adopted pin's history;
older valid declarations remain allowed. The new deterministic regression rejects
a newer record referenced by an older declaration, an unknown revision and a
revision beyond the adopted pin. Existing SDL cell bytes remain unchanged.

The adopted research pin also includes the test-discovery repair in
[#43](https://github.com/anderstvoss/controller-protocol-research/pull/43):
`e5256908f56e57d9910bbc6ae1b283492d743483`. The corpus tests load the Cemu reducer
relative to their source file, so parent-checkout discovery follows the same
path as research-root discovery. The repair changes no evidence records or
runtime helpers.

## W8 native consumers with caller calibration

W8 first records seven baseline arms, then six imported-block arms on merged
code `5a11e507ac973c23ca10beb36edb44fc35034376`. The research pin and acceptance
cells distinguish successful virtual observations from consumer limitations.
No imported image becomes a default, and no physical, calibrated-rate, audio,
radio, game or whole-controller acceptance is inferred.

| Consumer arm | Bounded result | Limit |
| --- | --- | --- |
| Native Dolphin 2503, MP | Imported MP calibration succeeds in two chunks; mapping view shows changing A, acceleration, two basic IR points and gyro | Explicit bindings, synthetic caller resources and UI gyro auto-calibration period zero; not rate accuracy or all-controls acceptance |
| Native Dolphin 2503, MP+Nunchuk | Mapping device visible; active mode 5 | A400F6/4 returns error 7 before MP calibration; changing motion not established |
| Cemu ARM64 AppImage 29f2f1f, MP | Base EEPROM read succeeds | Coexistence identity/init exchange includes A600FA error 7; decoded state unobserved |
| Cemu ARM64 AppImage 29f2f1f, MP+Nunchuk | Base read and 44 mapped A40020/16 reads succeed | Returned data matches MP's first half; not a clean Nunchuk-calibration or decoded-gyro pass |

The native Dolphin transport arms are bounded to 180 seconds each. MP includes
an additional 150-second mapping observation; it lists the Bluetooth
ControllerInterface device and shows changing application indicators. Its
acceleration preview displays 1.37 g and 2.27 g under the lab image; these labels
are not a physical calibration claim. Basic IR points are caller-driven only
in basic-capable report modes. No Real Wii Remote slot or game content is used.

The MP+Nunchuk F6 window is a known model boundary. Its 12,128 reads receive
12,128 error 7 replies, so it does not reach the MP calibration request. No new
register, codec, descriptor or API change is implemented to bypass that result.
A failed setup is not evidence of zero gyro or physical incompatibility.

The clean Cemu no-holder preflight was blocked by a foreign browser companion
holder before launch. Separate 35-second coexistence arms ran with that known
holder present, without opening a runner holder or modifying a foreign process.
Native requests are directly attributed; shared kernel traffic is not assigned
to a sole actor. The held MP+Nunchuk replies match the correctly mapped MP bank;
this does not establish application interpretation as Nunchuk calibration.
Those initial coexistence arms do not verify the clean Nunchuk path; the follow-up below does test delivery. Decoded Cemu gyro remains unverified.

All completed consumers exited and the shared temporary producer closed
idempotently; its HID, hidraw and four LED nodes were verified removed. Raw
captures, screenshots, identities and paths stay private. The adoption retains
the bank-isolation regression from the test-only `ec95e9a` followup, preventing
a missing addressed block from falling back to another device's calibration.


After the browser holder closed, two clean 35-second Cemu comparisons passed
no-holder preflight on a fresh producer with the same resources. The runner
opened no holder; the kernel remained bound. MP again made no extension
calibration read. MP+Nunchuk received 18 successful A40020/16 replies: 17 match
the imported Nunchuk block and one matches MP first half under shared mapping.
This establishes Nunchuk calibration delivery beyond the earlier refusal,
without proving application interpretation or decoded motion. Repeated
initialization remains the next unestablished checkpoint. Earlier coexistence
records are retained; no further register or API behavior is added. Fresh owned
nodes and consumers were verified gone after the clean comparison.


Cemu setup-order analysis narrows the boundary: MP has one IR setup and later
status/report-mode polling, while its native A4 initialization writes and
identity read receive error 7 after shared mapping changes. MP+Nunchuk has
19 repeat IR setups, preceded by successful calibration, successful identity
or present-extension status. Cemu's pinned provider reselects mode/IR from
status and extension state; it does not await ACKs in that path. The captures
lack timestamps and do not support a missing-ACK timeout or a unique actor
cause. This remains a consumer/shared-kernel limitation, without a model fix.


W9 Part A adopts research main `77dad7408cbdb9d3738059de85e6f366eeabce59`,
including W8 evidence merge `0870a91` (research PR #45) and the source-only
Dolphin attachment brief (research PR #46). The current corpus has 160 Wii
claims; its new attachment hypotheses do not promote the W8 live results or
implement compatibility shims. The `dolphin-controllerinterface-wii` cell is
scoped to MP-only with caller imports. MP+Nunchuk and Cemu retain their recorded
limits; future compatibility options and reruns belong to separate PRs.


W9 steering adopts research main `bd1357c70eeeca4a91b29e34671765e6a05ea5c0`
with research PR #47 (166 Wii claims). Cemu `29f2f1f` has a consumer source
limitation: its report `0x37` MotionPlus visitor operates on an extension variant
without MotionPlus (`wii-gap-cross-cemu-motionplus-visitor-unreachable`), and its
exported motion sample supplies zero angular arrays
(`wii-gap-cross-cemu-exported-gyro-zero-sample`). No virtualgamepad change can
produce exported Cemu gyro at that pinned version. The W8 shared-kernel setup
requeue observations remain a separate scope line; decoded motion is unobserved.

F6 and A40040 are Dolphin-specific setup gates; other consumers decode
MotionPlus without them (`wii-gap-cross-cemu-fa20-no-f6-40` and
`wii-gap-cross-linux-kernel-sdl-layer-contrast`). They are not universal
prerequisites and any future shims remain compat-class. Active passthrough
A40020 retains MP calibration, A40040 is the child copy, and inactive A40020
is the child's own calibration; the banks must never alias.

### Demo caller calibration files

In the Wii Remote creation card, Advanced options lets you choose an optional
EEPROM image and MotionPlus (32 bytes), Nunchuk (16 bytes) and Classic (16 bytes)
calibration files. Choose file opens the demo's directory browser; Go and Parent
navigate it, and selecting a file retains its path for creation. Clear removes a
choice. No image, factory value or preset is supplied by the demo.

Creation reads and validates each selected file before opening the provider,
using `WiiEepromImage::import` and `WiiExtensionCalibration::import`. Size,
checksum, CRC and file-read errors use the existing creation-error display.
The EEPROM API bounds the address window; it does not invent an EEPROM
checksum contract. Block layouts are cited by
`wii-gap-dolphin-motionplus-crc32-calibration`,
`wii-gap-dolphin-nunchuk-calibration-layout`,
`wii-gap-dolphin-classic-calibration-layout` and
`wii-gap-dolphin-calibration-additive-checksums`.

The file controls are disabled for uinput because it has no EEPROM or extension
calibration address channel. HID choices remain visible and retained for a
switch back to HID, but are not read or applied to uinput creation.


## Faithful versus compat behaviour

The provenance classes describe register behaviour, not acceptance of a physical
remote. Faithful means WiiBrew, physical evidence or independent consumer
codebases agree; compat means a consumer requires plausible behaviour without
independent physical readback. The project still aims at a full faithful model.

| Register behaviour | Class and records | Remaining limit |
| --- | --- | --- |
| Unencrypted child initialization/identity, known MP selectors and identity | Faithful: `wii-kernel-extension-init`, `wii-c5-extension-identifiers`, `wii-kernel-motionplus-pass-through` | Revision and physical fidelity remain separately bounded |
| IR and speaker control-register writes | Faithful control layout: `wii-c5-camera-no-points`, `wii-gap-wiibrew-speaker-init-config-registers` | No camera blob metadata or speaker audio acceptance |
| Caller EEPROM storage and imported calibration windows | Compat source-model resource policy: `wii-dolphin-memory-routing`, `wii-gap-dolphin-motionplus-crc32-calibration` and child layout records | No factory defaults, physical writable-map or calibrated-rate claim |
| Unsupported active MP selector deactivation | Compat existing bounded policy: `wii-gap-wiibrew-motionplus-deactivation` | Only known documented deactivation is faithful; other selectors are not a retail claim |
| Active F6 child-ID mirror, fixed F7 | Compat: `wii-gap-dolphin-attach-passthrough-four-byte-id-f6`, `wii-w9-f7-fixed-preparing-compat` | F7 is always 0x02 when served, never 0x00 or a readiness value. No challenge; 0x50 stays error 7 |
| Active A40040 child calibration copy | Compat: `wii-gap-dolphin-attach-active-passthrough-calibration-copy-40` | Exact imported 16 bytes only, never repaired or supplied by default |

`WiiRemoteCreationOptions` has two independent bool fields:
`motionplus_passthrough_id_mirror` and
`motionplus_passthrough_calibration_copy`. Both default on by owner decision.
`create_wii_remote_with_options` takes the same caller resources and revision
as the existing image constructor plus this policy; existing constructors use
the default. The values are copied at creation and have no live setter. Demo
Advanced options exposes both with a compat label; uinput disables them because
it has no addressed HID register channel.

Each shim applies only in active mode 05 with Nunchuk or mode 07 with Classic.
Turning either off restores the prior error 7 independently. MP-only mode 04,
inactive MP and absent or mismatched downstream extensions do not expose either
window. A400FA remains MP's own identity. Whole requests crossing F5/F6, F9/FA,
3F/40 or 4F/50 are refused before any prefix is returned; F5 is unmodelled and no
byte is invented to make a straddling request succeed. Within the mirror, F7
is fixed at 0x02 across repeated reads and polling.

The three calibration banks are distinct: active A40020/32 is MP's block,
active A40040/16 is the attached child's exact imported block, and inactive
A40020/16 is the child's own block. No alias or fallback between those banks is
allowed. Detach, replacement and missing imports never expose a stale copy.

To promote or retire either shim, an owner-authorized physical capture must
compare inactive child FA/6 and 20/16 with active F6/4 and 40/16 in mode 05,
scoped by external, integrated and TR revisions. F7 progress and challenge
readback would separately settle its fixed preparing policy. Until then both
remain compat, without physical or game acceptance. Part D will compare native
Dolphin MP+Nunchuk on/off and preserve MP-only Dolphin plus kernel evdev controls;
no such new consumer pass is claimed by this code change. Cemu gyro is outside
the lab scope at its pinned source-limited version.


## W9 compat consumer controls with the kernel bound

Part D tests code PR #256 at actual runtime head
`ebc3a030685e476242a4d6df96e17e14862e0398`, with both compat switches on and
off in separate virtual devices. The later owner integration head `fd21d9a`
adds prior adoption/docs/tests and a source comment; it changes no Wii runtime
behaviour. Experiment records retain the exact tested head.

| Arm | Bound and result |
| --- | --- |
| Native Dolphin 2503 MP+Nunchuk, on | Corrected 180-second repeat: 14,334 F6/4 error 7 replies; MP 20/32 and child 40/16 not reached |
| Native Dolphin 2503 MP+Nunchuk, off | 180 seconds: 14,659 F6/4 error 7 replies; same unreached calibration checkpoints |
| Native Dolphin MP-only, on | 150 seconds: exact MP calibration in two chunks and changing mapped gyro, acceleration and basic IR |
| Kernel evdev MP-only, on | Fresh 12-second control: ABS_RX/RY/RZ each change between two values; reused post-Dolphin node was absent before the earlier evtest attempt |

After native mode 05 mapping, each MP+Nunchuk arm has one combined A400F0=55
write and zero such native Dolphin writes. This is non-Dolphin output consistent
with bound hid-wiimote reinitialization, followed by inactive-bank probing.
Identical shared requests are not uniquely assigned by payload alone, and no
kernel stack trace is claimed. The first unpassed checkpoint is F6 with the
shim state gate unavailable, not a missing caller calibration block.
No model state gate, driver binding, bank alias or challenge behaviour changes.

The first on pilot is excluded because its coordinator exited at timeout before
recording orderly cleanup; a corrected isolated-process repeat verified cleanup
between consumers. The first reused kernel preflight stopped before evtest
because the MotionPlus node had disappeared after Dolphin. A fresh MP-only
control retaining its initial topology passed; this is not post-Dolphin recovery
acceptance. All temporary devices and owned consumers closed idempotently, with
HID/raw/LED removal verified. Raw logs, bytes, screenshots and identities remain
private. No Cemu gyro, physical, rate-accuracy, radio or game acceptance follows.
The MP+Nunchuk ControllerInterface cell is not extended to a pass.


These reduced results are adopted from research PR #50, merged as
`ceaa48a30b57485a0ef687800c09c4e197e8868e` (178 Wii claims, 99 corpus tests).
The W9 MP-only control used mode 05 without a child; the earlier W8 mode 04
observation remains separate evidence. No F6 or child-copy gate is required in
that MP-only state.


## Wii Remote Plus P1 creation policy

`WiiRemoteRevision::Tr` is shown as **Wii Remote Plus** under Advanced options;
the enum and internal -TR name remain unchanged. The family ID remains
`virtualgamepad.wii-remote`, with Plus-scoped rows in its acceptance manifest.
EarlyIntegratedPlus remains refused. No Original acceptance is inherited.

For an inactive Plus, A600FA/6 defaults to `00 00 A4 20 00 05`. The suffix is
faithful-class source evidence from two independent consumers, recorded in
`wii-gap-plustrace-wiiuse-builtin-id-and-classic-gap` and
`wii-gap-plustrace-cemu-builtin-identity-nonactivation`; it is not physically
measured. The two leading zero bytes remain the existing unknown bytes.
The Plus-only `motionplus_inactive_id_external_style` creation option is off
by default. When enabled it serves `00 00 A6 20 00 05` for pinned Cemu's
activation check. The GUI labels it compat, shows it only for Plus and disables
it for uinput. Cemu gyro export remains out of scope. Original ignores the
option and retains its external MP identity and existing deactivation policy.

After F0 deactivation, both Plus ID styles retain the previous mode byte,
following `wii-gap-wiibrew-motionplus-deactivation` for the external model.
The integrated Plus post-deactivation byte is unknown; this reuse is a model
policy, not physical evidence. Fresh inactive IDs still have mode byte 00.
Active IDs remain `A4 20 <mode> 05` in modes
04/05/07 for both revisions. FE/1 and FE/2 are bounded identity slices; invalid
F9/FF straddles fail as whole reads. F6, 0x40 and fixed F7 0x02 retain the same
active passthrough scope, caller imports and independent creation switches;
0x50 remains error 7. No calibration defaults or descriptor changes are added.

The physical status extension bit for a bare Plus across activation remains
unknown in the pinned records. Existing software behavior is retained: clear
while inactive, set when MP is mapped even without a child. Built-in presence
is not a physical assertion about that downstream status bit. A deterministic
test pins the unchanged behavior; no new faithful status-bit claim follows.

The approved supporting API snapshot deliberately adds the creation-option
field; the ordinary root API stays unchanged. Source pin: research main
83c026c (215 Wii-prefixed claims), including W9f #62, Plus P1 #63 and W9g #66. Kernel-
bound Plus qualification is recorded separately as wii-p1-* in research PR #63;
these live records are not adopted by the source-only Plus acceptance rows. Unbound
Plus passthrough is out of scope until W9f passes; #258 gates release use of
the special-driver window. Physical captures remain deferred under #52.


### Activation re-attach status compatibility (W9g)

`WiiRemoteCreationOptions::motionplus_activation_reattach_status` defaults to
true for Original and Wii Remote Plus. Activation from inactive
while the extension bit is already set queues an unplugged status then the
current status. Active mode switches keep the single status with the option on
or off. The existing reporting pause still requires a host 0x12
reselection. With the option off, the previous single status remains. An
initial MP-only activation from a clear bit still sends one status; deactivation
keeps its existing pair. The demo exposes the switch under Advanced, labelled
compat and disabled for uinput, which has no addressed HID status channel.

This is owner-approved compatibility behavior. The pair is documented for
**deactivation only** by `wii-gap-wiibrew-motionplus-deactivation`. W9f #62
records the MP+Nunchuk reporting pause while its MP-only control reselects
reporting. Neither source establishes physical activation behavior. Physical activation remains unmeasured; draft research #65 is not cited until
it merges. F6, F7, the separate 0x20 and 0x40 banks, caller
calibration and error 7 at 0x50 keep their existing scope. W9g #66 records an Original generic-bound decoded gyro/Nunchuk-stick pass
with the option on, the reporting pause with it off, and the F6-off error gate.
All arms activate from inactive, so narrowing the trigger requires no host
rerun. These consumer results are separate from physical fidelity; #258 still
gates release use of generic binding.
