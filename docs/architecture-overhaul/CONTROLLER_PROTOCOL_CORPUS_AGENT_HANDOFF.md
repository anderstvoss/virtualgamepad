# Controller Protocol Corpus — Agent Handoff and Build Specification

## Current authority and rewrite freedom — 2026-09-05

Development memories and imported conversation/context files are non-authoritative research leads. They cannot establish current implementation, physical truth, a gate pass, or fresh permission. Verify implementation against exact code revisions and merge/content history; verify host/protocol claims with scoped primary sources or experiment artifacts. Current user direction and reviewed decisions govern product scope.

This is early development: breaking APIs, removing obsolete modules, and replacing the internal architecture are acceptable. Preserve evidence and important correctness properties, not historical type names, binary names, crate topology, or dual-runtime compatibility. Git history is sufficient for recovering the old implementation; it need not remain in the shipping build. See [context reassessment](CONTEXT_REASSESSMENT.md), [branch review](BRANCH_REVIEW.md), and [ADR-0002](decisions/ADR-0002-early-development-rewrite.md).


## Revision and authority — 2026-09-04

This revision incorporates the final decisions in “Review And Draft Plan” and a code review of local `main` at `9b466e0`. This is an inspected baseline, not a claim about the latest remote revision or completed host validation. Reconcile the actual checkout before implementation.

`ARCHITECTURE_DECISION_EXPERIMENTS.md` section 17 is the single authority for execution dependencies and exit gates. This file defines its subject's requirements. Examples are illustrative unless explicitly identified as settled contracts. No experiment in this revision is marked passed merely because it is specified.


## 0. Purpose of this document

This document is the implementation handoff for building a new standalone repository whose job is to discover, preserve, validate, and publish controller protocol semantics independently of `virtualgamepad`.

The repository should be treated as a **research corpus + evidence system + experiment framework**, not as a runtime controller emulator.

The corpus remains an **independent GitHub repository**, but the `virtualgamepad` repository should include it as a Git submodule at a stable path such as:

```text
virtualgamepad/protocol-corpus/
```

This relationship is intentional: research remains independently versioned and reusable, while `virtualgamepad` developers and CI have direct access to the exact corpus revision associated with a code revision.

The motivation is direct: during `virtualgamepad` development, implementation knowledge repeatedly became mixed with protocol facts. As the project matured, this produced architectural pressure and some incorrect generalizations. The strongest example is the Steam Controller 2 / Puck work:

- OpenPuck intentionally used a compatibility-oriented identity (`28de:1142`) while the physical Puck capture identified as `28de:1304`.
- Early research focused heavily on report `0x45`, while physical captures from the official Puck / direct-wired controller showed materially different behavior, including heavy `0x42` traffic and lifecycle/status reports such as `0x79` and `0x7b`.
- A Steam-active experiment visibly triggered haptic/audio feedback, but hidraw-only capture could not record the host→device traffic responsible for it.

Those outcomes demonstrate that protocol knowledge must be tracked with provenance, conflicting hypotheses, and physical evidence.

The corpus should become useful to:
- `virtualgamepad`;
- other emulator / virtual-controller projects;
- input-stack developers;
- controller reverse-engineering work;
- validation of kernel / SDL / Steam behavior;
- future USB/Bluetooth realization efforts.

The corpus must remain valuable even if `virtualgamepad` disappeared.

---

# 1. Core mission

The repository mission is:

> Produce evidence-backed, reusable descriptions and validation fixtures for physical game-controller semantics and wire protocols, independent of any particular emulator, operating-system realization backend, or virtual-controller runtime.

The corpus should answer questions such as:

- What USB or Bluetooth identity does the physical device actually expose?
- What is the report descriptor?
- Which report IDs exist?
- What does each byte/bit/field mean?
- Which values are raw device coordinates versus OS-remapped coordinates?
- Which reports are periodic, edge-triggered, or initialization-only?
- Which feature requests are required?
- Which host outputs control rumble, LEDs, triggers, haptics, audio, displays, or modes?
- Which protocol state is per-device, per-connection, or per-transport?
- Which behavior differs between USB and Bluetooth?
- Which behavior is physical-device truth versus host compatibility policy?
- Which facts are physically observed, source-derived, inferred, or conflicting?
- Which controller revisions / firmware revisions differ?

