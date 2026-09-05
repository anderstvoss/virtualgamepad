# `virtualgamepad` Architecture Decision Experiments — Gate Register

## Current authority and rewrite freedom — 2026-09-05

Development memories and imported conversation/context files are non-authoritative research leads. They cannot establish current implementation, physical truth, a gate pass, or fresh permission. Verify implementation against exact code revisions and merge/content history; verify host/protocol claims with scoped primary sources or experiment artifacts. Current user direction and reviewed decisions govern product scope.

This is early development: breaking APIs, removing obsolete modules, and replacing the internal architecture are acceptable. Preserve evidence and important correctness properties, not historical type names, binary names, crate topology, or dual-runtime compatibility. Git history is sufficient for recovering the old implementation; it need not remain in the shipping build. See [context reassessment](CONTEXT_REASSESSMENT.md), [branch review](BRANCH_REVIEW.md), and [ADR-0002](decisions/ADR-0002-early-development-rewrite.md).


## Revision and authority — 2026-09-04

This revision incorporates the final decisions in “Review And Draft Plan” and a code review of local `main` at `9b466e0`. This is an inspected baseline, not a claim about the latest remote revision or completed host validation. Reconcile the actual checkout before implementation.

`ARCHITECTURE_DECISION_EXPERIMENTS.md` section 17 is the single authority for execution dependencies and exit gates. This file defines its subject's requirements. Examples are illustrative unless explicitly identified as settled contracts. No experiment in this revision is marked passed merely because it is specified.


## 0. Purpose

This document defines the experiments that must resolve important architectural uncertainty before large portions of the rewrite are locked.

Each gate specifies:
- the architectural question;
- why it matters;
- minimum prerequisite rewrite/tooling;
- exact experiment;
- evidence to collect;
- expected outcomes;
- which design decision the result locks.

The intent is to prevent “designing around uncertainty.”

An experiment is not itself the ADR. After the experiment, write/update an ADR:

```text
Question
Evidence
Experiment result
Decision
Consequences
Revisit condition
```

---

# 1. Gate A — Is the protocol corpus evidence model adequate?

## Question
Can the corpus represent physical truth, implementation knowledge, compatibility policy, conflicting hypotheses, and physical validation without ambiguity?

## Minimum prerequisite
No `virtualgamepad` runtime rewrite.

Need:
- independent corpus repository skeleton;
- add it to `virtualgamepad` as the `protocol-corpus/` Git submodule;
- claim schema;
- source schema;
- experiment schema;
- CI that initializes and validates the pinned submodule.

## Test
Seed two cases:

### DualSense
Represent:
- kernel-derived report facts;
- HHD behavior;
- physical USB capture evidence.

### Steam Controller 2 / Puck
Represent:
- OpenPuck compatibility identity;
- physical `28de:1304` capture;
- earlier `0x45` hypothesis;
- physical `0x42` / `0x79` / `0x7b` observations.

## Pass criteria
An agent can answer:
- what is believed;
- why;
- whether physically observed;
- whether implementation-specific;
- whether conflicting;
- what superseded an earlier hypothesis.

## Decision unlocked
Lock:
- corpus schema;
- evidence classes;
- claim states;
- compatibility-vs-hardware separation.

---

# 2. Gate B — Controlled DualSense bus/host regression experiment

## Question and current baseline

Which presentation, protocol, driver, timing, or topology differences explain DualSense sensor and Steam behavior? Local `main` at `9b466e0` already uses `BUS_USB` (`0x03`) and sends initial neutral input before returning from creation. Historical memories about BUS_VIRTUAL are evidence leads, not a diagnosis of current code. Moreover, archived finding commit `3a62825` and archive tip `a5310c6` also set UHID bus metadata to `0x03`. A virtual sysfs path is not proof that the configured bus field was BUS_VIRTUAL. Record this code/narrative conflict and identify the actually tested binary/configuration.

## Minimum prerequisite

E0 baseline/evidence inventory, session-specific host inspection, and the existing DualSense implementation or the smallest adapter that can vary bus metadata. No production protocol/runtime rewrite is required. Label source-derived fixtures honestly if physical capture is unavailable.

## Procedure

