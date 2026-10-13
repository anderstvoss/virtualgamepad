# Strategy for the five open alpha gates

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

This review preserves the complete criteria in [the gate audit](ALPHA_GATE_CRITERIA.md)
and the approved [six-gate plan](ALPHA_VM_SIX_GATE_PLAN.md). Gates 1, 4, 5 and 6
remain open. Gate 2 is reopened by Xbox standard-HID identity mismatch #148. Its historical reclosure was at `44bf06e` after native contact/feedback/consumer
close/removal/sibling acceptance; its historical reopened boundary is retained
in the output ledger. Gate 3 retains its pinned seven-phase acceptance at
`b817f48`; revalidate affected cells after relevant executable changes. Physical
comparisons and native timing
remain post-alpha issues, but VM audio continuity is still mandatory.

## Verify the verifier before another live claim

Pin source, tree, actual executable hashes and the scenario selector before
launch. Require scenario-specific output and complete cell inventory after exit.
A process returning zero proves neither that the intended binary was selected
nor that a requested optional branch ran. Retrieve root receipts independently,
check their scope, and verify owned cleanup and original service restoration.

A bounded run prepared from `a040f7e` selected an older observer image. It exited
successfully, but produced no new event-level timer receipt. It is excluded from
timer acceptance; raw receipts and a separate invalid-evidence audit are retained.
The next attempt must freeze the observer named by the current Cargo output and
require exactly one validated 2,000-record ledger matching the prebuild receipt.

## Gate 1: finish interaction before endurance

Use the control inventory as a finite checklist. Preserve deterministic event,
focus, release, Hold, lockout and removal regressions. Exercise every remaining
interactive control in an owned display with adjacent focus targets and more
controllers than the viewport. Inspect visible labels/focus/loading/errors and
read-only output presentation. Confirm window close and final cleanup.

Active Sony touch must be isolated for the GUI's own process, not just a separate
input producer. The installed two-action Steam fixture does not provide that
isolation. Prepare and validate a separate bounded GUI-owned isolation scope
before active touch trials; do not repurpose `steam-hid` or `steam-evdev` or broaden
their updater implicitly. Any new privileged permission needs administrator
provisioning. Record the existing native assistive-technology limitation honestly.

Fix only reproduced defects, then rebuild the final GUI once. Run its separate
two-hour neutral creation/removal soak after the interaction audit, without
Steam, compilation or audio trials. Reuse an earlier soak only if its accepted
GUI image is byte-identical. Close the gate only when every inventory cell and
final owned cleanup have evidence.

## Gate 2: correct Xbox identity and preserve Sony evidence

Fix and revalidate the Xbox identity/codec mismatch in issue #148; no whole-gate pass remains. Native contact/feedback/consumer-close/removal/sibling cells passed at `44bf06e`.
The output ledger's reclosure audit records the complete dispositions and pinned
images. No production source or lockfile changes follow the accepted native
classification candidate. Preserve the historical failed classifier/consumer
receipts; reopen affected cells after relevant production or image changes.
Full Steam compatibility and GUI-owned interaction remain their separate gates.

## Gate 4: separate USB functionality from timing

Preserve passed direct-ALSA, output, teardown and demo route-disappearance cells
when their images remain applicable. Complete the exact three-family, supported
ownership/direction inventory, including USB route A/B/disconnected/reconnected,
endpoint and client disappearance, required-worker failure, HID/audio isolation
and conflicting-open rejection. Use serial/VHCI ancestry and ACP_IGNORE isolation
before attachment and verify shared defaults and foreign resources are untouched.

The latest short trials still contain microphone silence and one native DS4
playback loss. Classify each loss using generated/submitted/host/queue/client/drain
counters before changing refill policy. Producer wake delays are evidence, not
proof of a library defect. Add deterministic regressions for demonstrated defects;
do not expand queues or relax marker checks to obtain a pass. Coordinate the
shared timing investigation with Gate 6. Short functional passes cannot close
sustained audio acceptance.

## Gate 5: execute the consumer matrix

The installed HID/evdev fixtures and revocable updater remove the repeated
administrator-install bottleneck for those two existing scenarios. The unchanged
packet update was accepted. This is deployment evidence, not Steam acceptance.

Reuse only the user-authorized retained private test profile. Recheck private
home/getent mapping, display, bus and device isolation with sentinels before
launch; record ARM64/FEX scope and use host-sized memory limits with reserve and
pressure monitoring. Run all four families through available HID and evdev
realizations, checking recognition, actual inputs, available outputs, selection,
removal and sibling survival. Complete supported USB consumer cells through their
separate attachment isolation. Use direct transport evidence for implemented
features Steam cannot express, with explicit scope and limitations.

Current classic input evidence covers South press/release for all four named
families, with normal Steam exit and complete cleanup in the Xbox-first trial.
Classic generic controller type and zero vendor/product remain explicit
limitations; that evidence cannot close evdev ABI, exact-family identity,
outputs or the remaining realization/lifecycle matrix. The producer's scheduled
single-controller removal is at 600 seconds, so its consumer trial uses a
700-second limit inside the unchanged 900-second fixture limit. Do not run both
sides with a 600-second limit and infer a removal pass from fixture output alone.

Keep a per-cell receipt and terminate/reap only owned Steam/FEX and fixture
processes. A visible home screen or detected controller name cannot replace
input/output and lifecycle checks. Do not claim unspecified game compatibility.

## Gate 6: bounded causal diagnosis, then qualification

Use the maintained prebuilt timer's deadline/wake/raw-clock/thread-CPU ledger to
classify delay outside PipeWire. Correlate late ranges with graph source/capture
ledgers, production timing and effective thread scheduling. This can distinguish
thread nonexecution from clock movement; it cannot by itself assign a host or
hypervisor cause.

Run the approved bounded discriminators, changing one variable at a time:
Rust versus C topology, ordinary versus the existing temporary scheduling grant,
and supported quanta 128/256/512. At most two short repetitions per experiment;
repeat only when new evidence changes the hypothesis. Prebuild everything and
exclude compilation, Steam, GUI soak and competing audio. Require event-level
submission/capture accounting, rather than stdin writes or aggregate totals.

Correct demonstrated harness/library defects with regressions. If guest controls
still miss deadlines independently of the product, retain that failure and identify
the next host/VM prerequisite; do not manufacture closure or defer this gate with
the separate native-timing issue.

Only after three independent 60-second measured trials of each required control
pass complete production, exact marker accounting and applicable rate/latency
limits, run all 72 PipeWire product trials, supported sustained USB/IP cells and
slow-consumer/routing/reconnect/failure/teardown checks. Deliberate stalls require
bounded counted loss, observable discontinuity and recovery.

## Execution order and stop rules

1. Verify installed input update access and repair executable/scenario evidence.
2. Finish non-audio interaction/Steam cells without competing live audio.
3. Run the final GUI soak separately, once the GUI is settled.
4. Run bounded timing diagnostics and finish outstanding USB functional cells.
5. Qualify controls, then run sustained matrices. Revalidate affected accepted
   cells and final source/MSRV/API/consumer/CI/SBOM evidence.

Do not mark a whole gate passed from a subset, compilation, elapsed effort or an
estimated completion percentage. Keep PR #133 draft until mandatory gates have
supported dispositions. Stop for user intervention only where an actual privilege,
host change or login prerequisite prevents further safe independent progress.
