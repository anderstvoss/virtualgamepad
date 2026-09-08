# Architecture gate status

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