1. Record code/corpus SHA, host kernel/configuration, SDL/HIDAPI/Steam versions and settings, firmware/reference unit alias, driver binding, and permissions.
2. Compare UHID BUS_VIRTUAL and BUS_USB with identical descriptor, feature behavior, semantic script, initial report timing, and cadence. Use controlled unique instance identities. Run sequentially and confirm the selected session, not a physical sibling or cached device.
3. Record a physical controller and the retained dummy_hcd implementation as reference conditions where available. Their absence limits the conclusion; do not fabricate reference equivalence.
4. Run a predeclared repetition count, retaining each result. If results vary, report mixed outcomes and investigate before promoting support.
5. Collect sysfs/udev/hidraw/evdev, feature requests and replies, input/output traces, driver binding, SDL discovery/HIDAPI/sensors, Steam discovery/gyro, rumble/LED/touch behavior, and cleanup. Test actual changing sensor values, not only enumeration.

## Outcomes and decision

- BUS_USB alone succeeds reproducibly: record the scoped bus/presentation result and tested host tuple; it may justify the preferred DualSense HID path. Dummy_hcd remains a selectable topology/composite realization.
- Both succeed: the old failure is not reproduced. Preserve that distinction; do not retroactively assign a cause.
- Both fail or results vary: inspect framing, feature state, probe timing, identity, driver binding, and consumer settings. Do not conclude topology dependence without ruling out competing explanations and a valid reference comparison.
- Required environment unavailable: mark affected acceptance axes blocked; allow independent in-memory work.

Gate P shares this apparatus but must vary driver binding separately from bus identity to avoid confounding the comparison. The ADR states tested configurations and revisit conditions, not a universal UHID verdict.

---

# 3. Gate C — How stateful must protocol personalities be?

## Question
Is a deterministic synchronous protocol-session API sufficient, or is internal async/concurrency required?

## Minimum prerequisite
In-memory protocol prototype only.

Need:
- virtual clock;
- fake host;
- protocol action queue.

## Test cases

### DualSense
- semantic input state;
- GET calibration;
- GET firmware;
- output report;
- timestamp progression.

### SC2/Puck-inspired
- periodic status;
- connection edge;
- host-alive watchdog;
- reconnect;
- feature request;
- timed input/report cadence.

## Candidate API

```rust
handle_host_event(event, now)
update_semantic_state(state, now)
poll(now)
```

## Pass criteria
All behavior is deterministic without background threads/tasks inside controller protocol. Gates C/J/O share one harness and ADR family; they are complementary checks, not three opportunities to redesign the executor.

Test action acceptance and delivery separately. Include partial batch success, non-repeatable connection edges, sequence/timestamp advancement and wrap, bounded queue exhaustion, fair servicing of multiple sessions, and uncertain delivery. The prototype must expose next deadlines/readiness and keep a controller serviced while semantic state is unchanged.

Use the runtime contracts in the rewrite handoff section 6.1 as constraints. Select concrete trait signatures only after the harness passes. Runtime scheduling may live outside the personality without making the library dependent on Gamepad Manager.

## Outcomes

### Pass
Decision:
- keep protocol layer synchronous;
- outer runtime owns polling/event loop integration.

### Fail
Document exact behavior impossible to express.
Only then consider async/actor abstractions.

## Decision unlocked
Core protocol/runtime concurrency model.

---

# 4. Gate D — Where does HID report framing belong?

## Question
Does the protocol personality own serialized report ID + payload, or logical report ID + payload with realization-specific framing?

## Minimum prerequisite
- DualSense USB protocol subset;
- fake UHID adapter;
- fake USB-gadget adapter;
- corpus fixtures.

## Test
For:
- neutral;
- Cross;
- triggers;
- touch;
- gyro;
- battery.

Drive same state through both adapters.

Compare exact wire bytes expected by:
- UHID;
- USB HID interrupt endpoint.

## Pass criteria
One invariant clearly explains all differences. Cover numbered and unnumbered input/output/feature reports, report-ID zero semantics, empty/truncated/oversized payloads, GET/SET success and error paths, and control versus interrupt delivery. Compare against independent physical or authoritative ABI fixtures, not only two adapters driven by the same generated expected bytes.

Freeze report-type, logical ID, payload-length, and ID-inclusion conventions before production adapter changes. Preserve information that one transport exposes even if another cannot; unsupported mappings must fail explicitly. Kernel envelopes remain adapter-owned; USB/BT controller-specific framing remains personality-owned.

## Decision unlocked
Final shape of:
- `HidInputReport`;
- `HidOutputReport`;
- GET/SET report contracts;
- cross-realization byte tests.

