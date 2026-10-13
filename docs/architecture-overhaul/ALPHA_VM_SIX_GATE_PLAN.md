# Six-gate acceptance on the current VM

## Binding alpha scope and execution plan — 2026-10-13

The release owner's [final scope decision](https://github.com/anderstvoss/virtualgamepad/pull/133#issuecomment-6148689485)
supersedes the broader historical acceptance requirements below. The user's
subsequent instruction also defers Steam/FEX acceptance to x86-64 testing in
[#144](https://github.com/anderstvoss/virtualgamepad/issues/144), **not a pass**.
FEX's input-event ABI boundary is a credible contributor; Steam's selected backend
and the cause of every consumer failure are not established. Native protocol and
transport traces establish controller correctness; Steam is separate compatibility
evidence. Preserve all historical failures without investigating deferred cells.

Execute once on one frozen source/build candidate:

1. Record source/tree, compiler/lockfile and demo/broker/worker hashes. Build once.
2. GUI: four default-realization families; Tab/Shift-Tab visible creation, face,
   stick, trigger and removal focus; face/stick input and neutral release;
   middle removal with working neighbours; window close and owned cleanup.
   Record lack of screen-reader support (eframe accesskit is not enabled).
3. Changed controller cells only: native reviewed SDL Xbox all-control HID/evdev
   sweeps, USB output EPIPE/no callback, DualSense companion contact movement/
   release and both-node removal. Reuse DS4 rollback and Sony output passes.
4. Run the existing seven provider phases once with owned restoration. Rerun
   only affected checks if broker/worker/USB product code changes.
5. USB audio: each of DualSense, DS4 and Xbox 360 delivers playback markers to
   host ALSA and microphone markers to the application; close/repeated close,
   untouched shared defaults/foreign resources and owned cleanup. Reuse applicable
   worker-kill/route-disappearance passes. Record silence/gaps/loss as limitations;
   do not investigate sustained continuity. Complete absence of flow is distinct
   from occasional gaps and still requires a supported disposition.
6. Steam is deferred by the user's later direction. Sustained audio/zero-loss/
   latency are deferred by the release owner to #135/#145; physical comparisons
   remain #136. Full control/accessibility and consumer/routing matrices move to
   beta. No independent controls, timing diagnostics, 72-cell matrix, new tooling
   unless needed to execute a listed check, or additional GUI soak. The accepted
   `46be2bf` soak suffices for alpha.
7. Run final automated checks and exact-head CI, including Windows, CodeQL,
   Gitleaks and all workspace SBOMs. Publish one short result table and known
   limitations in the PR, update #143/#144/#145, then mark ready for review when
   required checks pass. Do not merge, tag or release.

Historical whole-gate ratings and broader mandatory wording below are superseded
by this scope. Keep raw receipts outside source; do not add more evidence ledgers.

Historical disposition (superseded by the current scope correction and status ledger): **Gates 2 and 3 passed**. Gate 2's cell-by-cell
receipts are in [controller output acceptance](ALPHA_CONTROLLER_OUTPUT_ACCEPTANCE.md).
Gate 3 was revalidated at `4c3b6132d1fdede6dd384bf9dcf70cc9a406c523`;
Gates 1, 4, 5 and 6 remain open.
See [provider acceptance and regression map](ALPHA_PROVIDER_RECOVERY_ACCEPTANCE.md).
This current evidence supersedes older whole-gate ratings below. Reopen affected
cells after relevant code/binary changes. Alpha remains not ready.

Current closure criteria and verifier gaps are in
[the acceptance criteria audit](ALPHA_GATE_CRITERIA.md). Historical progress
entries below retain their original scope and may be superseded by that audit.

Continue in PR #133 on its existing branch. Native timing and physical comparisons
remain post-alpha #135/#136. Gates 1–4 and 6 remain alpha requirements; Gate 5 is deferred by user. Compilation
and short functional trials close only their stated cells. Keep historical failures
and raw artifacts outside tracked source.


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

## Input isolation prerequisite

`scripts/run-alpha-input-lab.py` defaults to a plan. Apply requires reviewed,
root-owned code and a hash-pinned test binary. The gated child drops privileges and
capabilities; client display/probe/library environment is applied only after that
drop. The parent installs a temporary rule for the child's exact creation-label
PID, assigns a separate `ID_SEAT` and `LIBINPUT_IGNORE_DEVICE=1`, verifies syntax
and reloads rules without triggering existing devices. Active Sony tests verify
input properties before touch injection and service required host probes while
waiting. Neutral scripts do not acquire this requirement.

The leader is observed with `WNOWAIT` and remains reserved through group/device
cleanup. Restoration refuses changed rule identity or remaining devices. A newly
created rule directory is removed only when empty. Regressions cover preparation
failure, timeout, interruption, gate EOF, repeated restoration and output quotas.
The installed scoped helper cannot replace its own root-owned code: administrator
installation is required. Preserve three fixed actions and no broad sudo.

## Acceptance cells and closure paths

| Gate | Work and evidence | Closure criterion |
| --- | --- | --- |
| Audio continuity | Compare C/Rust graph clocks, buffer handoff, channels, callback gaps and marker accounting. Prebuild and avoid competing tests. Correct demonstrated seams with regressions. | Three independent 60-second controls pass, then all 72 product trials and three-family slow-consumer/recovery checks pass without relaxed loss/duration assertions. |
| Provider recovery | Stage exact broker/worker hashes; preserve idle guards, original services and the global lock. Test successful attachment, worker/client death, bounds, pending-journal restart and identity-matched cleanup. | Bounded completion and owned cleanup, or truthful fail-closed rejection with tested operator restoration. Empty restart alone is insufficient. |
| Contacts/outputs | Prove PID isolation first. Exercise Sony HID and associated evdev nodes, both contacts/release, frames, feedback, partial construction, siblings and removal. | Exact observations and cleanup for representable controls/outputs. No contacts reach the shared desktop. Gamepad-only evdev results do not prove companion touch. |
| USB/IP audio/routing | Prepare attachment-identity-bound `ACP_IGNORE` before attach; verify the shared ALSA monitor ignores the lab card and shared defaults stay unchanged. Use direct ALSA/private graph access. | Supported three-family directions, routing transitions, disappearance/reconnect and teardown pass without touching foreign cards/routes/ports/journals. |
| Keyboard/accessibility | Owned display/state; keyboard traversal/activation, viewport selection, lockout, discovery loading/error/retry and output/error display. Regress demonstrated defects. | Interactive receipts plus regressions and owned cleanup. Neutral soak remains separate evidence. |
| Isolated Steam | Fresh data/config/state and display; inspect launcher's storage behavior first. Never copy/modify the real profile. Any login occurs in the disposable session. | Exact-candidate recognition and available input/output pass, with profile/device/process cleanup. No unspecified game compatibility claim. |

## Progress and final checks

The input runner and active-contact gate are committed with Python/Rust regressions.
Administrator installation and live isolation are pending. No active contacts were
injected by this continuation. Audio qualification remains failed; other cells stay
open until their receipts pass. This plan is not an acceptance result.

Run the five required Rust/Gitleaks checks, forced Rust 1.85, strict rustdoc/API/
inventory, Python/corpus/dependency/worker validators, root feature consumers and
exact-Git/offline rebuild. Verify final-head platform/security workflows and 17 SBOM
identities. Keep the PR draft until all six gates have supported dispositions.
Commit validated increments without rewriting history and retain exact revision,
tree, binary and lockfile receipts.

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


### Current execution phase after native refill investigation

Gates 1/4/5/6 remain open. Gate 1 awaits complete owned-display interaction
receipts and a final-image two-hour soak. Gate 4 is in native/mixed audio defect
correction: `538aaa5` passed short native microphone tests for DualSense and DS4,
but Xbox retained 128 measured silent frames. The bounded client-pump catch-up
requires fresh exact-image acceptance. Remaining ownership, routing and failure
cells must also be exercised. Gate 5 awaits the separately scoped input helper
and actual isolated Steam controller acceptance; login visibility is preparation.
Gate 6 remains causal diagnosis before independent qualification and the full
sustained matrix. Do not overlap builds, Steam, GUI soak or audio qualification.


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
