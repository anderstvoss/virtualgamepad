# Alpha remediation status

Current gate disposition: **Gate 3 passed; Gate 2 has a reopened DualSense evdev parity cell (#147)**. Gate 2's current
cell-by-cell audit is in [controller output acceptance](ALPHA_CONTROLLER_OUTPUT_ACCEPTANCE.md).
Gate 3 was revalidated at `4c3b6132d1fdede6dd384bf9dcf70cc9a406c523`
against the rebuilt worker after the public USB audio close correction. Gates 1, 4, 5 and 6 remain open.
See [provider acceptance and regression map](ALPHA_PROVIDER_RECOVERY_ACCEPTANCE.md).
This current evidence supersedes older whole-gate ratings below. Reopen affected
cells after relevant code/binary changes. Alpha remains not ready.

Current closure criteria and verifier gaps are in
[the acceptance criteria audit](ALPHA_GATE_CRITERIA.md). Historical progress
entries below retain their original scope and may be superseded by that audit.


Latest current-worker revalidation: all seven provider phases passed at
`4c3b6132d1fdede6dd384bf9dcf70cc9a406c523`, tree
`8c128314964d43dd9cd8d002c95437e8dd9d647f`. Broker SHA-256:
`f1b6f56178e5a971c42b28ee5d8d5a4026e820850687bc37d0abe91c38d35cfa`;
worker SHA-256: `b1d7339ac23d74de018915e5a3e041bb28b7ba860e8495442dc28eb9921acb7f`.
The original broker service returned inactive and its socket active. USB output
callbacks passed all 24 current requests independently of the failed audio suite.

The complete short functional matrix at that revision is:

| Family | Sample playback: missing/duplicate/corrupt | Native microphone: silent frames / pattern gaps | Sample microphone: silent frames / pattern gaps |
| --- | --- | --- | --- |
| DualSense | 0 / 0 / 0 | 96 / 0 | 6912 / 0 |
| DualShock 4 | 0 / 0 / 0 | 416 / 2 | 4384 / 0 |
| Xbox 360 | 0 / 0 / 0 | 240 / 4 | 5424 / 0 |

Each cell has 144000 measured frames (three seconds), with separately recorded
warm-up and edge capture. All six microphone cells failed. The six remaining
native-playback ownership combinations passed construction/connection/closure
checks only; native playback marker exchange is still missing. The aggregate
correctly returned failure, retained all 12 ownership verdicts and six marker
receipts, and recorded no coordinator cleanup errors. Factory diagnostic output
was 80854 bytes, confirming why the previous 64 KiB cap was insufficient.
Actual sample pacing counters include host silence per the frozen public API;
they must not be confused with the supporting worker's source-consumed counter.
All prior failures remain historical evidence, not overwritten passes.

Gates 1/4/5/6 stay open: full display verification/final-image soak, remaining
routing/native-playback/failure cells, scoped Steam input provisioning/consumer
checks, and independent continuity qualification followed by sustained matrices.

### Current-head infrastructure verification and GUI workload receipt

At `cf710a47a5ca15aa60d5d718037781d6c69cee47`, CI, CodeQL, full-history
Gitleaks, SBOM and Provider Tier B workflows completed successfully. CI included
Linux/macOS/Windows build/test, actual MSRV selection, policy, supply-chain audit
and pinned corpus jobs. The privileged provider job and dependency-review job
were skipped; workflow success is not positive live evidence for those jobs.
All 17 downloaded workspace SBOM identities, including `virtualgamepad`, matched
locked Cargo metadata. Current-worker local provider evidence remains separately
scoped above.

The all-feature GUI and neutral-soak images were rebuilt without overlapping a
live trial. A new 7200-second owned-display soak started from that revision;
the frozen soak image SHA-256 is
`211c492325de801cb9d3a95492db727d9671d93165eecf6d4059b517bab8f7f5`.
Its workload is two neutral DualSense UHID controllers with per-minute removal
and replacement, with audio disabled and no touch injection. It does not cover
all-family interactive operation. The ordinary unit has a 1 GiB memory cap;
initial steady samples are approximately 125 MiB RSS, 16–17 descriptors and
23 threads, with no children or observed memory pressure. This is running
evidence, not a completed or accepted soak.

The soak verifier now requires actual cycle and completion receipts, not only
elapsed time and a zero exit status. It rejects omitted, malformed, duplicate or
regressing cycles, incomplete duration and excessive diagnostics. Repeated close
requests from final egui frames are idempotent and do not count as extra cycles.
At most the final minute-boundary cycle can be absent; a two-hour run needs at
least 119 recorded cycles. Seven focused Python regressions pass. Two historical
logs contain 119 cycles and completion at 7200 seconds; they remain historical
and are not reused for current-image acceptance. The already-running supervisor
loaded its earlier verifier, so its terminal receipt must also undergo this new
workload check before acceptance. Final resource trends and owned cleanup remain
required. No production behavior, GUI image, public API or dependency changed.

The `9f0b6a1bf3a7959a9328cd0e5b23e10e0f21fdef` CI run exposed a
policy failure: the escaped-tab regular expression looked like a UNC path to
the unchanged local-path blocker. The parser now decodes the quoted RSS value
and validates its three fields. Additional malformed-memory cases retain the
seven passing soak regressions. The exact repository path-blocking expression
passes over all tracked text; the complete local pre-commit runner is unavailable
(`pre_commit` is not installed), so a new exact-head CI run must verify all hooks.
The earlier CI failure remains evidence and is not relabeled a pass.

Independent-control preparation now supports explicitly quiet daemon logging
and fixed quantum requests of 128, 256 or 512 in the owned ordinary-user graph.
Root invocation and malformed options reject before allocating resources. Quantum
metadata commands target only that graph, have two-second deadlines and must
succeed before policy/client startup. A metadata timeout reaps the owned daemon
and restores its directory. Existing factory defaults remain unchanged. All 37
broker-audio tooling regressions pass, including each quantum's partial-startup
cleanup. Requested quantum is recorded separately; it does not prove negotiated
rate/quantum or effective scheduling. No independent control has run through
this new option yet, and it cannot qualify Gate 6. The GUI soak continues alone;
there are no overlapping audio trials or local builds.

The ordinary sealed-image channel now has a distinct `D` request for the
maintained `gr-audio-linux` direct Rust graph control. It never executes an image
as root and retains the existing image hash/seal checks and descriptor cleanup.
The coordinator prebuilds/freezes the SDK observer and the independent integration
test separately before entering the existing `usb-functional` phase. Use
`scripts/run-alpha-usb-output-acceptance.py --independent-rust-control` for a dry
run, or add `--apply --report` with a new external directory after provisioning
the exact candidate and completing other live workloads.

Three fresh quiet private graphs request quantum 512. Each selected test gets
62 production seconds with the existing two-second warm-up and 60-second measured
window, a 90-second process deadline and retained live client-thread scheduling.
Only the reporting/test main process uses ordinary scheduling; audio data loops
still request the existing bounded private realtime configuration. Effective
policy must be read from receipts, not inferred from this request. Acceptance
requires complete generation, exact measured frame counts, zero measured
missing/duplicate/corrupt markers, the existing one-percent rate envelope and
p99 below 20 ms. Missing/duplicate summaries, zero selected tests, wrong images
or incomplete trial inventories reject. A measured marker failure is retained
across all three trials; timeout, protocol or cleanup failure aborts safely.
The SDK sample-factory request remains distinct and unchanged. A passing Rust
control cannot turn a failed broader USB functional phase into success, replace
the C controls, or close product acceptance.

The 41 broker-audio and nine coordinator Python tests pass; Rust formatting also
passes. A new sender regression verifies the distinct request and the same sealed
descriptor contract. Its compilation/execution still requires exact-head CI or
post-soak local validation. No new dependencies, production API or queue sizes
changed. No new control trial has run yet; the final-image neutral GUI soak stays
the sole live workload, and the earlier audio failures remain unresolved.

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


## Gate and revisions

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


The current continuation targets all six remaining VM gates. See the
[execution and closure plan](ALPHA_VM_SIX_GATE_PLAN.md). Input isolation and the
active-contact property gate are implemented with regressions; the installed `c44d7cb` trial is recorded below. Tooling does not close acceptance.

### Current alpha boundary: post-alpha evidence follow-ups

The release owner has deferred these two environment-dependent evidence gates:

- [#135: native Linux timing](https://github.com/anderstvoss/virtualgamepad/issues/135).
- [#136: four-family physical comparisons](https://github.com/anderstvoss/virtualgamepad/issues/136).

Missing native timing and physical comparison evidence alone no longer blocks
alpha. Until these issues close, no native timing or physical-equivalence claim is
supported. This supersedes their earlier mandatory-gate classifications below;
historical results and failures remain intact. Audio continuity, installed-provider
recovery, contacts/other implemented outputs, routing and manual GUI/Steam checks
remain alpha gates. The PR stays draft while those are unresolved.

The Rust direct audio control now records callback/buffer/empty counts and maximum
measured callback gaps, plus the first 16 deficient marker IDs, missing-frame counts
and producer timestamps. Diagnostics do not change queues, stream flags or zero-loss
assertions. Rust RAII buffer-return status remains explicitly unobserved; generated
markers are not relabeled as acknowledged graph submission. Deterministic tests
cover phase exclusion, partial marker loss, duplicate offsets and warm-up exclusion.
Fresh controls and product acceptance remain required.

### Latest VM acceptance checkpoint

Evidence source: `b64c4510a7d1cd0f92202a644e2abb04937b98d5`, tree
`ef1988a55a5fadbfc8809ed50de4580e0424c9fd`, base
`11284f01c58cb80be0d187efa2fca95641513fbf`.

All 18 local validation commands passed, including 564 Rust tests (62 ignored
live entrypoints), 151 Python tests, forced Rust 1.85, strict rustdoc, API checks,
exact-Git feature consumers/offline rebuild and dependency/protocol validators.
Exact-head CI, CodeQL and full-history Gitleaks passed. SBOM run `37281425192`
passed and its downloaded reports validated all 17 workspace identities, including
root. Provider Tier B run `37281428299` passed its contract job; the privileged
job was skipped and supplies no additional live evidence.

Fresh quiet audio qualification failed. The three independent C controls generated
and submitted all 2,880,000 planned markers but missed 999,936 / 913,408 / 950,784
frames, duplicated 9,216 / 5,120 / 7,168 and recorded 1,536 / 1,536 / 512 invalid
frames. Negotiation remained 48 kHz stereo; generation took 71.626 / 70.932 /
71.538 seconds, also failing the sustained-duration envelope. Maximum callback
gaps were approximately 68 / 80 / 74 ms. The C source is unchanged from the
previous three passing controls, so these failures do not establish a new C code
defect or identify a product queue defect.

Rust controls missed 36,352 / 10,752 / 0 measured marker frames, with no duplicates
or invalid markers. The failed trials showed whole-buffer deficits; source/capture
callback counts were 5,813/5,740 and 5,813/5,787, versus 5,813/5,813 in the passing
trial. Maximum measured gaps were approximately 37 / 81 / 15 ms. These are
localization evidence, not proof that scheduling caused the loss. Buffer-return
errors remain unobserved by the Rust RAII binding. All 72 product trials and the
slow-consumer reruns were correctly recorded as blocked by qualification, not run
or passed. The supervisor exited, loopback children were reaped and all private
PipeWire runtime directories were removed. No assertions were relaxed.

An interactive neutral GUI check used the prebuilt candidate in an owned Xvfb
with private configuration/state. Visual inspection and XTest events verified
name entry, creation of 21 Xbox 360 UHID controllers, scrolling/selecting index 19
beyond the initial viewport, removal of index 20 while preserving its selected
sibling, stop-all, and visible technical rejection of dummy_hcd without creating a
controller. The exact executable hash and screenshots are in the external receipt.
All owned processes, display/socket/auth and HID nodes were removed. This closes
those narrow interactive cells only: complete keyboard focus traversal,
touch/lockout, discovery retry, routing and isolated Steam remain unverified.
The earlier completed two-hour neutral soak remains separate passing evidence.

Five individually selected, 90-second-bounded live checks also passed using the
hash-verified b64c451 test executable, each in a fresh private PipeWire graph:
`pipewire_sample_flow_and_independent_cleanup`,
`pipewire_native_mode_has_explicit_caller_endpoints`,
`native_clients_exchange_samples_in_both_directions`,
`usb_bridge_input_stays_empty_until_explicit_activation`, and
`pipewire_close_during_processing_retains_clocks_and_recreates`. They verify
functional sample/native exchange, explicit ownership errors, input activation,
sibling survival and repeated close/recreation with retained clocks. Owned client
and graph processes were reaped and private runtimes removed. The bridge test
contains no attached USB/IP device; these passes close neither attached transport
acceptance nor sustained continuity. No ignored child entrypoint was blanket-run.

Alpha remains **NOT READY** for the remaining VM gates. Only the native timing
and physical comparison evidence has been deferred to #135 and #136.

### Complete-resolution continuation

Historical starting checkpoint (later execution updates follow): the candidate started from `23615da424f8b8d358838e5460e62c3b3969493f`
(tree `69e99e0dfd8f9c972bf48bb451b204cbe6fc7f1c`). Earlier results below remain
historical. The current execution boundary permits an idle reversible broker
maintenance window; installed files and persistent services are preserved.

- A3 gadget contract: provider preflight/open, direct broker host open and daemon
  admission now reject the implemented ID-only f_hid transport before construction.
  No kernel-version guess enables it. All four families and repeated rejection are
  covered; existing protocol/cleanup prototype regressions are retained. The demo
  and README explain the limitation rather than advertising prepared-host success.
- Supporting SPI adds `ProviderPreflightError::Unavailable { target, reason }` and
  the broker contract guard/reason. The ordinary root maps this to its existing
  `Unsupported` error; ordinary signatures remain stable. The broker wire format
  does not change. Downstream experimental/provider implementations must recognize
  the additional non-exhaustive error variant.
- A1 tooling: maintained independent C graph callbacks distinguish generation,
  graph submission and reception; marker self-tests reject hidden loss/duplicates
  and channel errors. The clean-candidate acceptance driver records builds and
  gates the complete 72-cell matrix on independent and direct graph controls.
- The first three sustained C controls failed despite full submission of 2,880,000
  frames each: missing 269,816 / 213,248 / 198,144, invalid 1,016 / 1,536 / 0 and
  duplicate 0 / 0 / 512. These are preliminary tool-candidate results, not final
  product acceptance. Later format-accounting refinements must be rerun separately.
- A3 installed acceptance: the reversible runner defaults to a non-mutating plan,
  keeps the global lock, stages hash-verified images, tests rollback deterministically
  and preserves cleanup errors. Privileged execution is **blocked**: current
  `sudo -n true` requires a password. The active installed broker was not stopped.
- A2/A4 physical and isolated GUI acceptance are **blocked**: no reference controllers
  are attached and neither Weston nor Xvfb is available. Native-host evidence must
  be returned by an external tester; no native result is claimed here.

See [the portable lab procedure](ALPHA_ACCEPTANCE_LAB.md) for administrator and
external-tester reproduction. A VM that cannot qualify independently remains
unqualified for sustained audio; exact-candidate qualified native results may close
native acceptance while VM failures remain visible. PR #133 stays draft.

#### Committed control run and delivery checkpoint

The acceptance driver ran at `c77b0012868ef79160a716613c6d530fa4bc184a`.
Its three C controls negotiated 48 kHz stereo and submitted every planned frame;
missing counts were 273,920 / 211,712 / 258,048, with 0 / 0 / 1,536 duplicates
and no invalid/partial markers. Direct Rust graph controls independently missed
2,048 / 3,584 / 5,120 frames. All six trials failed. The driver records all 72
product cells as **blocked**, not executed or passed; slow-consumer reruns are
also blocked by qualification. Historical product failures are retained below.

Source/tooling delivery checkpoint: `32aa38a058341cd4df31e9f323b2a2da683ed8e7`,
tree `54057ec1b306e3c30f38de9bf6c8e64de413b122`. Subsequent changes strengthen
installation fingerprints, explicit privilege dropping, output quotas, private-runtime
validation and sustained-duration reconciliation, with regressions. The C streaming
and product audio sources are identical to the failed control run; stricter receipt
validation does not establish a new passing audio result. Later delivery documentation commits must
be pinned in the external final-revision receipt and revalidated with exact-head
CI and Git consumers. The source bundle/build manifest is generated from the
final delivery head, not inferred from older binaries.

Four individually bounded live checks passed at `7d7a943d15f6fac056775d6c69ad4d0d012f98e5`: identity restoration, four-family UHID lifecycle/sibling isolation, three-family root audio creation/terminal cleanup, and bounded audio/HID threads. The final external receipt records their exact-head repeat. These checks do not establish sustained continuity, installed-provider security or physical parity.

#### Subsequent execution at the final delivery head

A fresh quiet qualification at `ad236b06ef983f1698b64b4ba473b1433746c95b`
produced three passing Rust direct controls, but C controls still missed
3,072 / 1,024 / 2,048 measured frames, with no duplicates or corrupt markers.
All 72 product cells remain blocked by independent qualification. This improves
the observed VM result without establishing reliable sustained continuity.

General sudo still requires a password, but the already-installed restricted host
helper was discovered to have delegated access and its hash matches the reviewed
source. It loaded missing uinput, journaled temporary creation access and restored
the ACL afterward. No helper lease remains active; the newly loaded module is
retained for administrator review, as required by its shared-resource policy.

The first live evdev feedback run failed because its historical harness assumed
one DS4 node. The corrected harness matches the complete creation-scoped physical
association, selects the primary regardless of enumeration order and verifies
removal of every component. Its deterministic regression rejects missing, foreign
and duplicate components. The corrected bounded live run passes all four families,
both normal feedback and consumer-death cleanup. DS4 reports two removed nodes.
This is neutral-state/feedback acceptance; contact injection, SDL/physical parity
and the isolated desktop checks remain unperformed. An administrator provisioned the first bounded broker delegation, but launch
failed before service changes because `/run` is mounted `noexec`. The runner now
stages binaries under a trusted executable `/var/lib` filesystem and rejects
`noexec` staging during preflight, with deterministic regressions. Executable
wrapper delegation must be reprovisioned there; the failed launch establishes no
installed-provider acceptance. The corrected delegation then refused before service
changes because the idle check counted systemd journal stdout/stderr as clients.
It now excludes only stdio sockets with verified reciprocal journald endpoints
and the configured journal process; real clients and unknown sockets still reject.
A deterministic regression covers both admission and refusal cases. No successful
privileged run is claimed by either failed attempt. The final external receipt
pins the test correction separately from the qualification revision above.

The subsequently installed, immutable scoped helper completed the bounded live
gadget scenario at `3b53921f2a6b8cf60ab649f4cc440c77e641bd33`: all eight
exact rejection replies and the non-root/no-groups/no-capabilities checks passed;
original units and installation fingerprints were restored and owned runtime
resources removed. This closes live rejection for the unavailable gadget, not
the full provider/audio security and recovery matrix. The prior failed trial
remains visible. Temporary directory suffixes can contain underscores, which the
broker instance parser rejects; the runner now normalizes them to hyphens, validates
the 32-character grammar and preserves bounded failure stdout/stderr in receipts.
Focused sanitized regressions cover both corrections. The privileged helper is
pinned to reviewed artifacts and cannot install arbitrary updated code itself.

This continuation added no Cargo dependencies. Installed broker images and
persistent scheduling, routing and desktop configuration were preserved. Human
administrators provisioned narrowly scoped lab delegation outside the repository;
broad passwordless delegation was revoked. An isolated desktop, four physical references, an administrator-run
provider window and external native results remain required. The previous two-hour
soak is historical; its requested final-candidate repeat remains unperformed.

#### Further host closure checks

At source `5091b6e4eba2e2565cb71a04c49cf126f9b2c450` (tree
`703cc4f5081f21bfc28cfcb42db99e3f2f647c9c`), three quiet maintained C controls
passed: each generated, submitted and received 2,880,000 frames over 60 measured
seconds with zero missing, duplicated, invalid or partial markers. The control
binary hash equals the independently rebuilt candidate hash. Graph xrun counts
are unavailable, not asserted zero. Earlier failures remain above.

The three fresh debug Rust controls still missed 31,744 / 27,136 / 36,352 frames.
Release optimization also failed all three controls. Neither changing the test
source latency request to 512 frames nor moving its callbacks to the main loop
passed a separate short diagnostic. Those experiments used external source copies;
no production stream flags, queue capacity or acceptance assertions were changed.
The 72 product trials and slow-consumer reruns remain blocked on full qualification.

A rootless Xvfb extracted into disposable storage now supplies an isolated display;
no packages or persistent desktop configuration were installed. A 60-second neutral
GUI soak passed and removed its display resources. The two-hour run is in progress
and must not be counted as passed until the final resource and cleanup receipt is
reviewed. Its source and binary are pinned separately from later tooling commits.
No touch injection or physical comparison is claimed.

The expanded rejection validator covers repeated exact gadget errors on the same
connection, malformed operations, truncated/oversized framing, the partial-frame
deadline, unexpected descriptor release and daemon authorization from a distinct
unprivileged identity. The runner registers both client units before startup and
preserves their bounded output in receipts. Only an explicitly requested,
root-owned unauthorized probe changes the *temporary candidate* socket to 0666,
so the negative test reaches daemon authorization rather than stopping at an OS
ACL. Installed socket permissions are preserved. Deterministic tests cover exact
replies, response bounds, unexpected-FD cleanup and distinct client identities.
The expanded live run at `5753228f2cbd12a58fad4a6db18d6c6baf155a63`
passed all 30 authorized-client checks, including descriptor release. Its
unauthorized case failed because the harness attempted a request after admission
had already closed the connection. Inspection confirms the daemon intentionally
rejects that UID before reading any frame. The corrected probe asserts immediate
EOF without sending, with regressions rejecting still-open or successful
connections. The failed trial restored original service state and fingerprints
with no cleanup errors; its partial result does not count as a passing whole lab.
Installing the corrected immutable probe is required before its live rerun.

#### Graph-control and neutral-soak hardening

The marker capture helper now drains every ready input buffer for a notification
and records ready-buffer/coalescing counters. A deterministic fake-buffer test
verifies complete marker receipt and release of all buffers even when an intervening
payload is malformed. Production PipeWire callbacks, queues and assertions are
unchanged; the helper correction is not proof of the cause of earlier VM loss.

At `b6fdccd9f5185fa73c8d6e1736cfa5af2ffeb25d`, all three corrected Rust controls
passed with 2,976,000 generated/received frames each and no loss, duplication or
corruption. They observed no coalesced callbacks. The fresh independent C batch
failed two of three trials: missing 666,368 / 61,440 / 0 and duplicates 5,120 / 0 / 0.
Complete submission and negotiated format did not establish repeatable graph
continuity. All 72 product cells and slow-consumer reruns remain blocked.

The prior GUI runs were stopped with owned-process identity checks; display sockets,
locks and owned HID nodes were removed. Their partial samples are not two-hour
acceptance. Inspection found that the validation helper inherited enabled audio
under the all-feature demo default despite intending a neutral workload. It now
explicitly selects audio-disabled creation options before opening any controller,
with a deterministic regression covering both inherited enabled/disabled states.
Normal demo defaults remain unchanged. The final neutral two-hour repeat must use
the new build receipt and complete final cleanup before its gate can close.

#### Scoped provider closure and current validation

The corrected installed lab at `84f4d397f9d73e53c74036bbc8ab08ab37cfc718`
passes all 30 authorized-client negative probes and the distinct unauthorized-UID
admission probe. The latter observes immediate EOF before sending. Both clients
run without supplementary groups or capabilities. The original installation
fingerprints and active broker/socket state were restored; no cleanup error or
owned runtime remains. Broad passwordless sudo is still denied. This closes these
specific negative cases, not successful worker construction or recovery coverage.

All 18 local validation commands pass at `a8ad387cd7185804d3db979773921915753bea6a`
(tree `4e9f3b649cdfdd37cca5b131bc04ea6f910b18aa`): 560 Rust tests pass, 59 live
entrypoints remain explicitly ignored, and 138 Python tests pass. Downloaded
exact-head SBOMs verify all 17 workspace packages, including the root. Conditional
privileged CI jobs remain skipped and cannot stand in for the missing live cells.
The first macOS CI attempt failed during Rust download before building; its
exact-head retry passes. Linux, Windows, MSRV, CodeQL and Gitleaks jobs also pass.

The audio-disabled neutral GUI repeat is running from the a8ad387 build in a
private rootless display. Its two-hour duration and final cleanup are still pending;
no intermediate resource sample closes the gate. The most recent b6fdccd audio
control failures above still block the 72 product trials and slow-consumer reruns.

#### Connection admission and empty restart extension

The maintained negative validator now exercises the eight-connection UID limit,
immediate excess rejection, sibling progress and admission release after disconnect.
The runner adds an opt-in empty candidate restart/reconnect phase, guarded by owned
journal identity, emptiness, unused VHCI port and absence of children. Deterministic
regressions cover missing bounds, unreleased permits, wrong replies, socket cleanup,
changed/nonempty journals, foreign children and restoration after restart failure.
The reviewed installed run at `2c1af2fd64dd014f4c20651c3bf70c589ec77d4e` passes
33 checks before restart, one unauthorized-peer check and 33 after restart. The
candidate PID changes, sibling/admission assertions pass, original installation
fingerprints and service state are restored, and owned runtime resources are removed.
It does not claim worker-death or stale-attachment recovery.

The SDL evdev harness also retained a one-node assumption. It now selects all
creation-associated physical identities, chooses the primary regardless of order,
ignores unrelated inventory and verifies removal of every component. A deterministic
DS4 companion/foreign/duplicate regression passes. A new ignored individual DS4
evdev mapping test exercises controls without touch injection. Live proof uses a
private build of the already-reviewed SDL 3.2.0 revision; its unused PipeWire audio
backend is disabled for compatibility with newer host headers. No installed SDL or
product audio behavior changes. Consumer acceptance results must be recorded separately.

The maintenance idle/restart guard previously inspected only the process leader's
children. Since Linux child ownership is per-thread, it now checks every task,
requires a present leader and stable process start-time/task inventory, and refuses
unverifiable ownership. Sanitized regressions cover a nonleader worker child,
disappearing task metadata, PID reuse and changed task inventory. A bounded ordinary-
user probe also confirms detection and reap of an owned child launched by another
thread. After administrator installation, the guarded snapshot at
`d307b5c393a2ccd45aae3e96d74ec3644b4bd266` passes all 67 checks again.
Original installation identities and active service/socket state are restored;
owned runtime resources are removed. Its earlier passing receipts are retained.
This closes the guarded negative/admission/empty-restart run, not successful
worker construction, worker death or stale-attachment recovery.

The independent C control now records callback counts/gaps, empty producer callbacks
and dequeued capture buffers to localize client scheduling stalls. Its self-test
checks measured-phase gap accounting, and the receipt regression confirms that
large gaps never excuse marker loss. Callback flags, queues, formats and acceptance
thresholds are unchanged. Fresh measurements remain pending until GUI/consumer
workloads and compilation have completed; diagnostic instrumentation is not a
passing continuity result.

A fresh disposable candidate checkout exposed another acceptance-tool defect:
loading the private lab helper wrote `scripts/__pycache__` before the clean-tree
check, causing preparation to reject its own newly generated artifact. Both audio
drivers now disable bytecode writes before loading source helpers. A fresh Git
fixture regression runs each plain CLI without local excludes and verifies a clean
checkout afterward. Existing dirty-source rejection remains strict; no ignored
source or relaxed identity check is introduced. The failed prebuild receipt is retained.

#### Current validation and bounded consumer evidence

At `9e64801b76666d13cc87182b3639827cb02ef673` (tree
`fa595e94c9aa8eb2a06e73fce1831ea87fbe25e9`), all 18 local validation commands
pass: 561 Rust tests, 60 explicitly ignored live entrypoints and 150 Python tests.
Exact-Git root consumers and offline rebuild pass. Linux, macOS, Windows, MSRV,
CodeQL and Gitleaks jobs pass at this exact head. Downloaded SBOMs cover all 17
workspace identities, including the root. The conditional privileged CI job is
skipped and remains missing evidence beyond the independently recorded lab scope.

The controls-only consumer batch at `12417df47288048821320b2d526f3c78083422b9`
(tree `2817ed916435520332d39b83c5bbba294dbe8642`) passes four UHID families
and DS4, Switch Pro and Xbox 360 evdev: 156 observations and three cleanup records
per test, 1,092 observations and 21 cleanup records total. Both DS4 associated
nodes are removed. Every case records zero touch events. DualSense evdev individual
mapping was not included; the full Sony scripts require desktop isolation.

The consumer source is reviewed SDL commit
`535d80badefc83c5c527ec5748f2a20d6a9310fe`; the first private build lacked release
tag metadata, so its strict backend classifier remained unknown despite the source
pin. Preserve that limitation. A separate tagged build reports the exact reviewed
`SDL3-3.2.0-release-3.2.0` contract and passes all seven repeats: 1,092 passing
observations and 21 cleanup records. The reported backend is HIDAPI for DualSense,
DS4 and Switch UHID, and Linux evdev for Xbox UHID and every evdev case.
These input and removal checks do not establish motion, contacts, reverse outputs,
physical comparison, Steam compatibility or audio. Raw receipts and hashes are
retained outside tracked source.

A clean disposable checkout of 9e64801 has completed source/binary preparation.
Its bounded audio coordinator waits for the complete two-hour GUI receipt, all
seven tagged consumer results and display cleanup before starting independent
controls. Product trials remain conditional on qualification. A queued run is not
passing evidence, and the historical audio failures remain release gates.

#### Completed controls coverage and additional scripted consumers

A new touch-free `dualsense_evdev_individual_mapping` live entrypoint closes the
missing individual-controls cell. At `44a6faa9bce15e7149590ee7aaeaaeab11ddd4e8`
(tree `b835276e1fbacf0d581c99a97c81445b27ce32f7`) it passes 156 observations,
three repeated cleanup records and zero touch events with the same tagged reviewed
SDL build. Combined with the seven tests at 12417df above, all four families have
UHID and evdev individual-controls evidence: 1,248 observations and 24 cleanup
records. These are two exact source receipts, not eight trials at one revision.
The intervening changes affect tooling, tests and evidence documents, not controller
production code. This does not close Sony contact or full output acceptance.

Four existing scripted tests also pass at 44a6faa: Switch UHID controls/motion,
Xbox standard-HID controls, Switch evdev and Xbox evdev. Each repeats three sessions
with device removal and consumer reap. Switch UHID has distinct accelerometer and
gyroscope samples without invalid timestamps/values; both evdev scripts receive
exact requested rumble magnitudes. UHID Switch/Xbox output fidelity remains separate;
standard-HID Xbox evidence does not establish XInput/xpad. All four record zero
touch events. Raw consumer hashes, observations and cleanup receipts are external.

All 18 local commands pass at 44a6faa (561 Rust tests, 61 ignored live entrypoints,
150 Python tests), and its downloaded SBOMs verify all 17 packages. Platform/MSRV,
policy and audit CI jobs pass, but the pinned-corpus job fails when its real rustdoc
fixture returns nonzero. That test previously discarded compiler diagnostics; it
now preserves stdout/stderr with a sanitized failing-compiler regression. The failed
run is retained. The cause and exact-head rerun remain unresolved until visible
compiler evidence is available; diagnostic improvement is not a corrected compiler.


### Sony touch-free motion and feedback acceptance

At `5f2f59db07252ae9d41888571c5ea46437914e60` (tree
`62aec02635cfa5918332c1fc39b0cdf7eb375b9e`), the new Sony touch-free SDL test
passes DualSense and DS4 through UHID and evdev, three repeated sessions per cell.
All 12 sessions pass control sweeps and cleanup. UHID delivers distinct valid
motion samples, exact rumble and RGB lightbar observations; evdev delivers exact
rumble feedback. All consumer observations record zero touch events. Both DS4
associated nodes are checked on removal. Evdev does not represent the HID lightbar
or motion contract; this evidence does not silently extend that surface.

The test-only script option prevents active contacts while retaining existing full
contact scripts. Its deterministic regression checks every u16 step with contacts
disabled and preserves enabled transition boundaries. This does not alter production
controller or demo behavior, remove old regressions, or weaken full contact tests.
The reviewed SDL consumer/library hashes and raw observations remain external.

All 18 local commands pass at this source: 562 Rust tests, 62 explicitly ignored
live entrypoints and 151 Python tests, plus exact-Git consumers/offline rebuild.
The two-hour GUI run and independent quiet audio qualification remain pending.
Sony contacts, other unexercised output modes, physical fidelity, full installed
provider crash/stale-attachment recovery, USB/IP audio/routing, manual GUI/Steam
and native-host timing remain separate release gates.


### Completed GUI soak and final-source quiet audio qualification

The audio-disabled neutral GUI run completes 7,200 seconds with status zero and
no HID nodes remaining. The GUI/driver processes, owned display socket/lock and
authentication file are gone. Across 120 external samples, steady state after five
minutes has 16 descriptors, 18 threads, no children and no swap; RSS ranges from
102,148 to 141,916 KiB and ends at 102,148 KiB. The internal descriptor enumerator
counts its own temporary directory descriptor (17); external sampling counts 16.
This closes the neutral sustained-lifecycle/resource/cleanup cell, not manual
keyboard, routing, errors, Steam or audio-enabled GUI acceptance.

The original source receipt is a8ad387. A final all-target/all-feature rebuild at
`7bf16cc25d14fbce1bef8739e8f8d477403eaf86` (tree
`83e532b1d306731f6190bb7f5bf08f5a99cfe801`) produces the identical GUI binary,
SHA-256 `aa3e1e97bfbd40f4e3243fe104806f0ebdbf0e429d68a1ede900ba119ab8990f`.
The running process image has that hash, and production source/manifests/lockfile
have no changes between those revisions. Preserve original and equivalence receipts;
do not silently relabel the original source run.

After complete GUI/consumer cleanup, quiet audio qualification runs at 7bf16cc.
All three independent C trials deliver exactly 2,880,000 generated, submitted and
received measured frames with zero missing/duplicated/corrupt markers. They do not
measure end-to-end latency, and graph xruns remain unknown. Rust direct controls
pass two of three; the first loses 2,048 markers despite all 2,976,000 planned frames
being generated, zero duplication/corruption and zero coalesced capture callbacks.
Its p99 is 169 microseconds; timing cannot excuse continuity failure. The other two
have zero missing/duplicate markers with p99 156 and 155 microseconds.

Overall qualification is false. All 72 product cells remain explicitly blocked;
slow-consumer reruns are not performed. This failure occurs without a product PCM
queue and does not establish a library queue defect or its cause. Preserve it along
with historical failures; do not relax zero-loss assertions or enlarge queues.
Final-head CI passes after retrying a Windows CRYPT_E_REVOCATION_OFFLINE dependency
fetch failure without weakening TLS. CodeQL/Gitleaks pass and all 17 downloaded SBOM
identities verify; conditional privileged CI remains skipped. All 18 local checks
pass at 7bf16cc (562 Rust tests, 62 ignored live entrypoints, 151 Python tests).

**NOT READY for alpha under the all-implemented-paths boundary.** Code defects
F1–F8 are addressed; failed audio continuity and incomplete provider/consumer
acceptance remain release gates. PR #133 remains draft. Code review is useful now,
but green builds cannot authorize an alpha release.

- Branch: `codex/gui-long-run-lag`; PR: https://github.com/anderstvoss/virtualgamepad/pull/133.
- Review baseline: `0c08fb485292adb4c361a7210efc04bb223f4642`.
- Remote main/base: `11284f01c58cb80be0d187efa2fca95641513fbf`.
- Both baseline trees: `b733354460af8162c2863b1f38cb4b1157c21680`.
- Implementation checkpoint: `1f10209d40b8ce52e10bec59508638aa7b029277`;
  checkpoint tree: `1d126b26b5989ffb3ca3c972a31981b83b8b83be`.
- Later documentation commits are resolved from the PR head. The handoff explains
  how the reviewer records the exact final head/tree without a self-referential
  document hash. Revalidate every subsequent code candidate separately.

The original review, coverage ledger and acceptance report remain external
historical evidence. This portable disposition replaces neither their failed logs
nor their revision boundaries. Attached plans and earlier issue closures are
context, not acceptance evidence. Unavailable Bluetooth, controller-matching audio
and proprietary XInput require truthful rejection, not implementation here.

## Finding disposition and regression map

| ID / priority | Trigger and previous consequence | Correction and source | Regression / disposition |
| --- | --- | --- | --- |
| F1 / P1 | Full subscription queue loses close message; close/Drop waits forever | `gr-controller-runtime/src/reverse_delivery.rs`: optional sender behind mutex, disconnect then join, self-close avoids join | Full/zero capacity, Drop, concurrent closers, publish race, callback panic, self-close and self-close during an external join pass. Finite callbacks must return; arbitrary callback preemption is not promised. |
| F2 / P1 | Stalled or oversized discovery command blocks required controller service and shutdown | `demo/src/gui/host_audio.rs`, `gui.rs`, `audio_lab.rs`: GUI-owned coordinator, zero-capacity worker handoff plus one coalesced pending request, generations, cached display, two-second command bound and 256 KiB limit, owned process-group cancellation/reap before reader join | Stalled descendant, successful parent with surviving descendant, oversized output, non-UTF8/nonzero exit, cancellation, stale result, retry/coalescing, worker crash/no respawn, continued service/removal/shutdown pass. Loading/error visible; no per-frame respawn. |
| F3 / P2 | UHID Start/Open/Close/Stop counted internally but never delivered to root callbacks | `gr-hid`, curated protocol implementations and common session: lifecycle hook returns optional observation after required state handling | Fake ordered events through all four root families, no replay and existing bounded-observation/required-reply tests pass. Live neutral UHID Start delivery passes all four families. No events synthesized by application close. |
| F4 / P2 | Five let chains reject Rust 1.85 | `demo/src/gui.rs`, `gui/audio_lab.rs`: equivalent supported conditionals | Forced Rust 1.85 workspace/all-target/all-feature check passes; existing behavior regressions retained. |
| F5 / P1 | Installed minimum compiler is bypassed by repository override | `.github/workflows/ci.yml`: explicit toolchain environment, actual rustc/cargo print and minimum-version assertion | Forced compiler check and negative newer-syntax fixture pass. Remote job/compiler log must belong to the exact candidate. |
| F6 / P2 | Removed packages/phase-gate CLI make scheduled/manual validation invalid | `.github/workflows/provider-tier-b.yml`: current deterministic tests and production/USB worker validators; required Ubuntu audio dependency endpoints | Cargo-metadata command inventory rejects removed packages/examples and missing scripts; Python tests pass. Privileged job remains explicitly disabled, so its skip is missing live evidence. |
| F7 / P2 | Uploaded SBOM contains 16 member reports, omits root | `.github/workflows/sbom.yml`, `scripts/collect-sbom.py`: metadata-driven collection and extracted-artifact verification | All 17 packages locally generated/validated, including root. Missing/duplicate/foreign identity, missing dependency inventory and stale output reject. Uploaded checkpoint artifact verifies all 17 identities; final-head verification and workflow run IDs are recorded in the PR description and external revision receipt. |
| F8 / P2 | Entry documents contradict demo defaults, topology and issue status | README, application/audio/support/demo docs and historical ledger annotations | Root defaults empty; demo ALSA/WIP USB/IP defaults distinguished; #112 closure is not native acceptance; new DS4 topology and lifecycle semantics documented. Historical failures preserved. |
| A1 / P1 | Samples/native audio loses markers even with zero reported queue drops | Marker producer accounting, graph-clock diagnostics, queue/drop/underrun counters, per-marker client accounting and explicit drain; simultaneous duplex for all four ownership combinations | Deterministic duplicate/corrupt/partial-frame and producer/drain tests pass. Protocol fixture now primes exact microphone frames before READY. **Continuity unresolved; assertions unchanged; PR stays draft.** |
| A2 / P2 | Combined DS4 node is classified as touchscreen and misses SDL gamepad discovery | Production `dualshock4/evdev.rs`: transactionally associated gamepad/contact nodes; primary retains controls/feedback, companion retains both contacts and release | Capability split, frame routing, partial creation rollback, reverse feedback, sibling identity/cleanup and root companion metadata pass. Four-family neutral evdev feedback and consumer-death cleanup pass, including both DS4 nodes. Reviewed SDL controls/removal pass for both DS4 realizations. Contact injection, full reverse-output parity and physical comparison remain unverified. |
| A3 / gate | Installed candidate provenance, privileged authorization/recovery and dummy_hcd report semantics unverified | Current provider/direct-open/daemon admission rejects incomplete f_hid semantics before construction; separate reversible installed-provider lab preserves original image/config/unit fingerprints and global lock | All-family/repeated rejection and zero-factory-admission regressions pass; earlier protocol/cleanup tests retained. Full gadget parity is explicitly unavailable. The scoped live rejection/framing/descriptor/admission lab passes 31 checks and restores cleanly. Successful audio-provider construction, worker/client-death, stale-lease and restart recovery remain unverified. |
| A4 / gate | Consumer breadth, long-run GUI, accessibility and physical/native timing evidence incomplete | Selection/removal tests through 1,024 positions; explicit neutral-owned-device GUI soak; selected root lifecycle/audio/reconnect tests | Deterministic selection/lockout/routing/error/lifecycle coverage passes. An earlier two-hour soak passed with owned resources removed; it is historical. The corrected two-hour neutral repeat passed with complete cleanup; see the completed soak and latest interactive checkpoint. Full keyboard/touch/routing/Steam checks remain open. Physical/native timing evidence is deferred to #135/#136. |

Source paths in this table are relative to the repository (`gr-*` sources are
under `crates/`). Tests run at the lowest practical seams and retain prior tests.
No broad architecture rewrite or acceptance-assertion relaxation was introduced.

## Public contract and architectural implications

Ordinary root signatures still match the 110-item frozen snapshot. Lifecycle
variants now deliver their intended semantics. `service` performs required host
handling before optional bounded observation delivery; application callbacks
cannot become the required protocol engine.

The supporting `gr_hid::Protocol::lifecycle` implementation signature changes to
`Option<Self::Output>`. External protocol implementations must return `None` or an
observation after state handling. Supporting curated `ControllerAssociation`
struct literals must initialize `companions`. These are SPI changes outside the
ordinary-root compatibility promise; all in-tree implementations/fixtures were
updated. Root consumers must still rerun their feature/borrowing/API checks.

DS4 uinput topology now has two owned input components. The primary surface
describes the complete logical controls; the contact companion advertises its
role and distinct creation-scoped association, without inventing observed paths.
Consumers must iterate components rather than assume one node. Both nodes share
logical ownership and are removed on rollback/close; SDL gamepad touch APIs cannot
represent the separate evdev contact presentation, an explicit surface restriction.

Discovery owns only its spawned process groups. Generation changes cancel stale
commands; normal controller workers never discover devices. Normal child exit is
observed with `waitid(WNOWAIT)` so its process-group identity remains owned until
descendant termination and reaping; EOF alone cannot establish cleanup. External subscription
close serializes joiners and releases the sender lock before joining. Self-close
must also avoid waiting on a join lock held by another closer.

The demo adds a direct `rustix` process feature for safe process-group signalling and unreaped exit observation;
version 1.1.5 was already locked transitively. No new ecosystem dependency or
advisory suppression is introduced. The GUI soak is an explicitly invoked example,
with neutral UHID state and no routing/touch injection.

## Validation scope

Local locked fmt/check/Clippy/workspace tests, Rust 1.85 all-target/all-feature
check, strict workspace rustdoc, ordinary API snapshot, root feature consumers and
cached offline rebuild pass. Workspace tests pass (553 passed, 59 explicitly ignored, no failures); ignored
child entrypoints are never blanket-enabled. Python tooling tests pass (103 tests). Corpus remote
pin verification passes at `a1789d6ed92b2325016dd78be765342f3ca19aa4`.
Cargo audit/policy checks pass with the previously acknowledged unmaintained
`ttf-parser` warning. Gitleaks passes. Inventory source links require rebuilding
root rustdoc after supporting source pages; semantic signatures remain unchanged.

Production-worker validation passes all three profiles with low and high FD slot
layouts. The USB/IP protocol probe passes all three families, including exact
invalid SET_REPORT stall and PCM patterns, in explicit protocol-fixture mode.
This mode is only for deterministic protocol validation; ordinary timed live
source behavior and silence/loss diagnostics remain unchanged.

Neutral root UHID creation/service/Start delivery/removal/recreation and identity
restoration pass. Selected root audio creation/terminal cleanup, bounded PCM/HID worker
counts, and processing-close/recreate checks pass in isolated graphs. These checks do not substitute for sustained marker continuity.

The full 72-trial audio matrix completed: **0 passed / 72 failed**, with all
three trials failing in each of the 24 required cells. The two-hour GUI soak exits successfully with normal teardown. Final-head remote
evidence and exact consumer/artifact revisions are recorded in the PR description
and accompanying external revision receipt, avoiding a self-referential hash.
Every sustained direction and duplex/mixed cell used three trials, a private
PipeWire graph, 62 seconds of markers (two-second warm-up boundary), bounded drain
and unchanged zero-loss assertions. Concurrent GUI/build work makes these VM
stress observations; no native-host latency or physical-fidelity claim follows.

## Live acceptance matrix

The following cells are emulated PipeWire on this ARM64 VM, at quantum 512.
Every entry is **0/3 passed**, with two-second warm-up and a 60-second measured
boundary plus bounded drain. Each trial retains its own status and marker/clock
log. These are failed acceptance observations, not a proven backend defect.

| Direction / playback–microphone ownership | DualSense | DS4 | Xbox 360 |
| --- | --- | --- | --- |
| Graph → library Samples | 0/3 | 0/3 | 0/3 |
| Library Samples → graph | 0/3 | 0/3 | 0/3 |
| NativeClient playback | 0/3 | 0/3 | 0/3 |
| NativeClient microphone | 0/3 | 0/3 | 0/3 |
| Duplex Samples–Samples | 0/3 | 0/3 | 0/3 |
| Duplex Samples–NativeClient | 0/3 | 0/3 | 0/3 |
| Duplex NativeClient–Samples | 0/3 | 0/3 | 0/3 |
| Duplex NativeClient–NativeClient | 0/3 | 0/3 | 0/3 |

For example, a DualSense sample-playback trial generated all 2,976,000 planned
frames but missed 11,264 measured client frames with zero reported queue drops
and 99,840 missed graph frames. Correlation with graph discontinuities does not
establish the cause. Priming, measured markers, silent drain, duplicate/invalid
frames and queue loss remain distinct; no loss assertion was relaxed.

All four UHID SDL individual-mapping tests pass: 156/156 observations per family,
three repeated/reused-ID sessions, and three explicit device-removed/consumer-
reaped records per family. Initial 180-second attempts were incomplete timeouts;
360-second reruns completed in 240–251 seconds. Those failed/incomplete logs are
preserved. SDL 3.5.0 input mapping is observed; the probe's version-specific backend
signature does not identify this build, so backend certification remains unknown.
No desktop touch injection was performed. This is not production DS4 evdev parity.
Switch controls/motion and Xbox standard-HID controls also pass their three-session
scripted tests. Sony full scripts include touch and were not run on the desktop.

All three existing slow-consumer soaks fail their first 60-second measured trial
with unexplained steady-state queue loss (28,672 / 86,016 / 72,704 frames for
DualSense / DS4 / Xbox). The deliberately stalled 200 ms interval also reports
loss, but later trials are not reached after the earlier assertion fails. Do not
credit those incomplete three-trial soaks as accepted slow-consumer continuity.
Private graph processes and runtime directories are absent after these runs.

Private-graph death also passes a separate owned-daemon probe for all three
required audio families: required-audio controllers fail and close, a HID-only
Switch sibling survives, and repeated close succeeds. Creation/worker-count and
processing-close/recreate tests pass separately. Live route-change continuity,
ALSA/USB/IP routing, installed worker crashes and native/physical timing are still
unverified; fake routing/lifecycle coverage does not establish those live gates.

The two-hour real GUI soak completed at 7,200 seconds and exited with status 0.
Its 118 per-minute samples/cycles held FD count 16 and thread count 6; RSS ranged
from 91,404 to 92,632 KiB (about 1.2 MiB growth). Exact PID-associated owned UHID
nodes were absent after teardown. This establishes the measured run and cleanup,
not a proof against every unbounded-growth scenario or manual accessibility.

Long live runs span implementation checkpoints. The no-audio GUI binary predates
later discovery cleanup; marker instrumentation was expanded during the matrix.
These phase-specific failed/limited observations are retained, not promoted into
exact-final-head acceptance. Final-head compiler, deterministic tests, consumers,
CI and artifact checks are pinned separately. Repeat live acceptance for any
subsequent behavior change before approving release.

## Continuation: graph baseline isolation (2026-10-04)

The original 72 failed stress trials above remain historical evidence. Fresh
quiet probes on the same host did not involve the GUI soak or review builds.
An eight-second DualSense Samples playback smoke passed; all seven other sample,
native and duplex smoke cells failed. Fresh 60-second measured Samples/native
playback probes still missed 6,144 / 11,776 frames. These limited reruns do not
replace the full family/ownership acceptance matrix.

A new ignored `latency_graph_direct_control` bypasses `Session`, endpoint bridges
and the library PCM queue. It sends the same graph markers directly to a named,
test-owned sink and retains exact producer, per-marker and latency assertions.
All three quiet 60-second measured controls fail: 4,608 / 3,072 / 3,072 missing
frames, with complete planned producer generation and no duplicate/invalid frames.
The source/capture helpers remain possible causes; this control alone does not
establish an environment root cause or exonerate the production backend.

A separate profiled control misses 18,944 measured frames, while PipeWire reports
101 source and 14 sink client xruns. Its p99 callback-to-callback observation is
343 microseconds. A short compiler check overlapped the end of that profiled run;
its results are diagnostic, not quiet latency acceptance. The three unprofiled
controls above exclude that overlap. A read-only scheduling snapshot observes
non-realtime data-loop policies (priority zero), which is a prerequisite concern,
not proof that scheduler configuration caused every missing frame.

An additional independent `pw-cat`/`pw-loopback` control bypasses both the library
and the Rust marker clients. It uses only named nodes in a private graph, two
seconds of silent warm-up, 60 seconds of markers and two seconds of silent drain.
All three trials fail: 22,528 / 32,768 / 40,960 missing frames; duplicates,
corruption and partial frames are zero. Every trial supplies all 2,880,000 marker
frames to client stdin. That is input-supply accounting, not proof of graph
submission. An eight-second control passes. The fixture and raw records stay
outside tracked source; this baseline measures continuity, not latency.

Sustained loss therefore reproduces on a path independent of virtualgamepad and
its Rust clients. The host graph baseline must be qualified before attributing
all failed acceptance to library queues or proposing queue-size/latency changes.
Library-specific loss could still coexist. No production audio behavior or loss
assertion changed in this continuation. The direct-sink targeting regression
preserves the existing exact-target capture configuration.

All private graph processes/runtime directories are gone after the probes.
Fresh preflight still reports missing uinput kernel registration and ConfigFS,
and unavailable UDC authorization. Persistent host configuration/services remain
unchanged. The PR stays draft and the alpha gate stays **NOT READY**.

## Remaining release backlog and acceptance prerequisites

1. **P1 audio continuity:** first qualify a prepared host graph with independent
   controls; retain failures and localize loss across producer,
   graph, queue, client and drain. Producer-all/queue-zero/client-missing already
   rules out incomplete planned generation or reported application queue overflow
   for some failed cells. Graph discontinuities correlate but do not prove cause.
   Fix only a demonstrated seam with deterministic regression, then repeat all
   family/direction/duplex/ownership cells without competing review workloads on
   a prepared host. Native-host timing is deferred to post-alpha issue #135; functional continuity remains required.
2. **P2 DS4 consumer parity:** prepare an isolated consumer environment with an
   active uinput kernel device. Verify exact gamepad and companion selection,
   controls, two contacts/releases, feedback, partial rollback, sibling removal
   and shutdown. Do not repeat desktop-affecting contact injection here.
3. **Installed provider/security:** administrator-tied candidate binaries, peer
   authorization and reserved VHCI/UDC resources are prerequisites. ConfigFS is
   missing, UDC authorization is unverified; a root socket peer is not candidate
   provenance. Run attached worker-death, malformed-client, broker restart/stale
   lease and recovery acceptance only on test-owned authorized resources. A
   failed unbind must remain visible and must not release a still-owned lease.
4. **dummy_hcd contract:** the continuation rejects current construction entirely
   before resource creation. Verify installed candidate rejection; no successful
   gadget acceptance is required or claimed. Retained unknown-ID prototype tests
   do not establish full report-type parity or a kernel STALL operation.
5. **Consumer/physical breadth:** hands-on keyboard/accessibility/error review,
   isolated Steam compatibility remain unverified. Native Linux timing and physical
   comparisons are deferred to post-alpha #135/#136. macOS/Windows CI is compile/test evidence, not live provider support.

Two post-alpha tracking issues were created at the release owner's request. No
persistent installed-service replacement, merge, tag or publication was performed. Historical failed
artifacts remain outside tracked source. The reviewer handoff gives portable
commands and rejection criteria for every finding.

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

### Live provider startup defect: duplicate generated identity

The immutable db48dc4 lab passed its identity/hash checks, then both rejection
and positive lifecycle phases failed with connection reset before attachment.
The two test-owned broker journals show `invalid or duplicate instance`.
Their restoration records contain no cleanup errors; the original broker socket
was restored active and the original service inactive. These are failed acceptance
receipts, not passing security or positive-attachment evidence.

A deterministic reproduction found adjacent Python string literals preceding
`.join()` made the configuration prefix the separator between VHCI port lines.
Four selected ports produced three `allow_uid` and `instance` entries. The broker
correctly rejected this configuration. The lab now builds explicit configuration
lines, with exact one-port/four-port regressions and invalid-instance rejection.
No broker validation, resource isolation or privileged access is relaxed.

The packet installer additionally accepts replacement of the complete existing
named-phase policy (or the legacy three-command spelling), with a regression
rejecting an unrecognized phase. Exact previous-helper hashing remains required.
A new administrator-installed immutable packet is needed to rerun live phases;
the currently installed code cannot replace itself. Journal access is separately
limited to a fixed no-argument reader of alpha-lab units. Six gates remain open.

### Complete sudoers policy preflight

Administrator installation of the 112360b configuration correction failed while
validating its temporary policy: the header began with `#112360...`, which sudoers
interpreted as a numeric user rather than a comment. The installed helper still
reports db48dc4 and the original socket/service states remain active/inactive.
No live scenario was rerun with the uninstalled correction. An unused root-owned
staging directory may remain from failed preparation; do not mistake it for the
active installation or remove it without authoritative ownership checks.

The header now starts with `# alpha lab revision`. Policy rendering is a single
pure function shared by installer execution and package preflight. Packaging
runs the real sudoers parser against the exact rendered bytes, including the
header, before writing the administrator installer. The earlier manually built
command-only preview was incomplete and is not sufficient evidence. A regression
parses complete policies for numeric-leading and letter-leading revisions with
real visudo when available; this check passed locally. No command scope or broker
contract changes. The complete corrected packet still requires administrator
installation because immutable privileged tooling cannot replace itself.

### Installed 688fc10 provider and short USB acceptance

The installed immutable lab's source/tree and binary hashes were verified. The
rejection phase passed 33 framing, FD ownership and admission checks plus one
unauthorized-peer check. The first positive lifecycle attempt failed when its
handed-off VHCI port was no longer active; this failure remains unexplained and
must remain visible. A bounded observation run then passed nine normal/abandoned
sessions across DualSense, DS4 and Xbox 360 with exact serial, ALSA ancestry,
pre-attachment exclusion, worker diagnostics and repeated cleanup checks.
Post-handoff process exit and pre-handoff connection exit phases also passed,
with capacity recovery. The pre-handoff case does not establish worker death
during construction, because no construction barrier is asserted.

Both worker- and broker-death phases completed their injected failure checks,
but failed overall in subsequent normal lifecycle probes with the intermittent
inactive-port symptom. Do not mark either complete recovery phase accepted.
The sibling/admission client passed, but restoration failed because its USB
sysfs device was still visible after the VHCI ports became free. The isolation
rule was retained; the device later disappeared. This is failed restoration,
not a passing overall sibling phase. Retained privileged evidence remains for
operator review; do not remove it based solely on remembered resource numbers.

Short USB/IP functional duplex passed separately for all three audio families:
three measured seconds (144,000 frames) plus two warm-up seconds, exact synthetic
microphone markers, playback validation, worker counters, unchanged shared
defaults and complete teardown. This is neither sustained continuity nor route
transition/native-ownership acceptance. No PipeWire qualification was run.
Host available memory remained above 18 GiB with zero measured full PSI.

The demonstrated teardown race now receives a bounded five-second wait for
owned USB disappearance before isolation-rule removal. Identity/content checks
still run after waiting; timeout retains the rule. Regressions cover delayed
removal, unchanged live rule while waiting, timeout and changed content. Lifecycle
startup failures additionally retain a bounded VHCI snapshot and non-consuming
broker reply/EOF evidence before cleanup, with socket-pair regressions. These
new lab changes require an immutable administrator update before live validation.
All six release gates retain remaining required cells and the PR remains draft.

### Installed 3ca0d97 live rerun

The installed 3ca0d97 identity/tree and immutable hashes were verified.
Sibling/admission passed including complete restoration, providing live evidence
for the bounded USB-disappearance wait. Worker- and broker-death phases still
failed overall in their follow-up lifecycle probes. Their new failure snapshots
show the selected VHCI port free and no readable terminal broker reply before
cleanup; kernel enumeration diagnostics are required before assigning cause.
The original broker service/socket states were restored inactive/active.

A new short USB functional rerun failed on DualSense: worker diagnostics reported
192 microphone silence frames and maximum PCM pump lateness of 11,636 microseconds;
playback remained exact, shared defaults unchanged, and cleanup succeeded. DS4
and Xbox cells were not reached because the validator stopped on that failure.
Retain the earlier 688fc10 three-family pass as a historical receipt; it does not
close reliability or continuity in the presence of this newer failure. Scheduling
is not established as the cause, and no buffer size or loss assertion is changed.
The old failed sibling lab retains its staging binaries/configuration, empty
journal directories and isolation rule for operator review; the user-supplied
inventory is not authority to delete arbitrary similarly named resources.
Host available memory remained above 18 GiB with zero measured full PSI.
No library behavior, dependencies or validation assertions changed in this rerun.
All six gates remain open and the PR remains draft.

### Kernel correlation: allowlisted-port selection in lifecycle probes

The supplied kernel capture shows successful DualSense/DS4 enumeration and a DS4
session on VHCI port 1 in the failing recovery run. The broker selects the first
currently free, unreserved administrator-allowlisted port; asynchronous teardown
can leave an earlier reservation briefly unavailable. The normal lifecycle probe
incorrectly assumed every handoff used port 0. Inspecting an unrelated free port
therefore produced a false attachment-failure diagnosis. The capture's connection
closures alone do not establish an unsolicited product disconnect: the probe
itself closes its handed-off session after validation failure.

The probe now resolves the exact handed-off device/bus identity among the complete
explicit port allowlist, requires one unique match, verifies that same port/card,
and checks that port during repeated cleanup. Sibling recreation follows identity
rather than a preferred port. Root fault injection validates the exact journal
against the full allowlist and retains its actual port for fail-closed recovery;
PID, executable, unit, UID and inode checks remain required. No arbitrary port
selection or fallback to foreign resources is permitted. The ordinary root API
and broker allocation policy are unchanged; all four lab ports remain covered.

Regressions cover a legitimate second port, a foreign first port, unauthorized
ports, wrong identities, malformed/duplicate inventories, invalid allowlists
before open, constructor-to-repeated-close propagation, exact journal selection
and full allowlist forwarding to recovery probes. These use synthetic fixtures,
not the supplied kernel logs. Earlier failed receipts remain historical failures;
live acceptance must be rerun after administrator installation of the corrected
immutable lab. This correction does not resolve or defer the measured USB audio
silence/lateness failure. The six-gate objective and draft PR remain unchanged.

### Installed 66f47dd recovery and disposable Steam login

The installed immutable lab was verified at 66f47dd. Individually bounded normal
lifecycle, post-handoff process exit, pre-handoff connection exit, worker death,
broker death, and sibling/admission phases all passed with restoration. This
supports the handoff-identity port correction without erasing prior failures.
The root death-injection phases include all three audio families and follow-up
normal lifecycle. The broader provider gate still requires remaining construction
and cleanup-failure cells, malformed/truncated/identity-changed journals and any
uncompleted sibling-failure cells; six passing phase receipts are not the entire
gate. Physical comparison/native timing remain the separate post-alpha issues.

Exact 66f47dd remote CI (native platform jobs and actual MSRV), CodeQL and
full-history Gitleaks passed. SBOM generation passed and all 17 downloaded
workspace identities, including root, were verified. The provider contract job
passed; its privileged job was skipped and remains missing remote evidence.
These receipts do not establish release readiness or validate later source heads.

A disk-backed, privately mapped Steam/FEX trial on public snap revision 280,
version 1.0.0.85, reached the disposable login screen at 120 seconds. Isolation
sentinels proved actual/passwd home hiding, private proc/dev and dropped caps;
no existing Steam profile or credentials were copied. The bounded trial ended
with a final X11 BadDrawable screenshot failure, with clean owned teardown, so
its raw receipt remains failed. The login screenshot establishes bootstrap
progress only, not controller recognition. The 2 GiB cgroup recorded no OOM kills
in the observed memory event sample; host reserve stayed above 16 GiB.

The ordinary-user Steam runner now supports an explicit Xephyr display server
for a visible private login window while keeping the same namespace, private
cookie, disk-backed home/tmp, memory cap and bounded cleanup. Regression checks
cover visible/hidden command shape, private cookie, owned display range and
missing parent display. Login requires the user's direct interaction in that
window; passwords must not be supplied to the agent. This adds no dependencies
or ordinary product API changes and does not count as Steam acceptance.

### Steam login OOM and host-sized budget correction

The visible c8afaf8 Steam trial reached sign-in; the user reports completing
login and reaching the home screen. The owned unit then terminated with
`oom-kill` after 2 minutes 49 seconds at its former 2 GiB cap. This is a failed
consumer trial, not accepted Steam evidence. Its owned cleanup completed;
historical receipts remain unchanged. Earlier samples with zero OOM counters
were observations before the terminal kill and do not disprove this outcome.

The ordinary-user runner now derives a hard limit of half detected physical
RAM and a soft limit of three quarters of that budget. Startup requires the
whole budget plus a separate reserve of at least 2 GiB or 15 percent of RAM;
live reserve and pressure checks remain active. Swap remains disabled for the
owned unit, with the existing bounded deadline and complete teardown. No host
configuration, acceptance assertions or product queue sizes change. Regressions
cover budget calculation, inadequate startup headroom and OOM classification.
The receipt explicitly describes whether existing profile credentials were
copied; it no longer asserts that interactive credentials were never used.
A new private profile requires another direct login. Login alone does not close
recognition, input/output, sibling/removal or isolation acceptance cells.

### User-authorized retained isolated Steam profile

The user confirmed sign-in and the home/store screen in the host-sized-budget
trial, then explicitly requested retaining that login for later trials. The
ordinary-user runner now has opt-in `--profile-directory` reuse of an identified,
private test profile. The real passwd home stays hidden and no normal account
profile is copied. The selected directory must be separate, owned, private,
non-symlink and carry the lab identity marker; unexpected entries fail closed.
A held file lock excludes concurrent use. Profile retention is explicit in the
receipt; default disposable-profile behavior remains unchanged. All process,
display, temporary namespace and sentinel cleanup remains mandatory even when
the identified test profile is retained. Test credentials remain outside tracked
source and are not exported as acceptance artifacts.

The owned trial was stopped after user-confirmed home access to prepare device
isolation and validation. It was not a completed controller acceptance trial.
Final pre-stop counters are preserved externally. Regressions use synthetic
profile state and cover reuse, actual-home rejection, missing identity markers,
symlinks, unsafe permissions, FIFO markers, concurrent locking and the exact
private-home bind. Steam recognition still needs an allowlist of test-owned
input/hidraw devices; exposing the host's complete device directories is excluded.

### Hostile-journal restart probe extension (live evidence pending)

The existing finite broker-death phase now tests truncated and malformed
root-owned journal contents after canonical pending-record rejection. Each
mutation requires the held inode and exact original bytes, is bounded, and is
restored only after startup rejected with the fixture unchanged. A separate
controlled inode-replacement trial verifies startup remains rejected and that
the old identity cannot authorize removal of the replacement. Both injected
identities are registered before restart; cleanup of the replacement uses only
its independently held creation identity before restoring the original inode.
No record value authorizes signaling a PID or detaching a port.

Synthetic regressions cover exact-inode/content mutation, symlink rejection,
bounded fixtures, all three rejected cases, original inode restoration and
refusal to claim success if startup admits the replacement. The fixed eight
phase names and sudo action categories are unchanged. This root orchestration
extension requires administrator installation of a new immutable snapshot before
its live cases can be accepted. Existing 66f47dd live passes remain narrower
historical evidence. Construction/cleanup-failure and remaining sibling-failure
cells are still unresolved; this extension does not close the entire provider
gate or audio, output, accessibility and Steam acceptance.

### Creation-count accessibility regression

The creation-count arrows already accepted real Space/Tab keyboard events, but
they emitted no descriptive widget information and painted no explicit keyboard
focus outline. A real egui regression failed on the missing increase label before
the correction. The arrows now identify their increase/decrease actions and draw
a focus outline. The same regression checks exact count changes, traversal through
both arrows to an adjacent control, descriptive output events and the painted
focus rectangle. Mouse and typed-number behavior remain unchanged.

This GUI executable change invalidates reuse of the earlier neutral-soak image
for final GUI acceptance. A new image must receive its own two-hour soak after
remaining GUI changes are settled. The focused regression passes; complete
interactive accessibility and the remaining five gates remain open. Separately,
exact e24c9ee Git consumers passed all eight feature combinations and cached
locked offline rebuild. All 17 downloaded e24c9ee SBOM identities verify.

### Hostile-journal live trial: cross-filesystem lab correction

The installed e24c9eeb363d3eb2825864665012b3c26e6fcbe3 snapshot passed
canonical, truncated and malformed pending-journal startup rejection for the
first family. The phase then failed before the inode-replacement check because
its backup rename crossed from the runtime journal filesystem to disk staging
(`EXDEV`). This is a confirmed lab defect, not evidence of a provider recovery
failure or a completed gate. The receipt records no cleanup errors and restores
the original broker service inactive/socket active state. Keep this failed run
as historical evidence.

The backup now stays on the journal filesystem, outside both pending-record
directories, under the unique owned instance/generation name. Held inode/content
checks and refusal to remove a changed identity remain required. A synthetic
regression uses distinct staging and journal filesystems and verifies the exact
original inode/bytes are restored, the backup is removed, and staging receives
no journal. The immutable installed lab must be refreshed before rerunning the
complete all-family hostile-journal phase. This correction adds no public API,
provider policy, phase names, sudo scope or dependencies. Full provider recovery
and the other five gates remain open.

### Hostile-journal rerun: temporary activation-limit exhaustion

The b7c8dbcbf0c703048bc997ea7c78fe45859521e9 live phase completed the
first family's canonical, truncated, malformed and controlled replacement-inode
startup rejection, held-identity refusal and operator restoration. The next
family's connection failed: the scoped journal shows the temporary socket hit
its trigger limit after repeated intentional startup failures. The overall
phase is failed, not accepted; no cleanup errors were recorded and the original
broker service inactive/socket active state was restored.

The lab now states the existing systemd non-accepting socket defaults explicitly
(two-second trigger window, twenty activations). Between intentional rejection
trials and before positive recovery, it resets only its own service/socket
failed state and waits just beyond that window. The socket trigger counter is
not assumed to reset on stop/start or service reset. No rate limit is disabled,
no burst is increased, and installed units are untouched. Synthetic regressions
exercise five sequential restarts, exact owned-unit scope and ordering, and
refusal to activate after a reset failure. A refreshed immutable lab and full
all-family rerun are still required. Remaining construction/cleanup-failure,
sibling-failure and other gate cells remain open.

Startup-rejection evidence now also requires the candidate service to have run
and exited with its expected error status; rate-limit refusal, signal termination
and missing execution cannot count as a passing journal-rejection trial.
Synthetic cases cover these false-positive paths without modifying any record.
The pacing follows systemd's socket trigger accounting, which is separate from
service start-limit state (systemd v257 `src/core/socket.c`, trigger-limit check).

### Trigger keyboard identification

A new real egui Tab-event regression reproduced an accessibility defect in the
production trigger stack: focused digital trigger buttons identified themselves
only as “Press”, and analog sliders supplied no control name. The controls now
emit the declared trigger label in their widget information. The regression
checks both names, traversal to an adjacent control and that traversal emits
only the exact neutral analog value on focus loss, never a trigger press.
Visual layout, mouse operation, controller values and ordinary APIs are unchanged.
This is deterministic focus/event evidence, not a complete screen-reader or
owned-display acceptance result. Final GUI soak still needs the final image.

The [GUI control inventory](ALPHA_GUI_CONTROL_INVENTORY.md) enumerates every
interactive boundary, including supporting extra-axis paths, its deterministic
evidence and outstanding owned-display acceptance. No row is closed by layout
or compilation alone.

### Keyboard access to information controls

A real Tab-event regression failed before the correction because the realization
information button rendered its transport explanation only while the mouse was
hovering. The shared information path now shows a tooltip anchored to the focused
control without requiring a pointer and supplies descriptive widget names. It
covers realization help, target capabilities, audio creation details and audio
session details. Mouse-hover behavior and the explanation contents remain intact.

Regressions verify that unavailable-gadget technical details are actually painted
with keyboard-only input, focus reaches the adjacent control, information closes
on focus loss, and both audio information controls display their explanations
without changing creation options. These tests establish their rendering/focus
boundary only. Full owned-display control traversal, active isolated input and a
fresh final-image two-hour soak remain required; no whole gate is closed here.
The scoped recovery lab update is still a separate pending prerequisite.

### Stopped-unit collection during recovery

The e9be9bf4dd1517b2ad5abb971b3a58bddd16baf5 privileged rerun failed
before pending-journal rejection: `reset-failed` attempted to reset a stopped
socket which systemd had already unloaded. Its initiating error is retained,
cleanup reported no errors, and the original service inactive/socket active
state was restored. This was an orchestration defect; it supplies no new passing
provider recovery evidence.

A separate ordinary-user synthetic systemd control exposed the corresponding
inactive-service collection boundary before packaging. Recovery now queries the
owned service state, resets only a retained failed service, skips reset for an
inactive definition, and rejects running/transitional/ambiguous states. The
socket's bounded trigger window still expires before activation; its limits are
not disabled or increased. New deterministic regressions cover stopped-socket
collection, reloaded inactive service and refusal to mutate an active service.

The corrected method passed four consecutive intentional rejection cycles and
a successful final connection on real disposable user service/socket units with
matching two-second/twenty-trigger settings. The control restored every owned
unit file/socket and reports no cleanup error. It is narrower than privileged
provider acceptance: it creates no USB attachment and does not close Gate 3.
A refreshed immutable privileged snapshot and full all-family rerun remain
required. The earlier failed run and the first diagnostic control failure remain
historical evidence outside tracked source.

Separately, exact 4ce67c1 platform/MSRV/security workflows passed; the MSRV log
prints Rust 1.85.1 during the actual check. All eight exact-Git consumers and
cached offline rebuild passed, and all seventeen downloaded SBOM identities
verified. Privileged CI and workflow-dispatch dependency review were skipped,
so neither is promoted to acceptance evidence for those boundaries.


### Recovery confirmation deadline after paced hostile-journal checks

The installed f29f2f0567ca2c511cc7117a09801dde236f719a rerun rejected the
first family's canonical pending, truncated, malformed and replacement-inode
journals and completed held-identity operator restoration. The full phase still
failed: the non-root client timed out awaiting confirmation after ten seconds,
then the supervisor encountered a broken pipe. Five required activation waits
alone total 10.5 seconds. The external failed receipt reports no cleanup errors;
the original inactive broker service and active socket were restored.

The client now retains its ten-second connection/authentication deadline and
worker-death confirmation deadline, but allows 45 seconds for broker-death
confirmation, which includes hostile-journal probes and completed restoration.
The five-second product connection/channel EOF checks, exact acknowledgement,
descriptor accounting and bounded owned-unit lifetime remain unchanged. No
confirmation is sent before restoration and no product/audio acceptance limit
is relaxed. Supporting-script change only; public Rust contracts are unchanged.

Two added deterministic regressions fail against the former deadline: paced
restoration must fit the confirmation budget while retaining every EOF check;
wrong confirmations and bounded timeout must still abandon/clean the session.
All 270 Python tests pass. The full privileged all-family rerun requires an
updated immutable client snapshot. This does not close Gate 3 or any other gate.
The failed f29f2f0 receipt is preserved outside tracked source. Review the
confirmation deadline separately from product service progress and refuse a
passing disposition without completed restoration and all-family receipts.


### Declared names for supporting axis keyboard controls

The keyboard inventory exposed unnamed supporting one-dimensional sliders and
generic stick metadata on two-dimensional pads. Each now identifies its declared
control; reset identifies the axis, and curated stick pads likewise use their
declared title. Input ranges, momentary release and pointer behavior are unchanged.
The real Tab/Space/WASD regression fails on the prior unnamed implementation and
passes with exact reset, movement, adjacent traversal and focus-loss neutralization.
This is deterministic GUI evidence, not native assistive-technology or full owned-
display acceptance. The updated GUI requires its own final executable/soak receipt.
All six gates remain open; immutable provider installation remains pending.


### Keyboard identification and activation of destructive symbol controls

Clear-name and per-controller removal buttons previously identified themselves
only as “×”. Their real production helpers now expose “Clear optional controller
name” and “Remove controller <declared name>” while preserving visible symbols,
layout and click behavior. A real Tab/Space regression clears the optional draft
and removes exactly controller 23 from a 28-controller scroll on a 160-pixel-high
surface, preserving both neighbors. It fails with the former symbol-only metadata.
The test covers the production buttons and list action boundary, not installed
provider cleanup or native assistive technology. Existing lifecycle regressions
remain intact. Full owned-display acceptance and final-image soak remain required;
all six gates remain open and the pending provider packet is unchanged.


### Neutral live GUI lifecycle isolation

The selected GUI UHID lifecycle test inherited enabled audio under the all-feature
configuration. It now invokes the same explicit neutral/no-audio configuration
as the owned soak before any controller is created. The deterministic regression
checks both inherited audio states and disabled exposure; the selected live test
asserts audio is disabled. The bounded live invocation passed two-controller
creation, repeated polling, individual removal, recreation and shutdown, including
zero owned HID nodes after cleanup. This receipt applies to the tested source
increment and does not prove full interactive accessibility or the final two-hour
GUI executable soak. No public behavior or dependency changed. All six gates
remain open; the immutable provider packet still requires installation.


### Whole-unit keyboard numeric controls and recovery acceptance

Battery percentage and touch-lockout seconds inherited egui's integral drag-value
default speed of 0.25. A single arrow step rounded back to the same integer, so
keyboard adjustment was inert. Both now use whole-unit steps and descriptive
metadata. The production battery widgets are extracted at their existing layout
seam; slider/entry traversal tests require exact 50→51→52 changes and disabled
no-ops. The touch duration regression requires 120→121 and clamps at 1/3600,
with adjacent traversal and a disabled no-op. Both fail with the former default
step. Pointer drag sensitivity likewise becomes one integer unit per point; ranges
and exposure rules are unchanged. No public Rust contract or dependency changes.
A rapid arrow immediately after Tab can also move focus before the upstream focus
filter is established. This diagnostic is retained as an unresolved keyboard
acceptance case; settled-focus tests do not close that case or the full GUI gate.

The installed bbc1e1eb93baffdccf2840ae130d0cb139043a34 broker-death phase now
passes all three audio families. Each rejects canonical pending, truncated,
malformed and identity-replaced journals, verifies exact terminal connection/channel
closure, owned worker exit and attachment removal, and performs held-identity
operator restoration. The original inactive broker service/active socket state is
restored. Failed prior receipts remain historical evidence. These recovery cells
are passed; construction-time worker death, combined construction/cleanup failure
and remaining provider cells prevent whole Gate 3 closure. Raw receipts stay
outside tracked source. GUI changes require final executable/soak revalidation.


### Rapid numeric focus initialization correction

The first-frame Tab→arrow diagnostic now has an explicit regression using the
original sequence without an intervening idle frame. The battery numeric widgets
request an extra egui pass when focus is gained, establishing the upstream focus
filter before the following frame. It fails without that request and passes with
exact per-input-frame values 50,50,51,51,52,52,52 and subsequent Tab traversal.
The settled-focus/disabled tests are preserved; changed flags are aggregated per
external input frame rather than counting internal layout passes. This resolves
the recorded rapid battery-focus case, not every possible numeric control or
whole native accessibility acceptance. Final rendered-image/soak checks remain
required; no ordinary-root API or dependency changes.


### Installed named-phase acceptance follow-up

Evidence revision: `bbc1e1eb93baffdccf2840ae130d0cb139043a34`, tree
`eabc429ebe3273871c35ce1317aaf972d711121e`. The installed snapshot completed
seven bounded phases: rejection, normal lifecycle, client exit after handoff,
client exit before handoff, worker death after readiness, broker death with hostile
journals, and sibling/admission recovery. Each passed its declared assertions and
restored the original broker/socket state. This closes those cells, not the entire
provider gate. Actual worker death during construction, combined initiating and
cleanup failures, and remaining forced sibling-failure cells still need evidence.

The eighth phase, USB functional audio, failed on the first DualSense cell and
therefore did not accept DS4 or Xbox 360. Playback delivered 240,000 exact frames
with no invalid markers or gaps. Microphone capture contained 11,760 silence
frames: 4,320 in warm-up and 7,440 in the measured interval. The worker reported
240,096 capture frames and 11,760 silence frames; the last client credit recorded
228,336 consumed marker frames and 228,720 submitted frames. The numerical
difference between capture and the last consumed credit equals the silence count.
These snapshots are not atomic: this supports a refill-starvation hypothesis but
does not prove an exact final conservation equation or exclude another defect.
The largest observed refill intervals approached 9.9 ms with an 8 ms operating
fill. Queue capacity and zero-loss assertions remain unchanged. No corrected
continuity behavior is claimed. Client and root cleanup reported no errors, shared
audio defaults stayed unchanged, and the original service state was restored.

Next audio investigation must align final consumed credit with worker counters
and correlate submitted marker ranges with capture/silence boundaries. It must
separate incomplete production from downstream loss before changing pacing.
Increasing queues or accepting silence cannot close this failure. Raw receipts
remain outside tracked source; historical failed runs remain preserved.

All six mandatory gates remain open. Later GUI-only commits require acceptance
against their own rendered executable; the installed provider receipt does not
establish whole-candidate acceptance. Physical comparisons and native-host timing
remain the separately deferred post-alpha issues. Self-replacement is not enabled:
allowing replacement of root-running code changes the earlier immutable-lab trust
boundary and requires an explicit choice of update scope.


### Quiescent USB microphone accounting regression

The diagnostic client now collects final microphone host time, consumed markers,
silence, completion and abandonment after ALSA and producer termination. It
requires two identical observations within three attempts and validates
host = consumed + silence and host = completed + abandoned. Moving, foreign or
inconsistent observations fail explicitly. The last producer credit remains a
separate historical sample; it is not substituted for final consumption. This
uses existing worker operations 3/5/6/7 and changes no worker protocol or ordinary
root API, producer pacing, queue capacity or acceptance threshold.

Five focused regressions cover stable final credit, bounded retries, unreconciled
counts, abandoned capture and exact credit replies. All 275 Python tooling tests
pass. This is a diagnostic correction, not live acceptance: the installed immutable
snapshot still contains the prior client. A reviewed payload update is needed
before rerunning the failed USB cell. The production failure and all six gates
remain open. No dependency was added.


### Direct C graph comparison preparation

The maintained independent C control now supports a direct producer-to-capture
connection as well as the existing loopback topology. The runner accepts only
`--topology loopback|direct` and supported quanta 128/256/512. Direct connections
resolve exactly the private control nodes and their FL/FR ports, reject ambiguity,
and create only those two links. Readiness and completion are bounded; timeout,
link failure and combined initiating/cleanup errors retain owned-process cleanup.
No product queue or marker/loss acceptance limit changes. Reports identify the
chosen topology and quantum; direct success alone cannot replace qualification
of the existing control or any product acceptance cell.

Five focused deterministic tests cover link identities/directions, readiness
failure, forced termination/reaping, unchanged loss rejection and independent
error preservation. The actual C self-test compiles with warnings denied. Live
comparison remains unperformed while the image-pinned neutral GUI soak is running;
audio experiments must not overlap it. Once it ends, prebuild the control and run
at most two short diagnostic trials per changed topology/quantum before deciding
whether new causal evidence warrants correction or qualification.

Portable reproduction (reports and binaries must remain outside tracked source):

```sh
cc -Wall -Wextra -Werror scripts/alpha-audio-control.c -o /tmp/alpha-audio-control $(pkg-config --cflags --libs libpipewire-0.3)
python3 scripts/run-alpha-audio-control.py --control /tmp/alpha-audio-control --seconds 3 --trials 1 --topology direct --quantum 512 --report /tmp/alpha-direct-control.json
```

Use a fresh report/ledger destination for each repetition. A three-second result
is diagnostic evidence, never a sustained qualification receipt. All six gates
remain open. No dependency or ordinary-root API changed.


### Indexed USB markers replace a demonstrated false-success pattern

A deterministic fixture reproduced a false success in the previous USB audio
checker: remove 97 measured frames and replay 97 later frames, preserving the
sample count. The repeating 97-frame samples are byte-identical under that
corruption, and the previous checker accepted them. This is a confirmed harness
blind spot, not evidence that the library caused the historical live losses.

Playback and microphone sources now use indexed signed-16-bit markers. Stereo
uses high/low words per frame; mono uses positive/negative-tagged frame pairs.
The tested duration is far below their index-wrap limit. The four-channel playback
pattern validates both additional channels too. Production-worker IPC fixtures
and the standalone USB probe use the same indexed scheme, with exact-byte protocol
checks. The probe advances its position only by admitted frames and preserves
partial-queue-admission behavior. No queue capacity, operating fill, service or
latency threshold changes, and no ordinary-root API or worker control protocol
changes. These are synthetic harness payloads; historical receipts retain their
original scheme and results.

Capture is decoded once across warm-up, the unchanged measured window and one
second of explicitly reported trailing capture for marker-edge validation. Mono pairs crossing either window
edge are validated together; an incomplete final pair cannot prove a measured
frame. Warm-up/trailing-capture counts remain separate. All measured frames must be valid
with no silence or sequence gaps. Playback additionally requires every indexed
frame in order on the worker's PCM channel. The trailing capture extends trial duration but
does not shorten measurement or relax loss assertions. The source remains active
until capture completes; this is not producer-stop/drain acceptance. Complete
production and shutdown drain accounting remain separately gated.

Four new Python regressions cover balanced period loss/replay in all three
families, mono window-edge/complete-pair behavior, index wrap/channel/framing/bounds and
partial/reordered/replayed pairs. Existing warm-up exclusion, steady-loss, channel,
frame-gap and partial-PCM regressions retain their assertions with indexed fixtures.
The Rust probe retains its wrap/partial-admission regression and adds odd-phase
mono admission. All 284 Python tests and both Rust probe tests pass. Process-level
USB worker validation passes three families; production-worker validation passes
all three families with both descriptor-slot layouts. These tests create no kernel
attachment or PipeWire graph and do not qualify sustained host audio.

The immutable installed lab still has its old client payload. Live USB revalidation
requires a reviewed payload update. Independent audio trials remain queued behind
the running neutral GUI soak. Historical silence and graph-loss failures are not
reclassified as passes; all six gates remain open. No dependency was added.

### Independent C ledger reconciliation

The maintained `scripts/analyze-alpha-audio-ledger.py` checks a single C control
receipt against its callback ledger and per-frame receipt counts. It requires
complete event/count trailers and exact producer/submission/capture counter
reconciliation; overflow, overlapping producer ranges and inconsistent counters
are rejected. Missing marker ranges distinguish incomplete production, negative
queue-return failures and unexplained loss after successful submission. Capture
chunk flags are observations, not proof that the graph caused the loss. The
ledger cannot independently separate graph loss from capture loss, and the tool
reports that evidence as unavailable. It never changes continuity limits or
turns a reconciled failing trial into acceptance.

Nine deterministic regressions cover complete evidence, balanced loss/replay,
production versus queue failure, flagged capture uncertainty, event/counter
mismatch, cursor/buffer bounds, startup/trailing-zero crossings and truncated/duplicate trailers. They pass without
starting audio, creating devices or compiling another GUI image. The existing
GUI soak continues against its frozen executable. The C callback binary is
unchanged; live comparison and reconciliation are queued after soak teardown.

To analyze a trial, extract its single callback receipt from the runner's
`trials` array and run `python3 scripts/analyze-alpha-audio-ledger.py --receipt
<receipt.json> --ledger <report.json.trial-1.ledger.jsonl>`. Keep both inputs and
the analysis outside tracked source. This supplies diagnostic evidence only;
all six gate closure requirements remain in force.

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

## Bounded realtime callback comparisons

After `8146f71`, external diagnostic C variants compared buffer quanta without
product sessions or queues. The source, sink and private daemon data loops and
main threads had verified round-robin priority 20 through existing RealtimeKit
admission. Each planned/generated/successfully submitted 144000 frames, with
zero duplication/corruption and complete reconciled ledgers:

| Topology | Quantum | Missing frames | Production seconds for three planned seconds | Result |
| --- | --- | --- | --- | --- |
| Direct | 512 | 2560 | 3.639789585 | Failed |
| Direct | 256 | 13056 | 4.309629169 | Failed |
| Direct | 128 | 34560 | 4.830507877 | Failed |
| Loopback | 512 | 2560 | 3.732355752 | Failed |

Every measured direct source callback filled the requested quantum; available
capacity was 12288 frames. No request was underfilled. The graph clock nevertheless
advanced by multiple quanta between some callbacks. Main-thread scheduling alone
and these smaller quanta did not resolve loss or the rate deficit. Callback CPU
cost, graph discontinuity and VM scheduling still require discriminating evidence;
these results do not identify the cause. Xrun totals remain unavailable.

These are three-second diagnostics using experimental C callback/concurrency
instrumentation outside tracked source, not maintained-control qualification or
product acceptance. They followed prebuilding and soak teardown, with no competing
compiler, GUI or Steam trial. All granted owned processes exited, and private graph
supervisors returned terminal failures after cleanup. No shared scheduling/service
configuration, queues, loss thresholds or latency limits were changed. No 60-second
qualification or product matrix is accepted by these results. Raw binary, grant,
clock, requested-buffer and ledger receipts remain outside tracked source.

## Callback cost and separate profiler evidence

A separate external C diagnostic at `4e830bb` still missed 3072 of 144000
submitted frames. Measured source entry-to-ledger CPU/wall maxima were
119959/120583 ns, and capture maxima were 142125/142709 ns; callback gaps
reached about 40 ms. The measured callback bodies are much shorter than the
512-frame period (about 10.67 ms). These observations do not time arbitrary
PipeWire/kernel work outside those boundaries.

A separately identified profiler-perturbed trial missed 11264 frames. Its complete
raw JSON snapshots expose follower counters: within the source's measured wall
bounds, the producer xrun counter progressed from 49 to 147, while the receiver
stayed at 8 and the dummy driver's counter stayed at 0. Those are distinct node
counters, not a fabricated aggregate `graph_xruns` value. PipeWire 1.4.2's
[`node_ready` implementation](https://raw.githubusercontent.com/PipeWire/pipewire/1.4.2/src/pipewire/impl-node.c)
increments a follower counter when its previous activation remains triggered or
awake at the next cycle. This supplies concrete producer activation/recovery
evidence in the profiled trial, without proving why completion was late or
assigning every lost marker to an individual xrun. Profiling changed the
measurement condition; it cannot substitute for quiet qualification.

The profiler output includes repeated cycle snapshots; do not count snapshots
as distinct cycles or sum cumulative xrun counters. Clock position/time steps
and per-node counter changes need separate interpretation. All profiled graph,
client and profiler processes were reaped. Quiet-trial xrun totals remain
unavailable unless separately measured. The next causal boundary is the producer
activation/wakeup/completion path, rather than enlarging queues or removing
continuity assertions. All six gates remain open.

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

## Idle/wake diagnostic disposition

The [criteria audit's idle/wake comparison](ALPHA_GATE_CRITERIA.md#idlewake-discrimination-at-9202985)
records new bounded evidence at `9202985173f5c29ddb7fc8c639ca6a06eff94b1f`.
A pinned quiet three-second experimental audio control delivered every marker
but failed production rate (3.375 s); adding a bounded ordinary active peer
passed that short diagnostic (2.997 s, zero loss/duplication/corruption).
Independent periodic wakeups show the same large improvement with an active
peer. These are diagnostic variants, not quiet qualification or a product fix.
All owned processes were reaped; thresholds and shared configuration remain
unchanged. Require a supported quiet-host correction and fresh pinned sustained
controls. Do not close Gate 6 using a CPU-burning helper, or infer that the
other five gates are closed. Raw receipts remain outside tracked source.

## Current installed candidate: three live phases passed

At installed revision `d4d8864a151b167046c38106f2295594569909e7`, the
USB functional, worker-death-after-readiness and broker-death/restart phases
all passed individually. DualSense, DS4 and Xbox 360 each delivered 144000
exact measured microphone frames with zero silence/gaps; playback markers,
quiescent accounting and shared-default preservation passed. Worker/broker
failure tests terminated owned resources, rejected hostile journals and
verified successful subsequent sessions. Original service restoration passed.
See the [criteria audit](ALPHA_GATE_CRITERIA.md#installed-candidate-functional-and-recovery-passes-at-d4d8864)
for exact hashes, scope and remaining cells. Raw receipts stay outside Git.

Reviewer: retain the earlier failed USB trial as history. A short functional
pass is not sustained continuity or routing/ownership acceptance. Worker death
after readiness is not death during construction. All six full gates remain
open; the accepted neutral GUI soak is still only one cell. Exact-head local
extended checks, Linux/macOS/Windows CI, CodeQL, Gitleaks, provider-contract and
SBOM workflows passed at `d4d8864`; all 17 downloaded SBOM identities were
verified. Privileged CI and dispatch dependency review were skipped. Later
changes here document receipts and do not change product binaries or APIs.

## Forced sibling failure preparation

The existing `provider-siblings-admission` phase is extended to require the
owned fault supervisor. After normal four-session admission/removal checks,
it keeps three audio-family siblings open while an additional worker is killed
for each target family. It verifies exact terminal errors/EOF, unchanged sibling
attachment identities, fresh worker diagnostics before and after replacement,
a fresh replacement generation, renewed admission rejection, descriptor baseline
and repeated owned cleanup. The finite named sudo actions are unchanged. The
root supervisor still selects only the journal/instance/generation/device and
pidfd-verified worker; it never accepts a caller-supplied PID or executable.
Its bounded initial readiness allowance includes the preliminary four-session
checks. Missing supervisor setup cannot yield a partial successful phase.

The live client previously allowed cleanup failure to overwrite the initiating
fault-test error. It now preserves both, with a deterministic combined-failure
regression. Additional regressions cover sibling survival/capacity recovery,
changed identities, partial failure with cleanup errors, invalid allowlists,
reused generations, descriptor leaks and routing through the owned supervisor.
These are tooling corrections and preparation, not live acceptance. The
installed `d4d8864` snapshot still runs the earlier phase; updated immutable
payload deployment and an individually bounded live run are required. Product
binaries, ordinary-root APIs, queue sizes and dependency policy are unchanged.
Construction-time worker death and live combined construction/cleanup failure
remain separate open cells even after this sibling phase eventually passes.

### Forced sibling recovery and autonomous updater increment

The installed `f05058c053423eb78b6e3580e0026d5f91309162` sibling-admission
phase passed: each audio family underwent an identity-verified worker kill while
three sibling sessions retained port identity and answered fresh diagnostics.
Replacement admission, full-capacity rejection, descriptor accounting, follow-up
normal/repeated/abandoned close and owned restoration passed. Raw receipts remain
outside tracked source. Construction-time worker death and live combined
construction/cleanup failure still remain required; this does not close Gate 3.

[Autonomous lab updates](ALPHA_LAB_AUTONOMOUS_UPDATES.md) describes the newly
authorized one-time bootstrap, fixed-mailbox root code replacement and later
revocation. Deterministic updater regressions pass; administrator bootstrap and
live update remain pending. All six release gates remain open.

### Autonomous deployment and pre-handoff worker-failure probe

Administrator bootstrap succeeded. The fixed updater installed candidate
`37115ff4cb51ca3495c69ed7f7a8d48fd44d8ac6` without trial/service actions.
The installed candidate's subsequent rejection phase passed and original
broker service inactive/socket active state was verified. Revocation remains
available; no general sudo shell was granted.

The worker-death phase now additionally arms a non-root client before its open
request, pins a staged production worker through executable/parent/UID/cgroup/
instance/generation identity and a pidfd, and kills it during construction.
The client must receive an exact pre-handoff error and EOF. A successful open
is a late-injection failure, never a pass. No fixture worker or production
readiness delay is substituted. Five new deterministic regressions cover
identity refusal, arming order, exact replies, late handoff and socket cleanup.
Live construction-failure acceptance is pending until its bounded run passes.
Combined initiating/cleanup failure and the other six-gate requirements remain
open. This supporting lab change preserves ordinary-root APIs and binaries.

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

### Combined-error live acceptance and all-family exit coverage

Installed `5227dada18b30630cd3cadd1e4967ae788b860f4`, tree
`aa500df88c889f17da3c7d7c60795b27758cd456`, passed the worker-death phase
including three pre-handoff kills and three combined worker/journal-cleanup
failures. Every combined failure preserved the exact initiating SIGKILL and
cleanup identity error. Three explicit operator restorations verified both held
journal identities and attachment removal; subsequent normal/repeated/abandoned
sessions passed. The root-owned receipt reports `passed`, with no initiating
lab failure. Original broker inactive/socket active state was rechecked.

An audit found the older client-death probes selected only DualSense. Both exit
phases now exercise all three audio families separately, requiring empty owned
journals, all authorized ports free and no broker children before a fresh
positive lifecycle suite. A zero broker PID means pending startup rather than
cleanup success. Three additional regressions cover family request encoding,
per-family sequencing and pending startup. Full exact-candidate provider phase
acceptance remains pending; no whole-gate pass is asserted by this increment.

### USB output policy correction and acceptance invalidation

See [controller output acceptance](ALPHA_CONTROLLER_OUTPUT_ACCEPTANCE.md) for
the confirmed interrupt/SET_REPORT validation mismatch, controller field ledger,
correction and regression map. Six production-worker process cells and three
USB protocol-fixture cells pass; all 329 Python tests, the mandatory checks and
forced Rust 1.85 check pass. Kernel-consumer output evidence remains separate.

Production worker and GUI-soak binary identities changed. Revalidate Gate 3
on the corrected worker and repeat the final GUI soak; the earlier pinned
provider and GUI passes must not be promoted to the new images. Other gates
remain open and alpha remains not ready. Native UHID raw observations and
ordinary-root/supporting-trait signatures are unchanged. No dependencies added.

## Provider revalidation after USB output correction

All seven provider phases passed again at `e5402d24c7ca8e0c209260c879feb205a95f8dae`, tree `6894a9c13cb9a6931b22115e06452ec3ccf03fd1`, including the changed production worker. Independently retrieved root receipts agree on source and image hashes. The original broker service returned inactive and its socket active. Gate 3 retains its pass for this candidate; the earlier revalidation requirement is fulfilled. The new GUI image still requires its own final two-hour soak. Live controller outputs, routing, Steam and sustained continuity remain open.

## Live USB output and DS4 rollback increment

Live `usb-functional` passed at `e47da258977e58e014d3121eac3d9e3205c6454d`:
DualSense and DS4 synthetic outputs traverse both kernel interrupt OUT and
control SET_REPORT paths with exact lengths and one ordered worker observation.
Xbox's generic HID profile rejects every tested output with EPIPE and no event.
The short duplex and final cleanup checks pass for all three families. A preceding
probe failure caused by zero-padded VHCI ports is preserved; its correction has
an exact-format regression. The probe transfers only an identity-verified owned
HID descriptor to a non-root client, with no input-group or node-permission changes.

The new opt-in `live_second_open_failure_removes_observed_gamepad` regression
also passes: three real DS4 gamepad creations followed by injected contact-node
open failure each remove the kernel-observed gamepad through production rollback.
No touch node or input frame is produced. Review the exact object/label checks,
fail-closed descriptor handoff, ordered observations and descriptor cleanup.
These close specific Gate 2 cells; typed USB application callbacks and remaining
complete isolation/lifecycle dispositions are still required. Gates 1, 2, 4, 5
and 6 remain open. All five required repository checks, forced Rust 1.85 workspace
check and 337 Python tests pass for this increment. No dependencies were added.

## Typed USB output cell acceptance and separate PCM failure

At source `9e5731e72818fe722b6c55d04b44c0f51a4b3865`, tree
`15f4c9e4e0bc8d23fa1929cad8842da8afe5d9d0`, the independently frozen
ordinary observer SHA-256 was
`06643664ea4f9ea93a35386b3c118828f45ebc5fbc19e051abeeb64e6b05e21f`.
Its selected live test passed all three families: eight exact ordered DualSense
callbacks, six exact ordered DS4 callbacks and six Xbox EPIPE rejections with
no callback. Both kernel endpoint paths, five idle service cycles after each
request, zero dropped outputs and repeated close were checked. The root client
receipt independently records typed callback completion for each family.

These Gate 2 output cells pass; the combined `usb-functional` phase and
coordinator deliberately remain **failed**. The separate unchanged PCM marker
assertions measured 7936/9376/10672 silence frames for DualSense/DS4/Xbox,
respectively. Producer wake intervals reached approximately 10 ms against the
8 ms operating fill. Playback delivered all 288000 markers per family with
zero gaps/corruption; microphone conservation reconciled without queue drops.
This narrows the microphone failure to producer refill starvation evidence,
not a demonstrated correction or permission to increase queues/relax limits.

Every family reports no initiating exception and no cleanup error. The root
supervisor and ordinary unit cleanup also report no errors; original broker
service inactive/socket active state was rechecked. Observer peak memory was
1.7 MiB, with roughly 18 GiB host memory available and zero measured memory
pressure. Broker and worker hashes remain identical to the accepted Gate 3
images. Raw failures and full receipts remain outside tracked source.

The five mandatory checks, forced Rust 1.85 workspace/all-target/all-feature
check and 349 Python tests pass. No dependencies or ordinary-root signatures
changed. Gate 2's remaining realization/lifecycle dispositions, final-image GUI
acceptance, routing, Steam and sustained continuity remain open. Review the
shared socket flag ownership/pause protocol, bounded receive deadlines and
retention of the original terminal error separately from cleanup failures.

## Public USB audio shutdown correction and exact-image revalidation

See [public-root USB audio acceptance](ALPHA_USB_AUDIO_ROOT_ACCEPTANCE.md).
The public sample factories now pass creation/service/normal and repeated close
for all three audio families. Deterministic regressions preserve channel ownership
until worker acknowledgement and retain genuine initiating/cleanup failures.
All seven provider phases passed again at `6576c4732d301683195ff66cc3b57836abfbcc24`;
its changed worker image is covered. The independent microphone marker cells
still fail and Gate 4 remains open. Gates 1, 5 and 6 also remain open.

Exact-head CI, CodeQL, full-history Gitleaks, Provider Tier B and all 17 SBOM
identities passed at `c6ff0259baa52d18a5694bdbd6e983e7eeeae582`. Those are
previous-head results; final-source CI and artifacts must be revalidated.

## Ownership evidence checkpoint at `9ffa522`

The public-root USB audio ownership matrix passed all twelve cases (three audio
families × four sample/native combinations) at source
`9ffa522ef434f3714363175ee223356607a80242`. Each of twelve native endpoint
instances was observed exactly once in the private graph while alive. Short
three-second direct-ALSA duplex markers passed all three profiles; observing
parent, selected child and combined phase returned zero with owned restoration
complete. See [the root acceptance receipt](ALPHA_USB_AUDIO_ROOT_ACCEPTANCE.md)
for the exact image, scope and retained historical failures.

Gate 4 remains open: this does not establish route A/B/disconnect/reconnect,
endpoint/client failure or sustained qualification. Earlier intermittent USB
enumeration failures also remain unresolved; post-close snapshots demonstrate
asynchronous removal without proving its role in those failures. Do not classify
a successful short matrix as complete repeated-creation reliability.

The rebuilt neutral GUI soak was started separately at that source with image
`2e130b8de1bec47ce582959c12c2e9054033a7d9f9a47e22610c01c4962d1c67` and
7200-second duration. It is **running, not accepted** at this checkpoint. The
first 30-second sample had 124380 KiB RSS, 16 descriptors, 23 threads and no
children; host memory pressure was zero. Final duration, growth and owned cleanup
must be verified from its terminal receipt before closing that Gate 1 cell.
No compilation, Steam or audio trial may overlap this soak.

Gates 2 and 3 retain their supported dispositions; Gates 1, 4, 5 and 6 remain
open. Review native node membership, bounded observer failure, asynchronous USB
teardown and whether the final GUI executable remains byte-identical to the
accepted soak image. This checkpoint changes tests/evidence, not public APIs,
product behavior, dependencies or acceptance limits.

## Isolated Steam device preparation at `ebc5273`

Gate 5 remains open. The prior Steam bootstrap deliberately constructed a private
`/dev` with no controller devices, so bootstrap/home-screen evidence could not
establish controller recognition. The maintained ordinary-user runner now accepts
`--input-owner-pid` and derives a bounded event/hidraw allowlist from that live
same-user producer's process-owned kernel labels. It pins process start time,
checks sysfs device numbers and character-node inode identities, exposes only
individual device nodes and validates the resulting namespace inventory before
Steam starts. The parent stops the consumer on producer identity changes or node
replacement; expected removal is recorded separately. No arbitrary device-path
argument, host permission modification or new sudo action was added.

An initial metadata-only namespace probe passed but a strengthened open probe
failed with permission denied. Ordinary host opens succeeded. Installed
bubblewrap documents `--dev-bind` as permitting device access; ordinary readonly
binds were inappropriate for these character nodes. Individual device-enabled
binds corrected this demonstrated harness defect. The sentinel now verifies both
mounted/opened identities and read/write open permissions without consuming
input or sending output, and closes every opened descriptor.

At source `ebc5273aacef565dfe144cbe7b58c366f99bf76d`, tree
`b5f27ed48255c4210b488f6a2147d6a070d038da`, the non-Steam namespace proof
passed for eight owned nodes of the live neutral two-DualSense GUI workload.
Real/passwd home remained hidden, capabilities were dropped, no unrelated
consumer nodes were exposed, all eight opens passed and cleanup was empty.
Steam was not launched and neither retained login state nor controller input
was read. This closes preparation for that device set, not the four-family
recognition/input/output/removal/sibling acceptance matrix.

Reproduce with a live ordinary test-owned controller producer (not an arbitrary
user process):

```bash
python3 scripts/run-alpha-steam-lab.py --apply --input-owner-pid "$REVIEW_PRODUCER_PID"
```

Regressions reject foreign/dead/reused producer identities, non-character nodes,
duplicate/invalid bindings, changed device numbers/inodes, unexpected namespace
nodes and replacement during open. Removal remains distinct from replacement;
failed open/identity checks cannot leak descriptors or start Steam. All 361 Python
tooling tests, format and whitespace checks passed. Rust compilation and full
workspace source checks are deferred until the active two-hour GUI soak finishes;
no Rust source, dependency or GUI executable changed in this increment.
The final Steam trial still requires all four families and available realizations,
actual consumer behavior, profile isolation and complete owned cleanup.

## Final-image GUI soak and four-family consumer preparation

The neutral production GUI soak completed 7,200.875 measured seconds with exit
status zero, no reported error and an empty cleanup-error list. Its source was
`9ffa522ef434f3714363175ee223356607a80242`, tree
`983963a88f653f1ff2c11c1f9b07c97f8cf0245a`. After completion, rebuilding the
current GUI example produced the same SHA-256 image:
`2e130b8de1bec47ce582959c12c2e9054033a7d9f9a47e22610c01c4962d1c67`.
The owned process, display and HID devices were absent after teardown.

This workload continuously serviced two neutral DualSense controllers and
replaced one every minute. External descriptor count remained 16; the final
sample had 23 threads and no child processes. RSS rose from approximately
124,380 KiB after warm-up to 144,488 KiB, including a step near minute 57,
then remained within approximately 36 KiB of that plateau for the remaining
hour. This is bounded growth in this workload, not proof for arbitrary inputs,
all families or audio-enabled operation. Gate 1 still requires the complete
interactive control inventory. No native timing or physical fidelity is claimed.

The new ordinary-root `steam_consumer_probe` example creates all four families
and remains neutral until an explicit private-socket `arm` command. Only arm
it after verifying the isolated controller-test screen. Its bounded pulse cycle,
per-controller removal, service scheduling, output accounting and repeated
cleanup are preparation tooling; producer success alone is not Steam acceptance.
It records control-directory and socket identities for external cleanup
coordination and rejects changed identities rather than removing replacements.

The first neutral four-family namespace check failed opening a raw HID node.
Its producer was terminated and its HID devices disappeared, but its control
socket directory remained: this failed receipt is preserved, not accepted as
complete cleanup. A follow-up ordinary-user diagnostic proved the same raw HID
access failure outside the namespace: one owned node was root-owned mode 0600,
while the other twelve event/raw nodes opened read/write successfully. That
follow-up used the producer's stop command, exited zero and removed its control
directory. No input was armed and no Steam consumer acceptance is claimed.
Temporary access and desktop isolation must be established for all owned nodes
before the full consumer trial; do not omit the inaccessible node or grant
access to foreign devices. Raw receipts remain outside tracked source.


## Maintained C control: quiet versus active CPU diagnostic

At `ff4b2978bcc423df93a98f0248074504dd9a494d`, one bounded comparison used
the unchanged maintained C control source and recorded compiler/binary receipts.
Both trials pinned their owned daemon/client tasks to the same CPU and verified
realtime priority 20. The quiet three-second trial generated/submitted all
144000 frames but received 120448, losing 23552 frames across 36 ranges; its
production took 3.684 seconds. The ledger shows successful queue returns,
callback gaps and graph-clock steps at the affected window. Loss localization
is still graph/callback delivery, not a product PCM queue.

Adding a bounded ordinary-priority active process on the same CPU delivered all
144000 frames in 2.997 seconds with no missing, duplicate or corrupt markers.
A subsequent 60-second diagnostic delivered all 2880000 measured frames in
59.999981445 seconds, with zero loss/duplication/corruption/partial frames,
zero ledger overflow and complete owned-process teardown. A live peer sample
confirmed ordinary execution and approximately 46.85 CPU seconds during its
first 47 wall seconds. The direct C trial measures continuity, not latency.

This strengthens the idle/wakeup scheduling hypothesis but does **not** close
Gate 6 or replace quiet-host qualification. A CPU-burning helper is not a
product fix. The initial wrapper also failed because it expected a separate
client data-loop thread; maintained C callbacks execute on the main loop.
That failed setup receipt is retained, and the corrected diagnostic grants the
actual main callback thread and observed daemon loop threads. No loss, rate,
latency or queue-size threshold was changed. A supported quiet correction and
all required controls/product trials remain necessary.

## Owned raw input preparation extension

The input-isolation runner now supports an explicit optional raw-HID access
rule for one validated non-root UID. Its default still changes no raw-node
permissions. The opt-in rule matches the gated child PID in complete HID
physical identity, sets owner-only access for newly created matching nodes and
never selects by vendor/product alone. Restoration now accounts for both event
and raw HID nodes before removing its identity-pinned temporary rule.
Two added deterministic regressions cover explicit admission, malformed/root
UID rejection, foreign process-prefix exclusion and retention while raw nodes
remain. All 363 Python tooling tests and an ordinary-user udev syntax check
passed. This source extension is not installed or accepted live yet; the existing
autonomous updater deliberately rejects changes to its fixed phase/payload scope.


## Demo capture-loss accounting correction

Inspection of the implemented demo routing path found two silent microphone
loss boundaries: its eight-packet host queue evicted old packets without
accounting, and `AudioRouter::input` repeatedly drained packets to keep the
newest one without accounting for the discarded frames. Both behaviors were
bounded but could conceal loss during slow-consumer/routing acceptance.

The private capture queue now counts complete discarded frames at both
boundaries and atomically returns its newest packet. Routing no longer performs
an unaccounted repeated drain. Existing host-routing error displays report the
cumulative discarded-frame count alongside any subprocess failure, preserving
both observations. The queue remains eight packets; frames, formats, root API,
latency and continuity limits are unchanged. Fresh input delivery resumes after
a stall while its historical loss remains visible.

Five new regressions cover exact overflow plus stale-read reconciliation,
no-overflow stale discard, combined loss/process error reporting, real bounded
subprocess-pipe stalls and child reaping, and rejection of a partial input frame
before queue admission. The real pipe sends ten stereo packets; two overflow
and seven stale discards account for exactly 2304 frames, with the newest packet
retained. No hardware or private data is used.

The mandatory fmt/check/Clippy/test/Gitleaks suite passed after correcting a
signed conversion in the initial regression fixture. This changes production
GUI code. The previous completed soak remains passing historical evidence for
its exact image; the final GUI image must receive a fresh independent two-hour
soak. Wait until interactive fixes are settled before starting that repeat.
Gates 2 and 3 remain passed because their controller/provider implementations
are unaffected. Gates 1, 4, 5 and 6 remain open.

Before this GUI correction, the `b09bb3d0645a98cf71061a1a8632e844526817d0`
checkpoint passed forced Rust 1.85.0 all-target/all-feature workspace checking.
Exact-head CI included successful Linux/macOS/Windows build-and-test jobs,
MSRV, lint, corpus, supply-chain and policy jobs. CodeQL and full-history
Gitleaks passed, and all 17 downloaded workspace SBOM identities validated.
The privileged remote provider job was skipped; the separately recorded local
provider acceptance remains its evidence. These receipts do not accept the
subsequent GUI correction or any remaining live gate.


## Maintained demo routing transition fixture

The ignored `live_private_pipewire_route_switch_disconnect_and_reconnect`
fixture exercises the production `AudioRouter` and host subprocess ports inside
the maintained private PipeWire runner. It admits only an explicit ordinary-user
lab opt-in, a private runtime directory and a graph without ALSA monitors.
Two owned stereo loopbacks expose distinct source/sink targets. Route A forwards
an exact signed stereo marker; route B forwards a different marker. Disconnect
removes all router ports and rejects forwarding; reconnect forwards a third
marker. Final router teardown joins/reaps host subprocesses, and both owned
loopback children are verified absent. The first bounded trial passed all four
stages and complete private-graph teardown; final source/image receipts are
kept outside tracked source.

This closes the demo host-routing transition cell, not the USB/IP transport,
endpoint/worker failures, all-family/native ownership exchange or sustained
continuity cells. No root contract or production behavior changes in this fixture.

Prebuild the demo all-feature test executable, then run that exact executable
in `scripts/run-pipewire-audio-lab.py` with a 40-second deadline, explicit
`VIRTUALGAMEPAD_DEMO_ROUTE_LAB=1`, `--exact --ignored --nocapture`, and the full
test name `gui::audio_lab::tests::live_private_pipewire_route_switch_disconnect_and_reconnect`.
Do not blanket-enable ignored tests or replace transport acceptance with this
virtual-node fixture.


## Public USB sample exchange: exact markers and first failure

At `3f369bb0fc74f59b3849f65eee5668af287e03fb`, the public-root USB factory
fixture was extended beyond ownership and native-node creation. Its frozen
ordinary-user test image embeds the maintained ALSA marker validator, verifies
the creation-scoped ALSA card's serial and VHCI ancestry, checks pre-attachment
exclusion, and exchanges synthetic duplex samples through the public audio API.
Playback counts each measured index separately; duplicated frames cannot mask
missing frames. Microphone capture retains the maintained two-second warm-up,
three-second measured window and bounded drain. No queue or acceptance limit
was enlarged. All twelve existing ownership cells remain required, and the
outer verifier now additionally requires all three family exchange receipts.

The first live run failed before streaming because the embedded helper lacked
an import and named a nonexistent isolation function. The corrected helper at
`ff1cd9d753e88264a4bb8f638f5ee6baba0cfe06` adds an entrypoint regression that
runs the captured scripts together and verifies both isolation checks. Four
Python helper tests, 22 broker-validator tests, two Rust marker regressions,
the complete Python suite and all five mandatory Rust/Gitleaks checks pass.
The ignored factory test is executed only through the prepared ordinary client.

The corrected live run passed DualSense duplex sample exchange: 144000 measured
microphone frames with exact markers and no silence/gaps, plus 144000 playback
indices with no missing, duplicated or corrupt frames. DS4 playback also had
zero missing/duplicate/corrupt measured frames, but microphone capture contained
112 silent measured frames and therefore failed. Xbox public sample exchange
was not reached. The separate direct ALSA/worker subtrials passed for all three
families. Both failed aggregate receipts remain preserved outside tracked source;
both root restorations reported empty cleanup-error lists. No product fix or
whole Gate 4 pass follows from these results.

The next bounded diagnostic records the public caller's largest refill-cycle
delays, service/read/credit durations and retained queue/underrun counters before
any assertion. It changes test instrumentation only. Distinguish incomplete or
late application production from worker loss before changing production code.
The private graph check is client-scoped; shared desktop defaults and foreign
resource isolation are independently guarded by the parent prepared lab.


## Public sample duplex acceptance at `4301161`

The bounded instrumented repeat passed at
`430116165de5a967d3d70c202dc2b992fe9a9a83`, tree
`c91fdfe6c26f9935f0d648fc79164d4909f9c6e8`; frozen root test image SHA-256
`a0616ded3e820ed4dc3e2baf30c3db2afe2565c3ac67e413f34e0662377416a4`.
DualSense, DS4 and Xbox 360 each delivered all 144000 measured playback indices
without missing, duplicate or corrupt markers, and all 144000 measured microphone
frames without silence or pattern gaps. Each microphone capture also separately
verified 96000 warm-up and 48000 drain frames. The public caller produced 288384
frames against 288000 host-consumed frames; the 384-frame operating fill is
unconsumed tail, not measured loss. Each reported 16 discarded microphone frames
from the fixture's explicit pretrial flush of sixteen synthetic zero frames,
zero microphone underrun and zero discarded playback frames.

The largest observed caller cycle intervals were 4071 microseconds (DualSense),
2758 (DS4) and 2821 (Xbox 360), below the unchanged eight-millisecond operating
fill. These passing-run timings cannot establish the cause of the earlier
112-frame DS4 silence; its failed receipt remains unresolved evidence for later
continuity investigation. No repeated trial was used to erase that failure.

The complete enclosing phase and typed observer exited zero. All twelve ownership
cells and twelve native caller-node observations remain required and passed.
The parent lab retained the original desktop defaults and restored its owned
resources without reported cleanup errors. The installed broker service returned
inactive and its socket active; only the pre-existing host ALSA card remained.

This establishes short public sample/sample duplex functionality for the three
audio families, with explicit marker accounting. It does not establish native or
mixed-ownership marker exchange, transport routing/failure scenarios, latency,
sustained continuity, or whole Gate 4 closure. The candidate changes only tests,
validation tooling and review documentation; provider/root production images and
ordinary-root signatures are unchanged. All five mandatory validation commands
passed for this instrumented candidate. Gates 2/3 retain their prior dispositions;
Gates 1/4/5/6 remain open.


## Native USB caller disconnect/reconnect at `0d09b13`

The public factory's sealed ordinary-user fixture passed at
`0d09b13fd0b88b683865aab7ca1c7c10f75a7a9d`, tree
`3b74389be1898370556bbee870955e464078a9bd`, frozen test image
`50e6d89b6e28b02f277d718a78bda62e42de4ca0b8d9d4063087ee936873df65`.
All twelve native caller endpoints across the three families and supported
ownership combinations completed two connect/disconnect cycles. A bounded
`pw-cat` client connected to the exact caller node; its original graph links
were recorded by object ID and serial. After terminating and reaping that
owned client, the fixture required both client-node absence and original-link
absence while preserving the controller endpoint's ID and serial. Reconnection
used a new owned client; final root service, health, removal and repeated close
checks remained successful. An earlier node-removal-only checkpoint at
`d303e027143cc831d962b23ceec0042a41370845` also passed, but the current
receipt includes stronger explicit link-removal evidence.

The same enclosing fixture retained all twelve ownership cases and repeated
exact public sample/sample duplex exchange for DualSense, DS4 and Xbox 360;
all measured marker assertions passed. Its ordinary factory subtest completed
in 29.04 seconds, inside the unchanged 45-second deadline. The parent phase
and typed observer exited zero; both report empty cleanup-error lists and
restoration to original broker/socket states. No foreign graph/card/default
was changed. Seven deterministic native-helper regressions cover exact links,
reused identities, early client death, failure reaping, initiating plus cleanup
errors, dangling links and unavailable link identity. The outer verifier rejects
missing or incomplete native lifecycle receipts even when ownership and marker
receipts pass. All 375 Python tests, mandatory fmt/check/Clippy/test/Gitleaks
checks, and forced Rust 1.85 workspace/all-target/all-feature checking pass.

These checks establish consumer connection/removal/reconnection and endpoint
preservation, not native or mixed-ownership audio marker delivery, endpoint or
required-worker failure, transport routing transitions, sustained continuity,
or whole Gate 4 closure. The earlier DS4 microphone-silence failure remains
visible and unexplained; these later successes do not erase it.

Exact-head platform CI, CodeQL, Gitleaks, provider contract and SBOM workflows
passed for the earlier `125043d2e6836e235b3aba1031083787a8404627` checkpoint,
including all 17 downloaded workspace SBOM identities. Remote privileged provider
validation was skipped and remains separately supported by local lab receipts.
That CI checkpoint does not validate later native-client tooling changes.


## Mixed native microphone investigation and harness corrections

The mixed native-microphone/sample-playback acceptance extension at
`47d1d04391fe92572ef7568199bd1c9cb99e6de3` requires three additional
family-specific marker receipts. Ownership, node presence and lifecycle receipts
alone cannot satisfy those cells. The public fixture deadline became 70 seconds
and the enclosing observer deadline 75 seconds to accommodate the additional
six-second streams. Each stream retains two seconds of warm-up, three measured
seconds, one bounded drain second and the existing zero-loss assertions.
Native producer stdin writes are explicitly not graph-submission evidence.

Three independent harness defects were demonstrated and corrected:

- `3f86cd554e6cad3a8e5935ea4a70492814cb0f50`: retaining the parent
  `Command` kept its copy of the stdout socket open after child exit. Consume
  the command at spawn; a real child regression requires receipt EOF after exit.
- `f6046a04582748ca51fc6bee5765ce5cf3e32aa3`: one 128-frame refill per
  wake-up under-produced audio, while treating underrun silence as consumed source
  inflated refill credit. Refill the existing 384-frame operating fill using
  actual source consumption. No queue capacity or acceptance assertion changed.
  The regression covers delayed wake-up and silence-adjusted credit.
- `c785548460434406d76a1dddaa1a569c70a993ca`: final accounting requested
  sample-only microphone counters on a native-owned endpoint. Do not perform
  those reads; report unavailable values and use the separate native diagnostics.
  A deterministic regression rejects accidental invocation on native ownership
  and preserves errors on sample ownership.

Historical failures remain: the first pacing receipt produced 205488 frames,
reported 89680 underrun frames and discarded 5136 microphone frames. After the
credit correction, discarded frames fell to the 16-frame pretrial flush, but
19360 underrun frames remained and the measured capture contained 9136 silent
frames. These results do not close sample continuity or establish its cause.

At `9dae92f7c0dfba9cfa914d958c09430424a90256`, the native-first
fixture reached streaming but then encountered the ownership-accounting defect.
Original resources were restored with an empty cleanup-error list. The actual
caller thread reported ordinary scheduling; its timer-slack file was unavailable.
New diagnostics record sleep/wakeup intervals and generation/write time without
claiming that resumption delay is exclusively kernel sleep time. The missing
native cells execute before known sample failures, while every required assertion
remains enabled. This changes evidence collection order, not acceptance scope.

The five mandatory validation commands passed for both diagnostic and ownership
corrections. Provider and ordinary-root production APIs are unchanged; no new
dependencies were added. Reviewers must check parent/child descriptor ownership,
sample/native counter boundaries, silence-adjusted source credit, exact marker
accounting, unavailable graph-submission evidence and complete owned restoration.
Gates 2/3 retain their accepted scope; Gates 1/4/5/6 remain open.


### Exact-image mixed-native result at `c785548`

An initial retry copied an older executable from a fixed target filename. Its
hash matched the earlier diagnostic image, so that receipt cannot validate the
ownership correction even though the orchestration reported the new source SHA.
The failed artifact remains visible. The maintained USB acceptance runner now
supports `--public-samples`, obtains the root test image from Cargo JSON, freezes
it and requires the sealed factory receipt's hash to match. Regression tests reject
missing, duplicate, failed and mismatched-image factory receipts.

The subsequent exact-image run at
`c785548460434406d76a1dddaa1a569c70a993ca`, tree
`a2de9dc5a1db4d6367d87c36501d2dc43efde791`, used frozen image
`2124d286defb330af1b7d438f47349dbe5da2ced3770b7b48947871707e1eab1`.
The new counter regression was verified in that image before the trial.
DualSense and DS4 each received 144000 valid microphone frames, no silence and no
pattern gaps in the three-second measured capture window. Their root sample
playback also delivered all 144000 indexed frames without loss, duplication or
corruption. Xbox 360 playback passed, but its measured microphone window contained
143920 valid frames and 80 silent frames, so the required fixture failed.

Startup/drain accounting remains separate: cumulative microphone underruns were
2000/2000/2032 frames for DualSense/DS4/Xbox 360 respectively. All three native
graphs reported 1024 dropped frames and a final 1920-frame queue; source graph
submission counts remain unavailable. These totals still require reconciliation
before sustained or complete-production acceptance. Short measured success is
not a claim that every produced frame reached the graph or final drain.
The native source supplied 288000 frames through stdin in each case; this is not
graph-submission proof. Xbox's capture had 1952 startup silence frames plus 80
measured silence frames, consistent with its reported 2032 underrun frames.

The enclosing phase and observer failed as required. Original resources were
restored with empty cleanup-error lists. No repeated green run substitutes for
this failure. Gate 4 remains open and Gate 6 is unqualified; next evidence must
locate native graph drops and the Xbox underrun rather than weaken assertions.


### Native drop baseline at `8ea4550`

At `8ea455069070ce15803db09e0b784c565c4af866`, tree
`3627707d5ca6b0e32b1b0049ae4ed58e24b4879e`, image
`b0953ece1a94104113f3816cc15702982fd830474571dc6f2c2ec0345cdcf60b`,
the cached pretrial graph-drop counter was zero for all three families and rose
to 1024 during each marker trial. Prior lifecycle clients therefore do not explain
these counts. The metrics update every 100 ms: this narrows their scope but does
not locate individual dropped graph buffers. DualSense/DS4 measured windows passed;
Xbox received 143968 valid frames plus 32 silence frames and failed. Its 1280
startup silence frames plus 32 measured silence frames reconcile to the 1312
reported cumulative underruns. All three playback marker windows passed.
No third unchanged diagnostic repetition is justified by these results.

The maintained launcher also encountered a startup race: it inspected the owned
unit before systemd assigned an invocation identity and treated that pending
state as a changed identity. The runner correction waits within the existing
startup deadline for an initially empty/all-zero invocation only when the
executable is absent or matches the frozen image. It still refuses foreign
executables and refuses cleanup without an established matching identity.
Regression tests cover pending startup, foreign paths and expected-identity
mismatch. The original failed orchestration receipt retains its cleanup-observation
errors. Subsequent inspection confirmed the observer was collected (MainPID zero,
unit not found), and the separately retrieved root receipt records restoration
with no root cleanup errors. Do not relabel that failed coordinator as passed.

Next causal instrumentation must distinguish graph EMPTY/CORRUPTED-buffer
accounting from PCM queue overflow and locate the Xbox underrun in media time.
The cumulative graph-drop counter alone cannot identify the mechanism. All
existing assertions remain in force; Gates 1/4/5/6 remain open.


## Native graph rejection reasons at `82d1ca5`

At `82d1ca5ba5d4b39ec869e70b00386ac10f664281`, tree
`430bf27c0275a4d8bc99266cf223051ca70699b9`, frozen public test image
`8390946f2ce27ab8589db54e88046ff873fa8fad3aa17301170a39226ace707e`,
the first DualSense native-microphone cell reported 1024 EMPTY graph frames,
zero CORRUPTED frames and zero queue-overflow frames. The total graph-discard
counter was 1024 and the cached baseline zero. This identifies the rejection
mechanism for that receipt; it does not locate the EMPTY frames in media time.

Its measured capture contained 143968 valid frames and 32 silence frames.
Startup silence was 1344 frames; the total reconciles with 1376 USB microphone
underrun frames. Sample playback delivered all 144000 indexed measured frames
without missing, duplicate or corrupt markers. The mandatory fixture failed at
DualSense, so later families were not reached. This demonstrates that the
intermittent steady-state microphone underrun is not Xbox-specific. Earlier
DualSense/DS4 short passes remain narrow historical observations, not unconditional
continuity acceptance. The EMPTY count alone cannot explain the 32-frame underrun.

The enclosing phase and observer failed; owned restoration completed with empty
root and coordinator cleanup-error lists. The corrected observer-startup guard
recorded a matching invocation before waiting and cleanup. Approximately 17 GiB
remained available and measured memory pressure averages were zero.

Implementation adds preallocated atomic per-endpoint loss-reason counters to the
PipeWire callback and preserves existing PCM discard semantics. Combined EMPTY
and CORRUPTED flags count once as corruption. Accepted sample data and queue sizes
are unchanged. No callback allocation, blocking I/O or callback logging was added.
Supporting `gr_audio_linux::Session::input_loss_frames` and experimental
`AudioInstrumentation::native_microphone_graph_loss_frames` are additive; ordinary
root signatures remain frozen. Observations are independent atomic reads, cached
by the native bridge every 100 ms, not a transactional snapshot. Unavailable
backends return `None` through the existing instrumentation seam.

Regressions verify separate reasons, combined-flag precedence, invalid partial
frames, queue overflow, total-discard reconciliation and readability after repeated
bridge close. Mandatory fmt/check/Clippy/test/Gitleaks, forced Rust 1.85, strict
root rustdoc, API snapshot (110 items) and inventory checks passed. An initial
API-check command used the wrong rustdoc root and failed; the corrected command
uses `--doc-root target/doc/virtualgamepad` and passes. No dependency was added.

Next diagnostic evidence must locate EMPTY buffers and USB silence in media time
and reconcile actual graph receipt/submission with worker consumption. Do not
infer queue overflow, enlarge queues or weaken zero-loss acceptance from these
results. Gates 1/4/5/6 remain open; alpha is not ready and PR #133 stays draft.


### Native microphone pacing investigation after `82d1ca5`

`3b9442d` adds a bounded, preallocated PipeWire loss-event ledger. Each event
records the input media offset, rejected frame count and available raw graph
clock ticks; it does not claim producer marker identity or latency. The ledger
retains 64 events, counts omitted events and exports outside callbacks.
`02b4a1b` exposes experimental USB microphone progress: serviced host frames,
silence, source consumption, completed/abandoned capture, source drops and
maximum worker lateness. These are sequential observations, not an atomic
snapshot. Ordinary-root signatures remain unchanged; no dependency was added.

Live evidence at `02b4a1b` reconciles 288000 serviced/completed DualSense frames
with 286256 consumed source frames and 1744 silence frames. There were no
abandoned captures or source queue drops. The graph ledger recorded only a
startup EMPTY range at media offset zero (1024 frames), but 32 measured capture
frames were silent. Graph receipt includes padding; stdin byte counts are not
actual graph submission evidence. These results narrow the investigation without
qualifying continuity.

`538aaa567c1ddcc9be30f92c1f3eb4009ea827a3` (tree
`6b0c5d29309ead25adb6b97bed4103c7ebbc2cf0`) corrects the native bridge's one-packet
refill after delayed host progress. Catch-up is bounded to six packets within the
unchanged 672-frame operating fill. Regression tests cover delayed progress,
partial progress, backpressure, errors and impossible acceptance counts.
The exact-candidate short native-microphone trial passed DualSense and DS4, but
Xbox 360 failed: 143872 exact measured frames plus 128 silent frames out of
144000. Xbox completed 288000 total capture frames, consumed 286592 source
frames, inserted 1408 total silence frames (1280 warm-up), and reported zero
abandoned frames/source drops. Restoration completed with no cleanup errors.
This is a short functional result, not sustained acceptance; Gate 4 stays open.

The next client-pump correction drains at most eight packets per wake-up, bounded
by the existing 1024-frame local queue. Previously a coalesced wake-up sent only
one packet before sleeping despite socket capacity. Deterministic regressions
verify full queued delivery for all three profiles, exact frame ordering and
samples, bounded backpressure with retained pending data, and terminal errors.
A live rerun is required; this correction does not by itself close Gates 4 or 6.

The client-pump increment passed fmt, locked workspace all-target/all-feature
check and Clippy, locked all-feature workspace tests, Gitleaks and forced
Rust 1.85 all-target/all-feature check. Both privileged binaries were rebuilt.
Initial Clippy findings in new test code (checked numeric conversion and similar
local names) were corrected; assertions and existing regressions were retained.


The exact-image rerun at `8ca53edd7d245ca6d80c3896d594f1ef3df6c281`
(tree `b196b597352c825fd3bc743766d6c110052ee133`, root test image
`a5235cb5c6e7dfad374822226018954d0b2b06725b769badd81f926efbf99678`)
still failed the first DualSense native microphone cell: 143984 exact frames and
16 silent measured frames. Worker progress reconciled 288000 completed host
frames, 286400 consumed source frames and 1600 total silence frames; zero
abandoned captures and source queue drops. The graph recorded only startup
EMPTY (1024 frames at media offset zero). Later families were not reached.
Owned restoration reported no cleanup errors; memory pressure averages were
zero. Retain this failure: the bounded pump correction is not a continuity fix.

The next instrumentation records the maximum interval between successful host
clock replies (including poll sleep, lock contention and IPC duration) and the
largest host-frame advance between replies. Request duration alone omitted these
intervals. The new experimental accessor returns independent maxima; they need
not originate from the same poll and are not audio latency or physical-delivery
evidence. Defaults remain unavailable on unsupported backends. Deterministic
regressions verify delayed intervals, maximum retention, regressing frame-counter
arithmetic and readability after repeated bridge close. Ordinary-root signatures
and all queue/fill capacities remain unchanged.

The host-clock instrumentation passed mandatory fmt/check/Clippy/test/Gitleaks,
forced Rust 1.85 all-target/all-feature check, strict root rustdoc and the
110-item ordinary-root API snapshot check. No dependencies were added.


### Host-clock interval evidence and provider revalidation

All seven immutable provider phases passed again at `8ca53ed`, including the
rebuilt client-pump worker image
`44fd5d4f4997a96fa962a30a0ab197659474ef5dfdb8bdc75508da7cc3b4f0e0`.
Source receipts agree on the candidate and broker/worker identities. The following
host-clock-only change does not alter those privileged binaries.

At `53f25e61c7c51a250a90775d1febecc80cbde69c` (tree
`d0fa231f8aadc2c17730433e5a41275f409ad5b8`), the initial short trial passed
DualSense but failed DS4 with 48 measured silent frames. Largest host-clock reply
intervals were 12463 and 13028 microseconds; largest host-frame advances were
624. This run is not quiet-host evidence: requesting consumer-tool help
unexpectedly executed consumer builds while the coordinator was active. The
checker previously ignored command-line arguments; it now parses them before
building. Regressions require help and unknown arguments to perform no builds,
while the default invocation still runs the complete matrix.

The separate quiet repeat at the same source failed DualSense with 160 silent
measured frames (143840 exact out of 144000). Its maximum successful host-clock
reply interval was 15515 microseconds, exceeding the unchanged 14 ms operating
fill. Largest host-frame advance was 624, maximum bridge loop gap 7656
microseconds and maximum credit request 4336 microseconds. Maxima are not a
correlated event ledger. Worker progress reconciled 288000 completed frames,
286288 source frames and 1712 total silence frames; no abandoned captures or
source queue drops. The graph again recorded only startup EMPTY (1024 frames).
Later families were not reached. Both runs reported empty coordinator cleanup
errors. The evidence supports a scheduling/clock-delivery lead, not proof of the
cause of each silent range. Do not enlarge queues or close either audio gate.


### Bounded scheduling comparison preparation

The USB functional lab's authorized ordinary client now receives the same
reviewed temporary scheduling bounds used by the independent-control lab:
`LimitRTPRIO=88`, `LimitRTTIME=200ms`, and fixed round-robin priority 20. The grant
is limited to the predefined USB functional phase and authorized client identity;
other phases, unauthorized clients, broker and worker retain their prior policy.
Unit teardown removes the temporary grant. No persistent scheduling or service
configuration changes. A regression covers every phase and the unauthorized
identity. Factory receipts report actual ordinary-parent scheduling and limits;
graph data-loop thread scheduling remains explicitly unverified.

This is a diagnostic comparison against the recorded ordinary-scheduling
failures, not a product behavior correction or continuity qualification. Queue
sizes, operating fill, marker checks and loss/latency limits remain unchanged.
If it passes, require event correlation, actual data-loop scheduling evidence and
ordinary-user deployment implications before closing either audio gate.


The bounded scheduling comparison at `3b5548d215f87580524fddd88bbe0f0834a876d5`
(tree `6ff26106834ec813370d5b46f0e1bf6dbbc6a328`) failed the first DualSense
native microphone cell: 143984 exact measured frames and 16 silent frames.
The ordinary-parent receipt verified policy 2 (round-robin), priority 20,
RTPRIO limits (88,88) and RTTIME limits (200000,200000) microseconds. Largest
successful host-clock reply gap was 6483 microseconds; largest host-frame
advance was 192; bridge-loop gap 1004 microseconds and maximum credit request
4469 microseconds. Worker completed 288000 frames, consumed 286656 source
frames, inserted 1344 total silence frames, and reported zero abandoned
captures/source drops. Coordinator cleanup errors were empty. Later families
were not reached. This does not close Gate 4 or qualify sustained Gate 6.

The shorter reply interval with remaining silence weakens the hypothesis that
reply gaps alone explain all loss. Next evidence must correlate graph/local/
worker queue occupancy and silence in host media time; independent maxima are
insufficient. Actual graph data-loop policies remain unverified. The comparison
passed all mandatory local checks and 383 Python tests before deployment.


The isolated typed-output observer at `96e94f73295f67ad580539fb1e4f95f182175baf`
passed all 24 requests for DualSense (12), DS4 (6) and Xbox 360 (6), with exact
root callbacks. The encompassing USB coordinator failed its accompanying ALSA
microphone acceptance; it must not be reported as a complete phase pass. In the
Xbox receipt, 288048 completed host frames reconcile with 268304 consumed source
frames and 19744 silence frames; 384 submitted frames remained unconsumed, zero
captures were abandoned, and queue drops were zero. Both initiating errors and
empty cleanup errors were retained. Earlier non-audio controller evidence is
unchanged; Gate 4 remains open. No leak or memory-pressure failure was observed.

Production worker validators passed all three profiles at both ordinary and
high descriptor numbers, including enumeration, exact report replies, ordered
observations, bidirectional PCM, diagnostics and closure. Corpus validation
passed. Downloaded exact-`96e94f7` SBOM collection verified all 17 workspace
identities. At this recording, exact-head CI and CodeQL were still in progress;
SBOM, Gitleaks history and Provider Tier B workflow conclusions were successful.
A successful Provider workflow does not establish execution of a skipped
privileged job; preserve the separately recorded live provider receipts.


### Bounded native microphone silence correlation

The experimental instrumentation now exports the first 64 observed worker
silence increases. Each event retains the two host-clock reply counters, silence
counters, reply interval and optional latest bridge sample: sample age, credited
host/submitted positions, actual accepted source frames, optional graph queue,
local client queue and unsent bridge suffix. Credited positions can include
startup/underrun adjustments and must not be confused with accepted source frames.
Local client fill excludes its pending framed socket packet and the worker queue.
Host counters include virtual request media; they are not producer marker IDs.

The ledger is preallocated during bridge setup and records only on the ordinary
host-clock polling thread. Export allocation occurs outside callbacks. It counts
omitted events without growing its storage. A bridge sample may precede or follow
the underlying underrun; missing or future-dated samples remain unavailable.
Independent queue reads are not a transactional pipeline snapshot. The worker
protocol is unchanged; the supporting `SampleStreams::queued_microphone_frames`
accessor and experimental diagnostic types/method are additive. Ordinary-root
signatures remain frozen. No dependency was added.

Regressions cover no-increase/regressing counters, reply/media bounds, unavailable
samples, distinct credited/accepted positions, bounded overflow without capacity
growth, coherent concurrent publication and retained evidence after repeated
bridge close. Required fmt/check/Clippy/test/Gitleaks, forced Rust 1.85,
strict root rustdoc, API snapshot/inventory and all root feature consumers with
warnings denied plus offline rebuild passed. Both privileged binaries were rebuilt.
Live correlation remains required; this instrumentation does not close a gate.


The first ledger trial at `254e413c6b2ae13a29532b7fb40fce7d58468c05`
(tree `acfceb1b1a2d8c0538ab85a9f8c7c795d67df945`) failed DualSense with 256
measured silent frames (143744 exact of 144000). Worker progress reconciled
288000 completed host frames, 285984 consumed source frames and 2016 total
silence frames; no abandoned captures/source drops. Nineteen silence events
were retained, none omitted. For the reply window 179424..179520, silence rose
1744..1824 and the latest bridge sample was 2249 microseconds old, with 256
local client frames queued. Later snapshots also retained local queued or
pending source data. These are post-observation samples, not exact underrun
occupancy. Coordinator cleanup errors were empty; later families were not reached.

Client-pump diagnostics now record actual pump-thread startup policy and Linux
kernel priority from owned `/proc/thread-self/sched`, plus the largest iteration
interval including park. File reads occur once before the ordinary pump loop,
never in PipeWire callbacks. Unavailable scheduling stays `None`. The existing
worker Op 6 reports PCM pump lateness and is included in experimental progress;
there is no new worker wire operation. Owned private graph thread policies are
captured at setup and teardown with a 64-thread bound; these are snapshots, not
policies correlated to individual loss events. Exited children are not inspected
through remembered PIDs. Missing/malformed task metadata remains unavailable.

Regressions verify strict unique scheduling fields, unavailable metadata,
retained client interval maxima, owned-child exit, bounded task enumeration,
exact Op 6 replies and terminal rejection of truncated/oversized timing replies.
Mandatory workspace checks, Rust 1.85, strict rustdoc, API snapshot/inventory,
root feature matrix with warnings denied/offline rebuild and Python tests passed.
No dependency or ordinary-root signature changed. Live timing correlation remains
required; the audio gates stay open.


### Native microphone credit follows consumed source, not inserted silence

A deterministic regression demonstrated that an additional all-silence host
packet incorrectly granted 48 source frames of refill credit. Host time included
inserted silence even though no queued source was consumed. Native refill now
uses the existing worker operation 5 consumed-source counter; operation 7 remains
responsible for activation, host progress and silence diagnostics. Both required
replies complete before a coherent private progress sample is published. Invalid
credit replies preserve the terminal error without publishing partial progress.

The operating fill remains 672 frames, packet size 128 and per-cycle refill
budget six packets. Queue capacities and loss/latency acceptance limits are
unchanged. Only actual consumed source replenishes credit; regressing observations
cannot create credit. The experimental bridge sample adds an explicit consumed
source position. Ordinary-root signatures and worker wire operations are unchanged;
no dependency was added. Host and source positions are separate observations,
not an atomic worker snapshot or exact marker identity.

Regressions cover silence-only host advance, real source consumption, stalled
credit, regressing consumption, the exact operation 7/5 request sequence, malformed
operation 5 replies and terminal shutdown. Previous lifecycle and refill regressions
remain. Mandatory fmt/check/Clippy/test/Gitleaks, forced Rust 1.85, strict root
rustdoc, API snapshot/inventory, all root feature consumers with warnings denied
and offline rebuild, and Python tests passed. Candidate broker/worker images were
rebuilt. This corrects a demonstrated credit-bound defect; live audio behavior and
sustained acceptance remain unproven. Gates 1/4/5/6 remain open.


### Live source-credit correction and active graph scheduling evidence

The quiet trial at `c03e8d5aa15bb7e12ccf035d48bf974d06a36712`, tree
`5d489f2c23319a739b26551c7cada6e8ab2f0370`, failed the DualSense native
microphone cell: 143280 exact measured frames and 720 silent frames out of
144000. Worker progress reconciled 288096 completed host frames, 285264 consumed
source frames and 2832 total silence frames, with zero abandoned captures and
source drops. Maximum host reply interval was 14732 microseconds; USB and PCM
pump lateness were 8307 and 7685 microseconds. The source-based ledger correctly
keeps submitted minus consumed credit within the unchanged 672-frame fill. That
credit correction does not close remaining loss or prove its cause. Later families
were not reached. Coordinator cleanup errors were empty; the original broker
service returned inactive and socket active. No sustained memory pressure appeared.

The lab now samples owned PipeWire and policy threads once per second while the
factory executes, retaining at most 64 snapshots and counting omissions. This
ordinary diagnostic observer performs no audio-callback I/O, changes no scheduler
policy, and stops before graph children are reaped. It also stops on interrupted
factory execution; a failed observation is explicitly unavailable evidence. A
stuck observer prevents ambiguous PID cleanup and retains the initiating and
restoration errors. Setup/end snapshots alone cannot establish streaming policy,
and active snapshots still do not establish policy at an exact underrun instant.
No private shared-session configuration, buffer capacity, loss assertion, dependency
or ordinary-root API changes accompany this instrumentation.

Regressions verify bounded collection/omissions, unavailable observations, finite
join failures, interruption ordering, never-started and repeated close. Acceptance
requires a fresh active-stream receipt, then causal correction and independent
controls before sustained product qualification. Gates 1/4/5/6 remain open.


The active-scheduling trial at `7ef64a545ae5f101e908b8c7d156bb9006c3c56d`,
tree `87e4eca5d88dce4345e3cb7eb13ceafc7e5f78a5`, retained eight one-second
snapshots spanning the seven-second factory execution, with zero omitted samples
and no observer error. The owned PipeWire data-loop thread used policy 0/kernel
priority 120 in every observed sample, including measured streaming. The policy
daemon's data-loop also used policy 0 when present. These actual observations
cannot be replaced by the ordinary parent's realtime grant. They identify a
scheduling difference, not causation or policy at every underrun instant.

DualSense still failed with 143376 exact measured frames and 624 silent frames
out of 144000. Worker counters reconciled 288144 completed host frames, 285184
consumed source frames and 2960 total silence frames, zero abandoned capture
frames and zero dropped source frames. Maximum host reply interval was 18398
microseconds, USB lateness 7429 microseconds and PCM pump lateness 137329
microseconds. Later families were not reached. Coordinator cleanup errors were
empty. This evidence changes the next experiment: inspect owned daemon scheduling
diagnostics and compare a verified temporary streaming policy, rather than assume
the parent grant propagated to graph processing. Loss assertions remain unchanged.
All mandatory checks, forced Rust 1.85, strict root rustdoc, API checks,
root feature/offline consumers, 391 Python tests and production worker validation
passed before the trial. Audio gates remain open; PR stays draft.


Private graph diagnostics now retain only the final 8192 bytes from the owned
PipeWire/policy log, with original size, truncation and unavailable-state metadata.
Export occurs after factory execution, outside callbacks; absence of xrun messages
is not a zero-xrun measurement. Sanitized regressions cover bounded truncation,
invalid UTF-8 and unavailable descriptors. Mandatory checks, Rust 1.85,
strict rustdoc, API checks, downstream feature/offline consumers and all 392
Python tests passed. The next isolated trial must inspect these messages before
assigning a cause to the observed ordinary graph-loop scheduling.


The diagnostic-log trial at `da26f9b265761704c0fdf4e7910e044dc1e418d8`,
tree `88c5da5c11a696edf390ec82d57a833e90edbec6`, failed DualSense with
143568 exact measured frames, 432 silent frames and three pattern gaps out of
144000. The native graph diagnostic recorded 2304 dropped frames; do not infer
zero graph loss from earlier trials. Owned PipeWire/policy logs were retained
without truncation (2026 bytes) and report failed session-bus connections in the
realtime fallback and optional portal components. The measured thread snapshots
remain the authority for effective policy; bus failures alone do not prove the
cause of loss or explain direct realtime setup failure. The next discriminating
experiment must inspect direct realtime setup and any verified isolated override,
without using the shared user bus or persistent host configuration. Coordinator
cleanup errors were empty, and VM memory-pressure averages remained zero. No
acceptance assertion was relaxed; Gates 4/6 remain open.


### Direct realtime capability discrimination

The private graph preflight now runs one fixed ordinary child requesting FIFO
priority 88, bounded to two seconds and the existing lab RTTIME/priority limits.
It rejects root invocation, malformed or wrong-policy receipts and timeouts. It
never changes the parent policy or persistent host settings. This probe separates
permission to acquire the requested policy from whether the graph acquired it.
Private daemon startup logs are now retained independently of the final log tail;
only the daemon receives diagnostic verbosity, not product clients or callbacks.
These diagnostic trials are not sustained qualification and logging effects must
not be treated as production latency evidence.

Regressions cover fixed child arguments/deadline, actual policy verification,
nonzero exit, malformed/oversized output, unavailable execution, root rejection
and interrupted graph startup. Mandatory workspace checks, Rust 1.85, strict
root rustdoc, API snapshot/inventory, root feature/offline consumers and all
394 Python tests passed before deployment. No dependency, public-root signature,
worker operation, queue size or acceptance limit changed. Live discrimination
remains required; all prior audio failures remain preserved.


The direct capability trial at `7e5c607a47d92c93e0e51fff3799382344ba42c2`,
tree `c6ae2c731efbfb79397dcad0edf330e3511d64f1`, proved that the fixed
ordinary child could acquire FIFO priority 88: policy 1, priority 88, RTPRIO
(88,88), RTTIME (200000,200000), exit zero. This rejects a blanket direct
scheduling-permission denial; it does not prove graph scheduling. DualSense
still failed: 143408 exact measured frames, 592 silent frames and eight pattern
gaps out of 144000. Coordinator cleanup errors were empty. The diagnostic
startup log exceeded its 8192-byte tail bound, so the graph now retains both
bounded prefix and tail with explicit truncation metadata. The truncation
regression verifies that early initialization evidence survives alongside final
messages. No graph policy or production audio behavior changed in this step.
The daemon's initialization decision remains to be inspected before choosing
any temporary configuration correction.


### Private direct-realtime configuration experiment

Installed Debian PipeWire source `1.4.2-1` confirms that module-rt can select
RTKit after a failed negative nice adjustment even when its direct realtime
privilege check succeeds (`src/modules/module-rt.c`, initialization near lines
1128..1140). The b2db1b9 startup log reports fallback clamping to priority 20;
the separate ordinary child proved direct FIFO priority 88 was permitted. This
is evidence for a specific private lab setup correction, not proof of the cause
of marker loss. That trial still failed with 143952 exact measured frames and
48 silence frames; cleanup errors were empty.

Only the temporary private XDG daemon/client configuration now sets nice level
zero, realtime priority 88, finite soft/hard RTTIME 200000 microseconds, direct
rlimits enabled and portal/RTKit fallbacks disabled. Existing scoped grants are
unchanged. These files are created within the owned graph directory and removed
with its verified teardown; no shared graph, user configuration or scheduling
policy is changed. Effective active graph thread policy must still be recorded,
and this diagnostic configuration does not qualify the host or product alone.

Regression checks cover all three daemon/client configuration files, finite
limits, disabled fallbacks, private path scope and refusal to overwrite existing
configuration. Interrupted/partial startup and cleanup regressions remain.
Mandatory workspace validation and all 395 Python tests passed before deployment.
No dependency, public API, buffer capacity or acceptance limit changed.


### Complete-family functional failure collection

The db68c69 private graph experiment verified FIFO policy 1/kernel priority 11
(priority 88) on the actual daemon data loop in all eight streaming snapshots.
Startup messages confirm the private override loaded and direct priority 88 was
acquired. The policy daemon's unused data loop remained ordinary scheduling;
it is not substituted for the measured driver. This demonstrates the lab
configuration correction, but DualSense still failed with 143856 exact measured
frames and 144 silence frames. Worker counters reconciled 288048 completed host
frames, 285744 consumed source frames and 2304 total silence frames; abandoned
captures/source drops were zero. Cleanup errors were empty. Sustained continuity
is still unqualified and requires independent controls.

The root USB functional fixture now collects marker verdicts for all three
families before rejecting the aggregate result. It retains every former host
success, loss/duplication/corruption, sample production and backend-health
requirement. Each controller must close twice with healthy terminal state before
the next cell starts; construction, protocol, deadline and cleanup failures still
abort immediately. Only a measured marker verdict may be collected after verified
cleanup. No failed cell can generate a passed=true receipt.

The ordinary validator requires exactly one successful receipt for each of the
12 ownership cells and rejects duplicates, failed cells and malformed suffixes.
Its bounded factory diagnostic output cap is 256 KiB to accommodate three
families' existing bounded ledgers; this is an output-log quota, not an audio queue
or a loss tolerance. Root supervision retains its existing output/memory limits.
Regressions preserve each acceptance predicate and verify incomplete, duplicate,
failed and malformed inventories. Mandatory workspace checks, Rust 1.85, strict
rustdoc, API inventory/snapshot, root feature/offline consumers and 396 Python
tests passed. No dependency or production API/behavior changed. Fresh complete
functional receipts and exact-current-worker provider revalidation remain due.

### Exact control-runner CI and failed-phase diagnostics

At `7d12e2846b112e22b16b9feb3293c066ce4d0127`, CI run `38120783718`
passed platform builds/tests, lint, forced MSRV, policy, audit and corpus checks.
The sealed independent-image sender regression passed on Linux; all 409 Python
tooling tests passed. Full-history Gitleaks, SBOM and Provider Tier B passed;
all 17 downloaded SBOM identities were verified, including the root package.
CodeQL was still running at this checkpoint. Skipped privileged/dependency-review
jobs remain distinct from positive acceptance evidence.

Failed USB phase commands can retain client stdout while the root `clients`
inventory is empty. The coordinator now writes independent-control diagnostics
from that command output before checking phase acceptance. It preserves the
revision and failed root status and makes no qualification claim. Successful
client records take precedence over their command copies; genuine duplicate
control records remain visible. The regression covers this failed-phase shape,
retained failures and duplicates, and continued refusal of whole-phase acceptance.
All ten coordinator regressions pass. Fresh live controls remain outstanding;
no loss, rate, latency, cleanup or acceptance assertion is weakened. The current
GUI soak remains separate and requires its terminal workload/cleanup receipt.

### Completed current-image soak and independent Rust qualification

The neutral GUI soak at source `cf710a47a5ca15aa60d5d718037781d6c69cee47`
completed 7201.5358 seconds, exit zero and no cleanup errors. The current workload
verifier confirms 119 increasing removal/recreation cycles (60 through 7140
seconds) and completion at 7200 seconds. Both owned GUI/display processes exited;
owned HID inventory is empty. There were 240 process samples: steady descriptors
16, threads 23, no children, and zero observed full memory pressure. RSS warmed
from approximately 122 MiB to 141 MiB; the final half-hour ranged from 144624
to 144724 KiB, without growing descriptor/thread/node inventories. This is a
bounded neutral-resource receipt, not proof of audio-enabled or active-touch use.

Rebuilding both all-feature GUI images at `46be2bf945b67fc664e03e4d03f7e72e09d1b531`
produced the same bytes: soak SHA-256
`211c492325de801cb9d3a95492db727d9671d93165eecf6d4059b517bab8f7f5`,
production GUI SHA-256
`167d8fd238f194aa91c87337ce6a4a332f2f102186dd42cc0a55e2da2469abf5`.
The endurance cell passes for these images. The interactive inventory and
isolated active-touch display evidence remain required for whole Gate 1.

At that same candidate (tree `493428d0c5fed00531667ce9ab9093088388452d`),
three independently constructed quiet Rust graphs passed 60 measured seconds
each. Every trial generated/received all 2976000 planned frames, including
96000 separately excluded warm-up frames. Each observed 2880000 measured frames
with zero missing, duplicated or corrupt markers. Measured production spans were
60.002594, 60.003067 and 60.002782 seconds. Application-to-application p99 values
were 229, 238 and 242 microseconds, below the unchanged 20 ms requirement.
Control image SHA-256:
`cb47f76b0c30cc11365b69814e39bc556391d83f435a1d93e6a4f36ad955bc4b`.
Ordinary client data-loop snapshots show FIFO policy 1/kernel priority 11
(requested priority 88); reporting threads remain ordinary. Rust RAII queue-return
status is unavailable and explicitly reported, not fabricated as success.

The scoped root phase and exact-image observer exited zero, all 24 typed USB
outputs passed, cleanup errors were empty, and original broker service inactive/
socket active state was restored. This closes the independent Rust control cell,
not independent C qualification, the 72 product trials, supported USB sustained
cells or Gate 6. Earlier microphone and independent-control failures remain
historical evidence. No loss/rate/latency limit or audio queue changed.

All five exact-candidate workflows passed; all 17 downloaded SBOM identities
were verified. The local Python suite passed 410 tests. Privileged remote and
dependency-review skips do not count as additional acceptance.

### C control neutral-buffer correction

A maintained direct C diagnostic at `ad419cb96ec7c1157612c4fca6865d936f36264f`
(three measured seconds, quantum 512, ordinary ungranted scheduling) failed:
144000 generated/submitted frames, 122624 received, 21376 missing, 1536 invalid,
3.609881669 seconds of production. The full callback ledger reconciles; it cannot
separate graph from capture loss after successful submission. This is diagnostic
evidence, not sustained qualification or a comparison under the Rust grant.

Inspection of the installed PipeWire 1.4.2 SPA header establishes that EMPTY
chunks represent neutral audio regardless of mapped bytes. The C harness was
decoding those bytes, allowing stale memory to appear as markers or corruption.
Its capture decoder now records EMPTY buffer metadata without decoding stale
payload. CORRUPTED chunks and malformed/partial framing still fail; absence of
measured markers still fails the unchanged exact marker requirements. Refilled
producer chunks explicitly clear prior flags. No product library changes.

Deterministic C self-tests cover stale nonzero and null EMPTY data, normal wrapped
frames, reused producer flags, corrupted EMPTY chunks, partial frames and malformed
chunk sizes. The previous marker/ledger regressions remain. All 17 control-tooling
tests passed. A fresh diagnostic is required; this correction alone does not
resolve lost frames, production pacing, C qualification or Gate 6.

### Bounded C-control grant and evidence transport

The C control now has a separate sealed-image request (`C`), distinct from the
Rust (`D`) and public SDK (`R`) requests. It runs only in the existing ordinary
client of the fixed USB functional phase. Root actions, worker policy, ownership
lock, maintenance/restoration boundaries and the three allowed action categories
are unchanged. The C callback main loop requests the existing RR priority 20;
the private daemon retains its finite direct-realtime configuration. Actual owned
thread scheduling is recorded. Each of three fresh direct graphs uses quantum
512 and 60 measured seconds, with a 90-second per-child deadline.

The preallocated C ledger exports after stream teardown through one inherited
pipe descriptor, then normal scheduling is restored for serialization. The
ordinary collector bounds raw evidence at 64 MiB and compressed evidence at
1 MiB per trial. A cancelled/stalled reader terminates and closes its descriptor.
Three length/digest-framed compressed artifacts go to the ordinary observer;
root's existing 1 MiB file/output quota is unchanged. No audio queue or acceptance
limit changes. The observer refuses truncation, oversized data, digest mismatch
and existing artifact replacement, and restores the peer's blocking mode.

The coordinator verifies the exact retained compressed bytes, bounds expansion,
reconciles every producer/submission/capture/marker counter, and independently
requires all three C trials to meet the existing zero-loss, format and 1% rate
requirements. Rust latency receipts cannot substitute for C evidence. A failed
root phase cannot become accepted through a passing control. C direct qualification
does not replace loopback qualification or the product matrices.

Regressions cover exact F/E replies and sealed FD ownership; bounded pipe export
and cancellation; graph/node/channel isolation; failed middle-trial retention;
timeout cleanup; positive sealed-image transport and damaged ledger rejection;
exact artifact/counter reconciliation and evidence preservation. C self-tests also
exercise inherited-descriptor export and malformed descriptors. Full validation
and live installed-candidate trials remain required before claiming qualification.
This changes validation tooling and test-only Rust support, not ordinary-root APIs,
production provider/audio behavior, GUI controls or dependencies.

### C main-loop priority discrimination

The three C direct trials at `6f36523858e064ef69c5ce99c4fbc7d246963a42`
(tree `cad8da6d2794b23ae76e0c51c229331c8bb4f5b8`) all failed. Control image
SHA-256: `94a61f5b40a2f4bb39280b638e09fcff942fcf74a9871b2d06c87b5dae442831`.
All produced/submitted 2880000 measured frames; received counts were 2478080,
2469888 and 2440704. Missing counts were 401920, 410112 and 439296, invalid
counts 11264, 10752 and 14848, and production spans 71.114245450, 71.390034159
and 72.189030534 seconds. All three complete ledgers reconciled and classify
missing ranges as unexplained after submission, not proof of graph-versus-capture
loss. No product qualification follows. Observer/root failure remains recorded;
owned cleanup was empty and original broker service/socket state restored.

Actual C snapshots contain one callback/main-loop thread at RR priority 20
(policy 2/kernel priority 79); the Rust control uses priority-88 data loops.
The next bounded comparison changes only the C callback priority from 20 to 88,
retaining RR policy, graph configuration, quantum 512, frame/rate/loss limits,
ledger transport and finite 200 ms realtime allowance. This uses the already
installed RTPRIO ceiling 88 and makes no persistent host scheduling change.
Diagnostic export still switches to ordinary scheduling after streaming.
Priority mismatch is a causal hypothesis, not an established explanation.
The fixed-argument regression now verifies priority 88.

### C realtime callback dispatch correction

The priority-only comparison at `3dea9f8f749e095501d0a56eae5ccc333828017f`
failed all three direct C trials. Missing measured frames were 414720, 312320
and 412928; invalid frames were 12800, 4608 and 7168. Production spans were
71.775261326, 68.328131783 and 71.306930325 seconds. The same control image
used RR priority 88, with unchanged graph, format, quantum and limits. Cleanup
errors were empty. Raising main-loop priority alone did not resolve the failure.

The C harness omitted `PW_STREAM_FLAG_RT_PROCESS`, causing PipeWire to dispatch
process callbacks asynchronously on its main loop. The maintained PipeWire
source and audio examples establish the realtime callback model. The control
now uses that model with two streams sharing one context/core and one requested
data loop. A lock-free callback-owner guard rejects dispatch from another thread
before touching shared marker/ledger storage. Main-loop readiness, progress and
error observations use lock-free atomics; unsupported atomic implementations
reject startup. Both streams are destroyed, synchronously quiescing their
callbacks, before counters are read or storage is exported/freed. Partial startup
uses the same idempotent cleanup path, including core/context destruction.

C self-tests cover same-owner reuse, different-owner rejection without ledger
mutation and repeated empty teardown, preserving previous marker/framing tests.
No product audio queues, loss/rate/latency assertions, public API or dependencies
change. Live qualification remains outstanding; this corrects the harness's
callback model but does not yet demonstrate that it explains all recorded loss.

### Realtime C trials: partial correction, qualification still failed

At `39257d38cf95a1d9aa9edfd3426c7587ac221d0e`, tree
`29f556ad0d5640c892ad1bceb9432f40b95dce73`, all three direct C trials
completed production/submission of 2880000 measured frames. Missing counts were
71680, 74240 and 63488; duplicated, invalid, partial, processing errors and
ledger overflow were zero. Production spans were 70.150250742, 69.870111116
and 69.855782450 seconds, outside the unchanged 1% envelope. The C image was
`deb79aeee238f47744e1c1ad5956a13e28f3c1f943a4e4fb8903a8747ce88f0d`.
Actual client snapshots show the shared data loop at FIFO priority 88 and main
loop at RR priority 88. All ledgers reconciled. Root/observer correctly failed;
cleanup errors were empty, original broker service/socket state restored and
memory pressure remained zero. All 24 typed controller outputs passed separately.

This corrected corruption and reduced measured loss but did not qualify C or
Gate 6. In trial 1, source callbacks submitted 512 frames each, but graph-clock
steps frequently exceeded 512 ticks (812 steps of 1024 and 88 of 1536 among
measured callbacks). Same-tick source-to-capture dispatch had median 0.034125 ms
and maximum 0.38025 ms. These observations direct investigation toward skipped
process cycles and graph topology/activation, without proving graph-versus-capture
loss or justifying larger queues. No further repeated qualification is warranted
until a new discriminating observation or correction changes that hypothesis.

All mandatory local fmt/check/Clippy/workspace tests/Gitleaks checks and the
Python tooling suite passed before the live run. Independent Rust qualification
and the neutral GUI soak remain valid for their byte-identical images. The four
remaining gates remain open; no release readiness or PR readiness is claimed.

### Bounded topology discrimination after realtime correction

A three-second ordinary-scheduler diagnostic of the committed C control retained
144000 generated/submitted frames, 1536 missing, zero corrupt/duplicate frames
and 3.644887877 seconds of measured production. `pw-top` recorded the owned
Dummy-Driver at 512 frames/48000 Hz, with zero reported errors in its sampled
instant; this sampled zero is not a complete xrun history. Node inventory confirms
the two streams share that driver. The same source/sink callback graph-clock
steps can exceed one quantum while source buffers remain exactly 512 frames.

Two uncommitted, bounded diagnostics tested separate source/capture contexts and
a virtual `Audio/Sink` capture node independently. Separate contexts retained
5632 missing frames and 3.718003544 seconds production; the virtual sink retained
1536 missing frames and 3.735354419 seconds. Both had complete 144000-frame
production, zero duplicates/corruption and verified owned cleanup. These tests
used ordinary scheduling, so they do not establish behavior under the scoped
realtime grant or replace qualification. Experimental sources, hashes and raw
receipts remain outside Git; neither speculative topology change was adopted.

The qualified Rust control should now be rechecked in the current environment
before treating all skipped C cycles as a harness-specific defect. Original Rust
qualification remains historical evidence; a current failure would require
investigating the prepared graph/VM before product qualification. No failed audio
trial is converted to acceptance and no queues or assertions change.

### Current Rust control contradicts unconditional graph qualification

At `e6ac82c78710fc54d071df8d2c229d49fe2529d0`, tree
`2ede501a2c9ce4da0c34d686a2e464a2b04e8861`, all three independently prepared
Rust direct controls failed the unchanged production-rate requirement. The image
is byte-identical to the earlier passing control:
`cb47f76b0c30cc11365b69814e39bc556391d83f435a1d93e6a4f36ad955bc4b`.
Each generated all 2976000 planned frames (including warm-up); total observed
frames were 2914560, 2923520 and 2910464. These totals include warm-up and must
not be relabeled exact measured-window loss. Source process maximum gaps were
45.926042, 43.149167 and 59.401459 ms; capture gaps were 46.296333,
43.329375 and 60.405541 ms. The test rejected the measured production span
before latency qualification. Both client and private daemon data loops retained
FIFO priority 88. All 24 typed outputs passed independently. Cleanup succeeded
and the original broker service inactive/socket active state was restored.

The previously passing Rust trials remain historical evidence, not a current
unconditional host qualification. Both independent implementations now fail in
this prepared VM. Product qualification remains stopped; investigate current
clock/scheduling/environment behavior before additional harness-only corrections.
The guest exposes eight CPUs, approximately 23.4 GiB usable memory, no observed
memory pressure, and the arch_sys_counter clocksource. Guest CPU steal counters
are zero; that does not prove the hypervisor never pauses the guest. No physical
or native-host timing claim is made. No loss or rate assertion is relaxed.

### PipeWire-independent ordinary timer diagnostic

A native C timer diagnostic at `c56cce5` ran ten seconds without PipeWire,
compilation, GUI or competing audio. Eight owned pthreads were pinned one per
available guest CPU. Each performed 2000 absolute CLOCK_MONOTONIC sleeps at
5 ms intervals, retaining target/wake, CLOCK_MONOTONIC_RAW and thread CPU time
in preallocated storage; export occurred after joining. All affinity/sleep
operations succeeded. Per-CPU maximum lateness in ms was 26.683962, 27.096462,
27.490630, 30.312084, 27.565084, 27.371794, 26.214376 and 28.843336.
There were 142 wakes over 20 ms across the eight threads. Each thread consumed
approximately 18–21 ms CPU time over the ten-second run. Source/binary hashes,
compiler flags and every raw sample remain outside tracked source.

This ordinary-scheduler result demonstrates wake-up delays outside the product
and PipeWire. It neither proves a hypervisor pause nor characterizes bounded
realtime scheduling. It cannot close native-host timing, qualify the VM, or
explain every audio loss. Next compare the same native timer under the existing
finite realtime grant, recording actual policy and preserving owned cleanup;
do not replace audio controls with timer success. Product qualification remains
stopped until the required independent audio controls pass in the current lab.

### Bounded realtime native-timer discriminator

The fixed USB functional lab now supports an explicit timer-only request (`T`)
after the ordinary typed-output checks. Unlike image requests R/D/C, T supplies
no executable or descriptor: immutable reviewed C source is compiled and executed
only after privileges are dropped. It samples at most eight CPUs from actual
allowed affinity, with 2000 absolute 5 ms sleeps each (ten seconds), preallocated
storage, FIFO priority 88, and inherited real-time CPU allowance at most 200 ms.
Missing/unbounded grants reject before compilation. Compiler and child deadlines
are each 15 seconds; subprocess timeouts terminate/reap the owned child.

Completion records source/binary/compiler hashes and flags, actual per-thread
policy/priority, wake-delay counters, resource limits and ordinary UID. The result
is explicitly aggregate-only; individual raw samples are unavailable for this
scoped measurement. A completed timer run can contain substantial lateness and
must never be interpreted as audio/native-host qualification. Root action names,
maintenance ownership/restoration and the existing three sudo categories remain
unchanged. No audio queue, public API, provider behavior or dependency changes.

Regressions reject root execution, missing/unbounded grants, wrong scheduling,
malformed counters, duplicate CPU identities and noninteger samples. They verify
exact F/E terminal replies and that a timer receipt cannot satisfy C audio
acceptance. A fresh installed-candidate timer diagnostic remains required.

### Installed realtime timer result: wake delays persist outside PipeWire

At `c1072847219b0efc51ee250b52d0e272ee41fa83`, tree
`2b3d3735cd5d1ecac38616ae18e17ba16042e168`, the timer completed all 2000
absolute sleeps on each of eight allowed CPUs, with zero affinity/sleep errors.
Actual per-thread policy was FIFO (1), priority 88. Priority limits were 88/88,
real-time runtime limits 200000/200000 microseconds. Maximum lateness per CPU in
ms: 24.787127, 26.525500, 27.288629, 26.931337, 28.419585, 29.761963,
25.799378 and 28.521628. There were 123 wakes over 20 ms. This isolated timer
used no PipeWire or product PCM queue; substantial wake delays persist under
the finite realtime grant. No raw scoped samples were retained, as recorded.

Source hash `7446b514a3805ff17e234bc84708b9f9c535b11e52f5e7fdaf6c8e2962914809`,
image hash `50d4f25c63d47e5aef2217b753c2b5706b6bd75327bf439b63cf2d3bb21dc98f`,
compiler cc Debian 14.2.0-19. The ordinary observer completed all 24 typed
outputs and the timer request successfully. The overall root USB phase still
failed its functional audio checks; the coordinator correctly refused phase
acceptance. Cleanup errors were empty, original service inactive/socket active
state restored, and memory pressure remained zero. These are diagnostics, not
native-host timing acceptance or audio qualification.

Both independent audio controls currently fail with slow production and missing
markers. The native timer strengthens the need to investigate guest/platform
scheduling and clock behavior before more harness-only changes. It does not prove
which kernel/hypervisor mechanism caused each loss. Host-side environment and
Parallels configuration evidence requires operator access outside the guest.
Keep the historical passing Rust trials, but do not claim current qualification.

The next bounded discriminator restricts only the owned timer child to the first
CPU in its allowed affinity. It retains the same source, 5 ms absolute period,
ten-second duration, FIFO priority 88 and finite runtime limit. Exact one-thread
and CPU identity are checked. This separates eight-thread wake traffic from a
single-thread timer baseline; it does not force guest-wide affinity, disable
services, burn CPU, or change persistent scheduling. Top-level boolean scheduler
values are also rejected by the receipt validator regression.

### Single-CPU realtime timer comparison

At `1e9478b21a9289fec22d15ddb716fc7a64313eca`, tree
`7f002e6ae8d2984fc471993cd7cee717b1ba30ce`, the same native timer image/hash
ran on the first allowed CPU (0), with one FIFO-priority-88 thread and unchanged
finite runtime limits. All 2000 sleeps completed without errors. Maximum wake
lateness was 29.899962 ms; 317 wakes exceeded 10 ms and 38 exceeded 20 ms.
Restricting the owned child to one CPU did not eliminate the delays, so the
previous eight-thread timer workload is not a sufficient explanation. This is
aggregate diagnostic evidence, not identification of a specific hypervisor or
kernel mechanism and not native-host timing acceptance.

The typed observer and timer request completed successfully; the overall USB
phase still failed its functional audio checks. Cleanup errors were empty and
broker service inactive/socket active state restored. Memory pressure remained
zero. Both current independent audio controls remain failed. Host-side Mac,
Parallels, resource/power and memory-pressure evidence has been requested because
those settings and observations are inaccessible from this guest. Do not infer
host memory health from the guest's free-memory/pressure counters. More audio
qualification is inappropriate before a new environment or causal correction.

### Owned-display creation keyboard checkpoint

At source `6ab55e879e17ee508ba77b6c010cb457ccaff238`, the production GUI
image SHA-256 was
`167d8fd238f194aa91c87337ce6a4a332f2f102186dd42cc0a55e2da2469abf5`,
matching the accepted neutral soak image. Private displays and temporary homes
were used; actual XTest key events targeted a window whose process identity was
checked before each action. No controllers, active touch, audio streams or Steam
sessions were created.

Reviewed screenshots verify family selection to DualSense, optional-name edit
and clear, count increment/decrement (1 -> 2 -> 1), visible focus, unavailable
`dummy_hcd` help, keyboard activation of Create with explicit rejection and an
empty controller list, and expansion of advanced options. The advanced follow-up
also verifies keyboard-visible audio information and backend selection from
PipeWire to ALSA. Ownership popups opened, but the intended native-client
selection was not established by the captured state; retain that cell as open.
The long creation error is clipped in the fixed log viewport; complete diagnostic
access still requires verification rather than a claim that the full text was
visible. Backend health remained a separate service-health indication.

The two completed batches contained 27 and 19 recorded actions, respectively.
Both ended with null runner errors and empty cleanup lists. Their GUI/display
processes were absent and their display sockets removed on independent recheck.
Teardown used signals to owned processes; it does not prove normal window-close
behavior. Raw actions, images and receipts remain outside tracked source.
Earlier harness failures (focus before mapping, an overlong Unix socket path,
and an interaction deadline) remain preserved and are not accepted runs.

This closes only the listed creation/display cells. Full controller controls,
active-touch isolation, ownership selection, diagnostic access, routing and
assistive-technology acceptance remain required. Gate 1 stays open; no GUI code
or executable changed, so this checkpoint does not invalidate the neutral soak.

### Audio selector focus names

The shared audio selector now reports its purpose and value on keyboard focus,
for example `Playback: Samples` versus `Microphone: Samples`. Before correction,
a real Tab-event regression observed an empty widget label for both selectors;
only the adjacent button had a name. The regression now verifies both names,
unchanged ownership values and traversal to the adjacent control. This is widget
metadata, not native screen-reader acceptance or proof of ownership selection.
No root API, transport, PCM queue or dependency changes are involved.

This changes GUI implementation; rebuild and compare the production/soak images
before reusing any final-image endurance claim. If their bytes differ, repeat
the two-hour soak separately. The earlier accepted soak remains historical
image-specific evidence, and Gate 1 remains open.

### Rebuilt GUI endurance checkpoint

At `df4c5e3bb440dc13ea1dbe688699accff6f8c486`, the rebuilt production GUI
SHA-256 is `7408a5b575319426c8cad86fd178d3a79673fce34e4cd706fda8753510ff3ef8`;
the neutral-soak image is
`5e65c276674485dd2c8ea3dab2780783a5a9a1831d90481fbdc5456fc8e7a548`.
Both differ from the earlier accepted images, so a new 7200-second neutral
creation/removal soak was started alone with the maintained runner. Its owned
unit was confirmed active/running with a 1 GiB memory cap; initial guest
available memory was approximately 17 GiB with zero full memory pressure.
This is a running trial, not an accepted result; require terminal receipt,
resource accounting and owned cleanup before crediting it. No audio/Steam trial
or compilation overlapped startup. The five mandatory local checks passed for
the selector correction, and its focused regression failed before and passed
after correction. No dependency changes. Gates 1/4/5/6 remain open.

### Creation selector names and interrupted endurance run

The family and target selectors had the same missing focus metadata as the audio
selectors. A real Tab-event regression reproduced empty names before correction.
The production family selector is extracted as a small render helper; both
selectors now report their purpose and value. The regression verifies family
and target names across all four targets, traversal to an adjacent button, and
unchanged selections. This does not introduce screen-reader support or alter
controller creation semantics.

The `df4c5e3` replacement soak was deliberately interrupted after 596.577 seconds
to finish the remaining GUI audit before committing to another final-image run.
Its receipt has `completed=false`, signal-15 interruption and empty cleanup.
The owned GUI/display processes and display socket were independently absent.
It is not two-hour acceptance; the previously accepted image-specific soak stays
historical. Do not restart final endurance until the remaining control work is
settled and the final executables are pinned.

Remote infrastructure evidence at `20c4e39ab5e6606458a37d92b9810f0115c18d3f`:
CI, Rust CodeQL, full-history Gitleaks, Provider Tier B and SBOM all passed.
Linux/macOS/Windows build/test, MSRV, policy, corpus, lint and supply-chain jobs
passed. Dependency review and privileged provider jobs were skipped and remain
missing evidence in their respective scopes. All 17 downloaded SBOM component
identities, including the root package, validated. This evidence applies to that
revision; the subsequent GUI correction requires affected revalidation.

The installed autonomous updater enforces exactly eight provider phase names,
fixed identities/ports, payload names and an exact helper template. It cannot
install the separate missing Steam input helper; do not circumvent these guards
by repurposing an unrelated provider phase. Input capability expansion still
requires administrator provisioning. Gate 1 remains in implementation/interactive
verification; Gates 4/5/6 remain open.

### Batched GUI field and read-only detail corrections

The remaining text/numeric field audit reproduced missing keyboard-focus names
for optional creation name, controller rename, creation count and gap measurement
period. Small production render helpers now attach purpose labels while preserving
current/previous edit text, hint metadata, numeric values and existing layouts.
The period controls the displayed service-gap measurement window; it is not a
service scheduling interval. Count bounds, arrow focus and clear-name behavior
retain their previous regressions.

Long diagnostic entries now accept read-only keyboard focus and reveal their
complete message outside the fixed log viewport. The regression checks the
whole rendered text rectangle fits its clipping bounds, named focus, traversal
and dismissal after focus loss. Output status indicators likewise expose their
existing status text on focus as well as hover. A motor-status regression
reproduced the pointer-only gap and verifies named focus, status text and adjacent
Tab traversal after correction. Neither operation activates controller inputs
or outputs, modifies the retained log, or enables native screen-reader support.

Four new regressions cover field names/traversal/value retention, edited name
metadata, clipped diagnostic readability/dismissal and read-only output status.
They accompany the existing selector and input lifecycle regressions. These are
deterministic UI receipts, not complete owned-display or controller acceptance.
The four existing host-dependent demo tests remain individually gated; no ignored
suite was blanket-enabled. Gate 1 stays open pending complete display/isolation
checks and final-image endurance. Finish the GUI batch before restarting the soak;
no earlier image's endurance receipt certifies this implementation.

### Owned-display diagnostic receipt for the GUI batch

At `89195f47dd92e722a0c27780008e7b3e1257fd22`, the rebuilt production GUI
SHA-256 is `119fb92361871fcadc2c0624b7f8b42a0a9e0cf05d9e378b088ea8577bd1f014`.
Two private-display runs recorded 20 and 21 actual key actions. They repeat
name edit/clear, count 1 -> 2 -> 1 and unavailable dummy_hcd rejection to reach
the error-log scenario. Reviewed captures show the complete technical rejection
message in the focused read-only popup and its disappearance after the next Tab.
Capture filenames are not state evidence: the first run's nominal dismissal
capture actually showed focused details; the second run explicitly captured the
subsequent dismissal. Keep both raw receipts rather than relabel the first.

Both runners report null errors and empty cleanup lists. Their GUI/display
processes and display sockets were independently absent. No controllers, audio
streams or touch input were created, and teardown was signal-driven. These
receipts close the long-error display cell only; they do not establish normal
window-close, active controller operation or full Gate 1 acceptance. Raw logs,
actions and captures remain outside tracked source. The full workspace tests and
mandatory checks passed after correcting an initial Clippy needless-borrow
failure; no warning suppression or dependency changes were introduced.

### Portable isolated input fixture packaging

The maintained `scripts/package-alpha-input-lab.py` defaults to a non-mutating
preview. Applying it requires a clean committed candidate, builds the producer
with the locked dependency set and two build jobs, and records source/tree,
compiler, lockfile and payload hashes outside tracked source. The administrator
installer exposes only `run steam-hid` and `run steam-evdev` through a separate
helper; it does not change the provider or autonomous-updater policy. Both
scenarios reuse PID-bound isolation, fixed non-root identities and a 900-second
limit. A retained control directory prevents reuse rather than being removed
without evidence. The generated revoker checks matching helper/policy identities,
requires an idle lock and empty workspace, removes only those two access files,
and retains the immutable snapshot.

Deterministic tests cover scenario rejection, exact non-root command arguments,
changed payloads, symlinks/FIFOs/oversized files, exclusive packet creation,
real sudoers parsing, initial/final policy-validation failures, partial workspace
ownership failure, stale control resources and revocation refusal. A failure
before the workspace ownership change initially left an unregistered directory;
registering each newly created resource before its next fallible operation fixes
that rollback defect. No privileged installer or trial has run for this package.
This is preparation for Gate 5, not consumer acceptance. It does not yet provide
GUI-owned input isolation and cannot close Gate 1. Gates 1/4/5/6 remain open.

### Repair of stale root USB acceptance example

Inspection found that `examples/usb_audio_acceptance.rs` omitted the maintained
checker's required instance/generation arguments. It also generated the retired
periodic microphone pattern, compared playback against a constant pattern and
expected two extra seconds although the checker produces three. The example now
validates explicit expected identity before creation, forwards it for serial/
ancestry/isolation verification, generates and verifies the indexed S16 marker
contract, and accounts for the two startup plus one capture-edge seconds.
Ordinary-root API signatures and production audio behavior are unchanged.

Four regressions cover exact checker arguments/identity validation, marker
channel and wrap encoding, loss offset by duplication/channel corruption, and
mono phase across blocks and trial restarts. The two earlier regressions remain.
This fixes a demonstrable harness incompatibility; it does not explain the
recorded live microphone gaps or qualify VM continuity. No new positive audio
acceptance is claimed; Gates 4 and 6 remain open.

### Native USB playback marker coverage preparation

The ordinary public-factory fixture previously exercised native playback only
through node/connection/removal checks. Its marker exchange now covers all twelve
family/ownership cells, including six native-playback cells. A bounded `pw-cat`
record client receives ALSA-injected indexed playback markers; a simultaneous
ALSA capture independently audits microphone markers from sample or native
ownership. Each receipt keeps two startup seconds, three measured seconds and
one edge second separate. Playback success cannot hide microphone failure.
Caller sample APIs are not invoked for native playback; unavailable root-read
metrics are reported as unavailable rather than invented zero observations.

The native collector bounds stdout bytes, stderr file size, capture duration and
termination/reaping. Unexpected client exit, truncated production, changed
endpoint identity and malformed PCM reject the cell. Matrix verification now
requires twelve distinct typed marker verdicts in addition to twelve ownership
verdicts. The factory orchestration deadline is 360 seconds (peer 365 seconds)
for the expanded suite, within the existing 600-second lab bound; measured
windows, loss assertions, queues and latency requirements are unchanged.

Five Python regressions cover native marker corruption/duplication/incomplete
production/host failures, caller admission, complete/truncated/stalled child
cleanup, nonzero exits hidden by complete stdout, and independent microphone/
partial-frame rejection. A matrix regression rejects missing, duplicate, failed,
malformed and connection-only receipts. A Rust regression requires host marker
success, complete sample production where applicable and backend health.
These are deterministic preparation results; the expanded live matrix has not
run yet and cannot close Gates 4 or 6. Production library behavior, ordinary-root
API and dependencies are unchanged. Keep the previous failed microphone cells.

### Remote validation at the USB example checkpoint

All five exact-head workflows passed for `60dee107c0fef2ef29ebc41f683339c94e793f44`:
CI run 38184285692, CodeQL 38184287620, Gitleaks 38184289413, Provider Tier B
38184291136 and SBOM 38184293015. Downloaded SBOM verification checked all
17 workspace identities including the root. Dependency review and privileged
provider jobs remain skipped; they supply no live acceptance. This remote
checkpoint does not cover later native-playback probe changes.

### Expanded USB trial interrupted by terminal service failure

At `9d096c1b2d9fdf34fa483fb07e4dd991dfa44f05` (tree
`abcc6695af3b25e9ad69ebe2e271dbea6ce80377`), the autonomous updater installed
the unchanged eight-phase scope. Broker and worker hashes matched their accepted
production images. The bounded output observer image was
`f32c6b027ecc0ab6dddadb829b12c805f31e45492307d4e322bc4f1e178e0e9e`.
All 24 typed outputs passed, independently of failed audio. DualSense sample
playback had zero missing/duplicate/invalid measured markers; its native
microphone had 1040 silent measured frames and two marker gaps. The next DS4
session returned terminal `Read { reason: "broken pipe" }` during service. The
fixture's unwrap stopped collection before native-playback cells were reached.
Those cells remain unrun, not accepted. The cause of the terminal worker/control
failure is unresolved and must not be attributed to scheduling without evidence.

The root command receipt retains failed client stdout/stderr and the initiating
error separately from empty cleanup errors. Candidate service/socket/client units
were independently not found/inactive, owned cgroup processes and USB serials
were absent and authorized VHCI ports were free. The original broker service
returned inactive/socket active. Observer memory peak was 83.4 MiB; about 17 GiB
remained available with zero observed memory-pressure averages. No GUI, Steam,
compilation or competing audio trial overlapped the live run.

The collector now treats a cell panic as failure after its owned child guard
unwinds, closes the controller twice, records retained terminal diagnostics and
continues the independent cells. Terminal diagnostics force a false verdict;
closure and closed-API checks remain required. A synthetic broken-pipe service
regression verifies owned-case cleanup before the next receipt and preservation
of ordinary false verdicts. This is a harness collection correction, not a fix
for the terminal service failure or audio loss. A fresh full matrix is required.

### Complete USB cell collection and record-stop contract

The `2c6eb91b8178b69feae1e3f5ff32cb4ed2675b93` trial (tree
`3c75a5ad59702a4b338831249e3929ecf2f480b6`, observer SHA-256
`be9f9049ddf5e3891c93b57935a2fd4e77c96fa1c7add46075a6c046866a5c67`)
completed collection of all twelve verdicts in 114.61 seconds. The sample-playback/
native-microphone Xbox 360 cell passed its short window. The other five sample-
playback microphone cells retained 96/512/288/528/96 silent measured frames.
All six native-playback cells rejected the capture client's exit status 1 before
emitting marker receipts. This was a harness-stop incompatibility, not evidence
of their marker integrity; those receipts remain unverified. All 24 typed outputs
passed separately. Owned units/cgroups/USB serials were absent, VHCI ports free,
original service/socket states restored and cleanup errors empty.

Installed `pw-cat` reports compiled/linked PipeWire 1.4.2. Inspection of its
`do_quit` and final `data.drained` condition, also present in the
[upstream 1.4.2 source](https://raw.githubusercontent.com/PipeWire/pipewire/1.4.2/src/tools/pw-cat.c),
shows that deliberate record termination returns 1 because it does not reach
playback drain. The collector now recognizes that explicit record-stop contract
only after complete bounded capture, a stop it requested, and empty diagnostics.
It retains raw exit status, requested-stop flag, expected contract and captured
byte count in the receipt. Unexpected status 1 without a requested stop still
fails; client errors, incomplete production, corrupt/duplicate/missing markers
and microphone loss remain failures. Bounded diagnostic text is retained on
capture failure. A regression verifies expected stop, rejection without that
contract, rejection of client diagnostics and rejection of an unrequested exit.
This correction requires a fresh live matrix; no native-playback cell is closed
from the discarded historical captures, and no loss/rate/latency limit changes.

## Consistent microphone progress snapshot after the `8089c64` trial

A fresh isolated twelve-cell USB matrix at source
`8089c64a38ea5676992a71b8a10681b810f8a518`, tree
`24a494026d1e8d828cbd07fba3580113da3cafed`, observer SHA-256
`4a996c013e79d918a08925214194ecfca9474ea8462801f37ad05774f5352193`,
failed all twelve duplex cells. All 24 typed outputs passed separately.
Xbox native playback delivered 144000/144000 measured markers with zero silence
or gaps in both microphone modes; their microphone checks failed. DS4 native
playback with sample microphone had 192 silence frames. Other microphone cells
retained silence/loss; no sustained qualification is claimed.
The corrected owned recording-stop contract produced three complete native
receipts retaining exit status 1, requested termination and complete byte counts.
Missing native receipts remain failures, not assumed passes.

DS4 native/native also terminated with `microphone silence exceeds host frames`.
Inspection reproduced a counter race: capture updates silence before host time,
while the control thread formerly loaded host time before silence. A reader
between those updates could falsely reject a valid worker. Paired updates and
operation-7 snapshots now share a short internal mutex. The critical section
contains only atomic counter accesses, without IPC, PCM work or user callbacks.
The wire reply and ordinary-root signatures remain unchanged. The deterministic
`microphone_progress_snapshot_waits_for_complete_packet_accounting` regression
holds the writer's intermediate state, verifies the snapshot waits, then checks
exact completed and non-silent subsequent progress. Existing malformed-reply
rejection remains intact. This fixes a demonstrated terminal race, not marker loss.

The live root supervisor and observer reported empty cleanup errors. Independent
checks found owned broker/socket/observer units absent/inactive, no owned USB
serial, all authorized VHCI ports free, and original broker inactive/socket active.
Memory-pressure averages remained zero. Raw receipts are outside tracked source.
The worker image must be rebuilt and installed before live race revalidation;
Gate 3's prior accepted image remains historical until affected provider cells
are revalidated. Gates 1, 4, 5 and 6 remain open; alpha and the PR remain unready.

Supporting-contract implication: `gr_usbip::worker::Counters` gains private
synchronization state and a public paired-read method. External struct-literal
construction must use `Counters::default()`; workspace consumers already do.
Ordinary-root APIs and the operation-7 wire format are unchanged. Review lock
contention and rerun worker/provider validation on the new image. All mandatory
local checks pass, including the corrected final Clippy run; no dependencies added.

### Provider revalidation on the consistent-counter worker

All seven bounded provider phases passed at
`b817f48d7fc50ac38de21046144e4f55d1595e6d`, tree
`1f37f93c1b1385799f41e32088ca110cb6546a54`: rejection, positive lifecycle,
client exit, client-before-handoff, worker death, broker death, and siblings/
admission. Independently retrieved root receipts agree on revision, passed
status and worker SHA-256
`2ebea0f5ab8e35ae1656ce3c4611da723fe0fd799e6b7ce0daafaca27890fbe4`;
broker SHA-256 remains
`f1b6f56178e5a971c42b28ee5d8d5a4026e820850687bc37d0abe91c38d35cfa`.
Original broker inactive/socket active state was independently verified after
completion. Gate 3 retains its pass for this worker. This provider suite does
not prove microphone marker continuity or close Gates 1/4/5/6. A fresh USB
functional matrix is still required to exercise the corrected progress race.

### Fresh functional traffic after the snapshot correction

The installed `b817f48` worker was exercised with ordinary observer source
`02370e44d3eb55705ac11510f5f8c1b54d27bff2` (documentation-only descendant),
tree `8cfbf343c5f0f2e50d8f6b89b05155f08c008aa9`, observer SHA-256
`12a6afe9cac773e6f141ad5e233e4a2c817a604772fed6bc5ca65853fa42e5b5`.
All twelve duplex verdicts still failed. All 24 typed output requests passed.
Neither the invalid progress-counter error nor terminal-controller failure
recurred in this run; the deterministic regression is the race proof, while
one live absence alone is not proof of every possible interleaving.

DS4 and Xbox native playback each delivered all 144000 measured markers with
zero silence/gaps in both microphone ownership modes. Their microphone silence
counts were respectively 48/240 and 144/144 for native/sample ownership.
DualSense native playback emitted no usable capture receipt in either mode;
its helper failure requires explicit child-exit diagnostics before assigning a
cause. Sample-playback/native-microphone silence was 192/32/240 frames for
DualSense/DS4/Xbox (Xbox also had four marker gaps); sample/sample silence was
6672/8224/3312. These remain failures. No loss limit was weakened.

Owned broker/socket/observer units were absent/inactive, no owned USB serial
remained, original broker inactive/socket active state was restored, and both
supervisors reported empty cleanup errors. Memory-pressure averages stayed
zero. Gate 4 remains open; none of these short cells qualify sustained Gate 6.

## Native marker helper exit investigation at `38b7c0c`

Raw helper exit diagnostics at observer source
`38b7c0c2552fd0bd519f480e033ecb336ce60cf0`, tree
`a4baad11742f739aa1705716e5010663aa523c7b`, observer SHA-256
`b3458e5301e10987928828e2a042e47f4aacfbe75ffba8585eb47b6b380c9ec1`,
confirmed both missing DualSense native capture receipts ended with SIGKILL
(raw Unix wait status 9). Other completed marker helpers retained status 0 or
1. Two sample-playback/native-microphone cells (DualSense/DS4) passed this short
run, but the matrix still failed. Historical failures remain; those two passes
are not sustained acceptance. No counter-race terminal failure recurred.
The client unit's recorded peak was far below its memory cap; this is not proof
of SIGKILL's source. The inherited realtime execution bound remains a hypothesis.

Post-capture CPU-heavy marker analysis now explicitly lowers only its current
thread to SCHED_OTHER after all owned audio clients have completed and before
any decoding. Prior/current scheduling is retained in each receipt. Active graph
and client scheduling, resource limits, marker/loss/latency assertions and queues
are unchanged. A scheduling refusal fails rather than being ignored. Regressions
verify the exact thread/policy transition and refusal, plus completion-before-
transition and transition-before-decoding for both captured directions. This
requires fresh evidence; no claim that the SIGKILL cause is resolved yet.

### Complete native marker receipts after ordinary analysis scheduling

The fresh matrix at observer source `7b9840e7ba56f1cf309f3584dcd316027837d59f`,
tree `047aa4a3cb641671a459998700bef2167f931299`, observer SHA-256
`dec9aa87c34cc7fa8f0c65022f1a8ad4a91aad6b92d9f06e57392dcb38973864`,
used the unchanged accepted `b817f48` worker. All six native helpers now emitted
complete receipts and exited with ordinary status 1 for measured failures;
none ended with SIGKILL. Every native receipt records prior policy 2 and
analysis policy/priority 0/0. This one-variable change supports the diagnosis
that realtime CPU analysis caused the missing receipts, while the kernel kill
source itself was not independently traced. Do not describe it as an audio
continuity correction.

Five native-playback directions delivered all 144000 measured markers with
zero silence/gaps: both DualSense modes, both Xbox modes and DS4 with sample
microphone. DS4 native/native had 176 silence frames. Native-microphone silence
was 624/432/432 for DualSense/DS4/Xbox; sample-microphone silence in those native-
playback cells was 5568/5184/5136. All twelve duplex verdicts still failed.
Sample-playback/native-microphone silence was 240/432/432 (DualSense one marker
gap); sample/sample silence was 6720/6528/8208. No assertions were relaxed.

All 24 typed outputs passed. Owned units were absent/inactive, owned USB serial
absent, authorized VHCI ports free, broker inactive/socket active restored,
cleanup errors empty, memory-pressure averages zero. Required local checks and
445 Python tests passed before deployment. The remaining audio investigation
must address measured loss rather than repeat the resolved capture-stop or
missing-receipt issues. Gates 1/4/5/6 remain open and the PR remains draft.

## Demo endpoint disappearance and routing recovery

The production demo AudioRouter's private-graph routing fixture was extended
beyond A/B/disconnect/reconnect. It now removes the selected test-owned loopback,
waits at most five seconds for an observable routing error, verifies the
unselected sibling is still alive, disconnects both ports, recreates the owned
endpoint, waits for bounded node discovery, then verifies the reconnected exact
stereo marker. Existing route-switch markers, forwarding rejection while
inactive, host-process cleanup and all original regressions remain.
No production behavior or dependencies changed.

The diagnostic image passed every stage in an isolated PipeWire graph. Its
source-file SHA-256 is
`1fe35301193005efd273b9ad7549478084199e2251abd0a753cd0121380cbe7a`
and executable SHA-256
`e0d54d7f943ecf5fb9f02e2fa297db21a452f043edee1e1a39837e830591d928`.
The diagnostic used an uncommitted fixture on `d4f7957`; a pinned committed run
is required for acceptance. This covers the demo host-routing layer only;
USB/IP transport, required-worker failures and sustained continuity remain
separate. It does not close the whole GUI control inventory or either audio gate.

### Pinned routing-disappearance acceptance

At source `32db5267c95d31589af21b08844ea6e968bf0b63`, tree
`4d129fda9f67ad2e1f534d0a9300e4de0bc769c9`, the frozen demo test executable
`e0d54d7f943ecf5fb9f02e2fa297db21a452f043edee1e1a39837e830591d928`
passed A/B marker routing, selected endpoint disappearance with an observable
error, unselected sibling survival, explicit disconnection, recreated endpoint
reconnection and exact stereo marker delivery. The selected ignored test ran
alone in a private graph; owned loopbacks were reaped and the private graph's
policy/daemon were stopped by the runner. This closes that demo host-routing
failure/recovery cell for the pinned image, not USB transport failure/routing
or sustained audio acceptance. Required fmt/check/Clippy/test/Gitleaks checks
passed before committing. No production behavior or dependencies changed.


### Prebuilt event-level timer diagnostic

The native timer diagnostic now exports all 2,000 preallocated event records
(target CLOCK_MONOTONIC, wake CLOCK_MONOTONIC, wake CLOCK_MONOTONIC_RAW and
thread CPU time) after the timing thread terminates. Validation requires exact
5 ms deadline increments, monotonic clock/cpu observations, complete records,
and reconciliation of maximum lateness and threshold counts. Collection does
not allocate, log or perform blocking output in the timing loop. This diagnostic
can distinguish observed clock movement from time without thread execution;
it does not establish audio continuity or identify a hypervisor cause by itself.

`scripts/build-alpha-native-timer.py` defaults to a read-only action description.
Its explicit ordinary-user prebuild requires a clean committed candidate and
an exclusive output directory, records source/tree/compiler/source and binary
hashes, and never executes the image. Build before any live timing trial.
The lab's supporting test transport accepts `N` with exactly one sealed image
and a matching digest. The receiver's execution deadline is 15 seconds; the
sender requires an exact terminal success within 25 seconds. Legacy `T`, which
compiled during the live trial, now receives explicit error before launching a
child. Ordinary-root signatures, worker protocol and production behavior are
unchanged. The sender's build receipt remains necessary: the receiver can verify
the transferred image digest, but cannot infer its compiler or source provenance.

Regressions reject incomplete or reordered events, incorrect aggregates,
malformed/absent descriptors and changed scheduling grants. Sender tests verify
sealed-image integrity, exact success/error completion and child reaping.
Prebuild tests cover no-op default, dirty/root rejection, exclusive artifacts
and hash receipts. This adds diagnostic evidence only; historical failed audio
trials and Gate 6 remain open until the required controls and product matrix pass.


### Verified event-level timer comparison at `a040f7e`

A corrected bounded run used the current all-feature ordinary observer image
SHA-256 `82a3b2f39431880b4f912d7a722b6943b6da5b636b073dfb18ec65bdd234a2b8`
and prebuilt timer SHA-256
`f9c60ec7b5b53c88df469155329ec031059a430afbb842891c64bff9fc732c7a`.
The root lab/source and C prebuild were pinned to
`a040f7e47b456dc30dc30a0ad1180de44c2ca40b`, tree
`cf539eb7d86b9208be530161a5a658e1057719d5`. The coordinator ran from the
later documentation-only `65ffc11` tree. Exactly one timer ledger was independently
extracted from the root receipt; all 2,000 records and aggregates validate, and
both source and binary hashes match the separate prebuild receipt.

On the first allowed CPU, the observed thread policy was FIFO, priority 88,
with zero errors, maximum wake lateness 1,144,917 ns and zero wakes above 10 or
20 ms. This ten-second result contradicts an unconditional claim that this VM
always misses the relevant timer deadlines. It does not resolve intermittent
historical failures, qualify a 60-second audio control, or close Gate 6.
The observer and root phase exited zero; cleanup errors were empty. Independent
checks found the observer unit absent, authorized VHCI ports free, and original
broker inactive/socket active state restored. Observer peak memory was 5.7 MiB.

The immediately preceding run selected an older observer and emitted no timer
ledger despite zero process exits. It is excluded from timer acceptance; raw
results and a separate invalid-evidence audit are preserved. Reviewers must
require actual scenario output and executable identity, rather than accepting
coordinator status alone. Unknown optional observer selectors currently fall
through without the requested probe; fail-closed selector validation and
scenario-completion reconciliation remain a tooling correction to implement.


### Fail-closed optional observer selection

The typed USB observer now parses its requested optional probe before connecting
to the lab or constructing controllers. Unknown/empty modes and missing/empty
prebuilt image arguments reject explicitly. An absent selector retains output-only
mode. Existing R/D/C/N and legacy-T rejection scenarios remain explicit.
After successful terminal acknowledgement, the observer emits a mode-specific
`public_probe_completed` marker; failures cannot reach that print.

A deterministic regression covers every recognized selector, unknown/malformed
values and required image boundaries. Existing sealed descriptor and terminal
reply regressions remain. Issue #146 remains open for maintained coordinator
artifact discovery and exact scenario/image receipt reconciliation; this fix
alone cannot make an older executable understand new selectors.


### Retained Steam profile runtime collision

The first installed HID fixture created nine event nodes and four HID nodes.
The Steam namespace sentinel verified all 13 exact owned devices openable,
private home/getent mapping, private proc/dev and dropped capabilities. The
consumer then failed before Steam startup: bootstrap tried to extract public
FEX assets into the retained test profile's already-existing runtime directory.
This is a demonstrated harness defect, not failed controller recognition.
The preceding workspace-parent attempt also correctly rejected a non-private
parent before launch; the retry used an exclusive private directory.

Bootstrap now extracts public assets and copies public FEX configuration into a
fresh directory under the owned disk-backed namespace /tmp. Runtime/server paths
point there, while authorized login/profile state retains its prior private
location. The outer workspace teardown removes disposable runtime assets.
A synthetic two-launch regression retains a fake login marker and prior runtime,
requires distinct fresh extractions/configuration and verifies neither retained
file changes. No real profile data is used as a fixture or published.

The neutral fixture was explicitly stopped without arming. Its root receipt
reports zero status and no initiating/cleanup errors; independent checks confirm
producer exit, no matching owned device identities and no control socket.
Both failed Steam preparation attempts report no cleanup errors. These are
isolation/deployment and harness-correction evidence only; Gate 5 still requires
actual consumer recognition/input/output and lifecycle checks after the fix.


### First isolated Steam HID consumer observations

At Steam-runner source `5ba0b94`, the installed input fixture remained pinned to
`d4f7957` (probe SHA-256
`f8f228892246e54a8e56bae2b05752d9f8e08e0a35956cde15a92c3ad1885f67`).
The corrected bootstrap reused the authorized private test login and reached
home and controller settings. The namespace sentinel opened all 13 exact owned
nodes, hid the real home, and verified private proc/dev and dropped capabilities.
Only Xbox 360 appeared in Steam's connected-controller list; DualSense, DS4 and
Switch Pro recognition is not accepted by this trial.

On the Xbox test screen, a controlled fixture sequence generated only South
pulses, but a captured active state showed Y, D-pad and stick activity rather than
an exact South-button response. This is a consumer mismatch requiring causal
investigation, not yet an identified library-code defect. Isolated device metadata,
consumer HID versus evdev selection and FEX/backend decoding are candidate
boundaries to discriminate. Do not mark Xbox input accepted from its device name
or substitute this partial sequence for full input/output coverage.

The fixture was stopped through its owned control socket; the exact owned Steam
unit was stopped and reaped. Root and ordinary cleanup report no errors.
Independent checks confirm producer absence, no matching owned nodes/control
socket, removed temporary Steam workspace, and an absent/inactive Steam unit.
The private test profile remains by user request. Raw screenshots/logs are private
outside tracked source and are not published. Memory cap was 12,279,873,536 bytes,
minimum measured available memory 15,314,513,920 bytes, maximum full avg10 pressure
0.42%; no memory termination occurred. This accepts isolation/bootstrap only.
Gate 5 remains open; compare evdev and narrowed enumeration/decoding controls next.


### Evdev comparison and reopened DualSense discovery parity

The next isolated Steam run exposed only five test-owned event nodes; no raw HID
nodes were mounted. Its controller list showed none. All five node opens passed
the namespace sentinel. The private namespace omits the host udev database;
this is a discovery-metadata difference to investigate, not proof that the product
cannot create usable devices. The host's owned-device property audit classified
DS4's gamepad, Switch and Xbox as joysticks, and DS4's companion as a touchscreen.
DualSense's combined node was classified only as a touchpad, with no joystick
property. This specific product discovery/parity defect is tracked in #147 and
reopens the affected Gate 2 cell; earlier unrelated output passes remain pinned.

The evdev fixture remained neutral, then was stopped through its owned socket.
Its root receipt reports status zero, no initiating/cleanup errors and rule removal.
The identity-verified Steam unit was also stopped. The ordinary collection process
terminated with SIGTERM (143) without a final JSON receipt; although the registered
producer, display and Steam/FEX processes are absent, the private workspace
remains as interrupted evidence. Full ordinary restoration is not accepted from
missing output. Investigate signal/collector teardown and preserve the private
workspace; no broad removal or foreign-resource cleanup was attempted.

Gate 3 alone currently retains whole-gate acceptance. Gates 1/2/4/5/6 remain open.
Next steps: correct DualSense gamepad/contact composition with parity/transactional
regressions, supply only owned discovery metadata inside the private consumer
namespace, and demonstrate bounded interruption/restoration. Do not falsify host
classification or remove implemented touch controls to make a test pass.

## DualSense evdev discovery correction — 2026-10-12

Code candidate `1b0029c119867daf0fd5f776f10c4fe10a7d7db3` (following
`1931d64979b6c56cd1d3119df4005db70bbe899a`) shares the existing transactional
DS4 gamepad/contact composition with DualSense. Both `BTN_TOUCH` (330) and
`BTN_TOOL_FINGER` (325) route exclusively to the touch companion. Primary
controls and conventional feedback remain on the gamepad. Both touch slots,
tracking IDs, coordinates and releases remain available on the companion.
Ordinary-root signatures are unchanged; DualSense association metadata now
includes its owned touch companion. The surface documents that SDL gamepad touch
APIs do not consume this separate evdev device.

| Boundary | Evidence / disposition |
|---|---|
| Exact capabilities and frames | Deterministic DualSense regressions verify both touch-classification bits absent from primary, retained on companion, both slots and releases |
| Transactional lifecycle | Fake second-open failure removes primary for both Sony families; DualSense repeated close and sibling feedback/service regressions pass; prior DS4 regressions retained |
| Linux discovery | Candidate producer SHA-256 `a1ad9e3bf237fa53cdf1c2cd4df28015beb32d9959c5755e4fe8f74694e60836`: settled live udev identifies DualSense primary as joystick and companion as touchpad; all four primary families are joysticks |
| Isolation and cleanup | Neutral-only scoped trial; private seat and shared desktop ignore flags retained; producer absent, owned nodes absent, rule removed, root status 0 with no initiating/cleanup errors |
| Historical failures | The first split still left `BTN_TOOL_FINGER` on primary and failed live discovery. Its receipts are retained. The final audit initially expected a touchscreen companion; the actual touchpad classification correctly reflects retained finger semantics |
| Validation | fmt/check/Clippy/workspace tests/Gitleaks and forced Rust 1.85 all-target/all-feature check passed |
| Remaining acceptance | Active DualSense companion contact/feedback/consumer revalidation and isolated Steam discovery/input remain open. This classification pass does not close whole Gate 2 or Gate 5 |

Reviewer: scrutinize shared Sony frame routing, exact finger/contact release,
association ownership, second-node rollback and sibling survival. No global udev
classification override, loss threshold change or queue enlargement was used.
Raw receipts remain outside tracked source. The existing six-gate scope remains
in force pending direct release-owner confirmation of the PR reviewer's proposed
scope change; an external generated comment is not confirmation from this chat.

## Isolated Steam discovery comparison — 2026-10-12

Runner `9ebb0cb` exposes a bounded snapshot of discovery properties only for
identity-pinned owned character devices. It rejects symlinks, oversized records
and device replacement; private seat/desktop-ignore values remain unchanged.
All 454 Python tooling regressions pass, including synthetic foreign/private
record exclusion and namespace binding tests. No host udev configuration changes.

With producer `1b0029c119867daf0fd5f776f10c4fe10a7d7db3`, the namespace sentinel
passed: all six owned evdev nodes openable, real/passwd home hidden, private
proc/dev and capabilities dropped. Steam then listed PS5, PS4, Switch Pro and
Xbox 360. It also listed an additional PS5 entry; the touch companion is a
candidate cause requiring investigation. Before the owned udev snapshot,
Steam listed no evdev controllers. This is recognition evidence only: input was
not armed and no exact input acceptance is claimed.

The bounded 120-second unit reached its configured timeout and the collector
reported failure/nonzero exit. Preserve that result rather than promote the
recognition observation to a successful full trial. Cleanup had no errors:
unit absent/inactive, display and producer absent, disposable workspace removed,
input rule removed, root fixture status zero. Minimum available host memory was
14,382,125,056 bytes; recorded unit memory peak was 5.1 GiB, without swap.
Private screenshots, logs and retained authorized test profile remain outside
tracked source. Gate 5 stays open for exact input, duplicate-entry diagnosis,
remaining consumer cells and complete acceptance.

## Single-family Steam fixture and companion identity follow-up — 2026-10-12

`3d686eaef3f90f290c23407c66fec112620dbbf7` adds explicit `arm-dualsense`,
`arm-ds4`, `arm-switch`, `arm-xbox360` and `pause` commands to the owned producer.
Only the chosen family pulses; siblings stay neutral. The legacy full-family
cycle remains. Six focused fixture tests verify closed command selection,
startup/release/removal and sibling suppression; workspace validation and Rust
1.85 pass. The immutable input updater installed the same two existing scenarios
with producer SHA-256 `0e1fe0cc1d4514c8048623e289593b00ff52d01dd47d19d1c78499770b333330`.

The prior all-family live trial completed owned shutdown and cleanup with zero
collector/root status. Independent evdev observations confirm primary South
press/release. Steam exact input remains unaccepted. The single-family trial
kept siblings neutral, but did not observe the expected highlight on the selected
PS5 screen and ended at its 300-second unit timeout with failure retained;
cleanup completed without errors. No exact input pass follows from recognition.

Steam reported two distinct Sony SDL GUIDs. Their name CRC bytes match the
primary (`5756`, DualSense Wireless Controller) and companion (`514c`, Virtual
DualSense Touch Contacts), using [SDL's CRC algorithm](https://github.com/libsdl-org/SDL/blob/SDL2/src/stdlib/SDL_crc16.c).
This corroborates companion misidentification, without proving the cause of the
input mismatch. Correction `82e0dcc` presents only the DualSense companion as a
generic touch device (vendor/product zero, version one), retains primary Sony
identity, both touch controls and creation-scoped association. An exact identity
regression is added; mandatory Rust checks and forced 1.85 pass. Fresh live
consumer revalidation is still required; Gate 2 and Gate 5 remain open.
Private profile/captures/logs remain outside tracked source. No dependencies,
persistent host changes or relaxed acceptance assertions.

## FEX input ABI differential — 2026-10-12

A bounded ordinary-user direct-read comparison used the installed `82e0dcc`
producer (SHA-256 `27e47c3148fd08e5cade18a4c3db78de97eb768999bf8d27a9bfa5624510ff27`),
FEX 2608 (binary SHA-256 `9562f9682a6fbe3efb36ffa164970e83aff5dbf73ff8f6464955ab7fb754ea19`)
and kernel `6.12.111+deb13-arm64`. A static i386 reader, without product queues or
SDL, opened only the process-owned DualSense primary read-only. It received two
48-byte chunks. Native 24-byte decoding and FEX data decoded as 24-byte records
both yield exactly South press, sync, South release, sync. Expected i386 16-byte
decoding instead yields invalid event types/codes. Reader and fixture exited zero;
no owned nodes remained; temporary input rule removal and cleanup passed.

The kernel rejects `PR_GET_COMPAT_INPUT` (`0x63494e50`) with `EINVAL`.
[FEX's compatibility change](https://github.com/FEX-Emu/FEX/pull/4106) requires
[a corresponding kernel interface](https://github.com/slp/linux/commit/897560fd00e1add63a9973c31fc437b006855680).
This confirms the direct FEX evdev ABI mismatch on this host, not the exact
backend chosen by Steam. Do not change product report layout to accommodate it.
Two preceding attempts waited for deferred runner output and timed out before
input testing; both cleaned up. Their failed receipts remain preserved. The
successful diagnostic instead verified the live producer, owned nodes and socket.

Next discriminate Steam's backend and test its supported classic joystick path
using only identity-verified owned nodes. Any classic-path success is separately
scoped; it cannot establish evdev compatibility or full output parity. Gate 5
remains open. Generic-companion Steam recognition/contact/feedback revalidation
also remains pending. Raw data and probe-build receipts stay outside tracked
source. Available host memory remained above 17 GiB; no heavy concurrent trial.

### Classic joystick differential

The same pinned producer and FEX reader then opened only the identity-verified
owned primary's classic joystick sibling. Each reader received 22 initialization
records separately from the measured South press/release. Both native and FEX
8-byte decoding yield identical measured timestamps, button zero, values one then
zero. Both exit statuses are zero; owned nodes absent and rule cleanup passed.
This supplies a viable next consumer experiment, not Steam acceptance. Next add
explicit, tested classic-joystick selection and owned-node namespace exposure;
then verify actual Steam recognition/input/output and cleanup. Preserve the
failed evdev result and do not extrapolate classic input into evdev/output parity.

## Isolated Steam classic-backend experiment — 2026-10-12

Runner `1551be1` adds opt-in `--classic-joystick`. Inventory selects only owned
`js` character nodes with process/sysfs/device identity checks; default evdev/HID
inventory is unchanged. The private namespace rejects foreign joystick nodes,
sets both SDL2/SDL3 classic hints, and records the selected backend. Two new
regressions cover foreign exclusion, replacement rejection and hint/default
separation. All 456 Python tests and mandatory Rust/Gitleaks checks pass.

The actual Steam experiment exposed exactly four owned classic nodes. Its
sentinel passed and both hints were verified in the owned Steam process
environment. Steam nevertheless displayed no connected controllers. Therefore
the working direct classic reader does **not** establish a working Steam backend.
No input pass is claimed. The registered unit was stopped after identity
verification, before its deadline. Collector and root fixture exit zero;
initiating/cleanup errors empty, workspace removed, display/producer absent,
temporary rule removed. The runner's `passed` verdict covers isolation/bootstrap/
restoration only; the controller consumer observation is failed.

Next resolve Steam's actual discovery/backend contract or qualify an available
consumer runtime supporting the observed ABI. Do not patch product event sizes,
copy normal account state, replace the kernel, or relabel failed discovery as
accepted. The opt-in classic comparison remains useful independent evidence,
not the final acceptance substitute. Generic-companion evdev recognition and
active contact/feedback remain separate pending cells. PR stays draft.

## Exact shipped SDL comparison — 2026-10-12

A bounded i386 reader linked to Steam's shipped SDL3 (`SDL_GetVersion=3005000`,
SHA-256 `ae4e7ec75bde30e4ddf70b11c16be95ed9c28d31095798e9309ee6392b29f702`)
ran under FEX 2608 in a private namespace with only four owned classic nodes.
It explicitly sets the classic hint before initialization. SDL enumerates all
four named family joysticks, classifies all four as gamepads and supplies their
button/axis mappings. The selected DualSense South pulse reads button mask one
then zero; siblings remain neutral. This narrows the failed Steam UI comparison:
classic support and mapping exist in the exact shipped SDL library. Steam's
controller type/filtering/configuration remains to be isolated; no precise Steam
cause is yet confirmed and Gate 5 remains open.

The SDL reader/fixture exit zero, temporary rule removal and cleanup pass, owned
nodes and diagnostic processes are absent. No normal account state, output
writes, desktop touch or persistent host configuration were used. The public
runtime was extracted into a disposable diagnostic directory; raw receipts and
build artifacts stay outside tracked source. This is a direct SDL observation,
not a replacement for Steam or the separately reviewed SDL probe contract.

## Settled Steam classic input observations — 2026-10-12

At runner `1551be16d3a275acf6c6b6ec9e41b28f43342ad6` with the pinned `82e0dcc`
producer, Steam eventually lists all four classic controllers without changing
settings. Its input screen shows the selected South press and release for
DualSense, DS4 and Switch individually; triggers/sticks and siblings remain
neutral. Xbox recognition is observed, but its input cell was not exercised
before the configured 600-second unit deadline. Standard gamepad classification
and vendor/product zero remain classic-backend limitations; named recognition
is not exact physical-family identity or output parity.

This supersedes the inference from the earlier early-time “no controllers”
screenshot: that run did not establish a settled discovery failure. Preserve its
observation and receipt without attributing a permanent Steam filtering defect.
The direct evdev ABI mismatch remains confirmed independently. The latest unit
expired with collector status one; root fixture status zero, cleanup errors
empty, owned workspace/display/producer/nodes absent and rule removed. Thus three
specific input observations pass, while the overall trial and Gate 5 remain
unaccepted. Xbox input, removal/sibling and remaining realization/output cells
still require evidence. Private screenshots and profile stay outside Git.

## Ownership-lock restart regression — 2026-10-12

The first mandatory workspace run at `c689755` failed the immediate restart
assertion with `WouldBlock`; isolated and complete reruns passed. A new bounded
regression deliberately pauses a child between fork and exec. The lock has
`FD_CLOEXEC`, but after the original owner closes it, reacquisition still returns
`WouldBlock` while the child holds its inherited open-file description. After
exec and reaping the child, immediate reacquisition succeeds. This demonstrates
the process-inheritance mechanism that can invalidate a parallel test's
immediate-drop assumption; it does not reconstruct the original failed run's
exact timing.

The existing restart/journal/idempotent-cleanup assertions now run in a bounded,
single-test subprocess, with the inherited-lock sequence covered explicitly.
The parent terminates and reaps a timed-out test child. No production lock,
recovery policy, queue, admission rule or public API changed. In particular,
there is no explicit unlock that would allow a second owner while an inherited
lock descriptor remains live. Preserve the initial failed receipt.

Focused regression and mandatory fmt, locked all-target/all-feature workspace
check, Clippy, workspace tests and Gitleaks pass. Review the child handshake,
close-on-exec assertion, timeout/reaping and retained exclusive ownership.
This resolves the test isolation gap, not any outstanding live acceptance cell.

## Xbox classic Steam input and normal shutdown — 2026-10-12

At unchanged classic runner `1551be1` and pinned producer `82e0dcc`, isolated
Steam lists all four named controllers. Selecting Virtual Xbox 360 and arming
only that family shows A pressed at one second and released at 2.6 seconds;
sticks and triggers remain neutral. Together with the previous scoped
DualSense/DS4/Switch observations, all four classic South input cells now have
consumer evidence. Classic generic type and zero vendor/product remain explicit
limitations; this does not prove full family identity, outputs, HID/USB paths or
whole Gate 5 acceptance.

Steam was exited through its own menu before the unit deadline. Collector and
root fixture both exit zero; bootstrap exit zero, initiating error absent,
cleanup lists empty, temporary rule removed, owned display/producer/workspace/
input nodes/control socket absent. The authorized private test profile was
reused and retained; no normal account credentials or profile were copied.
576 memory samples retain a minimum 14042464256 available bytes. Raw private
screenshots and receipts remain outside source. A preliminary attempt rejected
a non-private workspace parent before launching Steam; its root cleanup passed
and its failed receipt remains preserved.

Per-controller removal/sibling survival is still pending: the producer removes
DualSense at 600 seconds, coinciding with this trial's 600-second Steam limit.
The next lifecycle trial uses a bounded 700-second consumer window inside the
unchanged 900-second fixture limit, leaving time to observe removal and sibling
input before normal exit. No live deadline was silently extended, and no
acceptance assertion changed.

## Steam lifecycle trial: sibling-arm harness defect — 2026-10-12

The 700-second classic consumer trial at unchanged producer `82e0dcc` observed
kernel removal of DualSense at its scheduled 600-second boundary. The consumer
receipt records `/dev/input/js1` absent in 94 samples, but Steam's controller
list still shows all four entries in the post-removal screenshot. Stale private
bind mounts/discovery metadata are a hypothesis for the consumer result, not a
confirmed product removal defect. Sibling input after removal is unaccepted.

The subsequent Xbox arm request exposed a separate deterministic fixture bug:
`ArmFamily` compared every selected family's pulse deadline to DualSense's
removal time. It returned `not enough time for selected input before removal`,
ending the producer although three siblings were alive. Root fixture exit one,
collector exit one, bootstrap initiating reason `input owner is not live`;
this run terminated on the producer error, not the 700-second unit deadline.
Cleanup lists are empty, temporary rule removed, owned nodes/processes/display/
workspace/control socket absent. The minimum available memory was 14630780928
bytes; no memory-pressure cause was established. Preserve the failed receipt.

`selected_input_fits` now applies the early removal deadline only to DualSense;
all siblings retain the fixture shutdown deadline. The regression exercises
arming after removal, exact two-second release, boundary acceptance/rejection,
removed-family rejection, invalid index and overflow. Seven focused producer
tests pass. No product API, provider, desktop isolation, sudo scope or acceptance
assertion changed. Rebuild and autonomously stage the fixed producer, then
repeat only affected lifecycle cells and independently investigate stale
consumer discovery. Gate 5 remains open.

Validation of the sibling-lifetime correction: mandatory fmt, locked workspace
all-target/all-feature check, Clippy, workspace tests and Gitleaks pass; forced
Rust 1.85.0 workspace/all-target/all-feature check passes. The seven focused
producer tests include the new post-removal regression. No dependencies added.

## Corrected classic sibling lifecycle and native classification — 2026-10-12

Pinned producer `d67f7ba6782b8645e0fef7d04a08dae87724d4f5` (SHA-256
`1d1674e9952decbf7834f1b7fcf37feaacb96b59899a09678f34748aa5c929cb`)
passes the affected post-removal arm sequence. After scheduled DualSense removal
at second 600, the producer accepts Xbox arming at second 600 and emits South
true then false at second 602. Identity-checked native removal and unchanged
sibling identity precede the pulse. Steam's selected Xbox test screen shows A
pressed at one second and released at 2.6 seconds, with other displayed controls
neutral. The producer stays live until normal Steam menu exit; collector, fixture
and bootstrap all exit zero. Cleanup lists are empty, rule removed, owned input
nodes/control socket/producer/display/workspace absent. 589 memory samples retain
at least 14696333312 available bytes; 11 samples report the retired classic node.
This accepts the scoped classic sibling-input cell, not whole Gate 5.

Fresh native evdev discovery on the same image classifies all four primaries as
joysticks. The DualSense primary keeps `054c:0ce6`, while its generic companion
is `0000:0000`, version `0001`, and a touchpad without joystick classification.
The DS4 primary keeps `054c:05c4`; its contact companion is a touchscreen.
Every owned node carries the private test seat and desktop-ignore property.
This closes the current native discovery/classification cell; active contacts,
feedback and full affected consumer parity remain separate Gate 2 requirements.
No desktop touch was injected by this trial.

The earlier stale Steam-list observation is not erased. An independent sanitized
regular-file mount control confirms that an individually bind-mounted target
remains present with the same inode after its source is unlinked outside the
namespace. This proves the namespace retention mechanism, not the exact shipped
Steam backend or the controller's hotplug behavior. Current SDL Linux source is
an evidence lead for directory/udev removal handling, not a matched shipped-source
receipt. Future hotplug evidence must inspect both host and consumer visibility;
do not equate the existing host-side removed-node counter with consumer removal.
Private profiles/screenshots and raw receipts remain outside tracked source.

## Classic Steam namespace retirement correction — 2026-10-12

The classic-only consumer view now removes identity-pinned joystick aliases when
the original owned nodes disappear. Hidden backing bindings remain owned until
consumer teardown; the default evdev/hidraw path is unchanged. No root policy,
provider binary, public API or dependencies change. Four focused regressions
cover retirement/idempotence/sibling preservation, changed owner/directory/alias
identities and foreign entries, canonical bindings and namespace verification.
All 460 Python tooling tests pass. An actual generated namespace control verifies
alias disappearance independently of Steam, with zero exit and owned cleanup.
This does not accept Steam hotplug: fresh affected live consumer cells are pending.
Gate 5 remains open and the earlier stale controller-list observation is retained.

## Dynamic classic-view live checkpoint and discovery experiment

Runner `b8495138affbbda9065194723e0d2419b06f21fe`, unchanged producer
`d67f7ba6782b8645e0fef7d04a08dae87724d4f5`: Steam's shipped SDL 3.5.0
independently discovers four classic gamepads through the aliases. After scheduled
DualSense removal, inspection of the identity-pinned actual Steam process namespace
finds only the three sibling aliases and verifies Xbox backing identity. The first
observer incorrectly resolved an absolute namespace symlink against the host and
failed without producing an accepted receipt; the corrected lstat/canonical-backing
observer passes. Steam still lists the removed DualSense. Xbox South press/release
remains visible afterward. The overall run fails at its runtime deadline before
normal exit; root fixture exit is zero, collector exit one, initiating timeout is
retained, both cleanup lists are empty, owned inputs/processes/workspace absent.
680 memory samples retain at least 11290025984 available bytes, with 98 reporting
the retired node. No output acceptance is claimed.

The next classic-only discovery experiment sets `SDL_JOYSTICK_DISABLE_UDEV=1`.
The maintained SDL source and shipped library both identify this setting. It directs
SDL toward fallback/directory notification discovery appropriate to the isolated
owned-device view; it does not expose host udev events or foreign devices. Existing classic
hints remain; default evdev/hidraw settings are unchanged. Regression checks the
hint's exact value and absence in the default path. Fresh direct SDL hotplug and
Steam list/input/normal-exit evidence remain required. This does not establish the
exact Steam backend cause, close Gate 5, or alter controller/public API behavior.

## Causal classic-discovery result and interruption regression — 2026-10-12

At runner `a4c6fa8`, unchanged producer `d67f7ba`, two direct readers of the same
shipped SDL library identify revision `SDL-release-3.4.0-1359-g70e9cc86d`.
Both start with four gamepads and retire the same visible owned alias after kernel
removal. Udev mode exits zero with count four and no disconnected handle; directory
mode exits zero with count three and exactly the removed handle disconnected.
Both observe Xbox South press/release afterward. Steam using directory mode shows
the DualSense disconnection notification and a list containing exactly DS4, Switch
and Xbox; the selected Xbox test also shows press/release. This supports the
classic isolation-harness discovery correction; it does not establish every
Steam backend or a controller product defect. The sandbox retains networking for
login: the earlier network-namespace explanation was incorrect. The relevant
boundary is the isolated device view/static udev snapshot and observed discovery
mode, not a claimed network isolation mechanism.

The overall live run fails: the collector/tool exits 143 without a terminal JSON
receipt, the owned Steam unit records timeout, and the input supervisor reaches
its own deadline (child status -9, cleanup empty, rule removed). No normal-exit
or whole-lifecycle pass is claimed. Owned fixture/readers/display are absent,
owned input inventory is empty, the display socket is absent, and the matching
synthetic home canary is removed. The private interrupted workspace remains as
raw evidence outside Git. The exact source of the collector's termination is not
established. Do not erase the failed receipts or promote the scoped input/removal
observations to complete Gate 5 acceptance.

The maintained ordinary Steam supervisor now handles SIGTERM/SIGINT, enters its
bounded restoration path, ignores repeat interrupts during restoration, restores
prior signal handlers afterward, and starts its systemd launcher in a separate
session. Two focused regressions exercise real SIGTERM, outer canary/profile
cleanup and handler restoration, plus two actual owned subprocesses with repeated
signals during teardown. Both children are reaped; no Steam, root or real display
is used by those regressions. All 462 Python tests and mandatory repository checks
pass. Fresh terminal live receipts are still required before accepting shutdown.

## Live interruption restoration and GUI provisioning checkpoint

At `2c5f342`, a controlled SIGTERM to the identity-pinned ordinary Steam supervisor
produces the expected failed-run receipt with `Steam lab interrupted by SIGTERM`.
Bootstrap and outer cleanup lists are empty. The unchanged input fixture exits
zero with no initiating error, no cleanup error and its rule removed. Owned
producer/collector/input nodes/workspace/display socket are absent. 59 memory
samples retain at least 14543065088 available bytes. This accepts the live
interruption/restoration cell, not consumer completeness or normal shutdown.

The first probe refused startup because the earlier killed fixture left its
private control socket. That failure and failed coordinator cleanup are retained.
After verifying the known test owner was absent, its owned nodes were absent,
the socket had no listener, private ownership/inventory and current inode
identities matched, the stale test directory was moved intact into private
evidence quarantine; nothing was deleted and no provider journal was recovered.
The retry then starts and restores normally. Preserve this boundary: this is
operator restoration of that test artifact, not automatic lease recovery or
permission to clear unprovable provider journals.

A separate hash-pinned GUI keyboard package is prepared for administrator
installation. It permits one predefined, 900-second non-root GUI run on a
registered private display with existing PID-owned input isolation. It does not
change the two Steam scenarios, their updater, provider services or general sudo.
Preparation verifies malformed/oversized/symlink/nonprivate display records,
exact process identity fields, six invalid scenario selections, interpreter
syntax, helper hashes and sudo policy syntax. A matching revoker is supplied.
Installation is still required; no GUI-owned active touch or whole Gate 1 pass
is claimed. The final byte-pinned neutral soak follows the interaction inventory.
