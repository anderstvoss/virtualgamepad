# Demo audio integration and maintainer checkpoint

## Implemented application consumer

The demo consumes the ordinary root audio API. It adds optional feature forwarding
for `audio-pipewire` and `audio-usbip`; no external dependency is added. The root
and demo defaults still leave audio features off. USB/IP remains opt-in WIP.

The sidebar offers audio disabled/emulated and direction-wide Samples/NativeClient
ownership. Choices apply to new controllers. Unsupported family, feature and target
combinations reject before factories are called. Existing sessions keep their own
creation settings; the selected session can be recreated with current sidebar
choices. Known-invalid recreation choices leave the original open. Once replacement
creation starts, failure leaves that session closed; siblings remain independent.

The selected-controller panel shows typed component association, endpoint direction,
rate, semantic channels/group, clock domain, ownership and creation-scoped selectors.
Copy selectors only for this creation, and resolve anew after close/recreation.
Stable retained health/loss/error snapshots are displayed separately from service
metrics. Graph/bridge instrumentation does not drive application behavior.

For Samples playback the worker drains at most 512 complete frames per service
cycle, displaying received frame count, discontinuities and recent peak amplitude.
It does not play audio through a physical device. For Samples microphone a 440 Hz
square-wave test signal at about 3% amplitude is off initially and starts only on
request. Each cycle attempts at most 512 microphone frames, retrying an unaccepted
suffix. Counts describe accepted queue frames, not delivered host audio. These
bounded demo probes are not throughput/continuity/latency acceptance.

NativeClient owners use the displayed selectors in an explicitly chosen external
consumer; the demo does not read/write their PCM. Playback and microphone flushes
are separate actions. Stopping the tone drops its pending suffix and flushes queued
microphone audio; already delivered samples cannot be recalled. Flushing microphone
while the tone remains enabled allows subsequent test frames to be queued. Releasing
inputs never flushes audio. Audio actions use a bounded queue; a busy queue reports
that the action must be retried instead of dropping it silently.

HID servicing precedes bounded PCM work and the deadline is recomputed afterwards.
Every controller has its own worker. PCM/HID failure closes only that logical
controller without waiting for GUI repaint. Stop/removal/recreation joins the worker;
cleanup is idempotent and terminal diagnostics remain inspectable in the GUI and
state dump. Optional display contention never blocks worker service or shutdown.

## Review blocks

- PR #122: baseline and locked completion guidance.
- PR #123: read-only topology and renderer sizing, stacked on #122.
- PR #124: realization/audio contracts and migration, stacked on #123.
- Demo integration follows #124 as its own review block.

Branches contain incremental commits; local work remains on `main`. PRs are open
for review, not automatically merged. No final API freeze or release is declared.

## Hands-on run

On a deliberately prepared Linux UHID/PipeWire host, with normal PipeWire build
prerequisites available:

```bash
cargo run --locked -p virtualgamepad-demo --features audio-pipewire
```

For explicitly prepared USB/IP infrastructure only:

```bash
cargo run --locked -p virtualgamepad-demo --features audio-usbip,audio-pipewire
```

Select HID/UHID and enable emulated audio for DualSense, DualShock 4 or standard-HID
Xbox360. Switch Pro has no implemented audio profile; check that the unsupported
choice is explained. Samples ownership exposes the playback monitor, test tone and
flush actions. NativeClient ownership exposes selectors for an exact external
consumer. Audio options are immutable until recreation.

Review all four families' controls and reverse outputs. Create duplicates and mixed
audio/no-audio sessions in arbitrary order; remove first/middle/last, including rows
below the visible viewport. Release inputs independently of flushes, stop a tone,
recreate with changed ownership, and inspect retained cleanup diagnostics. Verify
that other controllers keep servicing. Test endpoint/audio failure only in isolated
prepared infrastructure, and record consumer and feature configuration alongside it.

Record concrete observations: family/target, ownership, triggering steps, expected
behavior, actual behavior, and whether it changes API shape, controller correctness,
UI ergonomics or support evidence. A successful unattended run is not feedback.

## Acceptance status

**Maintainer hands-on observations: pending.** No observations are fabricated and no
physical fidelity, sustained native continuity or installed USB/IP security/recovery
claim is promoted. Issues #112/#115/#116/#117 remain open. Incorporate actual findings
before the fresh independent whole-root API review. The final inventory/freeze,
separate quality/security pass and exact-revision release validation remain later
steps; API drift workflow changes require explicit authorization.

## Validation

- Locked workspace fmt/check/all-target strict clippy passed.
- Locked all-feature workspace tests passed: 488 passed, 54 ignored. Ignored host probes are not acceptance.
- Demo tests passed with default, PipeWire-only, USB/IP-only and combined features.
- Python tooling tests passed (89). Root-only downstream feature/offline builds passed.
- Policy hooks passed with prepared build-tool environment; history and source
  secret scans passed. No external dependency was added.
- Deterministic additions cover partial microphone writes, accepted-frame accounting,
  typed errors/selectors, unsupported creation and feature/ownership combinations,
  native sample-I/O exclusion, PCM-failure sibling independence, lifecycle action
  priority at arbitrary list positions and retained cleanup-error presentation.
- Existing queue/protocol/controller-parity, GUI input, worker/display contention,
  repeated close and multi-controller regressions remain intact.

These are implementation/consumer results. No maintainer hands-on session or new
physical-reference test has been recorded. Existing audio acceptance limits remain.
