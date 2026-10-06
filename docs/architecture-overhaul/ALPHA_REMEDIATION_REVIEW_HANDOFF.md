# Independent alpha remediation PR review handoff

### Shared-driver audio diagnostic and keyboard pad correction

At `f2fb5ca60b42cfebc422b327a9bb78fb4809743e`, a quiet 62-second Rust direct-control trial (two seconds of
warm-up, 60 measured) completed 2,976,000 planned producer frames but lost
4,608 measured markers with no duplicates. It returned 101 with the explicit
continuity assertion before percentile sorting. This is one diagnostic trial,
not qualification, and does not establish why the earlier realtime children
received signal 9. The bounded private graph snapshot shows both linked stream
nodes running under dummy driver 30, both negotiated stereo S16LE at 48 kHz.
The daemon and client data loops had ordinary scheduling, priority zero.
The measured callback gap reached about 31 ms. A different driver or negotiated
sample format does not explain this particular trial; scheduling causality
remains unproven. The snapshot consumes part of the existing test deadline,
has a 1 MiB output quota, never overwrites receipts, and never changes the graph.
Regressions cover malformed/oversized data, deadline reconciliation, occupied
receipts and fast-failure identification. Raw graph/process/binary receipts
remain outside tracked source.

The demo stick and snapping D-pad canvases accepted keyboard focus but ignored
keyboard input. They now use W/A/S/D for cardinal/diagonal positions and
opposing keys for neutral. Arrow keys and Tab retain normal focus navigation.
Releasing keys or losing focus returns momentary controls to neutral; Hold preserves the last position.
Visible focus outlines and labeled widget information expose the controls.
Pure regressions cover full integer ranges, release/focus loss, Hold, opposing
keys, pointer-state preservation, and exact D-pad press/release transitions.
A headless egui regression sends an actual key event, verifies an adjacent button
does not steal focus, then removes focus while the key remains down and verifies neutralization. This fixes these two controls;
touch keyboard access, complete accessibility and live desktop acceptance are
still open. Do not treat this correction as closure of the full GUI gate.


### Installed scheduling experiment: qualification still fails

The administrator-installed `e88a9078612668cbb5350799dc58c16b5680aafd`
lab completed with an aggregate failure. All 67 broker checks and all six
Sony input tests passed again. Its six audio controls failed. C trials completed
2,880,000 planned/submitted frames each but missed 176,128 / 156,672 / 149,504
measured markers, duplicated 512 / 1,024 / 512 and reported 3,072 / 0 / 0
corrupt markers. Production took 61.54 / 61.42 / 61.38 seconds for 60 planned
seconds. The observed C process used round-robin priority 20, priority limit 88
and a 200,000 microsecond realtime CPU bound. This confirms the temporary grant,
not the cause of loss. Graph clocks stayed monotonic at 48 kHz.

All Rust trials completed 2,976,000 planned producer frames but missed
11,264 / 8,704 / 9,216 measured frames, with no duplicated markers. Each
wrapper returned 247 (the child was killed by signal 9) after printing marker
accounting, before percentile results. The realtime CPU bound is a plausible
explanation for that termination, not proven kernel evidence. Independently,
the harness unnecessarily sorted all latency samples before rejecting known
continuity failures. It now rejects invalid, missing, duplicated or mismatched
frame accounting before percentile sorting. A deterministic regression checks
that all four rejection cases leave an unsorted sample array unchanged. The
zero-loss and 20 ms p99 requirements remain unchanged. The corrected harness
has not yet received an installed sustained rerun.

The temporary unit was collected, its cgroup was empty, its identity-checked
workspace was removed and all input-rule cleanup receipts were clean. Original
broker/socket services are active and only their verified journal connection
remains. No broad sudo or persistent scheduling configuration was introduced.
The 18 local checks and exact-head CI/security workflows at `e88a907` passed;
downloaded SBOMs validate all 17 workspace identities. Privileged CI was skipped.
Raw receipts, logs and process-limit evidence are outside tracked source.

