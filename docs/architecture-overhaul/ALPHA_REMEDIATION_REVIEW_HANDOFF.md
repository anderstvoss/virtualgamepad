# Independent alpha remediation PR review handoff

## Objective and candidate control

The complete-resolution continuation adds fail-closed dummy_hcd admission and
portable acceptance tooling. Review [the lab procedure](ALPHA_ACCEPTANCE_LAB.md)
and the continuation section of the status document before interpreting historical
results below. The permitted maintenance window supersedes earlier read-only host
restrictions, but no administrator-run trial has passed in this continuation.
Native acceptance may close supported audio while this VM remains unqualified;
retain VM failures and never claim VM continuity from native results.

Scrutinize the runner's current-PID/backlog checks, rollback after partial setup,
identity-bound cleanup, global-lock preservation, candidate image provenance,
non-root client/worker identities and original service restoration. Verify that
fingerprints include the actual configured ExecStart executable, rather than an
absent package-name alias; temporary instance names must obey the broker grammar
and failure receipts must preserve bounded stdout/stderr. Verify the
setpriv drop occurs before validator execution, output quotas fail visibly, and
private audio runtimes reject symlinks or shared permissions. Test phase
transitions are deterministic evidence; real systemd/mount/VHCI behavior still
requires privileged receipts. Supplying an unauthorized UID does not execute a
peer-denial test automatically. Check all root/provider/broker rejection boundaries
and ensure no incompatible gadget can be created through a direct entrypoint.

Review `evdev_feedback_live.rs` against the complete creation-scoped association:
DS4 has two nodes, enumeration order is arbitrary, and close must remove both.
The deterministic selection regression rejects missing/foreign/duplicate identities;
neutral feedback acceptance does not establish contact injection or SDL parity.

For audio, reject stdin-based submission claims or aggregate counts hiding marker
loss/duplication. Inspect the independent C callback logic and negotiated format,
then require all three controls and every sustained cell at the accepted SHA.