The corpus should make it possible to answer “why is this constant / state transition / report field implemented this way?” without having to rediscover the reasoning from source code history.

---

# 2. Explicit non-goals

The corpus must **not** become:

- a generic HID runtime;
- a generic USB device emulator;
- a virtual gamepad library;
- a runtime profile loader for `virtualgamepad`;
- a user-configurable arbitrary input-injection framework;
- a substitute for reviewed controller-specific Rust implementations;
- a place to copy third-party source code without respecting licenses;
- a dumping ground for unsanitized captures containing private device/account identity.

The corpus may describe arbitrary protocol facts, but `virtualgamepad` must consume only reviewed, curated facts and fixtures.

---


# 3. Repository relationship with `virtualgamepad`

The corpus is an independent repository and must have its own:
- commit history;
- issues;
- pull requests;
- releases/tags if useful;
- CI;
- contributor documentation.

`virtualgamepad` references it through a Git submodule.

Recommended superproject layout:

```text
virtualgamepad/
├── .gitmodules
├── protocol-corpus/       # gitlink/submodule
├── crates/
├── docs/
└── ...
```

The submodule is a **pinned commit relationship**, not a floating dependency.

At any `virtualgamepad` commit, the exact corpus revision used for:
- fixtures;
- generated constants;
- conformance tests;
- claim references;
- protocol research context

must be recoverable from the gitlink itself.

## 3.1 Developer workflow

Initial clone:

```bash
git clone --recurse-submodules <virtualgamepad-url>
```

Existing clone:

```bash
git submodule update --init --recursive
```

Corpus development while working from the `virtualgamepad` checkout:

```bash
cd protocol-corpus
git switch -c <corpus-feature-branch>
# edit/research/commit
git push
```

Then update the superproject pointer:

```bash
cd ..
git add protocol-corpus
git commit -m "Update protocol corpus revision"
```

The corpus commit must exist in the remote corpus repository before a `virtualgamepad` PR points at it.

## 3.2 CI requirements

`virtualgamepad` CI must:
- initialize submodules for corpus-backed tests;
- verify the submodule commit exists and is reachable;
- validate any generated snapshot/constants against the pinned corpus revision;
- clearly distinguish failures caused by missing submodule checkout from protocol/test failures.

Corpus CI remains independent.

## 3.3 Runtime/build packaging rule

The submodule is primarily a **development and verification dependency**.

Do not make ordinary runtime controller creation parse corpus YAML.

For package/release builds:
- prefer checked-in/generated Rust artifacts needed by normal compilation;
- corpus-backed validation/codegen may run in maintainer CI;
- if a build step genuinely requires corpus input, provide a source distribution strategy that does not surprise downstream Cargo users with an implicit Git/submodule requirement.

## 3.4 Why submodule rather than copied snapshot

A submodule provides:
- exact revision pinning;
- no duplication of research history;
- independent reuse by other projects;
- direct local access for agents/tools;
- straightforward cross-repo provenance.

The cost is explicit submodule initialization and two-repository commits when both research and implementation change. Accept this operational cost rather than collapsing the corpus into `virtualgamepad`.

---

# 4. Conceptual separation: four distinct knowledge classes

Every controller package should distinguish at least these four categories.

## 4.1 Physical semantics

Human/device meaning:

- Cross / A / B / X button;
- steering angle;
- throttle;
- gyro X;
- accelerometer Z;
- touch contact 0;
- headset attached;
- battery percentage;
- adaptive-trigger mode;
- wheel force effect;
- controller connection edge.

This is the semantic meaning of the device, independent of how it is serialized.

## 4.2 Wire semantics

How semantics are represented on the wire:

- report ID;
- report length;
- byte offsets;
- bit masks;
- endian/signedness;
- range;
- sequence fields;
- timestamps;
- CRC;
- feature-report IDs;
- output-report flags.

Example:

```text
USB input report 0x01
bytes 16..17 = gyro X, signed little-endian i16
```

## 4.3 Transport framing

Transport-specific packaging:

- USB HID report framing;
- Bluetooth HID report framing;
- BT sequence/tag/CRC;
- control request type;
- USB interface/endpoint;
- Bluetooth service / characteristic / HIDP channel.

Transport framing should not be conflated with the underlying semantic meaning.

## 4.4 Host compatibility behavior