---

# 5. Gate E — Can multiple UHID devices usefully approximate a composite controller?

## Question
Even though UHID cannot create multiple interfaces under one USB parent, can host software correlate multiple UHID devices well enough for useful compound controller realizations?

## Minimum prerequisite
- compound session;
- pure-ish UHID transport;
- synthetic or SC2/Puck multi-component test personality;
- topology inspection tooling.

## Test

Reference:
```text
physical composite USB controller
```

Approximation:
```text
UHID component A
UHID component B
UHID component C
```

Use controlled:
- names;
- VID/PID;
- phys/uniq relationships.

## Collect
- sysfs;
- udev;
- hidraw;
- evdev;
- SDL grouping;
- Steam grouping;
- controller functionality.

## Outcomes

### Host software associates components correctly
Decision:
- allow honest “compound UHID” realizations;
- metadata must say `actual_bus_topology=false`.

### Host software mis-associates components
Decision:
- controllers requiring shared topology do not expose UHID realization;
- use dummy_hcd/bus-level realization.

## Decision unlocked
Compound realization scope.

---

# 6. Gate F — Is UHID + host audio useful enough to be a realization?

## Question
Should a coordinated UHID controller plus PipeWire/ALSA endpoint be exposed as a first-class realization?

## Minimum prerequisite
- working DualSense UHID;
- minimal virtual playback/capture backend;
- compound lifecycle.

## Test
Create:
```text
DualSense UHID
+ virtual playback
+ virtual capture
```

Test one and two simultaneous controllers.

## Collect
- ALSA enumeration;
- PipeWire naming;
- Steam visibility;
- application visibility;
- hotplug;
- teardown;
- device association;
- multi-controller ambiguity.

## Outcomes

### Coherent/useful
Decision:
- optional first-class `linux.uhid.usb-audio`-style realization.

### Unrelated/ambiguous host devices
Decision:
- keep audio as application-side integration, not controller realization.

## Decision unlocked
Host-audio realization model.

---

# 7. Gate G — Broker startup, control-path capability, and latency

## Question

Can the selected gadget/kernel API and broker protocol support controller-owned dynamic requests, including host probing during creation, without moving mutable controller behavior into privileged code?

## Baseline and minimum prerequisite

At `9b466e0`, the broker services DualSense/DS4 startup features before exposing the session. Its gadget event path exposes GET report IDs and output bytes, not the full UHID request model. A client waiting for creation to return cannot answer a probe that creation itself is waiting on.

Need a small dynamic feature personality, staged broker handshake, private fake I/O, and a provisioned dummy_hcd host for the live portion. Begin capability/startup probes early; do not first complete a large broker rewrite.

## Capability inventory

For the supported kernel and gadget implementation, record whether report type, report ID, requested length, transaction identity, SET data/completion, and negative replies are observable/controllable. Distinguish interrupt output from control SET_REPORT. Test real operations; do not infer parity from matching Rust variants. Record kernel/API requirements and missing capabilities explicitly.

## Startup experiment

Negotiate IPC version and compiled profile identity, reserve a bounded session/event channel, initialize the unprivileged personality, then bind the curated gadget. Service requests while opening. Return a usable controller only after defined readiness or fail with bounded rollback. Test a probe arriving before create completes, concurrent creates, client death at every stage, timeout, stale replies after session reuse, and broker restart.

## Runtime/fault experiment

Exercise GET/SET/output, valid/error responses, malformed/duplicate/late replies, client delay/crash, queue pressure, close while pending, partial setup, and resource cleanup. Measure end-to-end latency and timeout counts under a declared workload. Freeze the acceptable deadline margin, run length, repetition count, and resource limits before collecting pass evidence.

## Outcomes

- Supported transactions, startup, and deadlines pass: move mutable protocol state to the client; broker retains compiled allowlisted construction and bounded transport operations.
- API cannot expose required semantics: document the capability gap and choose a reviewed mechanism/kernel requirement or restrict that realization. Faster IPC is not a remedy.
- Measured startup/timing requires a bootstrap exception: record exact immutable replies, provenance, generation/parity checks, version agreement, scope, and revisit condition in an ADR. Do not introduce a second mutable protocol implementation.
- Host unavailable: fake/startup contract results may pass independently; live capability/latency remains blocked and production replacement is not validated.