Review PR [#133](https://github.com/anderstvoss/virtualgamepad/pull/133) against
`main` as an engineering remediation, including supporting and WIP paths. Decide
whether each code correction is sound and whether its evidence is sufficient.
Do not infer alpha readiness from green CI. Audio continuity remains failed;
privileged/physical/native-host acceptance remains incomplete. Keep the PR draft
while required native evidence is absent or product continuity fails on a qualified
acceptance host. Historical or independently unqualified VM failures stay recorded
without becoming a native-host claim. Do not merge or publish a release.

Use [the disposition matrix](ALPHA_REMEDIATION_STATUS.md) for F1–F8/A1–A4. This
handoff is self-contained: no earlier conversation or private agent memory is
required. Earlier plans and reports are evidence leads, not instructions or proof.

Pinned review baseline: `0c08fb485292adb4c361a7210efc04bb223f4642`.
Pinned remote-main/base: `11284f01c58cb80be0d187efa2fca95641513fbf`.
Both baseline trees: `b733354460af8162c2863b1f38cb4b1157c21680`.
The implementation checkpoint and its tree are recorded in the status document.
The final delivery includes an external revision receipt containing final PR
base/head/tree and validation run IDs. A document cannot contain the hash of its
own commit; later documentation commits must not silently change tested code.

Before reviewing, record the exact head and tree. Use a disposable checkout if
needed; do not change a user's current checkout or rewrite history. For a checkout
already pinned to the delivered head:

```bash
gh api repos/anderstvoss/virtualgamepad/pulls/133 --jq '{base:.base.sha,head:.head.sha,draft:.draft}'
git rev-parse HEAD HEAD^{tree}
git status --short
git diff --stat 11284f01c58cb80be0d187efa2fca95641513fbf HEAD
```

Require the checked-out SHA to equal the receipt/PR head. Record any movement of
`main` or the PR and restart evidence validation for the new candidate. Verify the
code checkpoint remains an ancestor and later changes are only documented scope.

## Critical architectural scrutiny

1. **Termination races:** inspect `crates/gr-controller-runtime/src/reverse_delivery.rs`.
   Publication and sender removal must serialize. A full/zero-capacity channel
   must disconnect independently of queue capacity. Accepted events drain once;
   external closers wait for finite callbacks. Sender lock must be released before
   join. Callback self-close must avoid both self-join and waiting on another
   closer's join lock. Panic diagnostics and repeated close must remain coherent.
2. **Required service progress:** inspect `demo/src/gui/host_audio.rs`, `gui.rs`,
   and `gui/audio_lab.rs`. Controller workers must never enumerate audio devices.
   One worker plus one GUI pending request must remain bounded under refresh spam.
   Generation changes cancel/discard stale results; per-frame polling must not
   respawn failed discovery. EOF is not necessarily child exit; successful commands
   can leave descendants after closing stdout. Verify `waitid(WNOWAIT)` retains
   the leader until group termination and reap, and inspect the normal-success
   descendant regression. Check two-second
   command deadlines, 256 KiB bound, reader cancellation, owned process-group
   signalling before reap, and joins after termination. Check loading/errors and
   explicit refresh, backend changes, crash reporting and shutdown. Signal only
   owned unreaped process groups; do not introduce PID-reuse signalling races.
3. **Lifecycle delivery:** inspect `gr_hid::Runtime::service_inner`, the protocol
   lifecycle hook and all curated implementations/root output conversion.
   Required state handling precedes observation; output callbacks run after
   service. Preserve Start/Open/Close/Stop ordering and bounded overflow accounting.
   Application close must not fabricate host events or defer required replies.
4. **DS4 parity and ownership:** inspect `dualshock4/evdev.rs`, the DS4 driver
   native-open hook and root topology conversion. Check capability separation,
   both slots, tracking/release, legacy coordinates, SYN framing, full-snapshot
   retries and feedback completion. Reject silently discarded controls. Partial
   creation must close the first node, and final close must remove both exactly
   once without touching siblings. Association IDs must describe the requests
   actually sent to providers. Primary surface covers complete logical controls;
   separate-contact SDL limitations must be explicit. Live production consumer
   acceptance is blocked, not claimed from the prototype or synthetic tests.
5. **Marker accounting:** inspect `pipewire_live.rs` and `support/marker_source.rs`.
   Count generated/accepted markers, graph discontinuities, queue loss/underruns,
   per-marker client delivery, warm-up and bounded drain independently. Equal
   total frame counts cannot hide loss plus duplication. Missing/duplicate/corrupt
   frames must fail. The prefilled `usb_audio_probe --protocol-fixture` is an
   explicit deterministic test mode, not a streaming fix or live acceptance pass.
   Scrutinize simultaneous Samples/NativeClient combinations and teardown.
   Also inspect the direct graph control: it must bypass `Session` and library
   queues, select only its named sink, preserve exact marker/latency assertions
   and terminate both test clients. Shared Rust helper failures remain possible.
   Independent C-client loss supports a failed host graph baseline, not a claim
   that the library is defect-free or that every failure has one cause.
6. **Actual MSRV selection:** inspect `.github/workflows/ci.yml`, the five
   conditional rewrites and final job log. The compiler must actually be 1.85.x
   despite the repository's newer override; installing a compiler alone is
   insufficient. Reject a green job without the actual version/environment.
7. **Workflow and SBOM validity:** inspect Provider Tier B commands against Cargo
   metadata and run their current entrypoints. The disabled privileged job is
   missing evidence. Check necessary build dependencies and restricted endpoints.
   Verify all 17 SBOM package identities, including root, in the downloaded
   exact-head artifact. Missing/duplicate/stale/mismatched reports must fail.
8. **Cleanup and hostile inputs:** review broker framing/peer/admission/FD tests,
   terminal capability invalidation, partial construction and initiating plus
   cleanup errors. An unsupported dummy_hcd GET_REPORT must not silently succeed
   or wait until timeout. The current f_hid UAPI has only report ID and no explicit
   STALL/type/length interface; terminal unbind is not an invented negative reply.
   Full report-type parity and installed recovery remain unresolved. Never release
   a reservation merely because a failed cleanup path returned an error.

No new dependency-version policy, advisory suppression or broad architectural
rewrite is intended. The demo's direct rustix process feature uses the already
locked crate to safely terminate discovery process groups. Supporting lifecycle
trait and curated association struct changes require downstream SPI migration;
ordinary root signatures must remain frozen.

## Reproduction and required results

Run from the repository root. Keep reports outside tracked source:

```bash
ALPHA_EVIDENCE=$(mktemp -d)
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
gitleaks detect
RUSTUP_TOOLCHAIN=1.85.0 cargo check --locked --workspace --all-targets --all-features
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --all-features --no-deps
python3 scripts/check-alpha-api.py
python3 scripts/check-root-consumers.py
python3 -m unittest discover -s scripts/tests
python3 scripts/check-workspace-commands.py
python3 scripts/check-protocol-corpus.py --verify-remote
cargo audit
cargo deny check
```

Expect success, with the retained unmaintained ttf-parser warning disclosed.
Do not allow suppressions merely to obtain green checks. The consumer check covers
all eight root feature subsets plus cached offline rebuild. Check the inventory
against fresh supporting and root rustdoc source pages; cached root pages can
lack source links even when declarations are identical. Compare semantic public
surfaces separately; do not refresh the frozen snapshot to hide drift.

```bash
python3 scripts/generate-api-inventory.py --check
python3 scripts/generate-api-inventory.py --compare-doc-root target/alpha-api/doc/virtualgamepad
cargo build --locked -p gr-audio-worker
python3 scripts/validate-production-audio-worker.py --worker target/debug/gr-audio-worker
cargo test --locked -p gr-usbip --example usb_audio_probe
cargo build --locked -p gr-usbip --example usb_audio_probe
python3 scripts/validate-usb-audio-worker.py --worker target/debug/examples/usb_audio_probe
python3 scripts/check-alpha-git-consumers.py --revision "$(git rev-parse HEAD)" --report "$ALPHA_EVIDENCE/git-consumers.json"
```

Exact-Git consumers must use the pushed full SHA, never a moving branch. Preserve
the report and generated lock externally. Worker validators must pass all three
profiles and both production FD slot layouts. They establish process/protocol
behavior without kernel attachment, not installed authorization or sustained audio.

For F5, independently put newer-only let-chain syntax in a disposable edition-2024
crate with a newer `rust-toolchain.toml` override. Force 1.85 using the same
step-level environment: `rustc --version` must show 1.85.x and compilation must
reject the syntax. The actual workspace must pass. Inspect the final remote MSRV
log for both rustc/cargo versions and `RUSTUP_TOOLCHAIN: 1.85`.

For F7, inspect final-head SBOM workflow/run identity before download:

```bash
gh run list --branch codex/gui-long-run-lag --json databaseId,name,headSha,status,conclusion
gh run download "$ALPHA_SBOM_RUN" --dir "$ALPHA_EVIDENCE/sbom-artifacts"
python3 scripts/collect-sbom.py --verify "$ALPHA_EVIDENCE/sbom-artifacts/$ALPHA_SBOM_ARTIFACT"
```

Select the run/artifact variables from that exact head; do not substitute a main
or predecessor artifact. Require 17 matching metadata component names/versions
and dependency inventories. Verify negative missing/duplicate/foreign/stale cases
in `scripts/tests/test_collect_sbom.py` and workflow-command negative fixtures.

## Focused regression map

| Finding | Focused commands / assertions |
| --- | --- |
| F1 | `cargo test --locked -p gr-controller-runtime reverse_delivery`; full/zero queues, close/Drop, concurrent publish/close, panic and callback self-close |
| F2 | `cargo test --locked -p virtualgamepad-demo --all-features discovery`; stale/coalesced/no-respawn/crash, bounded commands/descendants and stalled discovery with continued service/removal/shutdown |
| F3 | `cargo test --locked -p gr-curated-controllers all_hid_protocols_observe_lifecycle`; `cargo test --locked -p virtualgamepad --lib`; ordered lifecycle conversion for all four families and existing observation overflow/required reply coverage |
| F4/F5 | Forced minimum compiler across workspace targets/features; newer-syntax negative fixture and actual remote MSRV log |
| F6/F7 | Python suite, command inventory, worker validators, local and downloaded exact-head SBOM identity checks |
| A1 | `cargo test --locked -p gr-audio-linux --features pipewire --test pipewire_live marker`; preserve duplicate/corrupt/partial-frame/producer-drain regressions and explicit protocol probe unit test |
| A2 | `cargo test --locked -p gr-curated-controllers dualshock4::evdev::tests`; presentation, topology, both contacts/releases, arbitrary removal, partial rollback, snapshot retry/feedback; root `supporting_companion` test |
| A3 | `cargo test --locked -p gr-privileged-broker --lib`; all compiled GET IDs reply-or-reject in one poll, terminal invalidation, initiating/cleanup error retention, framing/FD/admission/recovery; existing full HID/USB report-class tests |
| A4 | `cargo test --locked -p virtualgamepad-demo --all-features`; 1,024-position selection/removal plus retained lockout/routing/error/layout/service tests; selected live tests and real GUI soak are separate evidence |

Run selected live tests individually; ignored suites contain child entrypoints.
Never blanket-enable them. Preflight is read-only:

```bash
python3 scripts/host-preflight.py all
timeout --signal=TERM --kill-after=5 30 cargo test --locked -p virtualgamepad --test root_uhid_live all_families_service_start_and_close_siblings_independently -- --ignored --exact --nocapture
timeout --signal=TERM --kill-after=5 15 cargo test --locked -p virtualgamepad --test root_uhid_live root_identity_restoration_uses_fresh_session_and_preserves_logical_identity -- --ignored --exact --nocapture
python3 scripts/run-pipewire-audio-lab.py --timeout 45 -- cargo test --locked -p virtualgamepad --all-features --test root_audio_live all_family_root_audio_creation_service_and_terminal_cleanup -- --ignored --exact --nocapture
python3 scripts/run-pipewire-audio-lab.py --timeout 45 -- cargo test --locked -p virtualgamepad --all-features --test root_audio_live all_family_root_audio_bounded_pcm_and_hid_threads -- --ignored --exact --nocapture
python3 scripts/run-pipewire-audio-lab.py --timeout 45 -- cargo test --locked -p gr-audio-linux --features pipewire --test pipewire_live pipewire_close_during_processing_retains_clocks_and_recreates -- --ignored --exact --nocapture
```

These tests use neutral owned UHID devices and private graphs. Check exact owned
node removal, sibling survival and graph child/socket cleanup after each run.
The status document records completed mapping and graph-failure checks and the
failed sustained/slow-consumer cells. To reproduce mapping safely, run each of
`dualsense_uhid_individual_mapping`, `ds4_uhid_individual_mapping`,
`switch_uhid_individual_mapping`, `xbox_uhid_individual_mapping` in
`gr-curated-controllers --test dualsense_uhid_live`, with `--ignored --exact
--nocapture`, a private `VIRTUALGAMEPAD_SDL_PROBE` and a 360-second deadline.
All four run neutral/button/axis cases without touch injection. Check 156 passing
observations and three cleanup records per test; a timeout is incomplete evidence.
Do not run the Sony full touch scripts on a shared desktop.

Never remove pre-existing resources. Device-node permissions alone do not prove
kernel registration, broker authorization or binary provenance. Do not provision
modules, replace existing services, alter persistent routing or enable privileged
jobs as part of this review.

## Graph baseline control

Before repeating the product matrix, qualify graph continuity separately. The
status document records three failed quiet controls and independent C-client
corroboration. Reproduce the committed control individually:

```bash
cargo test --locked -p gr-audio-linux --features pipewire --test pipewire_live --no-run
VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS=62 python3 scripts/run-pipewire-audio-lab.py --quantum 512 --timeout 75 -- cargo test --locked -p gr-audio-linux --features pipewire --test pipewire_live latency_graph_direct_control -- --ignored --exact --nocapture
```

Repeat three times after building, with no concurrent review builds, profiler or
GUI soak. Record producer totals, missing/duplicate/invalid markers and the exact
binary/revision. Compile warm-up in the example command is excluded from the
marker clock, but direct invocation of the built test binary avoids competing
compiler work entirely. A pass is a graph control, not product acceptance.

For independent reproduction without Rust clients or library queues, create a
private `pw-loopback` pair with explicit `Audio/Sink` / `Audio/Source` names and
`node.autoconnect=false`; select those exact names with playback/record `pw-cat`.
At S16LE, 48 kHz, two channels, feed two seconds of zeros, then 22,500 markers
(each value 1..22,500 repeated for 128 frames in both channels), then two seconds
of zeros. Count each nonzero marker exactly 128 times, reject duplicates, channel
mismatch and partial frames, and retain bounded drain/cleanup. Client stdin
acceptance is not graph-submit accounting; this corroborating control makes no
latency claim. The original fixture/logs remain external evidence. Never create
or route these nodes in the existing desktop graph.

## Sustained acceptance and reviewer rejection criteria

For each of `dualsense`, `dualshock4`, `xbox360`, run three trials per direction:
`latency_graph_source_to_library_samples`, `latency_graph_library_microphone`,
`latency_graph_native_playback`, `latency_graph_native_microphone`. Also run
`latency_graph_duplex` for each of `samples-samples`, `samples-native`,
`native-samples`, `native-native`. Set family/ownership explicitly, preserve each
failure and continue recording the matrix rather than stopping at its first fail.

Example single trial (repeat with independently recorded status three times):

```bash
VIRTUALGAMEPAD_AUDIO_FAMILY=dualsense VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS=62 VIRTUALGAMEPAD_AUDIO_OWNERSHIP=native-native python3 scripts/run-pipewire-audio-lab.py --quantum 512 --timeout 90 -- cargo test --locked -p gr-audio-linux --features pipewire --test pipewire_live latency_graph_duplex -- --ignored --exact --nocapture
cargo build --locked -p virtualgamepad-demo --no-default-features --example gui_soak
timeout --signal=TERM --kill-after=5 7260 target/debug/examples/gui_soak 7200
```

Record warm-up, actual generated frames, graph clocks, queue drops/underruns,
missing/duplicate markers, client counts, drain and teardown. A planned 62-second
source does not alone prove 60 seconds of valid measured continuity. Slow graph
progress, incomplete production or drain timeout is a failure, not an omitted
cell. No missing-marker assertion or p99 threshold may be relaxed. Current matrix
runs overlap the GUI soak/build activity and are VM stress evidence; repeat on a
prepared host without competing review work before accepting latency.

The GUI example performs real rendering with two neutral owned UHID controllers,
per-minute remove/recreate, RSS/FD/thread samples and normal application teardown.
It does not establish manual keyboard accessibility, audio routing stability under
real-device changes, or selection of hundreds of live devices. Deterministic tests
cover arbitrary selection positions and routing/lockout behavior separately.

Reject a claim of complete resolution if any of these occurs:

- A former defect lacks a meaningful regression or a regression is weakened.
- Discovery still blocks required service, leaks readers/children, signals reaped
  process IDs, processes stale results or spawns per frame.
- Subscription termination can race into deadlock or lose accepted finite events.
- Lifecycle callbacks replace required handling, reorder events without loss
  accounting, or synthesize application-close host events.
- DS4 retains SDL classification by silently dropping contacts, loses feedback,
  misreports associated identity, or leaks one node after partial creation/close.
- A protocol request lacks an exact reply or explicit terminal rejection; dummy_hcd
  STALL/type parity is claimed despite the UAPI limitation.
- Producer/graph/queue/client/drain accounting is conflated, marker errors are
  waived, or protocol fixtures are presented as sustained live acceptance.
- MSRV logs show the newer override or SBOM omits/misidentifies a workspace package.
- A skipped privileged workflow, socket availability, issue closure, VM timing or
  historical review is promoted to current required acceptance.
- Final SHA, source tree, exact-Git consumer lock/report or native/security CI
  differs from the candidate being reviewed, or cleanup is not accounted for.

An engineering review may accept the minimal fixes while leaving documented
external gates. An unconditional alpha recommendation requires passing all
implemented paths, including WIP, with the missing live/consumer/physical/native
acceptance supplied. Current failed audio continuity prevents that recommendation.

### Additional closure scrutiny

Reproduce `scripts/validate-broker-rejection-live.py` only through the isolated
maintenance runner. Review its exact error frames, persistent-connection progress,
absolute partial-frame deadline and pipe EOF proof of unexpected-FD release.
`--unauthorized-probe` must be an administrator-reviewed root-owned copy. Check
that only the temporary candidate socket opens to 0666 and that the separate UID
is disconnected by daemon admission before sending a request, with no supplementary
groups or capabilities. A filesystem denial is insufficient evidence for that
case. Both client units must be registered before launch and stopped on failure;
retain bounded client receipts. The currently installed helper does not silently
self-update to this expanded protocol.

Preserve the passing three C controls at source 5091b6e and the subsequent failed
Rust controls. Optimization and short external diagnostic experiments are not
product acceptance. Verify the final isolated two-hour GUI receipt before closing
its gate; a successful short preflight alone is insufficient. Physical, touch,
routing, native-host timing and successful provider recovery remain separate gates.

### Graph input and GUI workload regression map

Check `one_notification_drains_all_ready_markers_and_returns_invalid_buffers` in
the marker-source support module: one notification must drain all pending buffers,
including returning malformed buffers before continuing. The new counters localize
coalescing; three passing trials with zero coalescing do not prove that previous
VM loss was caused by this issue. Production callback changes need separate evidence.
The independently failing C controls still prevent product qualification.

Check `neutral_soak_disables_inherited_audio_before_any_controller_creation` in the
demo. The validation example must force audio-disabled creation options regardless
of feature-dependent demo defaults. Review the final build/resource/cleanup receipt;
interrupted prior runs and a short preflight cannot substitute for two hours. Verify
that owned display, HID and process resources disappear and foreign resources remain.

### Scoped negative acceptance receipt

The installed corrected 84f4d39 lab passes 30 authorized-client cases plus one
unauthorized admission case. Independently inspect both UID/no-groups/no-capability
receipts, original installation identity and service-state restoration. Immediate
EOF is the daemon admission contract; do not substitute an error frame or accept
an OS socket-permission denial. The passing negative suite does not close worker
construction/death, stale leases, restart recovery or successful audio attachment.

Local validation at a8ad387 passes 560 Rust tests (59 explicitly ignored), 138
Python tests and all 18 release/tooling commands. Its downloaded SBOM covers all
17 workspace package identities. Preserve the initial macOS Rust-download failure; its exact-head retry passes,
as do Linux, Windows, MSRV, CodeQL and Gitleaks. The corrected audio-disabled
GUI soak is running; require its full two-hour receipt and cleanup before closing
that gate. b6fdccd's two failed independent C controls still block product audio
qualification despite three passing corrected Rust controls.

### Admission and empty restart extension

Review the validator's eight simultaneous acknowledged connections, short excess
closure deadline, sibling replies and bounded disconnect-slot reacquisition. Wrong
replies must fail immediately, not be retried until they disappear. Check cleanup
on every failing seam. Review `--restart-empty`: changed/nonempty journal identity,
occupied ports and unexplained children must refuse before stopping the candidate;
restart failure must still restore the original service state. Only recorded old/new
candidate PIDs and a repeated passing client establish empty restart/reconnect.
The installed 2c1af2f snapshot passes 33 initial checks, one unauthorized check and
33 reconnected checks, with changed candidate PID and successful restoration.
Worker-death, stale-attachment and successful USB audio recovery remain separate.

### SDL associated-component selection

Inspect the creation-association selector and DS4 companion/foreign/duplicate
regression in `dualsense_uhid_live.rs`. It must select the primary in requested
association order, require all companions and check their removal; unrelated GUI
or physical nodes must neither cause selection nor be removed. Individual DS4
evdev mapping is touch-free; full Sony scripts still require desktop isolation.
The private SDL consumer is pinned to the reviewed 3.2.0 source and its library
hash, with the unused PipeWire audio backend disabled. Retain the initial build
failure against newer headers and do not turn controller-only evidence into audio
acceptance. Require exact candidate/consumer hashes and per-session cleanup records.

### Broker task ownership guard

Verify that idle and empty-restart checks inspect child lists for every broker
thread, not only the leader. Require a present leader and unchanged process start
time/task inventory; metadata failure or PID reuse must refuse maintenance. The
nonleader-child and unstable-inventory regressions must pass. The bounded ordinary-
user live probe detects and reaps its owned child; this is process-inspection proof,
not installed audio-worker recovery. An older root snapshot must not silently
inherit the corrected guard without administrator review and hash-pinned installation.

### C callback diagnostics

Verify that callback-gap accounting excludes startup/drain baselines and reports
client scheduling only. The C self-test and receipt regression must pass. Counters
must not alter stream flags, queues or marker-loss assertions; graph xruns remain
explicitly unknown. Demand fresh quiet measured trials after the GUI and consumer
batches finish before interpreting the added diagnostics or closing qualification.

### Fresh checkout cleanliness

Run the plain acceptance/control CLIs in a fresh Git checkout without per-clone
excludes. Helper imports must not write source bytecode caches or invalidate the
clean-candidate preflight. The sanitized fixture regression verifies both entrypoints;
dirty or changing real source must still refuse. Preserve the initial failed
prebuild and demand a new source/build receipt for the corrected candidate.

### Latest evidence checkpoint

All 18 local validation commands pass at
`9e64801b76666d13cc87182b3639827cb02ef673`, tree
`fa595e94c9aa8eb2a06e73fce1831ea87fbe25e9`: 561 Rust tests, 60 explicitly
ignored live entrypoints and 150 Python tests. Exact-Git consumers/offline rebuild,
exact-head native platform/MSRV/security workflows and all 17 downloaded workspace
SBOM identities pass. Conditional privileged CI remains skipped.

The administrator-installed all-thread guard at
`d307b5c393a2ccd45aae3e96d74ec3644b4bd266` has now passed the 67-check lab.
Inspect the receipt for unchanged installed identities, restored service/socket
state, removed runtime and changed candidate PID. Earlier installation requirements
above describe historical checkpoints, not a pending refresh. Do not extend this
negative/admission/empty-restart result into successful worker or stale-lease recovery.

Seven controls-only SDL tests at `12417df47288048821320b2d526f3c78083422b9`
passed 1,092 observations and 21 cleanup records. Their reviewed-source consumer
initially lacked release-tag metadata; preserve its unknown backend classification.
The separately tagged reviewed build is repeating the seven cases, with DS4 evdev,
DualSense UHID and DS4 UHID passed so far. Require all final receipts rather than
intermediate progress. Check exact binary/library hashes, zero touch injection,
all DS4 companion removals and the absent DualSense evdev individual cell. Full
motion/touch/output/physical acceptance remains separate.

The two-hour GUI run and subsequent quiet audio qualification are still pending.
The prepared 9e64801 audio checkout is clean and its coordinator waits for GUI,
consumer and display cleanup. Reject any claim that preparation or queuing closes
a measured gate. Preserve historical audio failures even if new trials pass.

### Completed controls and scripted-consumer follow-up

The tagged reviewed SDL repeat at 12417df passes all seven tests: 1,092 control
observations and 21 cleanup records. Backend classification is HIDAPI for Sony and
Switch UHID, Linux evdev for Xbox UHID and evdev cases. The added touch-free
DualSense evdev individual test passes 156 observations and three cleanup sessions
at `44a6faa9bce15e7149590ee7aaeaaeab11ddd4e8`, tree
`b835276e1fbacf0d581c99a97c81445b27ce32f7`. Audit both source receipts; combined
counts do not mean eight cells were rerun at the same head. Full Sony contact/output
and physical acceptance remain open.

At 44a6faa, the existing Switch UHID controls/motion and Xbox standard-HID controls
scripts pass, as do Switch/Xbox evdev scripts with exact rumble feedback. All four
repeat three sessions and remove owned devices/reap consumers; no touch injection.
Check sensor timestamps/values and controller-side feedback, not just successful
SDL calls. Switch/Xbox UHID output fidelity and XInput/xpad remain outside this proof.

All 18 local commands pass at 44a6faa and SBOM coverage is complete. Preserve its
failed pinned-corpus CI job: the real rustdoc fixture failed while previously
hiding stderr. The diagnostic regression now requires compiler stdout/stderr to
survive failure. Require a visible cause and exact-head rerun before treating CI
as green; do not skip the real-rustdoc regression or weaken snapshot assertions.

### Sony motion and reverse outputs without active contacts

Review `sony_touch_free_sdl_motion_and_feedback` at
`5f2f59db07252ae9d41888571c5ea46437914e60`, tree
`62aec02635cfa5918332c1fc39b0cdf7eb375b9e`. It passes all 12 DualSense/DS4
UHID/evdev sessions using the reviewed tagged SDL build. Require exact consumer
and candidate hashes, control sweeps, valid distinct UHID sensor observations,
controller-side rumble/RGB observations and all associated-node cleanup. Every
record must have zero touch events; no active contacts may be injected.

The exhaustive disabled-contact regression must pass while the old contact-enabled
scripts retain their transitions. The supporting test helper changes only test
workloads. Do not count this as contact acceptance, other output-mode fidelity,
physical equivalence or evdev HID lightbar/motion support. All 18 local checks pass
at this source (562 Rust tests, 62 ignored, 151 Python); external live gates remain.