Behavior that belongs to a host implementation or compatibility strategy rather than the physical protocol definition:

- SDL initialization LED values;
- Steam-specific feature probes;
- Linux `hid-playstation` initialization;
- HHD compatibility quirks;
- OpenPuck compatibility identity choice;
- application-specific workarounds.

This category is critical. A workaround should never silently become “the hardware specification.”

---

# 5. Evidence model

Do not use a single scalar confidence score.

Each claim should have:
1. the claim value;
2. applicability;
3. one or more evidence records;
4. a state;
5. conflicts / alternatives if applicable;
6. revision history.

## 5.1 Suggested evidence classes

Use explicit source classes such as:

- `vendor_documentation`
- `physical_capture`
- `controlled_physical_experiment`
- `kernel_source`
- `kernel_selftest`
- `independent_implementation`
- `compatibility_implementation`
- `host_observation`
- `inference`

## 5.2 Suggested claim status values

Recommended:

- `reported`
- `independently_corroborated`
- `physically_observed`
- `physically_validated`
- `inferred`
- `conflicted`
- `rejected`
- `unknown`

Do not imply that “kernel source” is automatically equivalent to “physically validated.” Kernel code is high-value evidence but still an implementation.

## 5.3 Evidence precedence

Do not make precedence absolute, but default reasoning should be:

```text
controlled physical experiment
    > direct physical observation
    > vendor documentation
    > multiple independent implementations
    > single implementation source
    > compatibility implementation
    > inference
```

Exceptions must be documented.

Example: a physical capture may be incomplete, while a kernel driver may intentionally support multiple unseen hardware revisions.

---

## 5.4 Minimum schema contract for Gate A

Version the schemas and require `schema_version` on records. Validate claim IDs, source/experiment references, hypothesis status, applicability, and artifact hashes. Claim/hypothesis states use the vocabulary in section 5.2; lifecycle relationships such as `supersedes` and a rejection scope are separate fields, not ad hoc status values.

Applicability records controller model/revision, firmware when known, transport, and relevant host/compatibility scope. Report fields declare direction/type, logical report ID, whether raw bytes include that ID, offset origin, length, bit numbering, endian/signedness, units, coordinate frame, and valid/reserved values. Unknown fields remain explicit; example byte offsets in this handoff are not promoted facts.

Physical validation requires a linked controlled experiment with artifacts and a demonstrated semantic outcome. Source agreement alone cannot promote a claim to physical validation. Record source lineage so two implementations copied from one origin do not count as independent corroboration. Preserve conflicting hypotheses with applicability and supersession links.

Gate A seeds both DualSense and the SC2/OpenPuck contradiction case immediately. It freezes the minimum usable schema, not an exhaustive ontology of every future peripheral.

# 6. Stable claim IDs

Every promoted fact should have a stable claim ID.

Example:

```yaml
id: DS5-USB-IN-014

statement:
  field: gyro_x
  report_id: 0x01
  offset: 16
  length: 2
  type: i16_le

applies_to:
  controller: sony.dualsense
  transport: usb

evidence:
  - SRC-LINUX-HID-PLAYSTATION
  - SRC-HHD-DUALSENSE
  - EXP-DS5-IMU-006

status: physically_validated
```

`virtualgamepad` code should be able to reference claim IDs in comments/tests.

Do not use claim IDs as runtime dependencies.

---

# 7. Competing hypotheses must be preserved

Reverse engineering is iterative. Do not overwrite history as understanding changes.

Example:

```yaml
id: SC2-PUCK-IDENTITY

hypotheses:
  - value:
      vid: 0x28de
      pid: 0x1142
    evidence:
      - SRC-OPENPUCK
    classification: compatibility_implementation
    status: rejected
    rejection_scope: physical_identity

  - value:
      vid: 0x28de
      pid: 0x1304
    evidence:
      - EXP-SC2-PUCK-USB-20260731
    classification: physical_capture
    status: physically_observed
```

This preserves why an earlier implementation decision existed.

---

# 8. Hardware / firmware variant tracking

Every experiment must record at least:

- controller family;
- marketing model;
- hardware revision if visible;
- firmware revision if retrievable;
- serial or unit identifier in local experiment metadata;
- transport;
- cable/dongle/adapter if relevant;
- host OS/kernel;
- host software versions;
- driver binding.

