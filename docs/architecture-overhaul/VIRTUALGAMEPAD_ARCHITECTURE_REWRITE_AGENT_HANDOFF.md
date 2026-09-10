# `virtualgamepad` Architecture Rewrite — Agent Handoff and Implementation Specification

## Current authority and rewrite freedom — 2026-09-05

Development memories and imported conversation/context files are non-authoritative research leads. They cannot establish current implementation, physical truth, a gate pass, or fresh permission. Verify implementation against exact code revisions and merge/content history; verify host/protocol claims with scoped primary sources or experiment artifacts. Current user direction and reviewed decisions govern product scope.

This is early development: breaking APIs, removing obsolete modules, and replacing the internal architecture are acceptable. Preserve evidence and important correctness properties, not historical type names, binary names, crate topology, or dual-runtime compatibility. Git history is sufficient for recovering the old implementation; it need not remain in the shipping build. See [context reassessment](CONTEXT_REASSESSMENT.md), [branch review](BRANCH_REVIEW.md), and [ADR-0002](decisions/ADR-0002-early-development-rewrite.md).


## Revision and authority — 2026-09-04

This revision incorporates the final decisions in “Review And Draft Plan” and a code review of local `main` at `9b466e0`. This is an inspected baseline, not a claim about the latest remote revision or completed host validation. Reconcile the actual checkout before implementation.

`ARCHITECTURE_DECISION_EXPERIMENTS.md` section 17 is the single authority for execution dependencies and exit gates. This file defines its subject's requirements. Examples are illustrative unless explicitly identified as settled contracts. No experiment in this revision is marked passed merely because it is specified.


## 0. Purpose

This document is the implementation handoff for the architectural rewrite of `virtualgamepad`.

The current project was already developed as a standalone Rust library, independently of any higher-level Gamepad Manager slot/routing system. That standalone product scope is correct and must be preserved.

The reason for the rewrite is not a mistaken product boundary. The project matured enough to expose architectural cracks at the boundary between:

- controller-native semantic state;
- controller wire/protocol semantics;
- realization-specific host transport;
- reverse host transactions;
- compound device behavior;
- privilege boundaries;
- validation evidence.

The goal is to preserve the strongest parts of the current implementation while rebuilding the central protocol/realization seam.

---

# 1. Product definition

`virtualgamepad` should be:

> A standalone Rust library for creating curated virtual controllers/peripherals at selectable concrete host realizations while preserving as much physical identity, protocol behavior, controller functionality, and host compatibility as the selected realization mechanism can faithfully provide.

The library is not a slot manager and should not contain:
- player-slot assignment;
- physical-controller routing;
- Gamepad Manager hotplug policy;
- mapping graph policy;
- user-configurable arbitrary HID descriptors;
- arbitrary keyboard/mouse injection;
- generic arbitrary USB device construction;
- runtime YAML controller profiles.

---

# 2. Design philosophy

## 2.1 One controller API, multiple concrete realizations

A controller has one controller-native semantic model.

Examples of realizations:

```text
linux.uinput
linux.uhid.usb
linux.uhid.bluetooth
linux.dummy_hcd.usb-hid
linux.dummy_hcd.usb-full
future: linux.btvirt.bluetooth
```

A realization is one cohesive host-facing path.

Do **not** split realization and host entry point into independently selectable public dimensions.

`linux.uhid.usb` means a specific realization:
- controller USB HID personality;
- Linux UHID mechanism;
- HID stack entry;
- USB bus identity metadata but no actual USB device topology.

`linux.dummy_hcd.usb-full` means:
- actual virtual USB device;
- dummy_hcd host controller;
- ConfigFS gadget composition;
- HID and possibly USB Audio functions.

## 2.2 Realizations are not levels

Do not model:

```text
Level1
Level2
Level3
```

uinput, UHID, dummy_hcd, and btvirt overlap in capabilities.

Use descriptive metadata for fidelity/capabilities.

## 2.3 Controller protocol semantics do not belong in providers

Central rule:

> No controller protocol semantics in a realization provider. A realization transports or presents a controller personality; it does not define that personality.

Providers must never branch on “DualSense,” “Switch Pro,” etc.

## 2.4 Controller-native semantics remain typed

Do not introduce a giant universal `GamepadState`.

A DualSense, wheel, HOTAS, Wii Remote, SC2 Puck, etc. may have fundamentally different semantics.

Shared semantic types are allowed only when units/meaning are genuinely identical.

---

# 3. Current repository strengths to preserve

Current crates include:

```text
gr-audio-contract
gr-controller-contract
gr-controller-runtime
gr-controller-wire
gr-curated-controllers
gr-dualsense-wire
gr-privileged-broker
gr-provider-linux-dummy-hcd
gr-provider-linux-uhid
gr-provider-linux-uinput
gr-realization-api
```

