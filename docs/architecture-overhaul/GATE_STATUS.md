# Architecture gate status

PR #106 landed on `main` as `d3a683259778f57bfe60b37168a862f718252a18`.
The active follow-up is [post-106 core refinement](POST_106_PLAN.md). Older execution
batches below are historical; do not resume the merged rewrite branch.

This is the current status ledger. Definitions and dependencies live in [the gate register](ARCHITECTURE_DECISION_EXPERIMENTS.md), section 17. Do not mark a gate passed because its design or test harness exists.

| Gate | Question | Execution batch | Status | Owner | EXP / ADR evidence |
| --- | --- | --- | --- | --- | --- |
| A | Corpus evidence model | E1 | passed | Codex | [EXP-0001](experiments/EXP-0001-corpus-seed.md), [ADR-0003](decisions/ADR-0003-corpus-boundary.md); source/synthetic scope only |
| B | Controlled bus/host comparison | E3 | blocked | Codex | [EXP-0003](experiments/EXP-0003-uhid-migration.md); Linux baseline/bus startup comparison passes; controlled consumer/driver evidence outstanding |
| C | Stateful synchronous protocol contract | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| D | HID framing boundary | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| E | Compound UHID usefulness | E6 compound | not_run | unassigned | None |
| F | Host audio coherence | E6 host audio | not_run | unassigned | None |
| G | Broker capability/startup/latency | Early probe; E5 replacement | blocked | Codex | [EXP-0004](experiments/EXP-0004-gadget-capability.md); source API lacks full control metadata/completion, live profile unprovisioned |
| H | USB Audio implementation depth | E6 USB audio | not_run | unassigned | None |
| I | Realization variant granularity | Affected E6 variants | not_run | unassigned | None |
| J | Required replies and deadlines | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| K | Corpus generation boundary | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| L | BT personality over UHID | E6 BT protocol | not_run | unassigned | None |
| M | Actual BT realization viability | E6 BT bus after L | not_run | unassigned | None |
| N | Curated compatibility variants | Affected E6 family | not_run | unassigned | None |
| O | Autonomous cadence and delivery | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| P | Specialized driver behavior | E3 with B | blocked | Codex | [EXP-0003](experiments/EXP-0003-uhid-migration.md); Linux baseline/bus startup comparison passes; controlled consumer/driver evidence outstanding |

## Post-106 gates

| Gate | Status | Evidence and boundary |
| --- | --- | --- |
| Q | passed | [EXP-0018](experiments/EXP-0018-multi-uhid.md), [ADR-0012](decisions/ADR-0012-multiple-uhid-targets.md): deterministic exact-target/CREATE2 scope only |
| R | passed | [EXP-0019](experiments/EXP-0019-persistent-resources.md), [ADR-0013](decisions/ADR-0013-persistent-resources.md): synthetic memory lifecycle, not file durability |
| S | passed | [EXP-0020](experiments/EXP-0020-internal-topology.md), [ADR-0014](decisions/ADR-0014-protocol-edits.md): synthetic Wii-like topology, not production support |
| T | blocked | [EXP-0021](experiments/EXP-0021-control-exposure.md): representation and scoped Edge mapping/parser research pass; contrasting exposure/remap-only evidence and corpus adoption pending |
| U | blocked | [EXP-0022](experiments/EXP-0022-sdl-differential.md): harness/schema and bounded source-derived backend identification pass; source-claim adoption awaits corpus main. Physical acceptance is separate |
| Production Wii Remote | blocked | Requires explicit later maintainer authorization even after Q/R/S; no package, manifest or demo enablement |

Owner for Q–U: Codex. Prior A–P statuses and limitations are not promoted by
these extensions. Alpha tagging still requires the documented decisions and
readiness review; this PR does not publish or tag a release.

## Update rules

Allowed statuses: `not_run`, `running`, `passed`, `failed`, `blocked`, `not_applicable`. The last requires a scoped justification; it is not a pass. Claim an item before execution and name an owner. Link every completed or blocked run to an EXP record and relevant ADR, including negative/mixed outcomes and missing prerequisites.

Record evidence axes independently in each run. A fake-I/O result can pass while live kernel/Steam acceptance remains blocked. When repeated runs disagree, keep every result and report the gate as unresolved rather than selecting the successful run.

## Execution batches — 2026-09-06