Do not assume all devices sold under one product name are identical.

Suggested package layout can begin common-first, then split revisions only when evidence requires it.

---

# 9. Proposed repository structure

```text
controller-protocol-corpus/
│
├── README.md
├── LICENSE
├── CONTRIBUTING.md
│
├── schemas/
│   ├── controller.schema.json
│   ├── source.schema.json
│   ├── claim.schema.json
│   ├── experiment.schema.json
│   ├── report.schema.json
│   ├── compatibility.schema.json
│   └── fixture.schema.json
│
├── controllers/
│   ├── sony/
│   │   ├── dualsense/
│   │   │   ├── controller.yaml
│   │   │   ├── semantics/
│   │   │   │   ├── controls.yaml
│   │   │   │   ├── motion.yaml
│   │   │   │   ├── touch.yaml
│   │   │   │   ├── battery.yaml
│   │   │   │   ├── outputs.yaml
│   │   │   │   └── audio.yaml
│   │   │   ├── protocols/
│   │   │   │   ├── usb/
│   │   │   │   │   ├── identity.yaml
│   │   │   │   │   ├── usb-device.yaml
│   │   │   │   │   ├── hid-descriptor.bin
│   │   │   │   │   ├── input-reports.yaml
│   │   │   │   │   ├── output-reports.yaml
│   │   │   │   │   ├── feature-reports.yaml
│   │   │   │   │   └── audio-uac.yaml
│   │   │   │   └── bluetooth/
│   │   │   │       ├── identity.yaml
│   │   │   │       ├── hid-descriptor.bin
│   │   │   │       ├── framing.yaml
│   │   │   │       ├── input-reports.yaml
│   │   │   │       ├── output-reports.yaml
│   │   │   │       └── feature-reports.yaml
│   │   │   ├── compatibility/
│   │   │   │   ├── linux-hid-playstation.yaml
│   │   │   │   ├── sdl.yaml
│   │   │   │   ├── steam.yaml
│   │   │   │   ├── hhd.yaml
│   │   │   │   └── openpuck.yaml
│   │   │   └── fixtures/
│   │   │       ├── neutral/
│   │   │       ├── controls/
│   │   │       ├── motion/
│   │   │       ├── touch/
│   │   │       ├── feature/
│   │   │       ├── output/
│   │   │       └── initialization/
│   │   └── dualshock4/
│   │
│   ├── nintendo/
│   │   └── switch-pro/
│   │
│   └── valve/
│       └── steam-controller-2/
│           ├── wired/
│           ├── puck/
│           └── compatibility/
│
├── evidence/
│   ├── sources/
│   ├── experiments/
│   ├── sanitized-captures/
│   └── manifests/
│
├── tools/
│   ├── capture-usb/
│   ├── capture-bluetooth/
│   ├── hid-feature/
│   ├── report-diff/
│   ├── trace-render/
│   ├── sanitize/
│   └── validate/
│
└── docs/
    ├── RESEARCH_METHOD.md
    ├── EVIDENCE_MODEL.md
    ├── PRIVACY_AND_CAPTURE_POLICY.md
    ├── CLAIM_LIFECYCLE.md
    └── CONTRIBUTOR_EXPERIMENT_GUIDE.md
```

The exact names can evolve, but preserve the separation:
- semantics;
- protocol;
- compatibility;
- evidence;
- fixtures;
- tools.

---

# 10. Source registry

Every software/document source must be pinned to an exact version or commit.

Example:

```yaml
id: SRC-HHD-DUALSENSE
type: compatibility_implementation

repository: hhd-dev/hhd
commit: <exact-sha>

paths:
  - src/hhd/controller/lib/uhid.py
  - src/hhd/controller/virtual/dualsense/__init__.py
  - src/hhd/controller/virtual/dualsense/const.py

license:
  identifier: <exact SPDX identifier or LicenseRef>
  source_path: <license location at pinned revision>
  artifact_reuse: <reference-only / permitted-copy / derived-data / unresolved>
```

Important source families:

## Linux
- `drivers/hid/hid-playstation.c`
- `drivers/hid/hid-nintendo.c`
- relevant HID transport code;
- Linux HID selftests;
- evdev / force-feedback interfaces.

## HHD
Especially:
- UHID implementation;
- DualSense virtual personality;
- controller-specific compatibility handling.

