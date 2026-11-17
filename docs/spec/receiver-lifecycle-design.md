# Receiver lifecycle design (#165)

**Status: accepted in principle by the owner (2026-11-17), not yet normative.**
This document proposes an amendment to
[ADR-0022](../adr/ADR-0022-receiver-aggregates.md); see section 6 for the owner
decisions. Implementation remains post-Alpha 2 and waits for VG-1b. No
implementation, API authorization, support-cell promotion, physical observation
or host run follows from this draft.

## Evidence baseline

Implementation inspected at virtualgamepad commit
[12358d71d762fb438cbf01f5b120986279a8540d](https://github.com/anderstvoss/virtualgamepad/commit/12358d71d762fb438cbf01f5b120986279a8540d).
Research inspected at
[73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f](https://github.com/anderstvoss/controller-protocol-research/commit/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f).
The [eight-question decision table](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/docs/receivers-lifecycle-decision-table.md)
(research #135),
[Puck brief](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/docs/receivers-puck-consumer-lifecycle.md)
(#127),
[Xbox 360 brief](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/docs/receivers-xbox360-wireless.md)
(#128), and
[Xbox Wireless Adapter brief](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/docs/receivers-xbox-wireless-adapter.md)
(#134) are all merged at this revision. Claim IDs below refer to
`records/claim/<id>.json` at that research pin. The decision table describes the
Xbox briefs as draft at survey time; that is historical, not current.

The source-supported claims describe consumers, not measured retail behavior.
Reconnection-epoch implications and provider feasibility are inferences;
physical capacity, pairing, exact descriptors, delivery/actuation and hotplug
timing remain unmeasured. The #165 Puck brief (comment 6748217699) and SDL
correction (6748258725), and findings 6792777577, 6813075261, 6816759408 and
6817055627 are the design inputs. The DJ registry attribution follow-up
(6817237975) is independent research work; do not duplicate it here. No vendor
firmware analysis; any gap requiring it **needs private analysis**.

## 1. Requirements

| ID / question | Required receiver behavior | Claim IDs (source unless marked inferred/unknown) |
| --- | --- | --- |
| R1 — identity, capacity | Compiled family-specific address space and capacity. Puck: four fixed slot interfaces, 0–3; pogo is separate. Xbox 360: four interface/pipe consumer model (retail count unmeasured). XWA: dynamic allocation with 16 software entries, not a verified physical maximum. DS4: one dongle controller. DJ: heterogeneous children, indices 1–7, not a verified active count. No global four-slot ceiling. | `rcv-puck-kernel-per-interface`, `rcv-x360-binding`, `rcv-x360-four-slot-consumer`, `rcv-xwa-slot-reuse`, `rcv-pattern-ds4-dongle-presence`, `rcv-pattern-dj-receiver-children` |
| R2 — presence | Translate native presence signals within the receiver package, not provider or app. Puck kernel queries status and handles 0x79; SDL dispatches 0x46/0x79 (not 0x7b); data may imply connection. xpad uses per-interface presence work; xone queues add/remove and then identifies clients. DS4 gates input pending calibration. DJ pair/unpair and temporary link loss are distinct. Handle duplicate/coalesced events without fabricating edges. | `rcv-puck-kernel-presence`, `rcv-puck-sdl-presence`, `rcv-x360-presence`, `rcv-xwa-association-queue`, `rcv-pattern-ds4-dongle-presence`, `rcv-pattern-dj-linkloss-unpair` |
| R3 — node lifetime | Destroy/recreate native controller presentation after detach for Puck, xpad and xone. DS4 probe creates gamepad, sensor and touch nodes before connection; keep them on link loss. DJ paired children survive link loss, but unpair removes them and re-pair creates fresh objects. A retained node is not proof of protocol continuity; reused node path is not proof of same instance. | `rcv-puck-kernel-attach`, `rcv-puck-kernel-detach`, `rcv-puck-sdl-joystick-lifecycle`, `rcv-puck-reconnect-epoch-inference` (inferred), `rcv-x360-evdev-epoch`, `rcv-xwa-node-lifetime`, `rcv-pattern-ds4-node-persistence`, `rcv-pattern-dj-linkloss-unpair` |
| R4 — identity and state | Address, logical occupant/association, child presentation incarnation, host node name and application player assignment are separate. Native inputs, battery, indicators, output and protocol state are per occupant/slot as appropriate; no promise of physical bond persistence, stable xpad LED index or xone entry assignment. Receiver state is separate. | `rcv-puck-identity-battery`, `rcv-x360-identity-led`, `rcv-xwa-slot-reuse`, `rcv-xwa-per-client-surfaces`, `rcv-pattern-ds4-node-persistence`, `rcv-pattern-dj-receiver-children` |
| R5 — output and absence | Route by native interface/index/client and originating incarnation, not a player index. Absent endpoints cannot be assumed to consume output: xpad unregisters FF/LED while pending interface output may remain; xone frees client adapter; DS4 worker can still send through retained dongle; DJ removed child has no new endpoint; Puck output uses associated HID handle. Reject unsupported/unavailable destination without mutating another slot. Preserve required replies, no uncertain replay or claim of physical actuation. | `rcv-puck-output-routing`, `rcv-x360-output`, `rcv-x360-output-epoch-gap` (inferred), `rcv-xwa-output-routing`, `rcv-pattern-ds4-output-boundary`, `rcv-pattern-dj-routed-output` |
| R6 — failure | Child-local loss leaves siblings; receiver unplug, shared transport loss/reset or loss of required receiver interface is receiver-wide. DJ demonstrates the last case: unbinding a required receiver interface removes all children even if other interfaces remain. This is not a counterexample to isolation of child-only failure. Preserve bounded service and reverse/idempotent cleanup. | `rcv-puck-teardown`, `rcv-x360-teardown`, `rcv-xwa-failure-teardown`, `rcv-pattern-ds4-node-persistence`, `rcv-pattern-dj-interface-failure` |
| R7 — companions | Puck lizard/native/raw ownership is controller policy; opt-in capability-limited keyboard/pointer companions, not evidence of physical desktop node inventory. DS4 touchpad is not automatically a desktop pointer; xpad has no such companions in reviewed path; xone optional headset audio is distinct. DJ keyboard/mouse are actual heterogeneous paired children, not default companions of a gamepad. | `rcv-puck-lizard-arbitration`, `rcv-x360-companion-battery`, `rcv-xwa-per-client-surfaces`, `rcv-pattern-ds4-node-persistence`, `rcv-pattern-dj-receiver-children` |
| R8 — realization | Reduced independent nodes must be explicitly selected and labeled. Native Puck requires interface-aware USB composite; xpad and xone require USB-level vendor transport, not UHID metadata. DS4 dongle HID parser experiment may be possible in UHID, but faithful receiver identity requires USB evidence. DJ shared-interface grouping requires USB-level proof; single UHID HID child is only reduced. No implied fallback or working new USB provider. | `rcv-puck-kernel-per-interface`, `rcv-puck-physical-slot-unknown` (unknown), `rcv-x360-usb-provider` (inferred), `rcv-x360-sdl-evdev`, `rcv-xwa-virtual-boundary` (inferred), `rcv-xwa-mainline-xpad`, `rcv-xwa-sdl-linux`, `rcv-pattern-ds4-dongle-presence`, `rcv-pattern-dj-receiver-children` |

## 2. Gap analysis at the implementation baseline

References are to the pinned virtualgamepad tree above; line numbers refer to
that revision, not future changes. Covered means the contract or synthetic
fixture covers the behavior, not that a production receiver/provider exists.

| Area | Verdict and source locations |
| --- | --- |
| Receiver ownership / typed family registry (R1/R4) | **Covered in intent; needs change in representation.** ADR-0022:11–15; `crates/gr-curated-controllers/src/registry.rs:117–157,429–463` declares Receiver and hosted families but only a metadata fixture, not slot schema or realization. `crates/gr-hid/tests/receiver_aggregate.rs:11–25,48–67,325–351` fixes four addresses and arrays. Replace global fixture sizing assumptions with reviewed family limits; do not replace static family registry with runtime profiles. |
| Presence, node lifetime and identity (R2–R4) | **Needs change.** ADR-0022:17–23 assumes removal/neutralization; fixture attachment is optional and `receiver_aggregate.rs:353–414` resets inputs/rumble on identity change, advances one whole-aggregate generation and swaps companion groups. No distinction between paired but link lost and removed, no child incarnation token, no actual consumer node allocation. `674–716` tests synthetic reconnect and lizard policy, not an observed host node. |
| Preflight, rollback, companion scope (R7) | **Covered in fixture; untested production.** ADR-0022:17–23,33–38; `receiver_aggregate.rs:261–311,353–414,552–672,1013–1061,1107–1130`; `gr-hid/src/group.rs:92–126,168–221` (full path `crates/gr-hid/src/group.rs`). Preserve selected required components, bounded capability validation and reverse rollback. No heterogeneous DJ child support or proof of Puck keyboard/pointer descriptors. |
| Output routing and absent requests (R5) | **Covered narrowly; needs incarnation fencing.** `crates/gr-hid/tests/receiver_aggregate.rs:68–85,127–152,718–867` selects synthetic slot and rejects absent slot SET; existing output is only a slot/value pair. `crates/gr-hid/src/lib.rs:188–239,378–472,486–505` owns immutable pending input and required reply, validates session/request ordinal and retries definite-unsent. No slot incarnation in pending command envelope; `receiver_aggregate.rs:953–980` explicitly preserves old queued native bytes at replacement. Keep byte/order guarantee, add a publication barrier before a reused address becomes a new destination; prove cancellation/terminal behavior rather than silently retargeting old work. |
| Failure isolation and cleanup (R6) | **Covered for fixture.** `receiver_aggregate.rs:424–480,869–952,1088–1106,1132–1255`: primary failure closes aggregate; failed required companion disconnects its slot, services siblings and preserves cleanup diagnostics; inability to install validated disconnect (generation overflow) is shared terminal. `crates/gr-hid/src/group.rs:168–221` closes its required group, not arbitrary receiver siblings. `crates/gr-hid/src/lib.rs:361–377,507–518` terminal policy/close. **Untested** shared-interface loss, slot-address reuse while output in flight, partial USB fault. Distinguish recoverable errors from actual component terminal state; fixture currently closes on any primary service error. |
| Application readback | **Needs change.** `receiver_aggregate.rs:325–332,353–414,424–461` has one synthetic topology generation and returns per-cycle outputs; service can return a slot error instead of its successful primary observations. No production slot readback/journal. ADR-0019 provides retained generation/current topology; ADR-0024 defines accepted commands, explicit gaps and terminal readback; neither supplies a receiver handle automatically. |
| Production tier and API (R8) | **Untested / not implemented.** `src/controllers.rs:919–934` is wired Steam creation, not a receiver. [Fixture specification](receiver-aggregate-fixture.md) and [brief 08](../plans/pre-controller-buildout/08-epochs-and-receivers.md) are test evidence and historical plan, not a production receiver. #168's USB-provider gate remains independent. D3 closed; ADR-0021 remains Deferred. |

## 3. Proposed design

### 3.1 Receiver registry and ownership

Extend internal compiled Receiver metadata with a reviewed address scheme,
bounded capacity policy (including whether static addresses or bounded dynamic
entries), allowed hosted native family/variant for each address class, node
lifetime policy, receiver-required transport roles and child/companion roles.
The policy is family-owned and checked at preflight; providers expose only
transport operations and capability refusals. Do not infer the supported
physical capacity from a consumer allocation or publish an unverified limit
as a supported realization. Puck can use a statically sized four-slot
implementation; the common seam must not assume four, contiguous zero-based
addresses or homogeneous children. DS4 one, xpad four as a consumer model,
xone dynamic table and DJ one-based heterogeneous indices are future policy
cases, not new advertised entries. Keep bounded memory/service budgets per
family and realization. Do not implement generic arbitrary HID/USB composition.

One receiver handle owns its transport, receiver protocol, address registry,
lifecycle generation, cleanup and scheduler. A typed slot view/handle is
receiver-scoped, identifies one logical occupant and is invalid for mutation
once that occupant or presentation is gone. Use opaque receiver instance,
address and occupant generation/incarnation as routing keys; do not expose
physical identifying material or assume a persisted association across
detach/re-pair. The receiver owner arbitrates address reuse. Sibling handles
are unaffected by another slot's removal. Receiver close invalidates all
mutation handles, while owned terminal diagnostics remain readable. No
GamepadManager allocation, IUCM adaptation or generalized event injection.

Conceptual root and curated creation shape:
`create_steam_puck(CreationOptions, SteamPuckOptions)` -> typed receiver handle.
Name is a proposal; common creation selection is independent of the family
options (selected explicit roles, reviewed identity/variant and native policy).
Zero attached slots may be a valid receiver creation; no silent creation of
ordinary independent devices. Proposed typed attach/detach/link transition
operations belong to receiver, not provider or input edits. For Puck, a
receiver-scoped address selects a typed Steam slot, not a generic mutable
controller map. Later families should use the same
`create_<family>(CreationOptions, <Family>Options)` shape and their own
options/slots. Keep existing direct Steam constructor semantics.

A slot uses the post-VG-1b [native input model](input-model-state-and-mutation.md):
input-only `<Family>Inputs` (SteamController2026Inputs after its migration),
`inputs()`, `set_inputs`, fallible `edit_inputs`, native setters and
`neutralize` all share the family final-candidate validator. Accepted level
edits are pure until commit/service; do not overwrite pending bytes, battery,
identity, protocol clocks, presentation, attachments or outputs with an input
snapshot. An unavailable slot rejects active edits (including stale handles)
without dirtying siblings. Retained-node loss gates active input; the accepted
neutral safety policy is a vg decision, not evidence that DS4 emits a neutral
sample. A new occupant or presentation starts from neutral; require fresh input
rather than replaying held controls, including after retained-node link loss.

### 3.2 Presence and presentation state machine

Store presence and node lifetime separately; expose these effective states:

| State | Meaning / permitted operation |
| --- | --- |
| Vacant / no presentation | No active occupant or native child; receiver listener/required interfaces can remain. Attach is a validated transaction. A family may separately retain reviewed association metadata, never infer bond persistence. |
| Link lost, presentation retained | A declared/probe-created or paired child still exists, but its controller input is unavailable (also covers DS4 initially absent at probe). No new presentation epoch on link resume. The protocol may still service receiver/child queries; no active-input delivery. |
| Attaching / identifying | A bounded protocol preparation (e.g. DS4 calibration or xone identification) gates input; only publish successful required components. Expose as pending if the family has an observable pending stage; failed prepare does not invent an attached topology. The previous committed snapshot remains authoritative until publication. |
| Present | Published slot and available native inputs; output addressed in this occupant and protocol context. |
| Terminal slot / receiver | Closed slot incarnation or entire receiver; no new mutation or acceptance. A free address can be reused only after fencing/cleanup. A terminal receiver is never reopened in place. |

Policy families: create-on-attach/remove-on-detach (Puck, xpad, xone);
probe-created/retain-through-link-loss (DS4); pair-created/retain-through-link-loss,
remove-on-unpair (DJ). Temporary link loss for Puck destroys native
presentation only as part of the reviewed detach policy; keep receiver slot
interface alive. DS4 reconnect waits for readiness/calibration; disabled or
failed readiness stays unavailable. DJ separates pairing from link status.
Do not synthesize wireless pairing, undocumented status or actual Linux
registration observation. Treat duplicated status as no-op after validation;
coalesced consumer work may observe only the latest state. No guarantee that
every source edge reaches an application or OS consumer.

Attach transaction: resolve address and current occupant, validate allowed
family, options/identity, neutral inputs, capabilities, generation budget and
bounded queue capacity, preflight all selected required components, prepare in
deterministic order, install state/roles and publish one coherent snapshot.
On pre-publication error roll back only new resources in reverse order;
preserve previous accepted snapshot and diagnostics. For detach, fence old
work, neutralize/removal per policy, release held companion keys/pointer state
where possible, then publish absence and close old resources in dependency
order; post-publication cleanup failure is diagnostic unless shared integrity
is lost. If a single operation changes several slots, validate and publish
together or reject before opening any resource. External consumer discovery is
not an atomic transaction. Unplug/terminal failure takes precedence over
pending attach; repeated close is idempotent.

### 3.3 Three distinct generations

- Receiver lifecycle/topology generation: retained, checked, monotonic per
  receiver instance. An atomic publication changing slot membership, effective
  presence/availability, child incarnation or active companion roles advances
  once; duplicate/rejected requests, polling and journal eviction advance
  nothing. This is an ADR-0019-style receiver topology extension, not a change
  to Wii topology semantics. A readback includes each typed slot address,
  presence, active topology, declared presentation token and terminal reason
  where applicable; it remains readable after close. On terminal close publish
  all unavailable/removed plus terminal state, or preserve last generation and
  explicit terminal status if exhausted (never wrap).
- Slot presentation epoch: a token per receiver-owned **child presentation
  incarnation**, assigned at first publication and changed only after a child
  presentation has been destroyed and another published. On removal mark old
  incarnation terminal, without minting a new one. Puck native slot disconnect
  then reconnect creates a new epoch; DS4 probe/reconnect and DJ temporary
  link loss retain it; DJ unpair/re-pair and xone reuse create a new epoch.
  Companion-only changes, neutralization, lizard policy and report mode do
  not automatically replace a slot presentation. Use checked allocation and
  fail before IO on exhaustion. Do not equate this modeled lifetime with
  observed kernel/SDL node identity: the provider can prove only its own
  publication and protocol transitions unless independent host observation
  exists. Puck receiver HID interfaces survive native child detach; a host
  raw-client arbitration can independently recreate kernel input objects even
  when vg retains the receiver/slot presentation. Record actual host evidence
  separately when available.
- ADR-0024 stream epoch: command-history continuity scoped to the slot's
  controller protocol session, with its own opaque epoch and sequence. New
  presentation always means a new stream epoch; retained presentation does
  not establish uninterrupted protocol history. Protocol reset, uncertain
  delivery or link loss with unproved continuity closes its old stream and
  starts a fresh stream only on valid reinitialization. STOP/START preserves
  epoch only under the accepted journal continuity contract. Never compare a
  topology generation to a journal sequence or infer one from the other.

Owner decision: keep the receiver-specific incarnation rule in ADR-0022;
do not revive general ADR-0021/D3 or implement logical controller presentation
swap. Reopen ADR-0021 only on evidence that a logical controller needs
replacement of its whole presentation while retaining semantic resources
(e.g. proven Puck lizard or bootloader transition). This draft neither changes
ADR-0021 nor promises provider visibility of host-only re-registration.

### 3.4 Application events, state and journal

Expose retained topology and generation first (ADR-0019 pattern). After VG-2,
optional typed lifecycle observations can signal attach, unavailable-retained,
reconnected, removed, terminal and companion changes with receiver/slot
association, before/after presence and generation/token. A slow reader can
recover current availability/shape from one pollable snapshot; cannot infer
missed intermediate edges. Do not depend on callbacks to service host requests.

After VG-2, use the ADR-0024 typed in-process per-slot command journal at
semantic acceptance, plus
explicit lifecycle/rejected diagnostic discriminants when supported. A
receiver-level status or diagnostic belongs to receiver ownership, not a
fabricated controller accepted command. Each command is scoped to its slot
and stream epoch; no global cross-slot temporal order is promised. Match
journal transition with retained topology publication in the same local
service transaction, while allowing independent readers to observe at
different times. Distinct accepted identical writes receive distinct sequence;
retry does not; GET/read or absent rejection is not a new accepted command.
`host_outputs()` retains accepted latest values/revision, separately from
journal and optional observations. Typed per-slot service results distinguish
slot failure and sibling progress; retain sibling commands even if service
returns a slot error. Avoid exposing raw protocol packets as a generic event
bus.

Bound record count, payload bytes, drain batch and work per slot and receiver.
Retain final ring/high-water/gap/terminal metadata for the old owned stream
while its closed handle remains owned, as ADR-0024 stipulates. For replacement
provide a bounded handoff of the old closed incarnation to the owning caller
(or require drain before discarding it); receiver should not keep an unbounded
archive of departed occupants. Allocation and ownership of closed stream
views is an explicit later API design; stale cursors must never resume on the
new occupant and unavailable tails must be distinguishable. No producer
blocking, acknowledgment handshake or replay of effects on reattach.
USB/IP worker journal access stays refused pending separate versioned IPC
approval; retained snapshot operation 8 is not a stream. In-process
deterministic journal tests do not authorize a worker change.

### 3.5 Output to absent slots and failure classification

Receiver protocol first resolves destination and current occupant/session and
validates the native operation. Separate receiver-level status/query/control
from controller-directed mutation. For unavailable child, default refuse
controller mutation, report typed unavailable/rejected result at local API
and supply the evidence-backed native error/status where the transport admits
it. A one-way output has no invented acknowledgment; document the family
drop/rejection semantics. Retained node can receive host-originated requests
while link is down (DS4 output worker); service valid receiver-local operations
if defined, never assert remote execution or buffer stateful output until a
new occupant appears. If family absent semantics unknown, keep operation
unsupported for faithful tier pending evidence. Distinguish no destination
from malformed request and from delivery failure.

Once admitted, immutable required reply and input work keeps original session,
request ordinal, address, presentation/association and accepted bytes. Reply
to the original receiver request even if slot presence changed when that
endpoint and request still exist; acceptance is not host receipt. Before
address reuse/presence publication that could misroute stale work, drain
required original-session replies under bounded deadline; deliver old input
only within its valid original presentation/order. For definitely-unsent work
whose owning route no longer exists, explicitly cancel only where transport
contract proves cancellation; never rewrite bytes, silently drop a required
reply or deliver old frames to a new occupant. If safe delivery/cancellation
cannot be established, terminate the owning failure boundary. This is a
publication barrier, not the older compound full-frame retry. Only
DefinitelyUnsent retries immutable accepted work in the same owner boundary;
Submitted is transport submission, not physical effect; Uncertain is terminal,
never replayable. Do not relax existing regression on queued bytes; add tests
for ordering before reuse, explicit cancellation and terminal cases.

| Failure class | Proposed result |
| --- | --- |
| Validation/preflight or recoverable pressure | Reject candidate without changing accepted state; retain ordered pending work and bounded service, surface typed error. No sibling loss. |
| Isolated child link loss/detach | Execute its family node-lifetime policy; fence its output; preserve sibling identity, inputs, feedback and service. Retained-node loss does not allocate new presentation epoch. |
| Required slot-owned component fails; uncertain slot-owned transport | Terminate affected slot presentation if route isolation is proven; neutralize/fence and record slot terminal/cleanup, service other slots in bounded cycle. Report terminal rather than pretend retained-node continuity when required component is gone. |
| Primary/shared required interface fails, receiver unplug/reset, unscopable uncertain delivery, or invalid aggregate integrity | Receiver-wide terminal: cancel requests through transport close, close all required and child groups in deterministic reverse dependency order, preserve final per-slot diagnostics and terminal snapshot. No recovery in same handle or implicit independent devices. For DJ loss of one required receiver interface belongs here, not child-local. |

Keep a bounded service budget/fair round-robin for each live slot and shared
required replies, including empty receiver presence polling. Service must
not wait on applications or journal readers. If slot failure makes a
validated detach impossible (generation exhaustion, inability to fence shared
work), escalate terminally rather than leave incoherent live state. Cleanup
diagnostics must retain scope even across later reconnect.

### 3.6 Companion rules

Optional desktop roles are explicit family options, off by default and required
once selected: all preflight before opens; rollback new roles, preserve unrelated siblings;
neutral/release keys and pointer state on loss/close where possible, then
dispose of owned roles. LizardOnly suppresses native reports while retaining
native semantics according to the family policy, not a RealizationId, and
does not on its own authorize replacement or arbitrarily map a keyboard.
Do not infer real Puck desktop node layout from OpenPuck or raw-client
arbitration. DJ heterogeneous keyboard/pointer children are independently
paired endpoints, so a future DJ package needs explicit child roles and
evidence, not Puck's optional companion flags. Headset audio is a distinct
family subsystem, never a desktop companion by analogy.

## 4. First target and realization boundary

**Puck (#159/#165):** build common lifecycle seam and synthetic four fixed
addresses first. Source gives slot interfaces 2–5, IF 0–1 non-HID and IF 6
pogo (pogo not a fifth controller slot). Retail descriptors and presence,
pairing and slot routing need owner evidence. `rcv-puck-physical-slot-unknown`
remains open. Keep native Steam family options and inputs; do not copy wired
or OpenPuck compatibility identity into a receiver or rely on unclassified
0x7b traffic/watchdog constants.

| Tier / family | Intended scope, not a support claim |
| --- | --- |
| Puck reduced independent wired UHID/uinput | Explicitly reduced playable per-controller target only, without a common receiver identity/topology; no automatic fallback. The already existing wired constructor is separate; only offer a new receiver-owned reduced target if owner approves precise metadata/refusals. |
| Puck reduced receiver-ID UHID | Kernel/evdev or custom-driver experiment only. Pinned [SDL Linux HIDAPI](https://github.com/libsdl-org/SDL/blob/531acc0a76cf910b7616b2c5f63de2c5b3a4ba0a/src/hidapi/linux/hid.c#L779-L830) lacks a real USB parent/interface number on plain UHID; [SDL Triton](https://github.com/libsdl-org/SDL/blob/531acc0a76cf910b7616b2c5f63de2c5b3a4ba0a/src/joystick/hidapi/SDL_hidapi_steam_triton.c#L439-L456) accepts receiver interfaces 2–5. Native SDL Puck enumeration cannot be fixed by report payload alone (correction 6748258725). Steam client unknown. |
| Puck faithful USB-level | Select only after #168 proves real descriptor/control/interface/interrupt behavior including exact positive/negative GET/SET replies. Following the brief 17 owner precedent, USB/IP is the production target for a USB-level receiver, and `dummy_hcd` is a secondary experimental path used as needed. The current fixed HID gadget/f_hid is not adequate; neither target is existing validated composite support. Independent consumer bind tests for kernel and SDL, separate Steam test. |
| Xbox 360 receiver (later) | Four-interface xpad receiver needs modeled vendor-specific USB endpoints; SDL evdev is downstream. Reduced standalone uinput/HID is not xpad receiver. Require descriptors, provider and #258 amendment before production. |
| Xbox Wireless Adapter (later, conditional) | Dynamic xone association and USB/GIP need USB-level proof; out-of-tree xone is distinct from stock xpad. No GIP synthesis/fake, no firmware inspection, no receiver support if evidence or lawful capability missing. Firmware prerequisite gap **needs private analysis**. Reduced standalone xpad-shaped uinput is not adapter fidelity. #168 and owner authorization remain gates. |
| DS4 / DJ (comparison only) | DS4 retained-node parser-level UHID candidate, not real USB receiver proof; DJ UHID may model one child only, not shared composite receiver grouping. Full fidelity needs USB evidence and separate scope/audit. |

The #258 audit sign-off (comment 6777536122) covers its existing surfaces as
written, not future provider grants. Per the owner's 2026-11-11 rule, dev and
test host setup needs no per-change sign-off. Only receiver host setup that ships
as a release or default surface needs a #258 audit amendment and owner sign-off
before release: document the exact mechanism, who/required status, privilege,
scope/persistence, security and real-device impact, cleanup/reversibility,
alternatives/default and evidence. Keep test/dev provisioning explicit and off
by default; USB/IP is the production target, with `dummy_hcd` secondary for
experiments as needed (brief 17 precedent). No broad hidraw ACL, implicit module
setup or broker/worker protocol change. Neither path is validated by a source-only
paper. No RF, bond provisioning or real pairing operation is authorized.

## 5. Ordered implementation / PR boundaries (all proposed; not run)

Each PR is independently reviewable. The owner accepted the design in principle;
implementation remains post-Alpha 2 and waits for VG-1b family input/migration
work (including Steam step d), without touching its branches. Keep acceptance/
fidelity separate from deterministic code.

| PR / dependency | Scope and deterministic test oracle (NOT RUN) | Snapshot / migration |
| --- | --- | --- |
| 1 — regression seam | Extend existing synthetic fixture before production: 0, 1, 4 and family-specific larger/dynamic bounds without global four; invalid/duplicate addresses, exhaustion before preflight; both destroy/recreate and retained-link-loss (including DJ unpair/re-pair and DS4 probe absent); exact generation/epoch changes, duplicate/coalesced transitions, neutral gating, preserved siblings, rollback/cleanup and required companion failures. Keep existing regressions. | Internal only; expect no public snapshot change; verify. |
| 2 — ownership and fences | Factor only proven receiver lifecycle mechanism into production-internal family-owned seam, bounded route generations and fair servicing. Exercise absent SET/GET and one-way output, required reply queued at detach, definitely-unsent same-route retry, stale address/occupant reuse barrier, safe cancellation vs uncertainty, in-flight shared fault, DJ-like required receiver interface loss, primary error classification, sibling successful output retained despite local error, reverse close/Drop. Inject failures at each prepare/cleanup point. | Internal only, no supported receiver registry advertisement or IPC; verify snapshot unchanged. |
| 3 — typed Puck-facing API and reduced test target | After VG-1b Steam migration, typed creation/options/slot views, retained pollable topology and diagnostics first; optional typed lifecycle events and per-slot in-process journal are a separate follow-up after VG-2. Fixture or explicit reduced test-only target, no faithful Puck support. Tests: one validator path, absent/closed/stale slot edits, neutralize and metadata preservation, no alias/fallback or implicit wired conversion; in the later follow-up, independent repeated accepted commands vs retries, explicit journal gaps/reader lag/terminal drain/stream epoch separation and callback failure not starving required replies. If no reduced product target approved, keep API internal until PR 4. | Regenerate root `docs/api/ALPHA_API_SNAPSHOT.json` and affected Steam `docs/api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json`; curated/root compiling consumers migrated in same change, exact delta in `docs/ALPHA_API_MIGRATION.md`, no alpha compatibility aliases. |
| 4 — Puck USB provider / package, after #168 evidence | Separate USB/IP provider feasibility (secondary `dummy_hcd` experiments as needed) from slot protocol implementation; dev/test setup needs no per-change sign-off, but shipping release/default host setup needs an approved audit amendment. Begin with fake USB event/endpoint adapter and evidence-backed native descriptors/ID/status/reply routing tests, no speculative bytes. Test zero-slot listener, valid attach/status, GET/SET success and negative reply, independent slot routing, bounded simultaneous churn, per-slot lizard policy, shared failure and cleanup after denied access. Any worker wire change/journal needs separate approval. | Explicit manifest/surface/refusal and docs only in later implementation PRs; regen only affected API snapshots. No change to current wired realization meanings. |
| 5 — consumer and physical gate | Run denied/privileged setup, teardown and host tests below on exact candidate, fix issues with focused deterministic regressions; separate support matrix change after evidence and owner approval. | Publish only reviewed non-identifying evidence; no automatic support promotion or release gate changes. |
| 6 — later USB receiver families | Separate issue/PR per Xbox 360 then conditional XWA; no new common abstractions without actual need. Add dynamic and retained policies only as justified. GIP never faked. DS4/DJ comparison cases remain deterministic patterns until separately selected. | Each new family gets scoped snapshots, migration, capability/limitation documentation and host audit. |

### Live and physical test plan — NOT RUN

- Retail owner-operated sanitized reference inspection: descriptors and
  interface grouping, passive status sequences/attach/detach and slot count,
  identity scope, actual consumer node and SDL instance lifetimes; no private
  values in public repo. Keep electrical/radio actions and pairing owner-only.
- On exact virtual candidate and recorded kernel/SDL versions, compare zero,
  one and four occupied Puck slots, rapid repeated connects/disconnects and
  simultaneous churn while siblings produce distinct neutral/active inputs;
  assert correct interface binding, controls, independent output and no
  stale-instance mutation. Record actual evdev object/SDL add-remove rather
  than infer from a reused event number. Check raw/native arbitration and
  companion create/remove separately. Plain receiver-ID UHID native SDL
  exclusion is a source prediction to test, not live evidence.
- Inject loss during pending feedback; distinguish host acceptance, reply
  delivery, queue cancellation and physical execution. Unplug/reset and
  required shared-interface fault versus one isolated slot loss, repeated
  close, deny permissions, no leaked handles/nodes or global host-policy
  changes. Test setup is explicit and off by default; no per-change dev/test
  sign-off is required, and no unsafe driver unbind on real hardware.
- Benchmark exact creation/first-valid-neutral/removal with #166 oracle when
  available; no timing inference from source. Steam on x86-64 remains #144;
  SDL bind does not imply Steam acceptance. xpad/xone later need their own
  physical reference, binding/hotplug/output/teardown and capacity tests.
  Preserve failed/unknown and distinguish deterministic, virtual consumer,
  physical and transport evidence.

## 6. Owner decisions (2026-11-17)

Source of truth: [#165 comment 6960320698](https://github.com/anderstvoss/virtualgamepad/issues/165#issuecomment-6960320698).
The owner accepted all eight recommended answers with the two amendments in Q6.

1. **Slot declaration.** Each receiver family declares its own address scheme,
   capacity and node-lifetime policy (destroy/recreate or retain through link
   loss). Puck has four fixed slots; there is no global four-slot maximum or
   generic arbitrary USB/HID composition.
2. **Reconnect tracking.** A receiver-scoped per-slot presentation token changes
   only when a child presentation is destroyed and recreated, under ADR-0022.
   ADR-0021 stays Deferred (D3 closed 2026-11-09) and reopens only on evidence of
   whole-presentation replacement.
3. **Application visibility.** First expose a retained receiver topology snapshot
   and generation (ADR-0019); add optional typed lifecycle events and per-slot
   ADR-0024 in-process journal entries later, after VG-2. Topology generation and
   stream epoch remain separate; USB/IP worker journal stays refused pending
   separate approval.
4. **Output to absent slots.** Refuse by default; never queue or replay output
   to a later occupant. Required replies retain their original order and session.
   Allow only proven cancellation or terminate the owning failure boundary;
   uncertain delivery is never retried.
5. **Inputs across reconnect.** New occupants or presentations start neutral;
   held inputs are never replayed, and input is gated while link is lost.
   Identity, player assignment and protocol clocks are never input-snapshot data.
6. **Puck first realization.** Synthetic lifecycle tests and explicitly reduced
   kernel/evdev experiments come first; native Puck support in SDL and Steam
   requires a USB-level device proven under #168. Following brief 17, USB/IP is
   the production target and `dummy_hcd` is secondary and experimental, used as
   needed. Per the 2026-11-11 owner rule, dev/test host setup needs no per-change
   sign-off; only release/default host setup needs an audit amendment and owner
   sign-off.
7. **Failure classification.** A proven-isolated slot failure stays slot-local;
   shared required interface loss or unscopable uncertain delivery terminates the
   receiver (Logitech DJ case). Companions are explicit, required once selected,
   and off by default; no generic DJ heterogeneous runtime.
8. **Sequencing.** The Xbox 360 USB-level receiver follows Puck and its host audit.
   Xbox Wireless Adapter requires #168 evidence and an owner-approved non-faked
   GIP path, or remains unsupported. DS4 dongle and Logitech DJ remain
   deterministic policy fixtures until separately scoped.

## Scope exclusions and validation of this draft

Only this spec and the appended ADR-0022 proposal change. VG-1b steps d/e
(local agent), #367 release-prep, #372 neutral oracle, parallel Alpha 2
readiness/migration audit on #365, DJ registry follow-up, research #147
transport tag cleanup, and briefs 19–28 remain their respective owners'
work. No acceptance, release, snapshot, runtime or corpus files changed.
No hardware, host, firmware or timing tests run; all proposed tests above
are plans. Connector-only authoring: repository scripts/Cargo were not run
locally; rely on the draft PR's observed CI, not an assumed pass.