Important implementation outcomes worth preserving:

## 3.1 Transactional semantic state

Current runtime:
- clones candidate state;
- validates before accepting;
- rejected edits preserve old state and dirty status;
- accepted edits mark dirty;
- failed commit leaves valid state dirty/retryable.

Keep this behavior.

## 3.2 Exact realization selection

No fallback.

If caller asks for a realization and preflight/open fails, return a typed actionable error.

Do not silently create another realization.

## 3.3 Provider isolation

Separate Linux provider crates are directionally correct.

Preserve private Linux I/O seams and fake-I/O tests.

## 3.4 Compound lifecycle

Current `CompoundSession` already does:
- preflight all;
- deterministic open;
- rollback partial open;
- deterministic send;
- reverse event drain;
- reverse-order close.

Keep/adapt this.

## 3.5 Terminal close

Closed sessions must never reopen.

A new controller requires a new realization session.

## 3.6 Ordinary-user UHID access

Current project validated narrow `/dev/uhid` access through host setup policy without broad `input`-group membership.

Keep setup tooling separate from library execution.

## 3.7 UHID concurrency

Multiple same-family UHID devices can coexist using unique per-session `phys` / `uniq` values while retaining controller VID/PID/version.

Preserve this as a required invariant.

---

# 4. Current architecture cracks to replace

## 4.1 Closed realization enum

Current model:

```rust
enum RealizationTarget {
    Evdev,
    Uhid,
    DummyHcd,
}
```

This is too coarse and not extensible.