Key HHD architectural lesson:
- UHID is transport;
- controller personality owns descriptor, report encoding, feature replies, output decoding, USB-vs-BT semantics.

## OpenPuck
Use as:
- protocol hypothesis source;
- implementation reference;
- compatibility-policy evidence.

Do not assume its identities or behavior are physical-device truth without capture evidence.

## BlueZ
Use for:
- BT transport behavior;
- btvirt;
- testing infrastructure;
- pairing / HID service behavior.

## SDL / HIDAPI
Use for:
- host-observed compatibility;
- sensor discovery behavior;
- initialization behavior.

## Steam
Usually observed experimentally rather than source-derived.

## Existing `virtualgamepad`
Treat current implementation and its development captures as one evidence source, not as authoritative truth.

---

# 11. Physical experiment standard

Every experiment directory should contain a machine-readable manifest.

Example:

```yaml
experiment: EXP-DS5-USB-BUTTONS-001

device:
  family: sony.dualsense
  model: CFI-ZCT1W
  hardware_revision: unknown
  firmware: <captured if available>
  local_unit_id: lab-ds5-01

host:
  os: linux
  kernel: <version>
  architecture: x86_64

transport:
  type: usb

host_software:
  steam: <version or not-running>
  sdl: <version>
  hidapi: <version>

procedure:
  - hold neutral for 5 seconds
  - press Cross 20 times
  - release fully between presses

artifacts:
  - neutral.hidraw
  - cross.hidraw
  - usbmon.pcapng

question:
  identify Cross input field and edge behavior
```

Experiments should be reproducible by another contributor.

---

# 12. USB capture standard

A serious USB protocol experiment should use more than hidraw.

Preferred capture set:

- `lsusb -v`;
- `/sys/bus/usb` topology;
- HID report descriptor;
- hidraw input capture;
- usbmon;
- pcapng when possible;
- udev properties;
- driver binding;
- feature GET/SET requests;
- host OUTPUT traffic.

Reason:
hidraw alone can miss control traffic and host→device behavior.

The Steam Controller 2 Steam-active experiment proved this: visible haptic/audio behavior occurred without the responsible host output being present in the hidraw-only capture.

---

# 13. Bluetooth capture standard

Preferred:

- `btmon`;
- HCI/btsnoop capture where available;
- SDP / GATT / HID service metadata;
- HID descriptor;
- raw input/output;
- pairing trace;
- reconnect trace;
- host driver binding;
- application state.

Record whether behavior was:
- actual physical Bluetooth;
- UHID `BUS_BLUETOOTH` only;
- btvirt;
- other emulator.

Those are not equivalent.

---

# 14. Standard controller experiment suite

Each controller should progress through the following categories.

## 14.1 Identity
Capture:
- VID/PID;
- strings;
- versions;
- USB device/config descriptors;
- HID descriptors;
- BT identity/services;
- firmware reports.

## 14.2 Digital controls
For each control:
- neutral;
- press;
- hold;
- release;
- repeat.

Generate automatic byte delta analysis.

## 14.3 Analog controls
For each axis:
- neutral;
- min;
- max;
- intermediate points;
- direction/polarity;
- signedness;
- endian;
- raw range.

## 14.4 Touch
Capture:
- corners;
- center;
- contact add/remove;
- multiple contacts;
- contact ID reuse;
- touch-button interaction.

## 14.5 Motion
Use controlled orientations:

```text
stationary +X
stationary -X
stationary +Y
stationary -Y
stationary +Z
stationary -Z
```

Then controlled rotation about each axis.

Distinguish:
- raw sensor coordinates;
- physical coordinate convention;
- Linux coordinate convention;
- SDL coordinate convention.

## 14.6 Feature reports
Capture:
- request ID;
- requested length;
- returned length;
- raw bytes;
- timing;
- transport;
- repeated-request behavior;
- state dependence.

## 14.7 Outputs
For:
- rumble;
- LED;
- player LEDs;
- adaptive triggers;
- rich haptics;
- force feedback;
- audio-control bits;
- displays;
- power/mode controls.

Record:
- host command;
- wire bytes;
- physical observed effect.

## 14.8 Lifecycle
Capture:
- connect;
- first reports;
- initialization;
- disconnect;
- reconnect;
- sleep;
- wake;
- pairing;
- keepalive;
- host-alive watchdogs;
- periodic status.