Gate G blocks migration of broker protocol ownership, not unrelated UHID or corpus work.

---

# 8. Gate H — Is standard ConfigFS UAC1 sufficient for DualSense audio?

## Question
Can standard gadget UAC1 reproduce enough real DualSense USB Audio topology to be useful/compatible?

## Minimum prerequisite
Corpus must contain:
- physical DualSense audio descriptors/topology;
- sample formats;
- endpoints;
- alt settings.

Need:
- generic USB plan prototype;
- HID function;
- ConfigFS UAC1 function.

## Test
Create:

```text
dummy_hcd
  -> USB gadget
      -> DualSense HID
      -> UAC1
```

Compare real vs virtual:

- `lsusb -v`;
- interfaces/order;
- terminals;
- alternate settings;
- endpoints;
- packet sizes;
- PipeWire/ALSA;
- playback;
- capture;
- usbmon.

## Outcomes

### Close enough + compatible
Decision:
- standard UAC1 backend.

### Functional but topology differs
Decision:
- expose functional audio honestly;
- do not claim exact physical match.

### Compatibility requires exact topology
Decision:
- investigate custom gadget function / lower-level mechanism.

## Decision unlocked
USB audio implementation depth.

---

# 9. Gate I — What differences deserve separate realization IDs?

## Question
Should HID-only vs HID+audio, and host-side companion vs same-bus composite, be separate realization IDs?

## Prerequisite
Results from E/F/H.

## Rule under test
> A materially different host-visible device/function set or lifecycle contract is a distinct realization.

## Candidate examples

```text
linux.uhid.usb
linux.uhid.usb-audio
linux.dummy_hcd.usb-hid
linux.dummy_hcd.usb-full
```

## Decision unlocked
Final realization granularity/naming.

---

# 10. Gate J — Do reply-required protocols need internal async?

## Question
Can the runtime satisfy GET/SET_REPORT deadlines through synchronous host-event dispatch and explicit polling?

## Minimum prerequisite
Protocol session + fake time + fake realization.

## Test
Simulate:
- overlapping host events;
- deadlines;
- WouldBlock;
- delayed application callback;
- close while pending;
- timer-driven reports.

## Pass criteria
No controller-owned background executor required. Each consumed reply-required request has one owner and at most one completion submission; a live supported request completes with success/error within its declared bound. Teardown and unavailable transport cancel it explicitly. Late/duplicate replies cannot revive a closed session.

Test that SET success is sent only after protocol validation/state acceptance; a slow observer never owns the mandatory reply path. Distinguish consumer CLOSE→OPEN from terminal library close. Include startup GET/SET, unknown reports, malformed requests, and output received before a consumer OPEN. Reconcile the repository rule against leaving host requests pending across polling cycles: consume/dispatch and submit an immediate answer or defined error in the same service cycle; transport backpressure retains only the bounded unsent completion, not an unowned application callback.

## Decision unlocked
Same central runtime decision as Gate C, but focused on host request/reply pressure.

---

# 11. Gate K — What may the corpus generate?

## Question
Should corpus data generate implementation logic, or only facts/tests/constants?

## Minimum prerequisite
DualSense corpus seed available through the pinned `protocol-corpus/` submodule + small protocol implementation.

## Prototype A
Handwritten Rust constants.

## Prototype B
Generate:
- report IDs;
- offsets;
- lengths;
- masks;
- descriptors;
- fixture tables.

Keep state machine/packing handwritten.

## Reject
Generic YAML runtime interpreter.

## Expected decision
Corpus generates/verifies facts; Rust implements behavior.

## Decision unlocked
Corpus-to-code pipeline, including which submodule files are read directly by maintainer tooling versus converted to checked-in/generated Rust artifacts.

---

# 12. Gate L — Can Bluetooth semantics be validated independently through UHID?

## Question
Can `linux.uhid.bluetooth` accurately exercise the Bluetooth HID personality without actual Bluetooth pairing?

## Minimum prerequisite
- DualSense Bluetooth protocol personality;
- physical BT corpus fixtures.

## Test
Use:
```text
DualSenseBluetoothProtocol
 -> UHID BUS_BLUETOOTH
```

Compare:
- descriptor;
- report framing;
- CRC;
- sequence;
- GET/SET behavior;
- output;
- Linux driver behavior;
- SDL sensors.

## Decision unlocked
BT protocol/personality architecture.

---