E0 baseline and reviewed kit are committed (`e3bd06f`); source/host inventories explicitly retain unrecovered historical evidence. E1 minimal source/synthetic corpus is published and pinned (`18ca773`). E2 deterministic contracts are tested (`7202732`, corrected by `b2f67b2`).

E4 DualSense USB/UHID and E6 DS4/Switch/standard-HID Xbox UHID migration are implemented with fake-provider regressions; see [EXP-0003](experiments/EXP-0003-uhid-migration.md). A live uinput creation/cleanup test passed. B/P live UHID/consumer acceptance remains blocked; this does not promote the existing research-backed surfaces.

E5 broker replacement and E6 compound/audio/Bluetooth/compatibility extensions remain dependent on their own gates. The current frame runtime remains for uinput and the existing compiled broker path. This ledger does not claim completion of the full architecture roadmap. User direction now requires feature branches and PRs; implementation is on `architecture/protocol-session-rewrite`, PR #106, without a push to remote main.

## Host access batch — 2026-09-06

[EXP-0005](experiments/EXP-0005-host-access.md) records read-only preflight, per-provider
access policy, broker module-loading removal, administrator UDC authorization,
instance ownership recovery, and private SDL probe preparation. Deterministic tests
and live uinput cleanup pass. Basic UHID provisioning subsequently passed; B/P remain blocked on
controlled baseline/rewrite and consumer comparisons; G additionally needs actual reduced-capability execution
and the recorded kernel-interface decision. No gate is promoted by provisioning
code or an SDL build. CAP_SYS_ADMIN removal is not yet validated.

The [local development helper](LOCAL_HOST_HELPER.md) provides a one-time,
UID-scoped installation path for temporary creation access and extensible approved
module/job operations. Its deterministic tests and a fresh installed UHID grant,
unprivileged creation/input/close, restoration, and idempotent restoration pass.
The prior failed lease was already absent on resumption, so special udev-race
recovery remains tested only deterministically. Both device leases are inactive;
the helper ACL is removed, existing group policy remains, and the newly loaded
module is retained. See [EXP-0005](experiments/EXP-0005-host-access.md).

### Next acceptance prerequisites

B/P now need controlled baseline-versus-rewrite runs and separate Linux, SDL,
Steam and physical-reference evidence. Basic creation success does not validate
controller startup probes or consumer compatibility. G still requires reserved
gadget resources, reduced-capability execution and its independent protocol API
decision. Extensions remain gated as recorded above.

### Production DualSense kernel startup

[EXP-0006](experiments/EXP-0006-dualsense-live-startup.md) records three successful
controlled repetitions of production USB/UHID startup, playstation binding, input
children, idle servicing and observed removal. Initial identity-selector failures
and their corrections are retained in the record. B/P remain scoped as blocked:
baseline/bus/driver comparisons and independent consumer evidence are outstanding.

### Baseline/bus comparison

[EXP-0007](experiments/EXP-0007-bus-baseline-comparison.md) records the same
three-repeat USB/virtual-bus matrix on baseline and rewrite. Both revisions bound
USB to playstation and virtual bus to hid-generic, enumerated input/hidraw children,
serviced the semantic script and removed their devices. B/P remain incomplete:
consumer-observed motion/output, independently varied binding and references were
not measured. The historical consumer failure was not tested or assigned a cause.

### DualSense Linux/SDL core

[EXP-0008](experiments/EXP-0008-sdl-core.md) records three strong SDL runs per
baseline/rewrite, standard controls, changing motion/touch, rumble/RGB feedback,
concurrent repeated IDs and failure cleanup. This measured USB configuration
passes core Linux/SDL checks. B/P remain incomplete as broad gates: independent
driver binding, Steam/physical/reference evidence and other family matrices are
not promoted. Identity and probe failure fixes have deterministic regressions.

### Family review and current core exit

The [core acceptance matrix](CORE_ACCEPTANCE.md) is the current family-by-realization
review record. DS4 now has immutable creation-owned pairing identity and passed
three concurrent reused-ID sessions with independent removal.
[EXP-0009](experiments/EXP-0009-family-sdl.md) adds three SDL runs each for DS4,
Switch and standard-HID Xbox, with explicit output limitations. Xbox neutral
acceptance exposed and fixed unsigned-axis/routing errors. Current evdev
acceptance, remaining family concurrency/failure tests and Switch rumble fidelity
remain outstanding. [EXP-0010](experiments/EXP-0010-evdev-feedback.md) closes
the measured conventional FF completion path for all four families;
[ADR-0006](decisions/ADR-0006-conventional-feedback.md) records bounded ownership
and the callback API migration. Full evdev SDL parity remains outstanding.
G stays blocked on both reserved setup and the recorded protocol-interface gap;
no kernel replacement or broad provisioning was attempted.