This category is essential for stateful protocols such as SC2/Puck.

---

# 15. Audio research

For controllers such as DualSense, audio must be researched separately from HID audio-control bits.

Track:
- USB Audio Class version;
- AudioControl interface;
- AudioStreaming interfaces;
- terminal graph;
- alternate settings;
- endpoint addresses;
- direction;
- packet sizes;
- interval;
- sample format;
- sample rate;
- channel layout.

Distinguish:
- HID-side audio controls;
- actual PCM transport;
- host-side compatibility audio sidecars.

The corpus should enable later descriptor diffing between:
- physical DualSense;
- `dummy_hcd` UAC implementation.

---

# 16. Trace renderer

Build a human-readable trace diff tool early.

Example output:

```text
Report 0x01
  byte 8:
    0x00 -> 0x20
  semantic:
    Cross: released -> pressed
```

Motion example:

```text
gyro_x:
  raw: 12 -> 16120
  interpreted: +X rotation
```

Feature example:

```text
GET_FEATURE 0x05
  response length: 41
  fields:
    gyro calibration ...
```

The corpus should not require reading hexdumps to inspect routine experiments.

---

# 17. Privacy and capture policy

Raw captures may contain:
- MAC addresses;
- serials;
- pairing identities;
- per-unit calibration;
- account/device-linked Steam data;
- other unique identifiers.

Therefore separate:

```text
tracked repository evidence
```

from:

```text
local restricted raw captures
```

A tracked experiment may contain:

```yaml
raw_artifact:
  sha256: ...
  tracked: false
  reason: device_identity
```

Provide a sanitization tool and policy.

Never silently upload raw captures.

## 17.1 Fixture lineage and transformations

Every fixture declares `physical_capture`, `derived`, or `synthetic` origin. Keep private raw input outside Git; tracked manifests identify its hash and a non-sensitive local alias without exposing private paths. Derived fixtures record input/output hashes, tool revision, transformation parameters, identifier substitutions, and any length/CRC/checksum recalculation. Verify that sanitization preserves the property under test; if it cannot, mark the fixture unsuitable for that claim.

Retain independently captured golden cases for important protocol behavior. An encoder and its expected bytes generated from the same claim only test consistency, not correctness of that claim. Synthetic SC2 timing fixtures test runtime mechanics and must not be labeled physical SC2 acceptance.

## 17.2 Source/artifact ingestion

Record source revision and artifact-level origin/reuse terms before copying descriptors, snippets, or captures. Keep unresolved material reference-only until reviewed. Maintain source attribution through transformations and generated outputs. Select and document repository/tool/data licensing during corpus bootstrap; a license placeholder is not permission to copy source code.

---

# 18. Corpus-to-code boundary

Do not recreate the old runtime-profile architecture.

Preferred model:

```text
corpus facts
    ↓
generated constants / fixtures / validation data
    ↓
reviewed compiled Rust behavior
```

Good generation candidates:
- report IDs;
- offsets;
- masks;
- descriptor bytes;
- lengths;
- static enumerations;
- fixture metadata;
- claim-reference tables.

Keep handwritten:
- state machines;
- packing/unpacking behavior;
- timing;
- validation;
- host interaction logic;
- controller-specific semantic types.

Rule:

> Corpus may generate or verify facts; reviewed controller code implements behavior.

`virtualgamepad` must not load corpus YAML at runtime.

---

# 19. First target: DualSense USB

Bootstrap the corpus with **DualSense USB**.

Minimum source sweep:
1. Linux `hid-playstation`;
2. Linux HID selftests;
3. HHD;
4. OpenPuck;
5. SDL/HIDAPI;
6. existing `virtualgamepad`;
7. physical DualSense captures.

Initial deliverables:
- physical USB identity;
- HID descriptor;
- USB device/config topology;
- input report layout;
- output report layout;
- feature reports;
- touch semantics;
- motion semantics;
- battery;
- timestamps;
- pairing/identity reports;
- firmware reports;
- HID audio-control bits;
- USB Audio topology;
- initialization behavior;
- reference fixtures.

Do not promote any implementation-derived value to `physically_validated` without a matching experiment.

---

# 20. Second architecture stress target: Steam Controller 2 / Puck

