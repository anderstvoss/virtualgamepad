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
