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
  USB bus3 and cannot express the kernel consumer's nonzero fuzz. Minimum,
  maximum and flat values match the extracted source table. Decoder values can
  exceed the advertised ABS bounds; they are preserved, including the missing
  IR1023/1023 sentinel. There is no explicit evdev point-visibility/blob field.
- Wii LED-class and `power_supply` endpoints belong to the HID kernel driver;
  plain uinput cannot provide them. Its target surface states this limitation.
- Bluetooth-bus UHID uses Nintendo identity and an authored interim synthetic
  Generic Desktop/Game Pad collection with vendor-page fields and source-backed
  report sizes (W2 option B; retail-evidence successor#238). It is not a retail
  descriptor. Bus5 is local kernel metadata: no radio, pairing or BlueZ.
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
[research PR17](https://github.com/anderstvoss/controller-protocol-research/pull/17):
`wii-c5-bt-host-binding`, `wii-c5-evdev-native-values` and the four
`wii-c5-*-host-run` experiments. The acceptance manifest pins their reviewed
research revision. No fixture is a physical capture.

The C5 review fixes were checked on the same Linux6.12 host at tracked-clean
`1b431cd5eabdf55c5c84091311c15f8105a25c87`. The unchanged ignored probe passed
once, including kernel rumble and owned-node cleanup; all four LED writes remained
permission-denied. A separate temporary supporting-API harness observed Nunchuk,
then both Nunchuk and MotionPlus after insertion, then Nunchuk without MotionPlus
after removal, and disappearance of the owned HID object on close. This bounded
interface observation does not establish sensor or application acceptance. No
research record or historical timing sample was changed.

## W1 bounded follow-up observations

[Part A PR235](https://github.com/anderstvoss/virtualgamepad/pull/235) is the
implementation tested at clean `50161b913d0f82950c292465625e9cdd8854daf2`.
[Research PR22](https://github.com/anderstvoss/controller-protocol-research/pull/22)
retains five original `wii-w1-*` experiments, structured fixtures, executable and
source hashes, and the original bounded probes under `tools/wii_w1`. The current
manifest pins research main `732ca43c397f710e1632d825dbc03ccaf5637ad9`, which
retains the W1 records through merged research PR22. The earlier C5 paragraphs and timing table
remain historical observations of their own exact revisions.

On the same Linux 6.12.113+deb13-arm64 aarch64 host, the W1 probes opened all six
owned interface roles for each realization. Live minimum, maximum, flat and
resolution matched the source table; kernel fuzz was2 for accelerometer/IR/
Nunchuk,1 for Classic and4 for MotionPlus. Uinput reported fuzz0, its documented
provider limitation. Selected explicit inputs delivered these consumer values:

| Native input | Delivered event values |
| --- | --- |
| Accelerometer600/400/512 | 88/-112/0 |
| IR point123/234; other points absent | 123/234; missing1023/1023 |
| Nunchuk stick150/100; accel600/400/512 | 22/-28;88/-112/0 |
| Classic left50/20, right20/10, triggers4/9 | 18/-12,8/-12,18/8 in source event order |
| MotionPlus9000/8000/8192, all slow | 0/-1728/7272 in source event order |

Explicit neutral transitions restored the expected native values, including
accelerometer-512 and IR missing1023/1023 outside the advertised limits. This is
selected-value virtual delivery, not physical calibration, a full-domain sensor
test or a first-valid-neutral creation oracle. Unchanged core buttons have no
evdev edge, and this probe opens nodes after initialization. Each helper's fd
count returned4→4, owned nodes disappeared, and close remained idempotent.

The strengthened ignored probe asserted raw battery192→kernel capacity75,
rumble upload/play/stop/erase and retained native intent, replacement of Nunchuk
by Classic, and owned-HID disappearance. All four LED brightness writes still
returned EACCES13; initialization LED intent is not user LED-control acceptance.

SDL's installed Debian3.2.10+ds-1 runtime was selected explicitly, avoiding the
separate local3.5.0 installation. Both runs set HIDAPI=1, HIDAPI_WII=1,
WII_PLAYER_LED=1 and ALLOW_BACKGROUND_EVENTS=1. With normal controller-only HID
filtering the synthetic vendor-page collection was absent; setting
HIDAPI_ENUMERATE_ONLY_CONTROLLERS=0 exposed it, but owned hidraw O_RDWR returned
EACCES13. SDL instead opened the owned core through evdev:11 buttons,0 axes,
0 hats and no accelerometer/gyro sensors. Rumble reached retained intent through
that fallback. RGB LED failed; a successful player-index setter is not proof of
LED delivery. The core stayed connected across extension edits, but this is no
Wii HIDAPI extension or sensor acceptance.

The existing Dolphin Flatpak2609a (commit
`a22e93a81ac2898f9a4cd69bbcbaa7b8de52262f7b6f5cc4eb18211a3bd25d7e`)
ran offscreen with a fresh in-sandbox profile, Real slot1, continuous scanning and
controller-interface connection enabled. The exact owned hidraw existed inside
its sandbox but could not be opened. No internal backend discovery/connection,
report modes, memory reads, extension initialization or MotionPlus activation
was established. No ROM/emulation session was used; bounded timeout124 is
termination, not application acceptance. Uinput has no HID endpoint for either
of these HID-specific consumer cells.

These historical runs kept `wiimote` bound and made no root, installation, udev,
unbind, permission or module-policy change. Anders subsequently confirmed the
five temporary per-node grants authorized by
[the owner decision on176](https://github.com/anderstvoss/virtualgamepad/issues/176#issuecomment-6347519179).
The later granted reruns are separate new records in
[research PR28](https://github.com/anderstvoss/controller-protocol-research/pull/28),
not retained by this pin. The manifest preserves the historical blocked results
and missing neutral oracle; it does not claim that grants are still pending.
Research pin adoption does not adopt independent `wii-gap-*` protocol behavior:
Classic Pro, early Plus, descriptor and other existing refusals remain unchanged.

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

## Part C documentary protocol adoption

Research main `732ca43c397f710e1632d825dbc03ccaf5637ad9` separates documentary
protocol descriptions from physical evidence. The following behaviours cite
its records; none establishes retail fidelity or a firmware report clock.

- Unsolicited extension-status notifications pause data until a valid host
  report0x12 selects a mode again. ACKs and memory replies continue while
  paused. Solicited status0x15 does not pause data
  (`wii-gap-wiibrew-status-20-resend`, `wii-gap-wiibrew-data-report-powerup-default`).
- MotionPlus deactivation0x55 at0xA400F0 ACKs and publishes unplug/current-port
  statuses atomically. Initialized downstream state survives, data pauses, and
  inactive0xA600FA retains the previous04/05/07 plus05 suffix
  (`wii-gap-wiibrew-motionplus-deactivation`). No downstream means no plug flag.
- EEPROM reads ignore the high address byte and intersect the caller image
  with documented host window[0,0x1700). Out-of-window or straddling reads error;
  the full imported image remains available for explicit terminal resolution.
  Writes retain caller-image bounds and Dolphin-qualified0x22 errors; the
  documentary0x21 error disagreement and physical write protection remain open
  (`wii-gap-wiibrew-eeprom-address-mirroring`,
  `wii-gap-dolphin-out-of-range-write-ack`, `wii-memory-size-hypothesis`).
- Classic uses existing format01. Writes01/02/03 to0xA400FE explicitly return
  error7 rather than accepting an unimplemented format
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
`wii-gap-classic-pro-full-id-physical-unknown`). The subsequent W2 owner decision on#158 authorizes only the interim synthetic
Game Pad collection described below; the retail descriptor remains unknown.
Permissions, consumer discovery and timing remain separate acceptance questions;
the earlier C5/W1 measurements are never relabelled as Part C measurements.

## W2 interim descriptor and Part C follow-up

Owner option B on#158 comment6354044408 authorizes an independently authored
synthetic Generic Desktop/Game Pad Application collection. Its byte fields stay
on vendor page0xFF00 with the existing Variable flags. All report directions,
IDs and payload lengths are unchanged; speaker IDs14/18/19 stay absent.
The historical `wii-c5-functional-descriptor` fixture still records C5 and is
used only to verify the unchanged report body. No rnconrad candidate bytes were
copied or used as a template. Retail-evidence replacement is tracked in#238;
this interim descriptor makes no retail authenticity claim.

Unsupported active MotionPlus A4FE selectors now use the same atomic deactivation
path as55 atA400F0: ACK, unplug/current-port statuses, data pause and preserved
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
report0x12 reselection and resumed DATA. Repeating the bounded sequence with
explicit application commits still disconnects the old HIDAPI joystick; the
actual-default evdev control remains connected. Pinned SDL3.2.10 source
`877399b2b2cf21e67554ed9046410f268ce1d1b2`, HandleStatus1350–1384, requests a
mode reset and marks extension changes disconnected to reinitialize. Its
UpdateDevice1511–1576 separately implements an inactivity heuristic. The trace
and control support the explicit host reinitialization explanation rather than
a DATA-pause deadlock. Reopening/sustained HIDAPI acceptance is not established.
No firmware clock, workaround or conflicting protocol behavior is invented.

The trace includes host reports from the bound kernel driver, SDL and the
explicit owned-node MotionPlus deactivation probe; individual driver ownership
of every host packet is not asserted. Report timestamps describe successful
virtual UHID syscall boundaries, not physical delivery or latency acceptance.
New W3 research records retain filtered report sequences and diagnostic limits;
raw application logs and identifying metadata stay private.

The historical W2 N1 input-class count26→29 is attributable to an unrelated
USB-parent gamepad's input/event/joystick class entries: its retained udev
initialization time falls within that run. It has non-UHID ancestry and does not
match the virtual controller's association. Historical N1 remains blocked as
directed; no harness ordering change or replacement timing run is made.

The new Nunchuk six-byte and Classic format01 documentary claims corroborate
existing packing only. Formats02/03 stay refused; Classic Pro digital-trigger
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

W5 runs on unchanged main `b23cb41` with the W4 Linux 6.12.113 / SDL3.2.10 tuple.
Before SDL, two 20-second kernel-only MotionPlus evdev holders produce acknowledged
map04/05 writes and changing ABS_RX/RY/RZ at the exact expected driven extrema.
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

- **Cemu native Wii Remote: blocked before discovery.** No executable is installed
  on this ARM64 host and Flathub has no aarch64 Cemu ref. MP/MP+Nunchuk with and
  without a kernel holder are not run; no Cemu acceptance cell is added.
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
lifetimes across profiles and cannot support independent comparison. A fresh, human-granted repeat verified exact-profile
child cleanup between both 35-second arms (two TERM, no KILL). Both corrected traces
stayed empty; separate raw-access and trace controls passed again. All owned
consumers and sessions closed idempotently; HID, raw, LED and original uinput nodes
were verified removed. Raw reports, paths, identities, terminal output and private
profiles are not public fixtures. No Wii model/codec/descriptor/API/dependency
change or physical/calibration/radio/whole-controller promotion follows. The
existing SDL gyro acceptance cell is unchanged.

W7 pins merged research [#41](https://github.com/anderstvoss/controller-protocol-research/pull/41)
at `b4b68006837540e128901226cc275e4727c97eb8`. The corpus provides five scoped
`wii-w7-*-host-run` experiments, reduced fixtures, source/tool hashes and original
probes; [W7 details](../protocol-research/docs/wii-w7-consumer-runs.md).
