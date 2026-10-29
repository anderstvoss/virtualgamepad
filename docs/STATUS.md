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
- Shared UHID Bluetooth-bus transport is available, with deterministic curated
  fake-provider coverage; no shipped family declares that personality yet.
  Actual Bluetooth radio, pairing and BlueZ remain deferred.
- Steam acceptance is deferred (#144). Broader GUI review is deferred (#126).

## Next release

Steam S3 (14a/14b) stages native family metadata and planned targets, adopts
the exact fresh physical wired descriptor, and exposes the reviewed supporting
state/topology API. Field meanings and feature replies remain compat; haptic
effects, boot sequence, firmware tracking and physical cadence remain open.
There is no provider, driver, constructor, public registry advertisement or
acceptance cell. Root and Wii snapshots are unchanged;
the separate Steam snapshot includes supporting metadata/topology additions.
Steps 14c/14d and the C6 checkpoint remain gated by step 16 on the owner's
kernel 7.3+ host. See [Steam Controller 2026](STEAM_CONTROLLER_2026.md).

The next release targets validated controller creation per controller, with
per-controller acceptance issues and new Wii Remote, Steam Controller (2026)
and Xbox Series families. Work is sequenced to land the compiled controller
registry (#173) and a repeatable acceptance and timing harness (#166) before new
families are wired in.

The C1 registry checkpoint adds compiled family metadata and a typed registry,
with deterministic Bluetooth-bus fake-provider coverage. It changes no support
cell, public-root API, broker wire bytes or live acceptance claim.

The C3 facade step consolidates application-handle lifecycle and audio-failure
ownership behind private generic state and shared forwarding. Public signatures
and docs remain frozen; existing regression construction changes are mechanical,
as authorized on #176. This refactor changes no support or acceptance cell.

The C3 pre-C5 follow-up validates native attachment configuration atomically and
uses typed topology operations with derived IDs. Deterministic regressions cover
rejected transitions, missing neutralization and absent-control edits on both
backends. This is an internal seam repair, not a Wii implementation or acceptance
claim; fixed-family runtime layout and provider identity remain unchanged.

Step 10c exposes retained-output snapshots on ordinary handles for in-process
UHID and uinput under D2 (#176 comments 6325147128 and 6328169640). The API
snapshot grows from 110 to 122 items with non-exhaustive retained-state and kind
types; selected output metadata respects realization limits. The new native input
kinds remain unused by the four current families. USB-worker views return `UnavailableInRealization` under the owner
scoping decision; recovery is tracked in #203 and needs separate protocol
approval. No physical-output acceptance is implied.

C3 crate tidy extracts audio implementation into `gr-root-audio`, with stable
root reexports and root-specific creation error mapping. The wire crate fold
keeps `gr_controller_wire::dualsense` and its exact bytes. Feature behavior and
dummy_hcd disposition are preserved; no new external dependencies are added.

## Proposed runtime contracts

- [ADR-0019](adr/ADR-0019-live-topology.md): retained live-topology generations,
  with static uinput capability supersets and neutralization of absent controls.
  The production seam remains internal; public additions await owner approval.
- [ADR-0020](adr/ADR-0020-retained-outputs-and-kinds.md): retained accepted host
  outputs and typed surface kinds. D2 was granted on #176; step 10c exposes
  ordinary-handle snapshots and read-only typed capability descriptors.
- [ADR-0021](adr/ADR-0021-presentation-epochs.md): logical controller and host
  presentation epochs, Proposed under D3; step 13a remains deferred.
- [ADR-0022](adr/ADR-0022-receiver-aggregates.md): receiver-owned presentation,
  typed protocol slots, rollback and sibling-safe teardown; no Puck implementation.
- [ADR-0023](adr/ADR-0023-controller-subsystems.md): controller-owned bounded
  in-band subsystems, adopted under D4; no subsystem implementation or public API.

C4 step 13b adds a [synthetic receiver lifecycle fixture](spec/receiver-aggregate-fixture.md):
typed zero-to-four slots, transactional companions, exact required replies and
slot-local companion failure, sibling survival and shared failure/cleanup.
A companion failure disconnects its slot; a primary failure closes the receiver.
No Puck bytes, public receiver API or epoch implementation
is advertised; D3 still defers step 13a.

C4 step 14 uses a [compiled demo catalog and shared topology editor](spec/demo-topology-editors.md).
Native conversion stays in typed adapters; current-shape cache reconciliation
preserves present holds and drops removed controls. Existing GUI lifecycle test
modules remain unchanged; no public API or live acceptance claim is added.

The step 6 synthetic topology audit covers queued bytes/replies, saturation,
malformed writes, probe/edit overlap, terminal close, delivery uncertainty,
observation eviction, delivery feedback after edits and explicit host STOP.
Seventeen low-seam tests pass with no `gr-hid` runtime change. Production Wii
integration and live-topology generations remain separate work.

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

The four compiled families have per-function TOML manifests under `acceptance/`,
checked against actual declared realizations. All usable cells remain WIP; the
dummy_hcd refusal is explicit. Empty pending evidence is not acceptance.

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
terminal failure and static-superset uinput neutralization. Public topology
accessors remain deferred until a real consumer family and owner API approval.

## Internal persistent resources

The opt-in typed resource wrapper validates terminal disposition independently
of transport cleanup. The synthetic fifth family proves pre-open import
validation, isolated working values, sticky resource dirty, exact ACK retries and
kept/discarded terminal snapshots. Ordinary stateless sessions gain no fields.
The [integration recipe](spec/persistent-resource-integration.md) maps all scoped
minimum regressions; Wii semantics and an optional file helper remain separate.

## Post-checkpoint follow-ups

C3 [#209](https://github.com/anderstvoss/virtualgamepad/pull/209) landed as
`c82193e`, C4 [#215](https://github.com/anderstvoss/virtualgamepad/pull/215) as
`964038c`, and the research pin [#214](https://github.com/anderstvoss/virtualgamepad/pull/214)
as `7a809e1`. The pinned research revision is `f22d6ef`; these merges do not
promote missing live, consumer or physical acceptance cells.

Owner-directed small follow-ups precede any new checkpoint: receiver slot-failure
isolation (#165), remaining C1 registry checks, and documentation/harness repairs.
The receiver still exists only as a synthetic fixture, so #165 remains open.
ADR-0023 is adopted under D4. The owner-directed C5 session is described below.

## C5 Wii Remote buildout

The supporting `gr-curated-controllers::wii_remote` package opens explicit
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

On Linux6.12.113+deb13-arm64, the bounded host probe at ccb98dc bound `wiimote`,
matched all six interface names and key/axis masks, observed Nunchuk-to-Classic
replacement and standalone MP, and verified power_supply presence and kernel
evdev rumble reaching retained intent. LED-class writes failed permission checks.
Live absinfo/fuzz and sensor delivery, SDL HIDAPI, Dolphin, physical fidelity and
calibration remain unmeasured. The [Wii report](WII_REMOTE.md) and acceptance manifest record38 feature rows
across both realizations. Four10-cycle N1/N4 runs yielded100 creation/removal
samples at ccb98dc with resource restoration; consumer/oracle gaps and slow
early-initialization UHID removal remain explicit. There is no latency budget.

Classic Pro wire identity, early Plus discrimination, encrypted/calibration
register defaults, retail EEPROM size and visible extended/full IR blob metadata
remain No evidence. No firmware cadence is guessed: application edits/commits and
host requests drive reports. Research#16 is merged; research#17 publishes native
conversion evidence and bounded host/timing records without physical promotion.
