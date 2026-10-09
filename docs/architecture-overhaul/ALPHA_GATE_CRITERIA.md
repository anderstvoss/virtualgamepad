# Alpha gate criteria audit

This is the current closure checklist for the approved six-gate plan. It does
not replace failed receipts or broaden a narrow test into a complete gate pass.
Physical-controller comparisons and native-host timing remain post-alpha
issues [#136](https://github.com/anderstvoss/virtualgamepad/issues/136) and
[#135](https://github.com/anderstvoss/virtualgamepad/issues/135). Audio continuity
on this VM remains mandatory; that deferral does not waive it.

## Required closure evidence

| Gate, in execution order | Required pass | Correct rejection or limitation |
| --- | --- | --- |
| 1. Keyboard/accessibility | Every implemented interactive control in the [inventory](ALPHA_GUI_CONTROL_INVENTORY.md): exact keyboard events, traversal, labels, visible focus, release/Hold/suppression, disabled/loading/error states, off-viewport selection/removal; owned-display receipts, lifecycle regressions and the separate final-image two-hour neutral soak with bounded resources and complete cleanup. | Keyboard touch/motion are demo input methods, not physical gestures or sensor fidelity. Read-only outputs need readable presentation, not invented keyboard actions. The current demo does not enable eframe's native accessibility feature; deterministic widget labels are not screen-reader acceptance. Record this limitation explicitly rather than claim full assistive-technology support or silently add a new feature requirement. |
| 2. Controller outputs/isolation | Four families; every implemented representable output/report across HID, evdev and supported USB validation: exact values, replies, retained state and callback order; DS4 companion nodes, contact releases, partial second-node rollback, removal, siblings and repeated cleanup. Preserve accepted two-contact cells when their implementation and binary identity remain applicable. | A transport that cannot represent an operation must reject it explicitly with its technical reason and regression. Do not require invented rumble, RGB, motion or audio features on a family that does not implement them. |
| 3. Provider security/recovery | Authorized positive attachment/FD handoff plus unauthorized/malformed/oversized/FD cases; client death before/after handoff; worker death during construction/after readiness; sibling failure, admission/capacity recovery; initiating and cleanup errors; pending/malformed/truncated/identity-changed restart records; bounded completion and identity-proven owned restoration. | Fail-closed stale-journal startup is expected. Require safe operator restoration; do not require automatic recovery from remembered PID/port/socket numbers. A negative startup pass alone does not prove restoration. |
| 4. USB/IP audio/routing | DualSense, DS4 and Xbox 360 supported directions/layouts, direct ALSA functional markers and worker counters, supported duplex/ownership, route A/B/disconnected/reconnected, endpoint/client/worker failure, HID/audio isolation and teardown. Establish serial/VHCI-bound isolation before attachment; verify shared defaults and foreign resources are untouched. | Unavailable ownership/transport operations require explicit tested rejection. An ALSA command returning zero or short trial alone cannot prove sustained continuity. Unsupported dummy_hcd requires rejection before resource creation, not successful attachment. |
| 5. Isolated Steam | Proven private home/bus/display/device isolation; exact-candidate recognition, available input/output, selection, removal and sibling survival for all four families and available realizations; complete owned process/device cleanup. Record ARM64/FEX scope. | Login/home-screen visibility is preparation, not controller acceptance. The user-authorized retained private test profile may be reused; never substitute the normal account profile. No unspecified-game compatibility claim. |
| 6. Audio continuity | Three separate 60-second measured trials for each required independent C and Rust control, with complete production, zero missing/duplicate/corrupt measured markers, startup exclusion and bounded drain. Then 72 PipeWire trials: 3 families × (4 direction cells + 4 duplex ownership combinations) × 3 trials. Also supported sustained USB/IP direction/ownership cells and slow-consumer/routing/reconnect/failure/teardown checks. Deliberate stalls require bounded counted loss, observable discontinuity and recovery. | Preserve the C control's 1% production-rate envelope and application-to-application p99 <20 ms where the Rust/product test measures latency. The C control does not measure latency. Unavailable xrun instrumentation is unknown, not zero. A failed independent control blocks product qualification; successful compilation cannot override it. |

Every receipt must identify its source revision, executable hashes, environment,
measured scope, actual result and cleanup. Reuse evidence only after checking
that later changes did not affect the accepted cell. The GUI soak specifically
requires byte identity; otherwise repeat it independently of audio and Steam.
Do not reject a gate merely because an obsolete historical paragraph says a
now-accepted cell is pending.

## Evidence and verifier gaps found at `38d7b2f`

- The 7200-second neutral GUI soak completed successfully at source
  `e9b668862170486d0a1bf14aa75d0fe51b737910`, with clean teardown and bounded
  resource plateaus. Its executable and the normal candidate GUI build at
  `38d7b2f86dd9feced70a0ac09dc3d9f93a295067` are byte-identical:
  `c86305950f23f8294bbede493eaec2c279a0cd82f7a7d8cb63e19f78695ff6d9`.
  This closes that cell, not the interactive inventory. Earlier running-soak
  notes are historical.
- `scripts/run-alpha-acceptance.py` covers independent controls, the 72
  PipeWire cells and three family slow-consumer tests. Its aggregate `passed`
  cannot close the whole six-gate review, USB/IP sustained acceptance or every
  required routing/failure scenario.
- The Rust graph/product harness currently repeats a marker across a 128-frame
  block and counts occurrences by block. `marker_frame_errors` in
  `crates/gr-audio-linux/tests/pipewire_live.rs` cannot detect one lost frame
  replaced by one identical replay within that block. The producer in
  `tests/support/marker_source.rs::fill_markers` confirms the identical sample
  encoding. A green result from that accounting is insufficient proof of the
  approved per-frame continuity requirement. Strengthen the producer/consumer
  scheme with deterministic balanced-loss/replay regressions before declaring
  the product matrix accepted; preserve existing assertions.
- Rust graph/product checks verify planned production and exclude startup by
  timestamps, but their finite 62-second generation plus drain deadline does
  not by itself prove a 60-second measured steady-state rate. Record and verify
  the actual measured production duration/rate rather than relying on a test
  name, sample total or deadline alone.
- All recent C diagnostics still fail. An external experimental realtime
  callback variant generated/submitted 144000 frames, received 141952 and
  missed 2048, with 3.560425044 seconds of production for three planned
  seconds. Verified realtime scheduling is causal-investigation evidence,
  not qualification. It also uses an experimental executable outside tracked
  source and cannot certify the maintained candidate.

No loss, duration, latency, isolation or cleanup threshold is relaxed by this
audit. All six gates remain open, with the neutral soak cell accepted. Final
locked workspace/MSRV/docs/API/Python/corpus/dependency/worker/consumer checks,
exact-head platform/security CI and all 17 SBOM identities remain additional
release evidence, not substitutes for these acceptance cells.

The C runner now requests its bounded private graph/thread snapshot for every
trial and refuses to overwrite an existing aggregate report. Two deterministic
regressions check diagnostic selection and evidence preservation. All 295 Python
tooling tests and the five mandatory fmt/check/Clippy/test/Gitleaks commands pass
locally for this audit increment. These results validate tooling and documentation;
they do not close live acceptance or constitute final-head remote validation.

## Rust graph marker-accounting correction

The criteria audit's 128-frame-block alias is corrected in the graph-driven Rust
control and all graph-driven PipeWire matrix cells. Stereo frames carry a
base-32767 index; four-channel frames also verify complementary rear channels.
Mono carries a sign-tagged two-frame index pair, whose state is retained across
buffer boundaries. Unique observations are kept in preallocated storage; measured
replay/out-of-order arrivals, malformed pairs and channel corruption fail instead
of canceling missing frames in block totals. A mono pair proves its two-frame
encoding, not independent identification of two identical-valued scalar samples.

Five new deterministic regressions cover balanced loss/replay in all layouts,
every frame-boundary split, partial pairs/corrupt channels/reordering, startup
replay and silent drain, and index-base rollover/bounds. Previous producer,
partial-frame and coalesced-buffer regressions remain. The focused test binary
passes 15 deterministic tests; 19 host-dependent tests remain ignored by that
invocation. Source/capture callbacks allocate no new marker storage and perform
no logging or blocking I/O. Production library APIs, queues and GUI code do not
change. The historical pipe-driven latency examples retain their old encoding;
they are not the graph-driven 72-cell acceptance matrix.

This correction needs fresh live graph/control/product evidence. It does not
resolve observed graph loss, qualify the host, or fix the separate measured
production duration/rate verification gap. Preserve failed receipts and the
zero-loss and p99 thresholds. All six gates remain open; the accepted byte-identical
neutral GUI soak is unaffected.

## Fixed measured production window

The graph-driven Rust control and all graph-driven matrix cells now select
measurement by planned marker position: the first 750 blocks (96000 frames) are
warm-up; the remaining 22500 blocks at the 62-second setting are exactly
2880000 measured frames. Session/discovery/startup delay cannot change which
markers are included. Repeated priming, startup and silent drain remain separate.
The capture decoder uses that same boundary for replay/order checks and latency
samples. Historical pipe-driven examples retain their original behavior and are
not the sustained graph matrix.

Before percentile processing, the shared graph assertion checks every planned
block timestamp is present, measured timestamps are monotonic, and the observed
measured production span falls within the unchanged 1% rate envelope. The span
uses first/last block handoff timestamps plus one nominal final-block duration;
it is block-resolution producer evidence, not a new converter or native-host
latency measurement. The existing exact generated-frame assertions, indexed
marker checks, missing/duplicate/corrupt rejection, bounded drain and application-
to-application p99 <20 ms remain. Callback gap diagnostics use the fixed marker
boundary rather than time since session creation.

Four new regressions cover exactly 60 nominal measured seconds and incomplete
planned timestamps, inclusive rate bounds and just-outside/reversed/malformed
windows, delayed startup classification, and repeated handoff timestamps in
128/256/512-frame buffers. The focused binary passes 19 deterministic tests with
19 host-dependent tests ignored. This repairs the duration verifier gap; it does
not qualify the host or accept any of the failed live trials. Fresh pinned live
controls are still required before the 72 product trials. Production APIs,
queues, GUI code and dependencies are unchanged.

## Idle/wake discrimination at `9202985`

Bounded external diagnostics use source revision
`9202985173f5c29ddb7fc8c639ca6a06eff94b1f`. Their experimental C executables
are outside tracked source and are not maintained-candidate qualification.
Sharing one client context/core and realtime callback thread still missed 3072
of 144000 submitted frames; production took 3.521407294 seconds for three
nominal seconds. Separate client contexts alone do not explain the failure.

A separate owned periodic-wakeup control, without PipeWire or product queues,
measured 282 absolute 10.667 ms deadlines. Observed p99 lateness was 23.863 ms
with ordinary scheduling, 23.281 ms with verified round-robin priority 20, and
29.671 ms with that realtime child pinned to one virtual CPU. Each control
used about 5 ms of CPU in three seconds. An ordinary-priority active clock
control pinned to that CPU consumed about three CPU seconds in three wall
seconds and observed no consecutive clock-read gaps of 1 ms or more.

An ordinary-priority active peer on the same CPU reduced the realtime wakeup
control's p99 lateness to 0.050 ms. This is a changed measurement condition,
not a proposed product background workload. Moving the active peer to a
separate virtual CPU retained 25.201 ms p99 wakeup lateness. Keeping another
CPU active was insufficient in that trial; the observed improvement depends
on the tested wakeup CPU remaining active. The corresponding experimental
shared-core audio comparison retained the same format, quantum, marker and
rate assertions:

| Three-second diagnostic | Missing / duplicate / corrupt frames | Production span | Result |
| --- | --- | --- | --- |
| Pinned quiet graph/client | 0 / 0 / 0 | 3.375144710 s | Failed the unchanged 1% production-rate envelope |
| Same pinning with bounded ordinary active peer | 0 / 0 / 0 | 2.997339418 s | Short diagnostic passed; qualification unperformed |

The source callback gap fell from 35.217 ms maximum in the quiet case
to 10.743 ms with the peer. This supports idle/wake behavior as a causal lead;
it does not identify the guest-kernel or hypervisor mechanism, prove every
historical loss has that cause, or establish a quiet-host correction. Realtime
priority and pinning alone did not meet the required rate. No queue, loss,
rate or latency requirement changes, and no CPU-burning helper is added to
production or accepted as a substitute for quiet qualification.

Every owned diagnostic process was reaped; the recorded PID inventory was
absent after teardown. Shared services and scheduling configuration were
unchanged. About 18 GiB was available, with no concurrent build, GUI soak or
Steam trial. Raw source/binary hashes, event ledgers, grants, receipts and the
comparison remain outside tracked source. Next investigation must discriminate
idle timer delivery from activation scheduling and demonstrate a supported
quiet-host correction before three 60-second controls or product acceptance.
All six gates remain open; this evidence does not invalidate the accepted
byte-identical neutral GUI soak.

## Installed candidate functional and recovery passes at `d4d8864`

The administrator installed revision
`d4d8864a151b167046c38106f2295594569909e7`, tree
`c061531a2c343dd532ad0670b5fd17861a217f26`, with the same fixed named actions
and immutable root-owned payloads. Candidate broker/worker hashes were verified
before each bounded phase. Three individually run phases now pass:

| Phase | Accepted scope | Remaining boundary |
| --- | --- | --- |
| USB functional | DualSense, DS4 and Xbox 360 direct ALSA playback/capture; 144000 measured microphone frames per family were exact, with zero silence/gaps, and exact playback markers. Quiescent capture counters reconcile 288000 completed/consumed host frames (warm-up, measurement and edge capture), zero abandoned frames and 384 unconsumed submitted frames. Shared defaults unchanged; initiating/cleanup errors absent. | One three-second measured trial per family, not sustained continuity, route transitions or every ownership mode. |
| Worker death | Actual staged worker killed after readiness for each audio family; exact terminal worker-death error, broker/PCM EOF, descriptor baseline, attachment cleanup and subsequent normal/repeated/abandoned sessions. | Construction-time worker death and combined initiating/cleanup failure remain unaccepted. Forced sibling failure was subsequently accepted at the exact revision below. |
| Broker death | Actual staged broker killed with an established session for each family; worker termination, connection EOF, hostile pending/truncated/malformed/identity-changed journal rejection and held-identity operator restoration, then successful subsequent sessions. | Fail-closed startup plus owned restoration does not permit automatic cleanup of unprovable records. |

Every phase returned zero and its independently retrieved root-owned receipt
reports `passed` at the exact installed revision. Cleanup and original service
restoration commands succeeded; raw client output, counters and receipts remain
outside tracked source. Broker hash:
`f1b6f56178e5a971c42b28ee5d8d5a4026e820850687bc37d0abe91c38d35cfa`;
worker hash:
`8d698dd974b117ad677f42762aa3e9566be95440411e0a9a74ff4db34542be0c`.

These results supersede the first failed short USB microphone cell for current
functional evidence, while preserving that historical failure. They do not
prove why its producer stalled or resolve independent quiet PipeWire controls.
All six full gates remain open. The installed lab still has eight predefined
phases; additional construction/failure, routing, input and Steam scenarios
require reviewed preparation and appropriate immutable deployment. No broad
sudo has been enabled. A revocable fixed-mailbox root lab-code updater has been
authorized and prepared; administrator bootstrap and live update acceptance are
still pending. See [updater scope and revocation](ALPHA_LAB_AUTONOMOUS_UPDATES.md).

## Forced sibling recovery acceptance

The installed candidate `f05058c053423eb78b6e3580e0026d5f91309162`, tree
`38cc53e8bb7a014e0165e6687055c5ce50f4811e`, passed the expanded
`provider-siblings-admission` phase. Its broker and worker hashes match the
preceding table. For each audio family, the lab killed one ready worker while
three live sibling sessions retained their attachment identities and answered
fresh diagnostics. The failed session reported the exact SIGKILL terminal error
and connection EOF; descriptor accounting and owned cleanup passed.

A replacement occupied the recovered fourth slot; a fifth request received the
exact admission-limit rejection. All surviving and replacement sessions answered
fresh diagnostics, then closed normally and repeatedly. The subsequent normal,
repeated and abandoned sessions passed. The phase exited zero, the separately
retrieved root-owned receipt reports `passed`, and original service/socket state
was restored. Raw receipts remain outside tracked source.

This closes the forced-worker sibling survival and capacity-recovery cells only.
It does not establish construction-time failure, combined initiating/cleanup
failure, sustained audio or another release gate. All six gates remain open.

### Construction-failure acceptance and combined-error extension

Candidate `b8a76a3a4ae9c4f17d573ff121e8458c787f69a3`, tree
`90ef4ed1e8a6f87147f6a1d78993d6ed899276f8`, passed the expanded
worker-death phase. Production workers were killed through validated pidfds
before client handoff for all three audio families. DualSense and DS4 returned
exact startup EOF errors; Xbox 360 returned the exact SIGKILL worker error
during factory construction. Each broker connection terminated, no descriptor
handoff occurred, and subsequent normal/repeated/abandoned sessions passed.
The independently retrieved root-owned receipt reports `passed`; cleanup and
original service restoration completed. Raw receipts remain outside Git.

The next extension deliberately replaces a held, test-owned post-readiness
journal inode before killing the worker. It requires the exact initiating
SIGKILL error plus `attachment cleanup: audio ownership record changed; retained`,
then proves attachment removal and restores only independently held fixture
identities. Changed identity/content, an occupied backup, late handoff or
missing restoration acknowledgement fail. Three additional deterministic
regressions cover these boundaries. Live combined-error acceptance is pending.
Ordinary-root APIs, production binaries and queue/loss limits are unchanged.