# 13. Gate M — Is btvirt viable as a production realization?

## Question
Can btvirt/BlueZ provide a practical actual virtual Bluetooth-device realization?

## Minimum prerequisite
Validated BT personality from Gate L.

## Test
Attempt:
- discovery;
- pairing;
- HID service;
- connect;
- reconnect;
- input;
- output;
- feature behavior;
- multiple devices.

## Outcomes

### Practical
Decision:
- `linux.btvirt.bluetooth` product realization.

### Primarily test-only / operationally unsuitable
Decision:
- keep as validation backend or investigate alternative.
- do not redesign BT personality.

## Decision unlocked
Actual Bluetooth bus backend.

---

# 14. Gate N — Do curated compatibility personalities belong in the API?

## Question
Should a controller expose reviewed compatibility identities/behavior distinct from physical identity?

## Motivation
SC2/OpenPuck demonstrates that compatibility identity can be intentional and useful.

## Minimum prerequisite
SC2/Puck protocol baseline and identity switching.

## Test
Keep protocol semantics fixed, compare:
- physical identity;
- known compatibility identity.

Across:
- kernel;
- SDL;
- Steam;
- games/tools.

## Outcomes

### Physical identity works
Decision:
- one canonical personality.

### Compatibility identity materially improves support
Decision:
- explicit curated compatibility personality allowed.
- never arbitrary caller VID/PID.

## Decision unlocked
Identity/personality API.

---

# 15. Gate O — Does report cadence belong to protocol session independent of `commit()`?

## Question
Must protocol sessions autonomously emit reports even when semantic state is unchanged?

## Minimum prerequisite
Protocol-session prototype + virtual clock.

## Test
Use:
- DualSense timestamp behavior;
- SC2 periodic status;
- host-alive watchdog;
- unchanged-state periodic reports.

## Expected decision
Yes as a required design capability: the scheduler can emit periodic/lifecycle reports independently of semantic changes. The experiment determines per-personality cadence and advancement rules, not whether the architecture is allowed to support them.

Define the new submission contract explicitly; backward compatibility is not required. Validated desired-state acceptance, queued actions, transport submission, and consumer observation are distinct. Choose names/completion semantics that expose the distinction. Preserve recoverable accepted state and delivery bookkeeping without replaying already accepted edges. Idle-time service must not depend on callers repeatedly submitting unchanged state.

## Decision unlocked
State vs transport scheduling contract.

---

# 16. Gate P — Is specialized kernel driver binding required for expected behavior?

## Question
For identities such as DualSense, is specialized driver binding (e.g. `hid-playstation`) part of the intended UHID realization?

## Minimum prerequisite
Gate B infrastructure.

## Test
Compare:
- specialized driver bound;
- generic HID only.

Observe:
- feature requests at bind;
- sensor exposure;
- FF;
- SDL;
- Steam;
- output initialization.

## Decision unlocked
Realization validation contract and identity strategy.

---

# 17. Authoritative execution dependency table

This table supersedes the earlier R0–R16 phases and checkpoint ordering. E numbers name deliverable batches, not a requirement to serialize independent investigations. Gate results apply only to the recorded controller/personality/realization and environment.

| Batch | Prerequisites | Deliverables and exit criteria | What it blocks |
| --- | --- | --- | --- |
| E0 — Baseline | Actual checkout | SHA/dirty-state record; existing tests and host evidence inventory; plan authority/location; owner and artifacts for each gate; host prerequisites; regression commands and results classified | Unattributed baseline claims; losing recoverable work/evidence |
| E1 — Minimal corpus | E0 inventory | Independent repo and pinned submodule; schema/source/experiment/fixture records; DualSense USB subset plus SC2 contradiction seed; A passes; missing physical evidence stays explicit | Corpus-backed production conformance |
| E2 — Protocol prototype | Identified E1 fixtures; source-derived/synthetic inputs allowed with labels | C/D/J/O deterministic harness and ADRs; bounded delivery/reply/startup contracts; K generation boundary and independent expected fixtures | Freezing protocol types or replacing production I/O |
| E3 — Host baseline | E0; existing code/minimal adapter and recorded inputs | B/P controlled comparison; reproducible per-axis results or explicit blocked/failure status; no assumed Steam gyro fix | Promoting the corresponding UHID/consumer acceptance claims |
| E4 — DualSense USB/UHID slice | E1/E2 pass; E3 results inform support scope, with missing host evidence tracked separately | Shared personality, bidirectional runtime, pure UHID, curated realization IDs; semantic/evdev regressions; dynamic replies; lifecycle/concurrency; host regression comparison | Freezing dependent contracts before deterministic proof; host claims require their own acceptance |
| E5 — Shared dummy_hcd slice | E2 contract; E4 reusable personality; G passes for supported kernel/profile | Staged broker startup; shared protocol; cross-adapter fixtures; fault and live acceptance; no duplicated mutable semantics | Replacing the existing broker-backed path |
| E6 — Controller migration and extensions | Proven common contracts; each feature's own prerequisites | DS4 then Switch conformance; uinput parity; SC2 stress; optional compound/audio/BT gates below | Only the affected family or feature |