### Incremental evdev SDL acceptance

[EXP-0011](experiments/EXP-0011-evdev-sdl.md) records three passing standard-control
and conventional-rumble runs each for Switch, Xbox and DualSense. DS4's combined
touch/gamepad node fails SDL discovery on the tested udev. It retains touch and an
explicit surface restriction; a controller-owned compound presentation is the
next implementation prerequisite. Native button/profile fixes have individual
regressions. Full touch, auxiliary-control and physical fidelity are not inferred.

All four rewrite UHID/SDL profiles also pass three repetitions on kernel 6.12.107
in EXP-0011. Baseline comparisons and prior fault injection remain scoped to their
original records; this recheck does not close B/P broadly.

### Compound prerequisites for DS4 evdev

The frame runtime now routes required replies to one owned component without
resending input snapshots and delivers reverse records even if the next read
fails. Deterministic regressions cover every native reply class/status, reused
request IDs, rejected inputs/unknown components, explicit backpressure retry,
partial input retry and independent terminal cleanup. Controller policy still
owns completion/cancellation; this lifecycle helper does not introduce a second
protocol authority. DS4's two-node presentation and its live acceptance remain
pending. Gate E and other realization support levels are unchanged.

### DS4 split-presentation interruption

[EXP-0012](experiments/EXP-0012-ds4-compound-interruption.md) retains the injected
DS4 split-node prototype and lifecycle regressions. One SDL run completed before
a reported display-session crash interrupted acceptance. No test processes or
nodes survived the recovery inventory. Production adoption is blocked on isolated
consumer validation and crash investigation; the prototype is test-only and the
existing DS4 discovery limitation remains. This does not block independent core
work or grant additional host permissions.

### Independent family review and available physical references

All four HID families pass interleaved three-session removal/read-failure tests
with reused application/request IDs and exact replies for surviving sessions.
The demo now requests repaint at an earlier service deadline instead of always
waiting 4 ms. This preserves bounded polling fallback; it does not establish
hard realtime scheduling or service while the GUI thread is blocked.

[Physical validation policy](PHYSICAL_VALIDATION_POLICY.md) records the available
DualSense, Xbox Series and Steam Controller references. Other families are
best-effort; absent physical hardware does not block independent development.
DualSense physical fidelity, H audio topology and L BT fixtures remain separate
acceptance dependencies. No physical or live display tests were run in this batch.

### Demo service independent of repaint

Each controller now has a service worker, including evdev and standard-HID Xbox.
Workers complete polling independently of GUI repaint and publish bounded optional
display snapshots. Fake-controller regressions cover an unconsumed/locked display,
earlier service deadlines without extra motion ticks, output backlog bounds,
worker failure, stop and independent controller removal. Required replies remain
owned by the existing personality/session. No new hardware evidence is claimed;
shared-controller mutex stalls remain a scheduling limitation. Physical reference
and DS4 display-isolation policies remain unchanged.

### Lab GUI and gate resumption review

The demo exposes editable/reused application IDs, per-worker cycle/gap/log-loss
observations, private clipboard lab records and stop-all cleanup. GUI controller
access is nonblocking when the worker holds its mutex; brief UI edit ownership
remains. Socket checks are explicit, not repeated every repaint. Deterministic
checks cover busy editing, ID wrap/reuse and measurement records.

Read-only preflight on 6.12.107 still finds UHID/uinput creation access ready and
consumer access unvalidated; gadget ConfigFS availability, UDC authorization and
broker socket are missing. No host changes were made. [The lab guide](../DEMO_LAB.md)
provides the next controlled experiments for B/P, isolated compound touch, G and
F/H/L/M. No gate is promoted from GUI tooling or socket reachability.

### Source-backed mapping refinement