The VM remains unqualified; the 72 product trials and sustained slow-consumer
reruns remain gated. Remaining functional work is full provider recovery,
other output/sibling-failure cases, USB/IP audio/routing, complete keyboard and
accessibility checks, and isolated Steam acceptance. Physical comparison and
native timing remain only the two explicitly deferred post-alpha issues.
Reviewer: reject any attempt to close audio using these scheduling settings,
C production totals, or input/provider passes. Check fail-fast assertion order,
preserved limits, negative child exit status, and restoration independently.
Earlier preparation notes below are historical and superseded by this receipt.


### Quiet controls and temporary scheduling experiment

At `d963978977a31a92ecbaa3b9627bd730c422f879`, quiet C controls yielded one
pass and two failures: missing markers 0 / 171,008 / 735,232; duplicates
0 / 1,024 / 5,632; corrupt markers zero; producer durations 60.0 / 61.9 / 67.8
seconds for 60 planned seconds. Measured clocks stayed monotonic at 48 kHz and
advanced with wall time, with different absolute origins. Callback gaps increased
in the failed trials. This does not prove the failure's cause.

All three independent Rust controls failed. Trial 1 completed production but
lost 3,072 measured frames. Trial 2 produced no frames. Trial 3 generated
2,660,352 of 2,976,000 planned frames before its bounded deadline. Controls
bypass product PCM queues. The VM remains unqualified; do not run or accept the
72 product cells merely because one C trial passed.

An owned private-lab snapshot showed ordinary scheduling (including reset-on-fork
flags), priority zero and no realtime priority grant. The next experiment is
`scripts/run-alpha-scheduled-audio-lab.py`: default plan, immutable installed
images, one temporary systemd unit, non-root clients after capability/group drop,
priority cap 88, client round-robin priority 20, 200 ms realtime CPU limit and
540-second total deadline. Its cgroup owns all descendants. Private runtime files
live in an identity-checked owned workspace, removed with symlink-safe operations
only after unit restoration. Signals, startup failures, occupied/changed units,
timeouts and output bounds have deterministic tests. No shared daemon, persistent
scheduling policy, queue, stream flag, continuity or latency limit changes.