SC2/Puck should not be the first corpus target, but it is an important stress test.

Seed known development outcomes:
- physical Puck PID observed as `28de:1304`;
- OpenPuck compatibility identity may differ intentionally;
- report assumptions based on `0x45` alone were incomplete;
- physical traffic included `0x42`, `0x79`, `0x7b`;
- lifecycle semantics are stateful;
- host-output capture is required;
- some captures may contain identity/account-linked data.

Use SC2 to validate that the corpus can represent:
- conflicting hypotheses;
- compatibility personalities;
- periodic reports;
- lifecycle edges;
- host-alive state;
- multiple interfaces;
- physical versus compatibility identity.

---

# 21. Corpus validation tooling

CI should validate:

- all YAML/JSON against schemas;
- all claim references resolve;
- all source references resolve;
- status/evidence combinations are valid;
- no duplicate claim IDs;
- all binary fixture hashes match manifests;
- generated human-readable indexes are up-to-date;
- no accidental local raw capture paths;
- no unsupported absolute local paths in docs;
- no untracked generated drift;
- fixture lineage, schema versions, and evidence promotions satisfy section 5.4 and 17.1;
- implementation claim links and generated artifacts record the pinned corpus and generator revisions. Independent corpus CI must remain runnable without a virtualgamepad checkout.

Optional later:
- derive Rust constants;
- compare against `virtualgamepad` snapshot;
- auto-build report maps.

---

# 22. Contributor workflow

Recommended workflow for a new protocol fact:

```text
1. Ask a narrow question.
2. Register source/experiment.
3. Capture evidence.
4. Add or update claim.
5. Mark evidence class/status.
6. Add fixture if useful.
7. Render human-readable diff.
8. Add conflict if sources disagree.
9. Review.
10. Promote claim status only when justified.
```

Do not start from “we need byte X to be value Y.” Start from the behavior/question.

---

# 23. Repository maturity milestones

These corpus milestones describe deliverables under E1/E2 and later evidence work; they are not a separate mandatory serial schedule. Basic capture/sanitization tools precede any capture publication.

## Milestone C0 — Repository skeleton
- schemas;
- source registry;
- experiment schema;
- claim schema;
- privacy policy;
- CI;
- minimal DualSense and SC2 contradiction records for Gate A;
- initial sanitizer/manifest validation before tracked capture ingestion.

## Milestone C1 — DualSense research import
- Linux;
- HHD;
- OpenPuck;
- current `virtualgamepad`.

No physical-validation claims yet unless backed by existing captures.

## Milestone C2 — DualSense physical USB baseline
- descriptor;
- input;
- feature;
- output;
- motion;
- touch;
- audio topology.

## Milestone C3 — Tooling
- USB capture helper;
- feature-query tool;
- report diff;
- trace renderer;
- sanitizer.

## Milestone C4 — `virtualgamepad` synchronization
- versioned corpus commit/release;
- test fixture snapshot;
- claim-reference export.

## Milestone C5 — DS4 / Switch
Repeat process.

## Milestone C6 — Full SC2/Puck research expansion
Expand physical stateful/compound/output research. The minimum conflicting-evidence model was already exercised in C0/Gate A; do not postpone that architecture check until C6.

---

# 24. Definition of done for the initial corpus phase

Two readiness levels apply. Gate A/minimal E1 readiness requires valid schemas, traceable source-derived claims, labeled fixtures, and the contradiction seed; this permits deterministic prototypes. Production/reference readiness for a particular personality requires the following, scoped to the features it actually claims:

- DualSense USB has stable claim IDs for the protocol facts needed by the new personality;
- important claims identify source class and status;
- physical captures validate the input, output, feature, touch, motion, and topology behavior being promoted; missing evidence blocks those promotions rather than unrelated tooling;
- USB Audio capture/streaming completeness is required for audio claims, not the initial HID-only personality;
- raw capture privacy policy is enforced;
- report fixtures can be consumed by tests;
- the trace renderer produces understandable output;
- no runtime dependency on the corpus is required;
- an agent can answer “where did this byte/state transition come from?” by following claim provenance.

---

# 25. Architectural principle to enforce

> No protocol claim without provenance. Physical observation, source-derived knowledge, inference, and host-specific compatibility behavior must remain distinguishable.

This is the repository's most important rule.