[EXP-0013](experiments/EXP-0013-mapping-audit.md) corrects missing Sony digital
trigger bits, DS4 contact-release reporting and demo full-range stick conversion.
Printed/spatial face labels support individual-control lab checks. Deterministic
regressions pass; live mapping/fidelity evidence is unchanged, and no cause is
assigned to the prior display-session crash. All existing family limitations and
isolated-touch prerequisites remain.

### Individual controls and corpus CI access

[EXP-0014](experiments/EXP-0014-individual-evdev-mapping.md) records passing
Xbox/Switch evdev individual controls, unrelated-control neutrality and release
checks across three creations each. Probe readiness now precedes input activation.
DS4 touch remains blocked by the unavailable isolated environment; demo mutex
stalls and Gate G remain unresolved. No extension gate is promoted.

Authenticated corpus CI now uses a corpus-only read-only deploy key, preserving
the existing private/fork job boundary. Remote read access and the
authenticated pinned-corpus job both passed (CI run 34268191527, `fd407b3`).
Other build/acceptance gates remain independent.

### Explicit service contract and observer ordering

All four curated handles expose `service`, retaining `poll_output` as an alias.
The demo uses the explicit operation. Evdev required completions precede optional
callbacks for the consumed bounded batch; output eviction is counted. Closed and
failed native sessions no longer advertise polling interest. Deterministic tests
cover the ordering and lifecycle fixes; no live or physical gate is promoted.
The handoff's service naming gap is addressed. Shared demo edit-lock contention,
persistent identity and logical compound failure/association remain next work.

### Demo controller ownership and bounded native editing

The demo worker exclusively owns each controller; the UI retains detached native
state/surface snapshots. A one-slot queue carries at most 64 native edits per
batch. The UI waits for the corresponding applied snapshot before another batch.
No UI callback holds the live controller. Failed edits do not commit partial
state; the worker closes immediately. Full/disconnected queues are visible failures.
Stop is separate and takes priority over queued input at the next cycle.

Deterministic tests cover press/release order, stale snapshots, saturation,
sequence exhaustion, rejected batches, unexpected worker exit, queued-input stop,
locked optional display and independent removal. The former shared-controller
mutex limitation is removed. No live GUI/touch or physical claim follows; those
acceptance boundaries and Gate G remain unchanged. Persistent typed identity and
logical compound failure/association are the next independent architecture work.

[ADR-0007](decisions/ADR-0007-demo-service-ownership.md) records the integration
contract and its explicit input, rejection and timing limits.

### Pinned compatibility references and physical framing

[EXP-0015](experiments/EXP-0015-reference-layouts.md) records published corpus
revision `e65c044`, source-backed OpenPuck DS4 and HHD contact-layout fixtures,
and passing encoder regressions. Negative release/trigger-policy comparisons are
retained. A physical USB DualSense returned expected GET report IDs/lengths and
64 input frames with increasing timestamps. Read-only audio topology is observed,
not an audio gate pass. Trigger modules are broken and stick drift is reported;
no affected input/actuation fidelity is claimed. DS4 has no physical reference.
No Gate G, isolated touch, full B/P, audio or Bluetooth acceptance is promoted.

### Explicit Sony identity restoration

[ADR-0008](decisions/ADR-0008-explicit-sony-identity.md) separates controller-typed
persistent pairing identity from fresh transport/session state for USB/UHID.
All-byte flag validation, unsupported-target rejection, stable/fresh label tests,
neutral recreation and pending-reply cancellation pass deterministically.
Default creation behavior is preserved. Live reconnect/consumer association and
compound identity remain pending; no support level is promoted. Physical DualSense
is unplugged. Steam Controller development is deferred until the overhaul lands;
existing research provenance remains, but no Steam implementation begins in PR #106.


### Required-component failure containment

ADR-0009 makes terminal native compound provider errors close the whole selected
group before returning. Deterministic tests cover both component positions and
input/reply/read operations, cleanup failure, terminal I/O, repeated cleanup,
partial reverse delivery and preserved backpressure retry. This closes the helper
policy gap; it does not pass Gate E or isolated DS4 acceptance. Protocol-level
removal/deadlines still belong to the controller; compound identity/association
and live cleanup evidence remain pending. No host preparation or live test was
performed. DualSense is unplugged and Steam Controller development is deferred.


### Consumer mapping follow-up (EXP-0016)

