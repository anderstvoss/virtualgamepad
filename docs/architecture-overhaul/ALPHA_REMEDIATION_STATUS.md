# Alpha remediation status

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
