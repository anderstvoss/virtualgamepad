# Demo audio integration and maintainer checkpoint

## Implemented application consumer

The demo consumes the ordinary root audio API. It adds optional feature forwarding
for `audio-pipewire` and `audio-usbip`; no external dependency is added. The root
default features remain empty. The demo defaults enable USB/IP audio and ALSA
host routing; UHID/PipeWire requires `audio-pipewire`. Session audio still requires
explicit creation choices. USB/IP remains WIP.

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
Optional Output routing now sends mapped samples to an explicitly selected host
device. Input routing can capture an explicitly selected host microphone; neither
routing direction is enabled automatically. For Samples microphone a 440 Hz
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
- PR #125: demo integration, stacked on #124.
- Issue [#126](https://github.com/anderstvoss/virtualgamepad/issues/126): dedicated
  audio GUI refinement and actual maintainer acceptance; required for alpha closure.

PRs #122–125 and GUI refinement PR #127 are merged. PR #128 updates the
pre-commit tooling dependency pins. The post-merge baseline is `ec4c2ff`; its CI,
CodeQL, SBOM, Scorecard and history secret scan passed. No API freeze or release
is declared.

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

The dedicated breakout produced concrete maintainer findings: lag after a session
lasting more than three hours, audio routing and layout refinements, universal
momentary button feedback, and a default 30-second touchpad lockout timer. The
maintainer approved the resulting GUI for PR creation and subsequently requested
its reviewed merge. PR #127 contains the fixes and deterministic regressions.

This records acceptance of the reviewed GUI refinement scope. It does not assert
that every family × transport × ownership exercise in [#126](https://github.com/anderstvoss/virtualgamepad/issues/126)
was performed. That issue remains open for checklist reconciliation; its earlier
claim that no observations exist is obsolete. The settled GUI permits the fresh
whole-root API review to proceed; no broad hardware acceptance is inferred.

Issue #112 is closed following the maintainer’s local UHID/audio completion
clarification. Its unchecked native continuity/latency matrix and recorded failed
runs do not establish a timing guarantee. Issues #115/#116/#117 remain open.
USB/IP stays WIP; matching stays unavailable. The final freeze, separate quality
review and exact-revision release validation remain later steps. Workflow changes
for drift enforcement require explicit authorization.

## Validation

- Locked workspace fmt/check/all-target strict clippy passed.
- Locked all-feature workspace tests passed: 488 passed, 54 ignored. Ignored host probes are not acceptance.
- Demo tests passed with default, PipeWire-only, USB/IP-only and combined features.
- Python tooling tests passed (89). Root-only downstream feature/offline builds passed.
- Policy hooks passed with prepared build-tool environment; history and source
  secret scans passed. No external dependency was added.
- Deterministic additions cover partial microphone writes, accepted-frame accounting,
  typed errors/selectors, unsupported creation and feature/ownership combinations,
  native sample-I/O exclusion, PCM-failure sibling independence, USB/IP target
  labels/help and motion refresh parity, lifecycle action
  priority at arbitrary list positions and retained cleanup-error presentation.
- Existing queue/protocol/controller-parity, GUI input, worker/display contention,
  repeated close and multi-controller regressions remain intact.

The original validation above belongs to PR #125. After PR #127/#128, locked
workspace validation passed with 529 tests passed and 56 ignored, plus strict
rustdoc, 89 Python tooling tests and the root consumer/offline matrix. GUI
regressions cover bounded service history, stable layout, stream configuration
without meter-driven recreation, partial captured microphone writes, button
feedback and touchpad timeout/reset. The reported host 5.1/7.1 capture limitation
remains; no physical-reference or sustained timing acceptance is added.
