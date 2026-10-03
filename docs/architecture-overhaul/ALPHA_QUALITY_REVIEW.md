# Separate alpha implementation quality review

This block reviews implementation robustness after the post-GUI API audit and
freeze-candidate snapshot. It does not redesign the public contract or perform
privileged installed acceptance. The quality PR head identifies the exact reviewed
source; its parent is the separately reviewable freeze-candidate block.

## Review disposition

| Area | Inspected boundary and evidence | Disposition |
| --- | --- | --- |
| Ownership/teardown | Root controller close/reap/drop, audio Backend, PipeWire Session/ManagedStream, USB Pcm/Bridge, sample stream/control owners; root lifecycle and repeated-close regressions | Required audio lifetime stays coupled to one logical controller; sibling creation/removal is independent; ordinary cleanup retains causes |
| Partial construction | Root creation rollback, PipeWire registration/connect/deactivation, broker Owned/port lease and connection sessions | Fixed loss of disconnect failure after initial input deactivation fails; deterministic stage rollback test retains initiating and cleanup failures and checks one cleanup attempt |
| SPSC ordering | `gr-audio-contract/src/queue.rs`: sole producer/consumer cursors, Release publication/Acquire consumption, atomic sample slots and positions | No cloning of endpoints; exclusive mutable access; producer flush publishes a watermark instead of writing consumer cursor; clock exhaustion rejects before publication |
| Thread/control ownership | Private USB bridge/poller, `client_pcm`, root Send/not-Sync and mutex consumer tests | Stop signals precede joins; locks are held only for their owner operations; native calls/socket deadlines remain external scheduling dependencies, not real-time guarantees |
| PCM arithmetic | Queue capacity/channel bounds, checked frame positions, PipeWire offset/stride/format validation, scratch bounds, complete frames and partial writes | Existing malformed-payload, gap, flush, clock-exhaustion and concurrent queue regressions retained; no frame-versus-sample API change |
| HID/event ownership | Curated/runtime provider service, output conversion, exact report acknowledgements | Required replies remain internal and cannot remain optional caller work; optional observations are bounded and loss is reported |
| IPC/unsafe/FFI | Broker socket wire/FD handling, launch privilege drop, PipeWire safe bindings and buffer lifetime | recvmsg buffers live/aligned; CLOEXEC descriptors are owned/closed on rejection; truncated/ancillary input is rejected; worker launch fixes executable/IDs/capabilities and limits. Installed acceptance remains #115 |
| VHCI ownership/recovery | Reservation and audio journal/connection/session cleanup | Port reservation is not detach authority; retained records need generation/device evidence. Unverifiable resources are retained for administrator recovery, never detached by reusable port alone |
| Platform/package boundary | Root and key crates forbid unsafe; native dependencies are Linux cfg gated; workspace publish policy and MSRV checks | Default library remains minimal; internal packages are unpublished; no private corpus credential/build requirement; cross-platform runtime support is not inferred from compilation |
| Dependency policy | Cargo audit/deny, existing pin/lock and feature closure | Audit/deny pass with existing unmaintained ttf-parser warning and duplicate/license warnings; no new advisory suppression or dependency |
| Dead abstractions | Ordinary root re-exports, experimental instrumentation and retained manifest audio contracts | Manifest metadata remains a distinct SPI purpose; measurement tooling remains experimental; no compatibility shim or new family added |

## Resource bounds and outcomes

- PCM SPSC capacity is 1–65,536 complete frames, with at most 32 channels.
  A full queue accepts a prefix; application microphone writers retry the suffix.
  Irretrievable host playback is counted as discarded frames and a later gap.
- PipeWire queues hold 2,048 frames; scratch is allocated before processing and
  oversized/misaligned host payloads are rejected or split within fixed bounds.
  Underrun silence is observable; it is not delivered-marker evidence.
- USB sample clients use 4,096 playback / 1,024 microphone frames and 128-frame
  work blocks. Native microphone pacing uses consumed media time, not queue capacity
  as a steady-state target. Queue overflow/loss remains reported by existing counters.
- USB transfer payload is capped at 65,536 bytes and 1,024 ISO packets; framed
  limits are validated before payload processing. IPC descriptor counts/types and
  versions are validated; wrong/truncated frames terminate the session.
- Broker admission/port reservations and per-connection sessions are bounded by
  administrator policy; session stop/kill/reap deadlines and socket I/O deadlines
  are explicit in their owners. Privileged crash/restart proof is still #115.
- Optional controller outputs and GUI histories are bounded with loss counters.
  Retained diagnostic strings are supplementary error evidence, not a parsing API.
- Joins keep thread ownership rather than detaching. Host libraries/kernel scheduling
  can still delay shutdown; deterministic bounded tests are not a universal OS bound.

## Fix and contract impact

Previously, `ManagedStream::open` used `set_active(false)?` after registration and
connect. If initial deactivation failed, Drop disconnected but discarded a second
failure. The configuration completion helper now explicitly rolls back both failed
connect and failed initial deactivation, preserving both errors. On success it does
not disconnect. Subsequent Drop sees removed listeners and is idempotent.

The regression injects each stage plus a failed disconnect, checks exactly one
cleanup attempt/relinquished fake ownership, checks typed initiating-error retention
when cleanup succeeds, and rejects accidental rollback of a live stream. No public
signature or intended semantic change occurs: this restores the documented retained
cleanup contract. The root snapshot is unchanged.

## Validation and acceptance boundary

Required locked workspace checks, strict clippy/tests, strict docs, snapshot,
Python tooling, MSRV 1.85, audit/deny and secret scanning are recorded in the final
handoff. Existing protocol, queue, controller parity, GUI, lifecycle and construction
fault regressions are preserved. Ignored host probes are not passes. No additional
native-host, physical microphone, privileged installation or recovery run occurred.
USB/IP therefore remains WIP/excluded from accepted support; matching is unavailable.
This review is prepared for maintainer acceptance and does not close #115/#116/#117.