User testing reports DS4/Switch HID gyro working in Eden's SDL backend, but
Steam-only Sony axis routing and Switch neutral disagreement. Exact consumer
versions/mappings remain needed; no Steam bug or broad B/P pass is inferred.
The Xbox standard-HID consecutive-button descriptor was independently confirmed
wrong and corrected to the existing legacy xpad evdev key profile. Earlier
aggregate HID sweeps did not prove individual mappings. See
[EXP-0016](experiments/EXP-0016-consumer-mapping-disagreement.md) for regression and
consumer retest scope. Gadget acceptance remains blocked by G; Steam Controller
development remains deferred until landing.

The corrected Xbox UHID profile passes 156 exact individual-control/neutral SDL
3.2.0 observations across three creations on Linux 6.12.107 arm64, with exact
selection and cleanup. Eden/Steam retesting remains separate and pending.


### Landing scope and support labels

The [landing and alpha plan](LANDING_AND_ALPHA_PLAN.md) prioritizes current-core
hardening and review in #106. Remaining extension gates retain their blockers and
must pass before dependent features ship; they are not silently required to add
new controllers before the core can land. The [support matrix](../CONTROLLER_SUPPORT.md)
starts every cell at WIP, independently of experiment-level passes. Alpha
versioning and new-controller issues follow landing; no merge or release occurred.

### Native compound association prerequisite

ADR-0010 adds logical/creation/role metadata and prepared UHID/uinput labels.
Deterministic derivation, duplicate-role, every-open-position rollback and
retained-diagnostics tests pass. An ordinary-user uinput run verified its queried
kernel sysname, physical label and removal after repeated cleanup. No host policy
changed. Group protocol deadlines, fair service and production DS4 association
remain pending; Gate E and support cells are not promoted.

The protocol-group portion of ADR-0010 now has fake-clock evidence: every-role
service, overlapping request IDs, CLOSE/OPEN distinction, exact reply deadline,
uncertain delivery, prior observations and retained cleanup failures. It exposes
aggregate deadlines/readiness without introducing a worker. This resolves the
helper-level service owner, not isolated production DS4 association or Gate E.

### Native neutralization and diagnostics

ADR-0011 adds transactional neutralize() to all four handles, preserving metadata
and protocol state, with retryable committed delivery and terminal rejection.
Native/HID cleanup errors are retained and every handle exposes diagnostics.
All-family metadata/control/contact regressions pass. Observation ordering remains
local service order with bounded loss reporting. No live fidelity gate is promoted.

### Individual HID controls, output framing and lab correlation

[EXP-0017](experiments/EXP-0017-individual-hid-and-rumble.md) passes 156 isolated
control/neutral observations per family (624 total) on Linux 6.12.107 arm64 and a
clean pinned SDL 3.2.0 build. Twelve creation cleanups confirmed device removal
and child reaping. Sony/Switch used HIDAPI; Xbox used Linux event input. Initial
Sony Up failures are preserved as an unprimed consumer-probe sequence; polling
neutral before activation corrected the apparatus. Held-input-at-open remains
outside this transition result. No touch, Steam/Eden or physical pass is inferred.

Switch report 0x10 now requires counter plus both four-byte motor words; exact
SET length/completion regressions pass. Typed outputs preserve encoded motors,
with physical compressed-rumble fidelity explicitly unvalidated. Curated handles
now expose requested labels and cached creation-time host observations. The
DS4 test-only prototype exercises creation/role association with reused IDs;
production split-node association still requires isolated acceptance.

### Final core review and demo

The [assessment](ASSESSMENT.md) maps all eight scoped closure items to evidence,
limitations or explicit prerequisites. The final group audit fixes loss of an
observation from a subsequently failing component; uncertain actions are still
never replayed. Group bounds/recoverable-error fairness and acknowledged demo
neutralization now have direct regressions. Lab records include consumer build,
backend/mapping and cached association plus retained cleanup diagnostics.
Interactive GUI, isolated touch/association, Steam/Eden and physical acceptance
remain distinct. Extension gates and all-WIP support labels remain unchanged.

### Final code-review disposition

The [final assessment](ASSESSMENT.md#final-code-review) records the reviewed
boundaries, reproduced/fixed partial-output indicator defect, and new held-input
startup/reopen regression. No scoped core blocker remains identified. PR readiness
requires green final-head checks; it does not promote any gate or support cell.
Interactive GUI/consumer interpretation, isolated DS4 touch/association, physical
fidelity and the original extension prerequisites remain as recorded.