This is a diagnostic experiment awaiting administrator installation and live
verification. It does not establish that scheduling explains the failures or that
production clients will qualify. The realtime priority/CPU bounds use the Linux
[scheduling contract](https://man7.org/linux/man-pages/man7/sched.7.html) and
[resource limits](https://man7.org/linux/man-pages/man2/getrlimit.2.html).


### Installed two-contact milestone and audio clock investigation

At `1e07adf04d1fdd3bcb7451256f7f86fc7f0b8991`, the installed lab passed 67
broker checks plus all six input tests. Twelve SDL sessions passed; both Sony
HID families observed contact masks `3` for press and release with motion,
controls, sensors and exact rumble/RGB feedback. Six additional raw evdev
sessions (three per Sony family) verified both tracking IDs, exact coordinates,
movement and release at complete synchronization boundaries on exact associated
nodes. Repeated close and component removal passed. All runner cleanup receipts
were clean and original services/images restored. This closes these specific
two-contact frame cells; full output modes, sibling failure cases and the other
five functional gates still require their own evidence.

Audio controls now record first/last graph ticks, their rate numerator/denominator,
matching monotonic timestamps and clock discontinuities only in the measured
phase. Absolute source/sink tick values may have different units or origins and cannot
be compared directly. C self-tests cover startup exclusion, a valid zero origin,
and clock reversal/rate change. Receipt tests confirm plausible clock diagnostics
cannot excuse missing markers. No acceptance limits, queues or stream flags change.
The next quiet controls must run after prebuilding and input teardown, with raw
failures retained. Instrumentation alone does not qualify this VM.


### Two-contact acceptance expansion

Sony active scripts now exercise both contacts. The HID consumer assertion
requires down/up masks for both fingers plus motion. New individually selected
`dualsense_evdev_two_contacts_and_release` and `ds4_evdev_two_contacts_and_release`
tests read only exact associated component nodes after verified isolation. Three
sessions per family require IDs, exact coordinates, movement and release at
complete `SYN_REPORT` boundaries, followed by repeated close and node removal.
The decoder regression covers every byte split, suppresses incomplete frames,
and rejects `SYN_DROPPED`, negative slots and unexpected slots. These are new
acceptance assertions, not yet a recorded live pass. Do not close the contact gate
until the installed expanded suite passes; retain the earlier one-contact evidence.


### Installed input milestone at eb8d468

The installed lab at `eb8d46852cd0aa20d6689cc2f8b2a6686d730bca` passed all
67 broker checks and all four selected input tests: three sessions each for
DualSense HID, DS4 HID, DualSense evdev and DS4 evdev. Each session passed
SDL controls and exact controller-side rumble expectations, removed owned
devices and reaped its consumer. Both HID families observed first-contact
press/release/motion, changing valid sensors and exact RGB feedback. Evdev
SDL measured gamepad controls and rumble only, not companion touch frames.
All four runner receipts report no initiating/cleanup errors and removed rules;
the temporary rules directory is absent and original services/images restored.

The 18 local checks passed, including forced Rust 1.85, strict docs/API inventory,
all root consumers and exact-Git consumers/offline rebuild. The downloaded SBOM
contains all 17 workspace identities including the root. Remote full-history
Gitleaks identified the fixed hexadecimal regression creation ID assigned to a
variable named `token` as a generic API key. This is synthetic data. Rename the
fixture and suppress only that exact historical fingerprint in `.gitleaksignore`;
retain history and all other scanning. Revalidate remote security on the next head.

This milestone closes the isolation prerequisite and first-contact HID cell;
it does not close two-contact/raw evdev frames, full outputs, sustained audio,
positive provider recovery, USB/IP routing, complete keyboard or isolated Steam.


### Sony live results and compound ownership labels

The corrected `433b6e3df19bbb41bf0ad34f3e69ba3f3365df0f` installed lab passed
all 67 broker checks and nine SDL sessions: three each for DualSense HID,
DS4 HID and DualSense evdev. Sony HID observed first-contact down/up/motion,
controls, sensors, exact rumble and RGB responses. DualSense evdev observed
controls/rumble; the SDL gamepad backend does not expose its touch node.
Every completed session removed its devices and reaped its consumer.

DS4 evdev stopped before injection: compound physical labels lacked the
process marker used by isolation. Root restoration succeeded, all owned nodes
were removed and temporary rule/directory cleanup succeeded. Preserve this
failed cell. The updated compound format is `virtualgamepad/p<process>/<full
creation token>/c<role>`. All 128 creation bits and stable logical unique IDs
remain intact; requested association still gives exact component paths. The
longest label fits the 63-byte UHID physical field. Physical labels are diagnostic
ownership metadata, not peer authorization. Supporting-interface consumers
that parse the former physical format must adapt; ordinary-root signatures
are unchanged. Fresh installed-candidate verification remains required.

Two-contact observations, evdev companion raw frames/releases and remaining
outputs are still open; these short trials do not close the entire contact gate.


### Input isolation trial and correction

At `c44d7cb7f0e7de0f9b40cd26a9b777bb50bae2f5`, all 67 installed broker
rejection/admission/authorization/empty-restart checks passed. Original services
and installed images were restored. The first active-contact trial failed before
injection because the Sony kernel driver leaves input `phys` empty. Rule and
owned device cleanup completed; the temporary rules directory was removed.

The corrected runner matches the process-owned `HID_PHYS` line on the actual
HID ancestor as well as uinput's physical label. Cleanup inventories the same
ancestry. A fake sysfs regression covers empty Sony phys and excludes foreign
and similar-prefix PIDs. The Rust gate reports nodes and properties on failure
and still requires verified isolation before any active touch. An updated
administrator-installed snapshot and successful live verification remain required.
This failure does not establish a product contact defect or acceptance.


## Objective and candidate control

Review the [six-gate VM continuation](ALPHA_VM_SIX_GATE_PLAN.md). Scrutinize client
privilege drop before environment application, PID-specific rules, property checks
before touch injection, WNOWAIT identity retention, changed-rule refusal, bounds
and removal of only owned resources. Installation/live acceptance remain pending.

The release owner now defers native timing to [#135](https://github.com/anderstvoss/virtualgamepad/issues/135)
and physical comparisons to [#136](https://github.com/anderstvoss/virtualgamepad/issues/136)
as post-alpha cleanup. Missing evidence for those two items alone is not an alpha
blocker. No native timing or physical-equivalence claim follows. All remaining VM
functional/continuity/provider/contact/routing/manual gates remain mandatory;
retain historical classifications below as historical context.

Review Rust direct-control callback counts/gaps and deficient-marker location
reporting. They must not change stream flags, queue sizes or acceptance thresholds;
callback-gap phase baselines must exclude startup/drain. The Rust binding does not
expose RAII buffer-return errors, so generation counts must not claim acknowledged
submission. Partial loss offset by duplication must still fail the existing gate.

The complete-resolution continuation adds fail-closed dummy_hcd admission and
portable acceptance tooling. Review [the lab procedure](ALPHA_ACCEPTANCE_LAB.md)
and the continuation section of the status document before interpreting historical
results below. The permitted maintenance window supersedes earlier read-only host
restrictions, and later installed lab receipts record the guarded negative/admission/empty-restart checks.
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
installed-provider/contact/routing/manual acceptance remains incomplete. Keep the PR
draft while these VM gates or functional continuity remain unresolved. Native
timing and physical comparisons are explicitly deferred to #135/#136; their
missing evidence alone must not be treated as an alpha blocker. Historical or independently unqualified VM failures stay recorded
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

## Latest acceptance evidence to scrutinize

Use status checkpoint `b64c4510a7d1cd0f92202a644e2abb04937b98d5` and its
external receipts. Its 18 local checks and exact-head CI/security/SBOM validation
passed; the privileged CI job was skipped. The fresh three C controls all failed
continuity and sustained duration, while Rust controls failed twice and passed
once. Compare source/capture callback counts and marker deficit ranges; do not
turn their correlation with scheduling gaps into a causal claim. The 72 product
cells are blocked, not passing evidence. Preserve the earlier passing C controls
and failed Rust control as separate historical receipts.

The interactive Xvfb receipt covers 21 neutral Xbox UHID creations, off-viewport
selection, sibling deletion, stop-all and visible dummy_hcd rejection. Verify
screenshots and the exact executable hash, and require empty final owned HID
resources, reaped children and removed display/auth. This is XTest plus visual
inspection, not a claim of complete manual keyboard/accessibility, touch, audio
routing or Steam acceptance. Full focus traversal remains unverified. Keep the
completed two-hour neutral soak distinct from this shorter interactive check.

The latest status also records five separately bounded PipeWire functional and
lifecycle tests at b64c451. Review their exact executable hash and per-test commands;
independent cleanup, ownership errors and recreation are passing cells. USB-bridge
activation is a private-graph test with no attached USB/IP transport. Reject any
attempt to promote these short functional passes into sustained continuity or full
provider recovery acceptance.

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

### Completed neutral GUI and final-source audio receipts

Require the full 7,200-second neutral GUI receipt: status zero, no remaining HID
nodes, GUI/driver gone, display socket/lock gone and auth removed. External steady
samples have 16 descriptors/18 threads/no children/no swap, with bounded RSS ending
at 102,148 KiB. Internal FD enumeration temporarily includes its own descriptor.
This accepts neutral resource/lifecycle behavior; manual and audio-enabled GUI cells
remain open. The original a8ad387 image is byte-identical to the 7bf16cc final-source
rebuild; inspect both source receipts, running image hash and the empty production
source/manifest diff. Never replace the original revision in its receipt.

Quiet qualification at `7bf16cc25d14fbce1bef8739e8f8d477403eaf86` passes all
three C controls but fails the first of three Rust controls with 2,048 missing
markers; the other two pass. Graph xruns remain unknown, and C callback gaps are
not end-to-end latency. Require the preserved failed log and explicit 72-cell
blocked disposition; slow-consumer reruns were not performed. No product PCM queue
is present in the failing control. Reject attribution to library queues without
further evidence, relaxed marker assertions, or an unconditional release claim.

Final-head local validation, platform/security CI and downloaded 17-package SBOMs
pass. Preserve the initial Windows certificate-revocation fetch failure and exact-
head retry. Privileged CI skips do not close full provider recovery/USB routing.

## Sequential continuation: keyboard touch

The touch canvas now supports focused Space activation/release and bounded WASD
movement. Hold retains a contact; a new Space press toggles it off. Relative movement
uses the existing composition path. Focus loss and selection changes release only
the keyboard-owned momentary contact. Reset/lockout suppression requires key release
before rearming. The canvas has a descriptive label, visible focus and keyboard hint;
the lockout checkbox itself is labelled.

Five new regressions cover exact quick-press/release events, focus loss, held sibling
preservation, relative edge clamping, reset/removal, selection and suppression, and
actual egui Space/D events beside an adjacent focus target. All 14 touch tests pass.
The five mandatory workspace commands passed before this documentation increment.
No dependencies were added. This is deterministic evidence, not completion of the
interactive keyboard inventory or the final two-hour soak. Gate 1 remains open;
the new GUI image requires a fresh soak. Gates 2–6 retain their previous dispositions.

## Session-aware USB serial prerequisite

The supporting `Profile::for_session` constructor validates a broker instance
(lowercase ASCII letters/digits/hyphens, 1–32 bytes) and a nonzero generation, then
compiles serial `vg-{instance}-{generation:016x}` at string index 3. `Profile::new`
retains its static descriptor and rejects index 3. The ordinary root API is unchanged.

Supporting worker `Setup` and broker `Launch` now require an owned `instance` field
and implement Clone rather than Copy. The installed worker launch ABI now has eight
arguments: PROFILE DEVICE GENERATION IDENTITY INSTANCE CONTROL_FD PLAYBACK_FD
MICROPHONE_FD. Broker and worker must be staged together; older installed images are
not evidence for this candidate. Broker configuration supplies the instance; IPC
clients cannot choose it. Invalid identities reject before readiness/worker launch,
and invalid worker setup closes all four owned channels.

Exact string/control regressions cover all three profiles, descriptor index,
truncation, language, unknown strings and static rejection. Production-worker
validation covers the exact serial on both FD layouts for all three families,
bidirectional synthetic PCM, diagnostics and closure. The new pure isolation-rule
builder uses two udev stages to combine USB serial with platform VHCI ancestry;
rule-injection regressions and `udevadm verify` pass. **The rule is not yet installed
or integrated into positive preparation.** It must precede attachment, and shared
PipeWire defaults/monitor exclusion still require live receipts. The shared-profile
off fallback is not an acceptable closure path.

The scoped administrator snapshot remains unchanged. No positive attachment,
shared routing change, audio qualification or Steam launch was performed in this
increment. The complete named-phase lab package and its preparation/restoration
regressions remain required before requesting administrator installation.

Keyboard touch now also has an actual Tab traversal regression proving that the
adjacent control receives focus and exactly one momentary release is generated.
Gate 1 still requires its complete control inventory, owned-display checks and a
fresh final-image soak. Gates 2–6 remain open; alpha is not ready and PR #133 stays
draft. Physical comparison/native timing remain separate post-alpha #136/#135.

## Motion keyboard lifecycle follow-up

Motion sliders previously completed momentary input only on pointer click/drag
release. Arrow-key release and focus loss now also neutralize motion when Hold is
off; Hold retains the value. A real egui slider regression demonstrates that the
old pointer-only predicate misses key release, and checks actual press/release,
focus surrender and Hold behavior. Previous-frame focus is retained because the
normal egui focus-loss edge can disappear before the next draw after containing
view focus surrender. The same completion helper is used for gyro and accel axes.
This correction does not close the full interactive inventory or soak gate.

## Reboot-aware continuation and bounded acceptance tooling

The VM restarted before the previous GUI soak produced a final receipt. That run
is interrupted, not accepted. A maintained neutral soak runner now records the
boot ID, frozen executable hash, persistent samples and final cleanup; its owned
unit has a 1 GiB memory ceiling and checks global available memory and pressure.
Its 60-second smoke completed with clean teardown and a 243 MiB cgroup peak. A new
7200-second trial is running; it is not accepted until its final receipt and
resource trend have been inspected. Raw receipts remain outside tracked source.

The C independent audio control now assigns a unique stereo block/phase marker to
each measured frame. A regression demonstrates that a missing frame and duplicate
within an old 128-frame block could otherwise cancel in totals. Qualification
requires zero missing, duplicated, corrupt or reordered frames and a complete
bounded event ledger. This corrects a harness blind spot; it does not resolve the
historical 4608-frame graph loss or qualify the VM. No new product continuity
claim is made. Ledger exports happen after teardown, not in realtime callbacks.

Positive provider probes now cover four-session admission exhaustion, sibling
removal and fresh-generation capacity recovery, verified client exit, worker death
and broker death. Root fault injection pins process identity with pidfds, verifies
worker privileges/image/cgroup and journal identity, and rejects stale startup
before clearing only an authoritative matching owned record. Candidate unit
identity is checked before stopping it. Synthetic regressions cover changed
identities, partial construction, combined initiating/cleanup errors and repeated
restoration. These phases have not been installed or run against live attachments.
The old scoped helper remains incompatible with the session-aware serial ABI.

The maintained Steam runner hides the real account home, uses a private bus,
disposable public rootfs and disk-backed private home/tmp, and caps its owned unit
at 2 GiB. Memory guards interrupted earlier bounded bootstrap attempts; retaining
RAM-backed rootfs data was demonstrated in these new attempts, not established as
the cause of the earlier VM reboot. The latest attempt reached the actual Steam
setup window and timed out at 120 seconds, with complete owned cleanup. There is
no login, controller-recognition or Steam acceptance receipt yet. Rootfs extraction
streams regular files, drops their cache and confines archive links to the guest.
No user profile or credentials are copied.

Current checkpoint: 232 Python tooling tests pass; the five required workspace
commands passed before the final Python-only additions and need final-candidate
receipts. No dependencies added. Gates 1–6 remain open, PR #133 stays draft, and
alpha remains not ready. Reviewers must distinguish deterministic supervisor tests,
bounded bootstrap/smoke evidence and live product acceptance. Next interventions
must install one complete immutable named-phase package, not grant broad sudo.

### Immutable provider package follow-up

`package-alpha-provider-lab.py` prepares all eight maintained provider/short USB
phases together from a clean committed source tree. Its default only prints a
plan. The generated administrator installer authenticates all payloads, checks the
existing three-action sudo scope, preserves the old exact helper/policy and rolls
back both atomically on validation failure. The installed run selector accepts
only predefined phase names, never commands, paths, environment or identities.
Client and candidate service units now have hard memory limits. Package tests cover
root/repeated identities, explicit port allowlists, symlinks/FIFOs, tampered hashes,
broad old policies and failed post-install validation restoration. All 238 Python
tests pass. Packaging/installation does not run trials or change services.

This package provides positive provider and short USB functional acceptance, not
complete Steam/controller-output or sustained audio acceptance. Those remain
separate gates. Unrepresentable gadget paths still require explicit rejection.
The new GUI soak is running under the frozen accepted-smoke image; it must not be
restarted on an observation timeout. Its persistent boot/PID/hash record is the
reference until process completion or a verified reboot.

### Direct HID output evidence

`crates/gr-curated-controllers/tests/hid_outputs_live.rs` adds individually opt-in
ordinary-client tests using only process/family/instance-selected hidraw nodes.
No touch, motion or PCM is injected. Each scenario services production controllers,
checks exact host observations, rejects output replay during idle service, closes
twice and verifies that its own HID node disappeared. Live trials use bounded
ordinary-user units with a 256 MiB memory ceiling and 20-second deadline.

| Family / HID output boundary | New exact live evidence | Remaining scope |
| --- | --- | --- |
| DualSense | Four audio-path values, independent mic mute/indicator, speaker/mic/preamp levels, player/RGB indicators, both 11-byte trigger fields, rumble start/update/zero stop and callback ordering | Raw effects and host route observations do not establish physical actuation or PCM routing; evdev/USB parity remains separate |
| DS4 | Rumble start/update/zero stop, RGB validity enabled/disabled and raw retention | Evdev/USB output parity and compound-node failure/isolation scenarios remain separate |
| Switch Pro | Exact motor words on reports 0x10/0x01, player command observation and success reply; USB handshake commands 1–6 with exact 64-byte replies | Encoded words do not establish decoded physical amplitude/frequency or player LED actuation; other report/protocol cells remain separate |
| Xbox 360 standard HID | Existing deterministic surface test explicitly declares no HID output capability | Conventional rumble belongs to evdev; no XInput/xpad or HID rumble claim |

The four named live scenarios passed separately, including owned cleanup. Two
deterministic tests cover exact ownership selection and malformed/truncated reply
rejection. Normal workspace testing leaves the four live tests ignored; run each
by its exact name with `VIRTUALGAMEPAD_OUTPUT_LAB=1`, `--ignored --exact`, a bounded
owned unit and prepared ordinary-user creation/hidraw access. Never blanket-enable
ignored tests. These receipts advance gate 2, not all-realization acceptance.

The administrator package already prepared for 247e6fc remains the compatible
provider/worker candidate: this increment only adds tests and documentation, with
no library behavior or supporting ABI change. Do not claim newer exact-source
provider acceptance from that installed package without final-candidate receipts.

#### CI readiness barrier and portable fixtures

Exact-head 6c43825 native Linux/macOS/Windows builds, Rust lint, MSRV, CodeQL and
full-history Gitleaks passed. Its aggregate CI failed: the actual pidfd child test
could snapshot a process while exec/interpreter initialization was changing its
identity, and a fake home-path literal triggered the private-path policy hook.
The child now signals explicit readiness before reservation. Thirty bounded
owned-child repetitions pass; production ownership checks still reject every
identity change. Home fixtures use a synthetic non-account path. The exact policy
regex finds no matches in tracked text, and all 238 Python tests pass. No workflow
or policy rule is changed. New-head remote CI remains required.

At 6c43825, strict rustdoc, both API snapshot views, regenerated source inventory,
forced Rust 1.85, source-path feature consumers/offline rebuild, corpus provenance,
and production/USB protocol validators pass. The USB validator requires the
`gr-usbip` `usb_audio_probe` example rather than the privileged production worker;
a wrong initial invocation rejected at readiness and is retained as a failed
command receipt. Inventory source links were missing until dependency source
pages and the root documentation were regenerated; no baseline was refreshed.
Dependency audit/policy pass with the audit's allowed unmaintained-ttf-parser
warning (RUSTSEC-2026-0192); do not describe this as a warning-free audit.

The new neutral GUI soak remains live, under its memory cap, and has no completion
receipt yet. Administrator installation of the prepared 247e6fc immutable package
is still pending. All six release gates remain open.

### Scoped installer syntax correction and completed neutral soak

The administrator attempted the 247e6fc package installation. Its exact-scope
check rejected the existing sudo policy because three commands appeared on one
comma-separated line, while the installer accepted only separate rule lines.
The failure occurred before stage creation or helper/policy replacement. The
installer now accepts either spelling, requiring the same account, root run-as,
fixed helper path and exactly one each of status, run and receipt. Extra commands,
arguments, duplicate actions and broad sudo remain rejected before mutation.
Regressions execute the generated installer against sanitized combined/split
rules for both named and numeric accounts, including fail-closed rejection.
No permission is widened and the administrator must install the corrected packet.

The 64222a5 neutral GUI image completed 7,200 seconds with exit zero and empty
cleanup errors. Its SHA-256 is
`e78018adf58977ce26de84e8e4738ac31f76556b86fbf43d9b8c4b4d0cc990ea`;
the current local executable is byte-identical. Both owned processes are gone.
After startup, descriptors stayed at 16 and threads at 23, with no child processes.
RSS was 144,008 KiB at five minutes and 144,456 KiB at the end (448 KiB increase);
the last half-hour increased by 12 KiB. This is a bounded neutral-soak receipt,
not proof of zero allocation growth or acceptance of interactive accessibility.
Host available memory stayed above 19 GiB and measured full memory PSI averaged
zero. Audio, contacts and Steam were excluded. This receipt does not close the
remaining interactive gate or any audio/consumer gate. Preserve the previous
reboot-interrupted run as unaccepted. Raw receipts remain outside tracked source.
