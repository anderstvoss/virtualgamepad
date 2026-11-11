# Status

Current release: `0.1.0-alpha.1` on `ae9e87b`, published 2026-10-14 as a Git-only
pre-release. This page records current dispositions only. Alpha 1 planning,
gate and evidence records are in [history/alpha-1](history/alpha-1/README.md).

## What Alpha 1 accepted

- A reviewed, frozen ordinary-root API (110 items), enforced in CI by
  `scripts/check-alpha-api.py` against [the snapshot](api/ALPHA_API_SNAPSHOT.json).
- DualSense, DualShock 4, Switch Pro and standard-HID Xbox 360 on `linux.uinput`
  and `linux.uhid.usb`, with deterministic protocol, lifecycle and cleanup tests
  and bounded smoke-level host checks.
- Opt-in emulated audio (UHID/PipeWire and USB/IP) as functional WIP; seven scoped
  broker security/recovery phases passed (#115).

Every [support matrix](CONTROLLER_SUPPORT.md) cell is still WIP. Smoke checks and
an API freeze do not promote a cell.

## Known limitations

- Sustained audio continuity and latency are not validated; dropouts may occur
  (#135, #143, #145).
- Physical-reference fidelity is not validated (#136). Xbox 360 is a standard-HID
  presentation, not XInput/xpad emulation.
- Controller-matching audio profiles are unavailable (#116).
- `linux.dummy_hcd.usb-hid` is retained as experimental and not recommended
  under owner decision D5 (#176 comment 6325079527): it requires root and host
  provisioning, and remains unavailable through ordinary-root constructors
  because Linux `f_hid` lacks complete GET_REPORT metadata and negative replies.
  Near feature-complete UHID realizations are the supported target for controller
  buildout; dummy_hcd and USB/IP are optional faithful tiers, never defaults.
- Wii Remote ships a local UHID Bluetooth-bus realization. Actual Bluetooth
  radio, pairing and BlueZ remain deferred under #164.
- Steam acceptance is deferred (#144). Broader GUI review is deferred (#126).

## Next release

**Alpha 2 — preparation, not yet released.** The five checkpoint merges and
follow-up work below are verified on `main` as of
[`f27e6114`](https://github.com/anderstvoss/virtualgamepad/commit/f27e6114f742b5044b4ca735ae17b4d20807f654).
This merged-step list stops at that exact `main` commit. Release closeout
is tracked under [#365](https://github.com/anderstvoss/virtualgamepad/issues/365), with the owner gate decisions in
[comment 6792411358](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358).
The [Alpha 2 release-notes draft](release/ALPHA_2_RELEASE_NOTES_DRAFT.md)
and [release checklist](release/ALPHA_2_CHECKLIST.md) are provisional;
they do not authorize a tag or publication. All cells in the
[support matrix](CONTROLLER_SUPPORT.md) remain WIP.

| Checkpoint | Merged PR and exact `main` merge SHA | Disposition |
| --- | --- | --- |
| C5 · Wii Remote | [#233](https://github.com/anderstvoss/virtualgamepad/pull/233) · `714958dc844214a52d4f25ad544a58f37a1e1e42` | Linux uinput and local Bluetooth-bus UHID package; not real Bluetooth |
| C6 · wired Steam Controller 2026 | [#301](https://github.com/anderstvoss/virtualgamepad/pull/301) · `ad80ae6ea8f0e434b026b0c3dfd94ee3f06ba7df` | Wired USB UHID and reduced uinput; no production Puck |
| C7 · API convergence | [#329](https://github.com/anderstvoss/virtualgamepad/pull/329) · `692f41bfac8d300472edb35575b8f652674bfc0f` | Common `create_<family>(CreationOptions, <Family>Options)` shape for the six then-existing families |
| C8 · core closeout and host audit draft | [#337](https://github.com/anderstvoss/virtualgamepad/pull/337) · `4ac42325468b11a9edc3f6ba3295983ebaf04b43` | Closed; includes ADR dispositions, virtual-host acceptance review and the draft permissions audit |
| C9 · Xbox Series | [#342](https://github.com/anderstvoss/virtualgamepad/pull/342) · `69c2283fe6fd6ab143f650018c0d24522eaa4f9a` | Seventh compiled family; reduced xpad-style uinput only |

The Alpha 2 host setup/permissions gate [#258](https://github.com/anderstvoss/virtualgamepad/issues/258)
is **closed and signed off**, on C9's `69c2283`, by
[owner comment 6777536122](https://github.com/anderstvoss/virtualgamepad/issues/258#issuecomment-6777536122).
The signed-off [host audit](ALPHA_2_HOST_AUDIT.md) is authoritative for
creator/consumer permission boundaries, administrator opt-ins and test-only
privileges. No implicit new host policies are part of Alpha 2.

Follow-on core changes on `main`:

- [#350](https://github.com/anderstvoss/virtualgamepad/pull/350) ·
  `abcdaf8c4f35167cbd65610d86c3c637b3136b5a`: USB/IP worker
  retained-output *snapshots*, not a continuous host-output stream;
  a **real USB/IP validation on kalux is an Alpha 2 gate**, still unrun as
  of the draft baseline ([#365 decision](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358)).
- **VG-1** [#352](https://github.com/anderstvoss/virtualgamepad/pull/352) ·
  `c5d8340d3b4c00a756f2d6611ae380be4af82071`: whole-state
  `set_inputs` for the original four families. [#348](https://github.com/anderstvoss/virtualgamepad/pull/348)
  was initially stacked on the audio branch; #352 brought the change onto `main`.
- **VG-1b in progress** ([#322](https://github.com/anderstvoss/virtualgamepad/issues/322),
  [input-model spec](spec/input-model-state-and-mutation.md)):
  step a [#353](https://github.com/anderstvoss/virtualgamepad/pull/353) ·
  `2890223d79d6a5ec53778e8d588737e20e4a3fb6` (validation/clocks);
  step b [#363](https://github.com/anderstvoss/virtualgamepad/pull/363) ·
  `5bea7a097f66c9526fae45ea32b5a8a936029580` (transactional edits);
  step c1 [#364](https://github.com/anderstvoss/virtualgamepad/pull/364) ·
  `77ec6e39457ddf123ef732ac0e28237307e05f8c` (Steam
  reduced-uinput refusals);
  step c2 [#366](https://github.com/anderstvoss/virtualgamepad/pull/366) ·
  `b41c82da593f179ad2933098dedb4dd658e2aaf0` (Wii battery separated into controller-owned
  `WiiRemoteBattery` metadata);
  step d/DualSense [#370](https://github.com/anderstvoss/virtualgamepad/pull/370) ·
  `f27e6114f742b5044b4ca735ae17b4d20807f654` (`DualSenseInputs` and independent
  `DualSenseStatus` read view). Other step-d families and step e are not
  complete at the `f27e6114f742b5044b4ca735ae17b4d20807f654` baseline.
  **TODO(VG-1b e):** after the local agent's final ledger lands,
  refresh the Alpha 2 API-delta summary and remove this placeholder.
  Do not edit the active migration ledger or application/per-controller API docs
  in this release-prep lane.

[Phase 2 #322](https://github.com/anderstvoss/virtualgamepad/issues/322)
closes only after **VG-1b steps a–e** and the Steam #304 kernel observations
are recorded in research (currently
[research PR #122](https://github.com/anderstvoss/controller-protocol-research/pull/122),
open at this baseline). The first-valid-neutral timing oracle
[#166](https://github.com/anderstvoss/virtualgamepad/issues/166) is a
**known limitation, not an Alpha 2 gate** ([owner decision](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358)).
[VG-2 #344](https://github.com/anderstvoss/virtualgamepad/issues/344)
and [Accepted ADR-0024](adr/ADR-0024-host-output-stream.md) (merged via
[#354](https://github.com/anderstvoss/virtualgamepad/pull/354) as
`eaff3d77c2b73dd3668d5b37d54c7927f94251d0`) belong to the separate
audio track; the design is accepted but its stream implementation is not
landed, and neither is **an Alpha 2 gate**.

New-controller work after Alpha 2 is tracked separately:
[DualSense Edge #170](https://github.com/anderstvoss/virtualgamepad/issues/170),
[Switch 2 Pro #171](https://github.com/anderstvoss/virtualgamepad/issues/171),
[Xbox One #355](https://github.com/anderstvoss/virtualgamepad/issues/355),
[Elite Series 2 #356](https://github.com/anderstvoss/virtualgamepad/issues/356),
[Xbox Nano #357](https://github.com/anderstvoss/virtualgamepad/issues/357),
[Elite Series 3 preliminary #358](https://github.com/anderstvoss/virtualgamepad/issues/358),
[Elite Series 1 #359](https://github.com/anderstvoss/virtualgamepad/issues/359),
[Switch 2 GameCube #360](https://github.com/anderstvoss/virtualgamepad/issues/360),
[Wii U Pro #361](https://github.com/anderstvoss/virtualgamepad/issues/361)
and [Joy-Con 2 #362](https://github.com/anderstvoss/virtualgamepad/issues/362).
The original [GameCube adapter #296](https://github.com/anderstvoss/virtualgamepad/issues/296)
and [Joy-Con #297](https://github.com/anderstvoss/virtualgamepad/issues/297)
also remain future-family work, not Alpha 2 acceptance.

## Runtime contracts

- [ADR-0019](adr/ADR-0019-live-topology.md): retained live-topology generations,
  with static uinput capability supersets and neutralization of absent controls.
  Wii uses this seam, including through the ordinary root.
- [ADR-0020](adr/ADR-0020-retained-outputs-and-kinds.md): retained accepted host
  outputs and typed surface kinds. D2 was granted on #176; step 10c exposes
  ordinary-handle snapshots and read-only typed capability descriptors.
- [ADR-0021](adr/ADR-0021-presentation-epochs.md): logical controller and host
  presentation epochs, Deferred under E4. Step 13a was dropped.
- [ADR-0022](adr/ADR-0022-receiver-aggregates.md): receiver-owned presentation,
  typed protocol slots, rollback and sibling-safe teardown. C4 shipped the
  synthetic fixture. The production receiver handle remains tracked under #165.
- [ADR-0023](adr/ADR-0023-controller-subsystems.md): controller-owned bounded
  in-band subsystems, adopted under D4. Steam has opt-in PCM compatibility; its
  retained-packet and observation policy is an owner decision on #322.

C4 step 13b adds a [synthetic receiver lifecycle fixture](spec/receiver-aggregate-fixture.md):
typed zero-to-four slots, transactional companions, exact required replies and
slot-local companion failure, sibling survival and shared failure/cleanup.
A companion failure disconnects its slot; a primary failure closes the receiver.
No Puck bytes, public receiver API or epoch implementation
is advertised.

C4 step 14 uses a [compiled demo catalog and shared topology editor](spec/demo-topology-editors.md).
Native conversion stays in typed adapters; current-shape cache reconciliation
preserves present holds and drops removed controls. Existing GUI lifecycle test
modules remain unchanged; no public API or live acceptance claim is added.

The step 6 synthetic topology audit covers queued bytes/replies, saturation,
malformed writes, probe/edit overlap, terminal close, delivery uncertainty,
observation eviction, delivery feedback after edits and explicit host STOP.
Seventeen low-seam tests pass with no `gr-hid` runtime change. C5 supplies the
production Wii integration.

## Registry checkpoint C1

The compiled family registry now links the four existing definitions to their
realization surfaces, intended tiers and permission classes. USB/IP descriptors
and audio worker/broker dispatch derive from shared compiled metadata. Closed
profile/controller enums are replaced by compiled IDs. Follow-up regressions check
gadget-tag uniqueness and gadget report lengths against metadata. Audio tag
selection reports missing metadata as an error, and curated UHID creation derives
the bus from its selected USB or Bluetooth realization. These checks do not enable
any undeclared family realization or claim physical Bluetooth. The ignored DualSense
startup probe now checks the kernel HID_ID bus before accepting a rerun; its
BUS_VIRTUAL comparison arm is retired because USB selection owns BUS_USB.
Historical measurements are unchanged. This preserves the
ordinary-root snapshot, broker wire versions and existing descriptor bytes.
[ADR-0018](adr/ADR-0018-compiled-broker-allowlist.md) is Proposed pending owner
review. D5 keeps the unavailable dummy_hcd path experimental and not recommended;
its root/provisioning requirements do not change.

## Acceptance runner failure ownership

Peer service failures observed during another creation's node or SDL wait are
retained on the failed creation and drained into its own sample's errors. They do
not become the observed sibling's missing stage or disappearance failure. Failed
peers are not repeatedly serviced; every creation still receives terminal cleanup.
This deterministic harness repair does not rerun or amend historical measurements.
LiveTopology is already non-Copy in the integrated C3 implementation.

## Acceptance manifests

All six compiled families have per-function TOML manifests under `acceptance/`,
checked against actual declared realizations. They record the scope of source,
virtual-host and physical evidence and keep refusals and missing evidence explicit.

The acceptance `xtask` records monotonic API/node/consumer/removal stages,
raw failures, cold/warm samples and nearest-rank summaries without budgets.
The deterministic fake backend is CI-tested; Linux uses public constructors and
creation-associated sysfs nodes. First-neutral remains unmeasured until a
consumer oracle exists; SDL probe completion is an upper bound, not its exact
discovery instant. Live evidence remains separate.

## Internal live topology

An opt-in retained topology sidecar validates compiled subsets, counts accepted
shape changes and guards absent controls without adding fields to ordinary
sessions. The fifth synthetic family covers attach/detach, atomic rejection,
terminal failure and static-superset uinput neutralization. The Wii root handle
exposes the current topology and retained generation.

## Internal persistent resources

The opt-in typed resource wrapper validates terminal disposition independently
of transport cleanup. The synthetic fifth family proves pre-open import
validation, isolated working values, sticky resource dirty, exact ACK retries and
kept/discarded terminal snapshots. Ordinary stateless sessions gain no fields.
The [integration recipe](spec/persistent-resource-integration.md) maps all scoped
minimum regressions. The Wii package provides the typed resource, and the demo
provides opt-in Save/Load.

## Post-checkpoint follow-ups

C3 [#209](https://github.com/anderstvoss/virtualgamepad/pull/209) landed as
`c82193e`, C4 [#215](https://github.com/anderstvoss/virtualgamepad/pull/215) as
`964038c`, and the research pin [#214](https://github.com/anderstvoss/virtualgamepad/pull/214)
as `7a809e1`. The pinned research revision is `f22d6ef`; these merges do not
promote missing live, consumer or physical acceptance cells.

The small C3/C4 follow-ups are listed in the
[disposition report](plans/pre-controller-buildout/POST_CHECKPOINT_FOLLOW_UPS.md).
Current tracking moved to [#322](https://github.com/anderstvoss/virtualgamepad/issues/322).

## C5 Wii Remote buildout

C5 [#233](https://github.com/anderstvoss/virtualgamepad/pull/233) landed as
`714958d`. The Wii package opens explicit
`linux.uinput` and Bluetooth-bus UHID (bus5) realizations under D6/D7. Original
and TR identities remain distinct; early integrated Plus identity is No evidence.
There is no USB-UHID fallback, radio/pairing/BlueZ or speaker realization.
Speaker support stays separately gated under D7 and ADR-0023.

The controller owns native buttons, raw accelerometer and typed IR point-set
inputs, Nunchuk/Classic and external MotionPlus/pass-through, retained host
LED/rumble intent, raw battery and live attachment generations. The source-backed
codec services legal modes and status/write/read replies; malformed edits reject
atomically. Imported EEPROM has an explicit caller-defined extent and terminal
snapshot/discard policy; no retail size or calibration defaults are invented.
The C5 review follow-up adds MotionPlus-only disconnect/reconnect edges, bounded
Stop/Start attachment handling and atomic ACK/status queue reservation. Repeated
close and Drop call the UHID provider's close only once; focused regressions cover
these fixes. Research records and historical timing measurements are unchanged.

Native evdev creates separate core/accel/IR nodes and prepares optional extension
and MP companions before changing shape. Its source conversions preserve values
beyond advertised ABS bounds and the missing-IR1023/1023 sentinel. Target surfaces
and regression tests document provider bus/fuzz limits, IR metadata, LED/battery,
EEPROM and Classic Pro restrictions. Demo creation and native editing are wired.

The historical C5 UHID probe bound `wiimote` and observed extension changes,
kernel rumble and battery. Its LED writes were permission denied. Later bounded
runs measured selected live axes, obtained temporary per-device grants, and
recorded scoped SDL, kernel and Dolphin results. The Original virtual EEPROM
Save/Load round trip also matched bounded kernel and Dolphin calibration
readbacks against a private reference. Four C5 timing runs yielded 100 N1/N4
samples at `ccb98dc` with resource restoration. First-neutral timing, calibrated
physical motion, broader consumer combinations and general physical fidelity
remain open. [Wii evidence and limits](WII_REMOTE.md) ·
[Original EEPROM acceptance](https://github.com/anderstvoss/virtualgamepad/issues/162#issuecomment-6735616471)

Classic Pro wire identity, early Plus discrimination, encrypted/calibration
register defaults, retail EEPROM size and visible extended/full IR blob metadata
remain unverified. Wii speaker audio is deferred. Actual Bluetooth is outside
the local Bluetooth-bus UHID realization.

## C6 Steam Controller 2026 wired buildout

C6 [#301](https://github.com/anderstvoss/virtualgamepad/pull/301) landed as
`ad80ae6`. It shipped wired USB-bus UHID and reduced uinput, the reviewed
supporting API, synthetic serial identity, and the native demo editor. The
kernel-or-SDL step 16 gate is closed. Wired USB/IP, receiver/Puck and Bluetooth
remain separate work.

D9 records N1/N4 timing, bounded SDL observations, and #304 kernel decoding
and ABS ioctl comparisons for both realizations. Reduced uinput has no sensor
companion and cannot reproduce the kernel's fuzz and resolution attributes.
The wired Steam client follow-ups observed listing and Begin Test with the
compatibility profile. The two legacy empty-attribute controls did not list.
Reduced uinput also did not list in its bounded test. The report distinguishes
listing, UI stability, input delivery and the desktop consent prompt.

Opt-in MIDI/PCM client tests verified virtual payload/feedback delivery and
cleanup. LED tests record bounded brightness and RGBW intent. Physical actuator
calibration, firmware/boot sequence, serial format, cadence and broader game
behavior remain open. [C6 D9 report](C6_D9_ACCEPTANCE.md) ·
[Steam controller scope](STEAM_CONTROLLER_2026.md)