Replace architecture-wide dependency on it with:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RealizationId(&'static str);
```

Do not build a growing bitset enum.

Candidate built-in IDs:

```text
linux.uinput
linux.uhid.usb
linux.uhid.bluetooth
linux.dummy_hcd.usb-hid
linux.dummy_hcd.usb-full
linux.btvirt.bluetooth
```

Names should be finalized only after the architecture decision experiments.

## 4.2 Controller encode branching on realization

Current driver shape asks the controller to encode based on `RealizationSelection`.

This creates duplicated protocol packaging.

Replace with:
- semantic state;
- transport protocol personality/session;
- realization adapter.

## 4.3 UHID static feature intelligence

Current UHID provider stores static feature responses and answers some `GET_REPORT` itself.

Remove this.

Feature behavior belongs to controller protocol state.

## 4.4 One-way runtime

Current core is mainly:

```text
state -> encode -> FrameSink
```

Modern controller protocols are conversational:

```text
device -> host input
host -> device output
host -> device GET_REPORT
host -> device SET_REPORT
```

The runtime must explicitly support bidirectional protocol sessions.

## 4.5 Wire facts scattered across implementation crates

Current descriptors/feature data are distributed through:
- `gr-controller-wire`;
- `gr-dualsense-wire`;
- controller crates;
- providers/broker.

Move knowledge ownership to controller protocol personalities, backed by the external protocol corpus.

## 4.6 Monolithic curated-controller crate

Split per controller over time.

Suggested end state:

```text
vg-controller-dualsense
vg-controller-dualshock4
vg-controller-switch-pro
vg-controller-xbox360
```

Do not migrate every controller simultaneously.

---

# 5. Target architecture

```text
Public typed controller API
          │
          ▼
Controller semantic state
          │
          ▼
Stateful protocol personality/session
          │
          ▼
Concrete realization
          │
          ▼
Host mechanism
```

Detailed:

```text
DualSenseState
    │
    ├── DualSenseUsbProtocol
    │       ├── linux.uhid.usb
    │       └── linux.dummy_hcd.usb-*
    │
    ├── DualSenseBluetoothProtocol
    │       ├── linux.uhid.bluetooth
    │       └── future linux.btvirt.bluetooth
    │
    └── DualSenseEvdevPresentation
            └── linux.uinput
```

---

# 6. Protocol personality must be stateful

Do not implement it as only:

```text
encode_input(state)
decode_output(report)
```

Development outcomes from SC2/Puck prove protocol-owned state may include:
- sequence counters;
- connection-edge state;
- periodic reports;
- timestamps;
- pairing/calibration identity;
- host-alive watchdog;
- initialization state;
- timers;
- pending replies;
- reconnect behavior.

Recommended conceptual contract:

```rust
pub trait HidProtocol {
    type SemanticState;
    type OutputEvent;

    fn device_definition(&self) -> &HidDeviceDefinition;

    fn update_semantic_state(
        &mut self,
        state: &Self::SemanticState,
        now: ProtocolTime,
    ) -> Result<Vec<ProtocolAction<Self::OutputEvent>>, ProtocolError>;

    fn handle_host_event(
        &mut self,
        event: HidHostEvent,
        now: ProtocolTime,
    ) -> Result<Vec<ProtocolAction<Self::OutputEvent>>, ProtocolError>;

    fn poll(
        &mut self,
        now: ProtocolTime,
    ) -> Result<Vec<ProtocolAction<Self::OutputEvent>>, ProtocolError>;
}
```

The vector-returning sketch is illustrative and incomplete without delivery feedback and bounded ownership. Exact trait design must satisfy section 6.1 and Gates C/D/J/O before production adoption.

Prefer a synchronous deterministic API driven by explicit time unless an experiment demonstrates that internal async is necessary.

---

## 6.1 Required state, delivery, and service contracts

- Semantic edits remain cloned-candidate transactions: rejection changes neither live state nor dirty status. Accepted revisions are tracked separately from protocol sequence/timing state.
- Select the new API for clarity, not backward compatibility. Distinguish desired-state acceptance, queueing, transport submission, and consumer observation with explicit outcomes/completion semantics. Failed delivery preserves recoverable accepted state and pending-action bookkeeping. The old `commit()` name/signature is not mandatory.
- The runtime owns a bounded pending-action queue. Each action has session/component identity and delivery bookkeeping. The personality defines snapshot/coalescing versus edge/request semantics. Never coalesce required replies or non-repeatable lifecycle edges.
- Explicit feedback distinguishes accepted submission, definitely-unsent retryable failure, and uncertain delivery. Retain definitely-unsent work without regenerating or replaying already accepted edges. On uncertain delivery, use a protocol-specific resynchronization or fail the session; do not promise exactly-once host effects.
- Specify whether sequence/timestamp advancement occurs at generation or accepted submission for each protocol, based on evidence. Retrying an unsent action must not accidentally advance it twice. Multiple components cannot promise atomic host visibility.
- Bound queue size, work per service cycle, request lifetime, and per-session fairness. Give required replies timely service without indefinitely starving input. Return explicit pressure/error outcomes; never silently drop important actions.
- Expose pollable readiness and the next monotonic deadline. The outer runtime drives service even when state is unchanged. Personalities stay deterministic and executor-neutral; an ordinary library embedding must have a documented way to keep them serviced independently of `commit()`.
- Required host requests are handled by the protocol, not optional subscriber callbacks. Validate SET semantics before success acknowledgement. Each request has one completion owner; late/duplicate/closed-session replies are rejected. If transport backpressure prevents immediate submission, retain a bounded completion and deadline.
- Terminal library close rejects future edits/emissions and cancels pending work. Consumer UHID CLOSE followed by OPEN is not terminal close. Preserve START/STOP/OPEN/CLOSE distinctions and driver-maintenance behavior.

Test these rules at fake-clock/protocol seams before integrating Linux. Preserve the repository's immediate host-request servicing rule: dispatch an answer or defined error in the service cycle that consumes a request, with only bounded transport retry afterward.

# 7. Generic HID contracts

Introduce a controller-neutral HID crate.

Suggested concepts:

```text
HidIdentity
HidDeviceDefinition
HidInputReport
HidOutputReport
HidGetReportRequest
HidGetReportReply
HidSetReportRequest
HidSetReportReply
HidLifecycleEvent
HidHostEvent
HidDeviceEvent
```

Important unresolved boundary:
- does protocol own serialized report ID + bytes?
- or logical report ID + payload, with realization serializing framing?

Lock this using the personality/transport byte-boundary experiment before broad migration.

---

# 8. UHID end state

The UHID provider should become boring.

Responsibilities:

```text
open /dev/uhid
UHID_CREATE2
read START / STOP / OPEN / CLOSE
UHID_INPUT2
read OUTPUT
read GET_REPORT
send GET_REPORT_REPLY
read SET_REPORT
send SET_REPORT_REPLY
UHID_DESTROY
pollable readiness
diagnostics
```

It must not know:
- DualSense;
- DS4;
- Switch;
- calibration;
- feature response contents;
- rumble semantics;
- LED semantics;
- controller timing.

Use `UHID_START.dev_flags` as runtime authority for numbered input/output/feature behavior. Preserve the event payload; do not collapse lifecycle events to an undifferentiated diagnostic counter. Startup requests and output must be serviceable without waiting for a consumer OPEN or an application commit.

The current provider acknowledges SET success before downstream handling. Remove that early acknowledgement when transferring reply ownership; protocol acceptance must precede exactly one completion submission. See section 6.1 and Gate J. Validate event-specific framing against the [Linux UHID ABI documentation](https://docs.kernel.org/hid/uhid.html), pinned in the corpus source registry.

Keep a private fake I/O seam for deterministic tests.

---

# 9. Critical UHID re-validation

Historical memories report working Steam gyro through dummy_hcd where an older UHID path failed. The inspected `9b466e0` code already uses `BUS_USB` and sends initial neutral input during creation. Identify the original failing revision and evidence before attributing the difference to bus metadata.

Do not generalize that to “UHID cannot support Steam gyro.”

The imported HHD analysis suggests comparing USB/BT bus metadata. However, archived finding commit `3a62825` already configures UHID with `BUS_USB`; its historical BUS_VIRTUAL explanation is not established by that code. Verify the actual tested revision/configuration, and keep sysfs virtual topology distinct from configured bus metadata.

Run explicit architecture decision test:

```text
DualSenseUsbProtocol
    ↓
UHID BUS_VIRTUAL

vs

DualSenseUsbProtocol
    ↓
UHID BUS_USB
```

Compare:
- kernel driver binding;
- hidraw;
- evdev;
- sensor discovery;
- SDL HIDAPI;
- SDL sensors;
- Steam gyro;
- feature probes;
- output.

Gate B/P establishes scoped host evidence and product claims using controlled versions, timing, identities, and reference conditions. It does not presume a new BUS_USB fix or retire dummy_hcd. Both-success, both-failure, mixed, and unavailable-environment outcomes must be recorded explicitly.

---

# 10. Kernel driver binding is part of realization behavior

For curated identities such as DualSense, test whether expected specialized drivers bind:

```text
UHID
  ↓
hid-playstation
  ↓
evdev / sensor / FF behavior
```

Do not validate only that `/dev/hidrawX` exists.

If `hid-playstation` binding materially enables expected behavior, the `linux.uhid.usb` validation contract should include it.

---

# 11. uinput end state

`linux.uinput` should be the best Linux evdev realization possible for each controller.

Do not intentionally reduce it to buttons/sticks.

Support controller-owned declarations for:
- keys/buttons;
- absolute axes;
- relative axes;
- hats;
- LEDs;
- switches;
- force-feedback codes;
- other valid evdev surfaces.

This is important for:
- wheels;
- flight controls;
- arcade controllers;
- unusual peripherals.

Do not create one hard-coded “gamepad evdev profile.”

---

# 12. DummyHcd end state

DummyHcd remains a real selectable realization, not merely a historical validation tool.

Roles:
- actual USB enumeration;
- actual USB parent/topology;
- HID control requests through USB;
- composite USB functions;
- topology-sensitive application compatibility;
- maximum-fidelity USB realization.

Separate likely realizations:

```text
linux.dummy_hcd.usb-hid
linux.dummy_hcd.usb-full
```

Names remain tentative until experiments.

`usb-hid`:
- actual USB device with HID function.

`usb-full`:
- curated physical USB topology such as HID + Audio.

---

# 13. Generic USB device/function modeling

Do not make dummy_hcd synonymous with HID.

Introduce generic internal curated USB composition:

```rust
struct UsbDevicePlan {
    identity: UsbDeviceIdentity,
    configurations: Vec<UsbConfigurationPlan>,
}
```

Functions may include:

```text
HID
Audio
future curated vendor/accessory function
```

This does not imply arbitrary caller-defined USB gadgets.

Only reviewed controller packages may produce plans.

---

# 14. Audio architecture

Separate three concerns:

## 14.1 HID audio-control semantics
Examples:
- mute;
- volume;
- routing;
- jack state;
- haptic/audio mode bits.

These belong to controller protocol.

UHID can carry them.

## 14.2 PCM stream semantics
Actual audio samples.

Need a real audio stream API.

## 14.3 Bus topology
Whether audio is:
- an independent host-side PipeWire/ALSA companion;
- part of the same physical USB composite device.

Candidate realizations:

```text
linux.uhid.usb
    HID only

possible linux.uhid.usb-audio
    UHID + host audio companion

linux.dummy_hcd.usb-full
    HID + actual USB Audio Class topology
```

Do not claim a host audio sidecar is a physical USB composite controller.

---

# 15. Audio API direction

Current `gr-audio-contract` is scaffolding, not a complete PCM API.

Future internal API should model actual sample transport.

Direction names should be perspective-explicit, e.g.:

```text
HostToController
ControllerToHost
```

Avoid ambiguous `Input` / `Output` names.

The library should not require an internal mixer.
Caller should be able to:
- consume speaker/headphone/haptic stream;
- inject mic stream;
- discard or inspect audio;
- route elsewhere.

---

# 16. Privileged broker boundary

Preserve the strong existing security model.

Unprivileged client must not submit:
- arbitrary ConfigFS paths;
- arbitrary USB descriptors;
- arbitrary VID/PID;
- arbitrary module names;
- arbitrary shell commands;
- arbitrary UDC selection.

Use opaque compiled realization IDs:

```text
sony.dualsense.linux-dummy-hcd-usb-full-v1
```

Broker owns privileged mapping.

However, controller protocol intelligence should move out of the broker if timing experiments permit.

Desired path:

```text
USB host
  ↓
dummy_hcd
  ↓
broker
  ↓ bounded typed request
unprivileged protocol session
  ↓ typed reply
broker
  ↓
USB host
```

Run Gate G for transport capability, startup, latency, and failure behavior before making this absolute.

Creation must negotiate broker/client IPC and compiled-profile compatibility, establish the bounded session channel, and initialize the unprivileged personality before binding the gadget. Service probe requests while opening; expose readiness only after its defined criteria. Client death, timeout, malformed replies, and partial setup trigger bounded rollback.

The existing broker services DualSense/DS4 feature requests before returning a session. A replacement must not wait for create to return before enabling the handler that create needs. Inventory the selected kernel/gadget API's report type/ID/length, transaction correlation, control SET, negative completion, and interrupt-output support; do not assume UHID-equivalent capabilities.

Immutable allowlisted descriptors and construction profiles remain valid broker responsibilities. If measured constraints justify immutable bootstrap responses, derive them from reviewed controller facts, verify parity/version agreement, and document a narrow ADR exception. Mutable controller state must not acquire two competing implementations.

---

# 17. Compound realization model

A controller realization may own multiple host-visible components.

Keep this as a controller-owned concept.

Examples:
- SC2 Puck multiple logical interfaces;
- controller-native keyboard/pointer companion;
- UHID + host audio companion.

Do not turn this into a generic device compositor.

A useful internal model:

```text
Controller realization
├── component A
├── component B
└── component C
```

Component lifecycle:
- preflight all;
- open deterministic order;
- rollback partial open;
- deterministic commit/send;
- reverse-event routing;
- reverse-order close.

Distinguish:
- multiple independent host devices;
- components sharing one actual bus device.

---

# 18. Composite UHID question remains empirical

Multiple UHID devices cannot literally become interfaces under one USB parent.

The unresolved question is whether real consumers care.

Run a dedicated experiment using:
- a synthetic or SC2/Puck compound personality;
- same identity grouping conventions;
- SDL;
- Steam;
- sysfs/udev topology inspection.

Decision:
- if host software correctly associates components, allow compound UHID realizations with honest metadata;
- if not, controllers requiring shared topology should expose only bus-level realizations such as dummy_hcd.

Do not force a universal answer.

---

# 19. Bluetooth architecture

Separate protocol validation from real Bluetooth realization.

Stage 1:

```text
DualSenseBluetoothProtocol
    ↓
linux.uhid.bluetooth
```

This validates:
- descriptor;
- report framing;
- CRC;
- sequence;
- feature/output behavior.

Stage 2:

```text
DualSenseBluetoothProtocol
    ↓
future linux.btvirt.bluetooth
    ↓
BlueZ
```

This validates:
- discovery;
- pairing;
- connection;
- reconnect;
- actual Bluetooth device behavior.

Failure of btvirt must not invalidate the Bluetooth protocol personality.

---

# 20. Realization metadata

Use metadata for descriptive capability/fidelity, not dispatch.

Example shape:

```rust
struct RealizationDescriptor {
    id: RealizationId,
    properties: RealizationProperties,
    functions: &'static [DeviceFunctionClass],
    requirements: HostRequirements,
    evidence: ValidationEvidence,
}
```

Potential properties:
- creates_input_device;
- creates_hid_device;
- creates_actual_bus_device;
- bus_type;
- supports_composite_functions;
- topology_fidelity;
- audio_topology;
- expected_kernel_driver.

Do not make these independently selectable configuration dimensions.

---

# 21. Validation evidence is multidimensional

Retire a one-dimensional:

```text
ResearchBacked
HostValidated
PhysicallyValidated
```

model.

Track axes such as:
- protocol fixture conformance;
- provider unit/fault tests;
- kernel enumeration;
- expected driver binding;
- hidraw;
- evdev;
- concurrency;
- SDL gamepad;
- SDL HIDAPI;
- SDL sensors;
- Steam;
- audio enumeration;
- audio streaming;
- USB descriptor match;
- Bluetooth pairing;
- reconnect;
- physical reference comparison.

Provider-complete does not imply Steam-validated.

---

# 22. Test infrastructure to preserve

## 22.1 Private fake-I/O seam

Every host provider should separate:
- public realization/session;
- private I/O implementation;
- fake deterministic I/O used in provider-local tests.

Faults to test:
- open failure;
- short write;
- would-block;
- malformed event;
- teardown error;
- double close;
- invalid reply;
- partial setup.

## 22.2 One-shot + interactive smoke tests

Keep both:
- deterministic one-shot smoke test for CI;
- interactive mode for manual host inspection.

## 22.3 Manual architecture gates

For high-risk transport changes, maintain:
- automated checks;
- explicit manual reviewer guide;
- sign-off record.

## 22.4 Negative tests

Examples:
- DualSense-only feature cannot compile/use on DS4;
- wrong realization cannot accept wrong protocol personality;
- closed session cannot emit;
- duplicate components fail;
- invalid USB composition fails;
- controller A cannot answer controller B feature request.

---

# 23. Protocol corpus integration

The protocol corpus remains an independent Git repository but is checked into the `virtualgamepad` working tree as a **Git submodule**.

Recommended layout:

```text
virtualgamepad/
├── protocol-corpus/        # independent repo, pinned gitlink
├── crates/
├── docs/
└── ...
```

The gitlink itself is the authoritative corpus revision for a `virtualgamepad` commit.

This is preferable to maintaining a second hand-written revision lock file unless a generated manifest is useful for diagnostics. If a manifest is kept, it must be derived from/validated against the actual submodule commit rather than becoming a second source of truth.

Use the submodule directly for development-time:
- protocol fixtures;
- claim lookup;
- conformance tests;
- code generation of constants/tables where approved;
- trace/descriptor comparison;
- documentation references.

`virtualgamepad` must **not** runtime-load corpus YAML as a generic controller definition system.

Recommended integration pattern:

```text
protocol-corpus/
    ↓
maintainer codegen / fixture extraction / conformance tests
    ↓
reviewed compiled Rust personality
```

Normal runtime artifacts should remain compiled and curated.

## 23.1 Developer workflow

Fresh clone:

```bash
git clone --recurse-submodules <virtualgamepad-url>
```

Existing clone:

```bash
git submodule update --init --recursive
```

Research changes are committed in the corpus repository first. The `virtualgamepad` repository then records the new corpus commit by updating the gitlink.

A code PR that depends on new corpus facts should make the dependency explicit:
1. corpus PR/commit;
2. `virtualgamepad` submodule pointer update;
3. implementation/tests using that revision.

## 23.2 CI

Corpus-backed CI must initialize the submodule.

CI should verify:
- submodule path exists;
- expected commit is checked out;
- generated constants/fixtures are synchronized with that commit;
- implementation claim references resolve;
- corpus-backed conformance tests pass.

## 23.3 Release/downstream packaging

Do not force ordinary downstream users of the Rust library to understand Git submodules if it can be avoided.

Preferred:
- generated Rust constants/fixtures required for runtime compilation live in normal crates;
- the submodule is needed for maintainer validation/codegen/research;
- ordinary runtime compilation uses shipped generated outputs; maintainer generation is explicit and separate;
- package verification builds ordinary runtime artifacts without a corpus checkout or network access, with normal dependencies pre-cached;
- generated artifacts carry corpus SHA/schema/generator provenance and are checked against the gitlink in corpus-aware CI. See `PROTOCOL_CORPUS_SUBMODULE_WORKFLOW.md`.

## 23.4 Independence rule

Even though it is nested in the working tree, `protocol-corpus/` is not a normal directory owned by the `virtualgamepad` repository.

Do not:
- directly commit corpus files from the superproject;
- duplicate corpus history into `virtualgamepad`;
- treat the submodule as a runtime plugin directory.

Corpus can generate facts, but Rust implements behavior.

---

# 24. Suggested crate topology

Final names are not urgent.

Target ownership:

```text
vg-core/
    shared controller/runtime primitives

vg-realization/
    RealizationId
    manifests
    lifecycle contracts
    evidence/requirements

vg-hid/
    generic HID protocol primitives

vg-audio/
    audio stream/function contracts

vg-usb/
    curated USB device/function plan types

vg-controller-dualsense/
vg-controller-dualshock4/
vg-controller-switch-pro/
vg-controller-xbox360/

vg-provider-linux-uinput/
vg-provider-linux-uhid/
vg-provider-linux-usb-gadget/
vg-provider-linux-audio-pipewire/   optional
vg-provider-linux-btvirt/           future

vg-privileged-broker/
vg-validation/
```

Dependency rule:

```text
controller semantic model
    ↓
controller protocol personality
    ↓
realization adapter
    ↓
generic host provider
```

Never:

```text
provider -> controller-specific protocol logic
```

---

# 25. Feature support expectations

Legend:
- Native = mechanism directly represents feature;
- Strong = should be faithfully realizable;
- Partial = approximation or OS-surface loss;
- Sidecar = extra host device/service;
- No = mechanism cannot provide it.

| Feature | uinput | UHID USB | UHID BT | dummy_hcd USB HID | dummy_hcd USB full | future btvirt |
|---|---|---|---|---|---|---|
| buttons/hats | Native | Native | Native | Native | Native | Native |
| analog axes | Native | Native | Native | Native | Native | Native |
| unusual axis sets | Native | Native | Native | Native | Native | Native |
| relative axes | Native | HID-dependent | HID-dependent | HID-dependent | HID-dependent | HID-dependent |
| evdev presentation | Native | Strong | Strong | Strong | Strong | Strong |
| HID descriptor fidelity | No | Strong | Strong | Strong | Strong | Strong |
| vendor HID fields | Partial/No | Strong | Strong | Strong | Strong | Strong |
| input report byte fidelity | No | Strong | Strong | Strong | Strong | Strong |
| HID output reports | Partial | Strong | Strong | Strong | Strong | Strong |
| HID feature GET/SET | No | Strong | Strong | Strong | Strong | Strong |
| initialization protocol | Partial | Strong | Strong | Strong | Strong | Strong |
| IMU | Partial | Strong | Strong | Strong | Strong | Strong |
| touch | Partial/Strong | Strong | Strong | Strong | Strong | Strong |
| battery/status | Partial | Strong | Strong | Strong | Strong | Strong |
| LEDs | Native where supported | Strong | Strong | Strong | Strong | Strong |
| simple rumble | Native FF | Strong | Strong | Strong | Strong | Strong |
| complex Linux FF | potentially strongest | protocol-dependent | protocol-dependent | protocol-dependent | protocol-dependent | protocol-dependent |
| adaptive triggers | No generic mapping | Strong | Strong | Strong | Strong | Strong |
| vendor haptics | Partial/No | Strong | Strong | Strong | Strong | Strong |
| HID audio controls | Partial | Strong | protocol-dependent | Strong | Strong | protocol-dependent |
| actual PCM endpoint | No | No | No | No | Native USB audio | BT-profile dependent |
| host audio companion | Sidecar | Sidecar | Sidecar | Sidecar | usually unnecessary | Sidecar/profile-dependent |
| actual USB device | No | No | No | Native | Native | No |
| USB interface topology | No | No | No | limited | Native | No |
| multiple USB interfaces | No | No | No | limited | Native | No |
| non-HID USB functions | No | No | No | No | Native | No |
| actual Bluetooth pairing | No | No | No | No | No | Native |
| physical bus topology | No | No | No | Strong USB | Strongest USB | Strongest BT |

This matrix describes mechanism-level potential, not implemented controller support or an ordered fidelity ladder. Final claims require per-controller, per-realization, per-feature evidence and tested environment versions. Distinguish representable, implemented, tested, and unsupported states; a failed audio/BT gate limits only the corresponding feature claim.

---

# 26. Technical-debt traps

## 26.1 Do not treat every output as rumble
Need typed controller outputs:
- simple rumble;
- advanced haptics;
- adaptive triggers;
- force feedback;
- LEDs;
- display output;
- power/mode commands.

## 26.2 Do not treat `commit()` as one report
Protocols may have:
- multiple report IDs;
- periodic reports;
- keepalives;
- timers;
- independent lifecycle events.

## 26.3 Do not assume one controller = one HID device
Allow controller-owned compound realizations.

## 26.4 Do not make shared API gamepad-shaped
Foundational crates should not require:
- two sticks;
- four face buttons;
- two triggers.

## 26.5 Do not equate HID semantics with USB semantics
UHID USB personality is not an actual USB device.

## 26.6 Do not make audio permanently “sidecar-only”
Audio may be a same-device USB function in full USB realizations.

## 26.7 Do not expose arbitrary compatibility identity
If curated compatibility personalities exist, they must be explicit reviewed variants.

---

# 27. Esoteric peripheral benchmark

Wheels / flight sticks are **not a design target**, but the architecture should pass this benchmark:

> Adding a curated racing wheel or HOTAS should look like adding another controller package, not rewriting the core.

## Racing wheel benchmark

Potential semantic state:
- steering;
- throttle;
- brake;
- clutch;
- H-pattern shifter;
- paddles;
- buttons;
- rotary controls;
- status.

Outputs:
- constant force;
- spring;
- damper;
- friction;
- periodic force;
- gain;
- autocenter;
- LEDs/display.

Expected realization fit:
- uinput: excellent Linux FF/native input surface;
- UHID: physical HID/PID or vendor protocol fidelity;
- dummy_hcd: exact USB topology/vendor interfaces if needed.

This benchmark validates:
- controller-native semantics;
- stateful host-output protocol;
- no “rumble-only” abstraction;
- broad evdev presentation.

## Flight stick / HOTAS benchmark

Potential:
- many axes;
- hats;
- sliders;
- twist;
- throttle;
- mode switches;
- dozens of buttons;
- rotary encoders;
- multiple logical components.

Architecture should support it without adding universal HOTAS fields.

## Out-of-scope benchmark

Do not evolve into an arbitrary USB device emulator.

If a peripheral is not meaningfully an interactive controller/peripheral personality, it may be outside `virtualgamepad`.

---

# 28. Execution batches and migration gates

Use the E0–E6 table in `ARCHITECTURE_DECISION_EXPERIMENTS.md` section 17 as the sole sequencing authority. The earlier R0–R16 sequence is superseded.

The first production deliverable is one complete DualSense USB/UHID slice: personality, bidirectional service loop, explicit reply ownership, lifecycle, bounded delivery, and curated realization selection together. Do not strip provider feature handling before a functioning protocol dispatcher replaces it. Gates C/D/J/O settle this contract in memory; B/P establishes host behavior before acceptance promotion.

The second deliverable reuses that USB personality through dummy_hcd after Gate G validates startup and API capability. Preserve current uinput and other-family regressions throughout. Retain the old revision and valuable fixtures in Git; the shipping tree may replace the old path outright. Do not add runtime fallback.

Only then broaden controller migration. DS4/Switch need corpus-backed fixtures and family-specific acceptance; SC2-inspired synthetic cases stress the initial contract but do not certify SC2 hardware. Full SC2/Puck work follows its evidence and compound/identity gates.

Audio, compound topology, and actual Bluetooth are independently gated extensions. They need not all succeed before a useful USB/HID library ships. API stabilization requires multiple controller implementations and the wheel/HOTAS review benchmark, not completion of every speculative backend.

Crate renaming/splitting may be part of a coherent clean rewrite; no compatibility facade is required. Every batch records baseline, changed behavior, regression results, supported-host limitations, reviewer guide/signoff, and preserved rollback path. Respect repository branch/CI policy during implementation; this plan's proposed CI changes are not an instruction to bypass local repository rules.

---

# 29. Public API direction

Likely:

```rust
let mut ds = DualSense::builder()
    .realization(dualsense::LINUX_UHID_USB)
    .create()?;
```

Discovery:

```rust
for r in DualSense::realizations() {
    println!("{}", r.id());
}
```

No:
- arbitrary HID constructor;
- arbitrary descriptor injection;
- runtime profile loader;
- automatic fallback.

---

# 30. Rewrite scope assessment

Preserve:
- standalone philosophy;
- controller-native state;
- transactional edits;
- exact/no-fallback creation;
- provider separation;
- compound lifecycle;
- reverse-event infrastructure concepts;
- fake-I/O testing;
- host-setup separation.

Heavy rewrite:
- `gr-realization-api`;
- `TargetAwareControllerDriver`;
- encode→`ProviderFrame` coupling;
- UHID static feature handling;
- controller wire ownership;
- broker protocol knowledge;
- audio contract.

Retire/transform:
- `gr-controller-wire`;
- `gr-dualsense-wire`;
- giant curated-controller crate.

---

# 31. Architectural invariants

1. A provider never interprets controller-native protocol semantics.
2. A protocol personality never mutates host policy.
3. A realization is selected exactly; no fallback.
4. Closed sessions are terminal.
5. Failed delivery preserves recoverable accepted semantic state and pending-action bookkeeping; API names and dirty-state representation may change.
6. Multiple same-family virtual devices can coexist.
7. Controller-specific protocol behavior is backed by corpus claims/fixtures.
8. Compatibility policy is distinguishable from physical-device truth.
9. Full USB topology claims require bus-level realization evidence.
10. Runtime YAML/profiles do not define shipping controller behavior.
11. Compound devices are controller-owned, not generic arbitrary composition.
12. Privileged broker accepts only curated bounded realization operations.
13. The corpus remains independently versioned; each `virtualgamepad` commit pins one exact corpus commit through the Git submodule.
14. Ordinary runtime behavior does not depend on parsing corpus files.

---

# 32. Definition of architecture success

The rewrite is successful when:

- DualSense USB protocol is implemented once and reused by UHID and dummy_hcd;
- UHID contains no DualSense/DS4/Switch feature knowledge;
- dynamic feature requests flow through protocol state;
- protocol timing/lifecycle is deterministic and testable;
- realization IDs are extensible;
- uinput remains rich rather than crippled;
- dummy_hcd supports actual USB topology;
- audio architecture distinguishes HID controls, PCM, and bus topology;
- corpus fixtures drive conformance tests;
- adding DS4/Switch does not require provider changes;
- a wheel/HOTAS architecture review finds no core redesign requirement;
- a future Bluetooth bus backend can reuse the BT protocol personality.

---

# 33. Highest-priority principle

> Preserve controller semantics and protocol behavior once; let concrete realizations determine how that personality enters the host.

That is the core of the rewrite.
