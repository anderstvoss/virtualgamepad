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
- Bluetooth personalities and actual Bluetooth are not available.
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

## Proposed runtime contracts

- [ADR-0019](adr/ADR-0019-live-topology.md): retained live-topology generations,
  with static uinput capability supersets and neutralization of absent controls.
  The production seam remains internal; public additions await owner approval.
- [ADR-0020](adr/ADR-0020-retained-outputs-and-kinds.md): retained accepted host
  outputs and typed surface kinds; public additions remain gated by D2.
- [ADR-0022](adr/ADR-0022-receiver-aggregates.md): receiver-owned presentation,
  typed protocol slots, rollback and sibling-safe teardown; no Puck implementation.