E2 and E3 may progress independently. G capability/startup investigation may begin alongside E2; a failing G must not block a valid UHID slice. Preserve a recoverable baseline revision and branch-only evidence, not two active implementations. Crate splitting and API replacement may accompany the rewrite where that produces a cleaner change; keep reviews bounded by coherent behavior.

## Feature-specific dependencies

- E (compound UHID) gates advertising a particular compound presentation, not the existence of controller-owned compound lifecycle support.
- F (host audio) gates a host-audio realization; H gates the selected USB audio implementation. Before H/F, record required endpoint formats, streaming behavior, multi-instance association, tolerable descriptor differences, and tested consumers. “Close enough” must refer to those criteria.
- I names only the host-visible variants established by E/F/H. Basic uinput/UHID/USB-HID IDs need not wait for audio or btvirt naming.
- L validates BT personality over UHID; M depends on L and gates actual BT realization only. BT failure does not prevent USB DS4/Switch migration.
- N requires a relevant physical/compatibility baseline and gates only explicit compatibility variants. Synthetic E experiments cannot validate SC2 hardware fidelity.
- Wheel/HOTAS remains a design review benchmark, not an implementation milestone. Review long-lived FF effect ownership and many-axis/compound support without adding generic peripheral fields.

## Every gate's execution contract

Before a run record owner, code/corpus/source revisions, question, prerequisites, exact commands/procedure, artifact destination, and predeclared acceptance thresholds. Hardware runs record device/firmware alias, kernel/drivers, consumer versions/settings, repetition count, and cleanup checks. Timing runs specify load and deadline margin; streaming runs specify duration and tolerated errors. Do not choose thresholds after seeing results.

Use `not_run`, `running`, `passed`, `failed`, `blocked`, or justified `not_applicable`. Record separate deterministic, kernel, physical, SDL, Steam, audio, and BT evidence axes. A skipped test is never passed. Preserve mixed/negative results and all repetitions. Missing hardware blocks only dependent evidence and production claims, while independent tooling work continues.

A gate exits with an EXP record and ADR describing decision, remaining limitations, and revisit trigger. Each production batch includes automated correctness checks, explicit host limitations, a supported-host reviewer guide, and a review record. Preserve the previous revision in Git; maintaining it in the active build is unnecessary. Old memory-specific empty phase-signoff commits are not prerequisites. No automatic provider fallback is introduced by migration.

---

# 18. ADR template

```markdown
# ADR-XXX — <decision>

## Question
What architectural question was unresolved?

## Context
Why does it matter?

## Experiment
Reference EXP-XXX.

## Evidence
Summarize measured/observed results.

## Decision
State exactly what is now selected.

## Consequences
Positive and negative consequences.

## Rejected alternatives
List alternatives and why rejected.

## Revisit condition
State what new evidence would justify reopening the decision.
```

---

# 19. Experiment record template

```markdown
# EXP-XXX — <experiment>

## Architectural question

## Hypotheses

## Prerequisites

## Build/rewrite state
Exactly what code must exist before running this test.

## Test topology

## Procedure

## Evidence captured

## Pass/fail criteria

## Results

## Interpretation

## ADRs affected
```

---

# 20. Immediate priorities

Execute E0 first, then a minimal E1/A seed. Use that evidence to run E2 (C/D/J/O/K) and the E3 B/P baseline investigation without waiting for audio, complete SC2 reverse engineering, or btvirt. Probe G startup/API feasibility early enough to influence the common request contract.

All gate statuses start `not_run` in this handoff. Historical outcomes remain evidence references until their artifacts and applicability are verified.
