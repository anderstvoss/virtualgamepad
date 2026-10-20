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
- `linux.dummy_hcd.usb-hid` is unavailable: Linux `f_hid` lacks complete
  GET_REPORT metadata and negative replies.
- Shared UHID Bluetooth-bus transport is available, with deterministic curated
  fake-provider coverage; no shipped family declares that personality yet.
  Actual Bluetooth radio, pairing and BlueZ remain deferred.
- Steam acceptance is deferred (#144). Broader GUI review is deferred (#126).

## Next release

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

C3 retained-output snapshots are available below the root for in-process UHID
and uinput. USB-worker views return `UnavailableInRealization` under the owner
scoping decision; recovery is tracked in #203 and needs separate protocol
approval. No root API growth or physical-output acceptance is implied.

C3 crate tidy extracts audio implementation into `gr-root-audio`, with stable
root reexports and root-specific creation error mapping. The wire crate fold
keeps `gr_controller_wire::dualsense` and its exact bytes. Feature behavior and
dummy_hcd disposition are preserved; no new external dependencies are added.

## Proposed runtime contracts

- [ADR-0019](adr/ADR-0019-live-topology.md): retained live-topology generations,
  with static uinput capability supersets and neutralization of absent controls.
  The production seam remains internal; public additions await owner approval.
- [ADR-0020](adr/ADR-0020-retained-outputs-and-kinds.md): retained accepted host
  outputs and typed surface kinds; public additions remain gated by D2.
- [ADR-0022](adr/ADR-0022-receiver-aggregates.md): receiver-owned presentation,
  typed protocol slots, rollback and sibling-safe teardown; no Puck implementation.

ADR-0023 is a draft subsystem proposal under D4; no in-band subsystem
implementation or public API is approved.

The step 6 synthetic topology audit covers queued bytes/replies, saturation,
malformed writes, probe/edit overlap, terminal close, delivery uncertainty,
observation eviction, delivery feedback after edits and explicit host STOP.
Seventeen low-seam tests pass with no `gr-hid` runtime change. Production Wii
integration and live-topology generations remain separate work.

## Registry checkpoint C1

The compiled family registry now links the four existing definitions to their
realization surfaces, intended tiers and permission classes. USB/IP descriptors
and audio worker/broker dispatch derive from shared compiled metadata. Closed
profile/controller enums are replaced by compiled IDs, without changing the
ordinary-root snapshot, broker wire versions or existing descriptor bytes.
[ADR-0018](adr/ADR-0018-compiled-broker-allowlist.md) is Proposed pending owner
review. The unavailable dummy_hcd path is retained while D5 is open.

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
