# VirtualGamepad + Controller Protocol Corpus — Agent Start Here

## Current authority and rewrite freedom — 2026-09-05

Development memories and imported conversation/context files are non-authoritative research leads. They cannot establish current implementation, physical truth, a gate pass, or fresh permission. Verify implementation against exact code revisions and merge/content history; verify host/protocol claims with scoped primary sources or experiment artifacts. Current user direction and reviewed decisions govern product scope.

This is early development: breaking APIs, removing obsolete modules, and replacing the internal architecture are acceptable. Preserve evidence and important correctness properties, not historical type names, binary names, crate topology, or dual-runtime compatibility. Git history is sufficient for recovering the old implementation; it need not remain in the shipping build. See [context reassessment](CONTEXT_REASSESSMENT.md), [branch review](BRANCH_REVIEW.md), and [ADR-0002](decisions/ADR-0002-early-development-rewrite.md).


## Revision and authority — 2026-09-04

This revision incorporates the final decisions in “Review And Draft Plan” and a code review of local `main` at `9b466e0`. This is an inspected baseline, not a claim about the latest remote revision or completed host validation. Reconcile the actual checkout before implementation.

`ARCHITECTURE_DECISION_EXPERIMENTS.md` section 17 is the single authority for execution dependencies and exit gates. This file defines its subject's requirements. Examples are illustrative unless explicitly identified as settled contracts. No experiment in this revision is marked passed merely because it is specified.


## Purpose

This file is the top-level handoff for agents working on the paired efforts:

1. a new standalone **controller protocol research corpus**; and
2. the architectural rewrite of **`virtualgamepad`**.

Read the documents in this order:

1. `CONTROLLER_PROTOCOL_CORPUS_AGENT_HANDOFF.md`
2. `ARCHITECTURE_DECISION_EXPERIMENTS.md`
3. `VIRTUALGAMEPAD_ARCHITECTURE_REWRITE_AGENT_HANDOFF.md`
4. `PROTOCOL_CORPUS_SUBMODULE_WORKFLOW.md`

The corpus is intended to become the evidence/protocol source. It remains an **independent Git repository**, but `virtualgamepad` includes it as a **Git submodule** so development and CI always operate against an explicitly pinned corpus commit.

Recommended layout:

```text
virtualgamepad/
├── protocol-corpus/        # Git submodule -> independent controller-protocol-corpus repo
├── crates/
├── docs/
└── ...
```

`virtualgamepad` may consume corpus files at development/build/test/code-generation time, but it must not become a generic runtime YAML/profile interpreter.

---

# 1. Project boundary

`virtualgamepad` is already a standalone library and must remain independent of higher-level slot/routing policy.

It is **not** Gamepad Manager's slot-assignment system.

Its product goal is:

> Create curated virtual controllers/peripherals at selectable concrete host realizations while preserving as much physical identity, protocol behavior, functionality, and host compatibility as that realization can faithfully provide.

The rewrite is driven by architecture cracks discovered as the library matured, not by a need to change that standalone product scope.

---


# 2. Git submodule relationship

Use a normal Git submodule rather than copying/vendoring the corpus into the repository.

Conceptually:

```text
controller-protocol-corpus.git
        │
        │ independent repository/history/releases
        ▼
virtualgamepad/protocol-corpus/
        │
        └── superproject records one exact corpus commit
```

The `virtualgamepad` repository should contain a `.gitmodules` entry and a gitlink at `protocol-corpus/`.

Typical setup:

```bash
git submodule add <controller-protocol-corpus-url> protocol-corpus
git commit -m "Add controller protocol corpus submodule"
```

Fresh development clones should use:

```bash
git clone --recurse-submodules <virtualgamepad-url>
```

or, after an ordinary clone:

```bash
git submodule update --init --recursive
```

To advance the corpus revision used by `virtualgamepad`:

```bash
cd protocol-corpus
git fetch
git checkout <desired-corpus-commit-or-branch>
cd ..
git add protocol-corpus
git commit -m "Update controller protocol corpus revision"
```

Important rules:

- corpus work is committed/pushed in the corpus repository first;
- `virtualgamepad` then updates its submodule pointer in a separate commit/PR;
- CI must initialize the submodule;
- tests/build tooling should fail clearly if the submodule is absent when corpus-backed checks are requested;
- normal runtime library use must not require network access or a Git checkout;
- release/source packaging must either include the pinned corpus snapshot where required for build/test generation or ship already-generated artifacts so downstream consumers are not forced to resolve Git submodules.

This gives agents direct local access to corpus data while preserving independent research history and exact reproducibility.

---

# 3. Central architecture decision already made

The public concept of a realization stays cohesive.

Examples:

```text
linux.uinput
linux.uhid.usb
linux.uhid.bluetooth
linux.dummy_hcd.usb-hid
linux.dummy_hcd.usb-full
future: linux.btvirt.bluetooth
```

Do not expose independently selectable public dimensions such as:

```text
backend = UHID
transport = USB
entry_point = HID
```

Those combinations are constrained enough that independently selectable dimensions create invalid states.

A realization ID represents the complete host-facing path.

---

# 4. Central internal separation already made

Internally, preserve:

```text
controller semantic state
        ↓
stateful transport protocol personality/session
        ↓
concrete realization
        ↓
host mechanism
```

Example:

```text
DualSenseState
    ↓
DualSenseUsbProtocol
    ├── linux.uhid.usb
    └── linux.dummy_hcd.usb-*
```

and:

```text
DualSenseState
    ↓
DualSenseBluetoothProtocol
    ├── linux.uhid.bluetooth
    └── future linux.btvirt.bluetooth
```

The same physical/wire protocol must not be reimplemented once per realization.

---

# 5. Provider rule

> Providers transport/present protocol data. They do not define controller protocol semantics.

No UHID provider logic should know:
- DualSense calibration;
- DS4 feature reports;
- Switch protocol details;
- rumble meaning;
- LED meaning;
- pairing semantics.

No dummy_hcd broker logic should know controller protocol semantics unless a measured timing constraint proves a narrowly bounded exception is necessary.

---

# 6. Protocol session rule

Do not model the controller protocol as only a stateless report encoder.

The protocol session may own:
- report sequence;
- timers;
- timestamps;
- pairing identity;
- calibration state;
- initialization state;
- pending host requests;
- keepalive/watchdog state;
- connection-edge state;
- periodic-report scheduling.

This is required by already observed SC2/Puck behavior and may also simplify DualSense behavior.

Prefer deterministic synchronous methods driven by explicit time until an architecture experiment proves internal async is needed.

---

# 7. Corpus rule

> No protocol claim without provenance.

The corpus must distinguish:
- physical observation;
- controlled experiment;
- vendor documentation;
- kernel source;
- kernel selftest;
- independent implementation;
- compatibility implementation;
- host observation;
- inference.

Compatibility policy must not silently become physical-device truth.

Key historical example:
- OpenPuck compatibility identity and physical SC2/Puck identity differ.
- physical captures contradicted an earlier simplified report model.

---

# 8. Important historical outcomes that must shape implementation

## UHID
- Current UHID provider already supports CREATE2, INPUT2, OUTPUT, GET/SET_REPORT, replies, and lifecycle handling.
- Current defect: it still owns static feature replies.
- Multiple same-family UHID devices have been successfully supported through session-unique `phys`/`uniq`.
- Development memories describe an older `BUS_VIRTUAL` Steam gyro failure, but the archived POC finding commit `3a62825` already sets UHID `bus_type: 0x03`. The bus-cause narrative is conflicted; recover the actual failing run inputs before assigning causality.
- The inspected `9b466e0` DualSense implementation already sets `bus_type: 0x03` (`BUS_USB`) and submits initial neutral input before creation returns. Gate B is a controlled regression/causality comparison, not a presumed new bus-setting fix.
- HHD informs the experiment; it does not prove this implementation passes Steam/SDL acceptance.

## DummyHcd
- Same-host dummy_hcd + ConfigFS is a valid selectable software realization, not only a validation tool.
- It demonstrated successful Steam gyro where historical UHID did not.
- Keep the broker privilege boundary narrow.
- It is the natural path for actual USB topology and composite functions such as USB Audio.

## Runtime
- transactional semantic-state edits are good and should survive;
- failed commits preserve dirty/retryable accepted state;
- close is terminal;
- compound open/rollback/close semantics are useful and should survive.

## Validation
- provider-local fake I/O seams found real defects and should survive;
- one-shot + interactive tests are both useful;
- manual gates caught things automation missed;
- validation evidence must be multidimensional.

---

# 9. Critical uncertainties that must be resolved experimentally

Do not hard-code assumptions about these before running the corresponding gate in `ARCHITECTURE_DECISION_EXPERIMENTS.md`:

1. Which measured bus, protocol, driver, timing, or topology differences explain current and historical DualSense sensor/Steam behavior.
2. Whether multiple UHID devices are useful enough to approximate a physical composite controller.
3. Whether a synchronous protocol-session API is sufficient.
4. Where report-ID/framing ownership belongs.
5. Whether broker roundtrips can keep dummy_hcd controller protocol entirely unprivileged.
6. Whether standard ConfigFS UAC1 reproduces enough DualSense USB Audio topology.
7. Whether UHID + host audio is coherent enough to deserve its own realization.
8. Whether compatibility personalities belong in the curated API.
9. Whether btvirt is viable as a product realization versus validation-only backend.

These are architecture gates, not implementation TODOs to resolve by preference.

---

# 10. Execution entry point

Follow the E0–E6 dependency table in `ARCHITECTURE_DECISION_EXPERIMENTS.md` section 17. It supersedes the earlier R0–R16 sequence and scattered checkpoint lists.

Start with E0: record the baseline SHA and working-tree state; inventory tests and evidence; identify host/hardware prerequisites; establish the plan and evidence locations. Then E1 builds a minimal corpus including the SC2 contradiction seed. E2 settles framing, scheduling, delivery, and replies in memory. E3 measures current host behavior using the existing implementation or the minimum experimental adapter.

E2 and E3 may advance independently once their inputs are identified. Missing physical hardware blocks physical evidence and acceptance claims, not synthetic protocol work. Production restructuring may proceed after the relevant protocol contracts are tested. Retain the old revision and evidence in Git; host acceptance remains a separate claim gate, not a requirement to keep obsolete code active.

E4 delivers a complete DualSense USB/UHID slice with the bidirectional runtime. E5 reuses that personality through dummy_hcd after broker feasibility/startup Gate G. E6 migrates other controllers and separately gates compound/audio/Bluetooth extensions. Audio or btvirt uncertainty must not block independent DS4/Switch work.

## Durable handoff and status

The authoritative handoffs now live together under `docs/architecture-overhaul/`, with a contributor-guide link. The original `.agents/` paths are redirects. These files are prepared for version control but are not committed merely by being present; E0 must verify the reviewed plan commit before remote execution. Use `README.md`, `BASELINE.md`, `GATE_STATUS.md`, and `INITIAL_WORK_QUEUE.md` here for operational context.

Each execution item records owner, prerequisites, code/corpus revisions, commands, artifact location, result (`not_run`, `running`, `passed`, `failed`, `blocked`, or justified `not_applicable`), and ADR. A skipped host test is not a pass.

Preserve usable interactive and scriptable validation surfaces, but choose their organization for the new architecture. Historical `vgpd-demo`/`gr-cli` names and a mandatory two-binary split are not constraints. Inspect the unmerged workflow branch for reusable reporting ideas without restoring its profile/tier runtime.

---

# 11. Architecture benchmark beyond gamepads

Wheels and flight sticks are not current design targets, but they are a required review benchmark.

The architecture passes the benchmark if adding a wheel/HOTAS would primarily require:
- a new typed semantic model;
- controller-specific protocol personality;
- controller-owned evdev presentation;
- selected realizations;

without rewriting core runtime/provider concepts.

Pay particular attention to:
- many axes/buttons;
- relative controls;
- multiple components;
- long-lived force-feedback effect objects;
- displays/LEDs;
- vendor HID;
- non-rumble host output state.

Do not add generic wheel/HOTAS abstractions merely to satisfy the benchmark.

---

# 12. What not to do

Do not:
- restore runtime YAML controller profiles;
- build an arbitrary HID descriptor constructor;
- expose arbitrary VID/PID;
- create automatic realization fallback;
- make providers branch on controller family;
- treat all outputs as rumble;
- assume every commit equals one report;
- assume every controller is one HID device;
- make audio permanently sidecar-only;
- call UHID an actual USB device;
- call multiple UHID devices an actual composite USB device;
- claim validation based only on enumeration.

---

# 13. Required agent behavior around uncertain architecture

If work reaches a gate that is still unresolved:

1. stop expanding production architecture around the assumption;
2. implement only the minimum tooling/prototype required by the experiment;
3. run/document the experiment;
4. write/update the ADR;
5. then continue implementation using the resulting decision.

This is intentional. The rewrite should be evidence-driven.

---

# 14. Handoff documents

- `CONTROLLER_PROTOCOL_CORPUS_AGENT_HANDOFF.md`
  Full corpus mission, evidence model, repository structure, experiment standards, privacy model, tooling, and milestones.

- `ARCHITECTURE_DECISION_EXPERIMENTS.md`
  Gate-by-gate questions, prerequisites, experiments, evidence, and decisions unlocked.

- `VIRTUALGAMEPAD_ARCHITECTURE_REWRITE_AGENT_HANDOFF.md`
  Full target architecture, current-code assessment, provider/runtime/broker/audio design, feature matrix, technical-debt warnings, migration phases, and peripheral benchmark.

These documents plus `PROTOCOL_CORPUS_SUBMODULE_WORKFLOW.md` form the implementation context. The gate register owns sequencing; the corpus handoff owns evidence rules; the rewrite handoff owns runtime/provider contracts; the workflow owns submodule operations.
