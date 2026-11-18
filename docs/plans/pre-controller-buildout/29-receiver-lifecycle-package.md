# Brief 29: Receiver lifecycle package (#165, #159)

Read [ground-rules.md](ground-rules.md), [brief 08](08-epochs-and-receivers.md),
[brief 14](14-steam-controller-wired-package.md), [brief 17](17-gamecube-adapter-package.md),
[receiver lifecycle design](../../spec/receiver-lifecycle-design.md),
[ADR-0022](../../adr/ADR-0022-receiver-aggregates.md), [Steam wired guide](../../STEAM_CONTROLLER_2026.md),
[input model](../../spec/input-model-state-and-mutation.md), ADR-0019, ADR-0024,
#165, #159, #168, #258 and #322 first. Brief 08b's deterministic fixture shipped;
this brief refines its production follow-up, not 08a's deferred general epoch
proposal. Planning only; implementation is not authorized by merging this brief.

**Lane:** Runtime / Controller (Steam Puck first).
**PR boundaries:** 29a–29i, one independently reviewable implementation PR per
step against main; no new checkpoint branch.
**Blocked by:** this brief merged; post-Alpha 2 owner start authorization;
VG-1b complete (including Steam step d and remaining migration work);
reviewed evidence adopted at the repository submodule pin. Later steps have
additional gates below: VG-2 family journal integration for 29d; #168 provider
proof for 29e–29i native USB work; #258 amendment and owner sign-off for
release/default host setup; eligible hosts/reference hardware for live gates.
Keep parallel VG-1b and Alpha 2 work untouched.

## Goal and decided context

Implement one receiver-owned transport, protocol, family-specific address
policy, typed logical occupancy and independently scoped native controller
slots. Begin with shared lifecycle mechanics and the Steam Puck's four fixed
addresses 0–3; do not impose four on other families. The accepted ADR-0022
contract remains in force; its amendment is accepted in principle, not yet
normative, and requires owner review of implementing PRs. Use the
[owner decision #165, comment 6960320698](https://github.com/anderstvoss/virtualgamepad/issues/165#issuecomment-6960320698)
(2026-11-17; Q1–Q8) as the decision record throughout.

- **Q1/Q2:** compiled per-family address scheme, capacity and node-lifetime
  policy. Receiver instance, slot address, occupant, child presentation
  incarnation, host node name and player assignment are distinct. The
  receiver-scoped presentation token changes only when a child presentation is
  destroyed and recreated. ADR-0021 stays Deferred (D3 closed 2026-11-09);
  reopen only with evidence of whole-presentation replacement.
- **Q3:** retained receiver topology snapshot and checked generation first,
  patterned on ADR-0019; optional typed lifecycle events and per-slot ADR-0024
  in-process journals later, after VG-2. Stream epoch and topology generation
  are separate; USB/IP worker journal stays refused pending separate approval.
- **Q4/Q5:** absent controller output refused by default, never queued or
  replayed for a later occupant. Required replies retain original session and
  order; only proven cancellation is allowed, otherwise terminate the owning
  boundary. Never retry uncertain delivery. A new occupant or presentation
  starts neutral, no held-input replay; gate active input through link loss
  (fresh input required on resume). Identity, player assignment and protocol
  clocks are not input snapshots.
- **Q6(a,b):** deterministic tests and explicitly reduced kernel/evdev
  experiments first. Native Puck SDL/Steam needs real USB-level evidence under
  #168. USB/IP is the production target; dummy_hcd is secondary experimental,
  used as needed. Dev/test host setup needs no per-change sign-off. Only setup
  shipping as release/default needs an audit amendment and owner sign-off.
- **Q7/Q8:** proven-isolated slot failures stay local; required shared
  interface loss or unscopable uncertain delivery terminates the receiver.
  Companions explicit, required once selected, off by default; no generic DJ
  heterogeneous runtime. Xbox 360 USB receiver after Puck and its host audit;
  Xbox Wireless Adapter only with #168 evidence and owner-approved non-faked
  GIP path, otherwise unsupported. DS4 dongle and DJ stay deterministic policy
  fixtures until separately scoped.

Non-goals: generic USB/HID composition, dynamic runtime profiles, GamepadManager
player allocation, IUCM conversion, implicit independent-device fallback,
generic mutable gamepad/event bus, arbitrary desktop injection, real RF/pairing,
bond provisioning, undocumented reports, vendor firmware analysis (needs private
analysis), worker IPC changes, new release gates or promotion based on source
alone. Direct wired Steam creation, accepted capabilities and reports remain
unchanged; do not reuse wired identity/descriptor or OpenPuck compatibility
identity for the official receiver.

## Evidence and function matrix

VG inspected at [a0398b75](https://github.com/anderstvoss/virtualgamepad/commit/a0398b75010ba134839379f0c0dc87fba77cab1f)
(the design's earlier implementation baseline was 12358d71); research at
[a67781e2](https://github.com/anderstvoss/controller-protocol-research/commit/a67781e2412eceea9440d20dbb350ad5d3352c25).
VG currently pins research at eba02ad610e264a2518a699394255cc7f1d670dd;
reading research main is not adoption. All claim IDs below denote
[records/claim/<id>.json](https://github.com/anderstvoss/controller-protocol-research/tree/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim)
at that research revision. Adopt a reviewed pin and validate the corpus before
implementation; cite the relevant claim and its pinned source/line ranges
beside each implemented constant. Restate GPL-backed sources, never paste them.

| Research brief / merge | Verified claim IDs (source expectations except where marked) |
| --- | --- |
| [Puck lifecycle](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/docs/receivers-puck-consumer-lifecycle.md), #127 at [b9f1f076](https://github.com/anderstvoss/controller-protocol-research/commit/b9f1f076ae7c25a08a7b7272dcafd32f738f3e3b) | `rcv-puck-kernel-per-interface`, `rcv-puck-kernel-presence`, `rcv-puck-sdl-presence`, `rcv-puck-kernel-attach`, `rcv-puck-kernel-detach`, `rcv-puck-sdl-joystick-lifecycle`, `rcv-puck-identity-battery`, `rcv-puck-output-routing`, `rcv-puck-lizard-arbitration`, `rcv-puck-teardown`; `rcv-puck-reconnect-epoch-inference` (inferred), `rcv-puck-physical-slot-unknown` (gap). |
| [Xbox 360 receiver](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/docs/receivers-xbox360-wireless.md), #128 at [7652f68a](https://github.com/anderstvoss/controller-protocol-research/commit/7652f68aab41ff278a4d0a66b708af6811b3b593) | `rcv-x360-binding`, `rcv-x360-four-slot-consumer`, `rcv-x360-presence`, `rcv-x360-evdev-epoch`, `rcv-x360-identity-led`, `rcv-x360-output`, `rcv-x360-teardown`, `rcv-x360-sdl-evdev`, `rcv-x360-companion-battery`; `rcv-x360-output-epoch-gap`, `rcv-x360-usb-provider` (inferred), `rcv-x360-physical-unknown` (gap). |
| [Xbox Wireless Adapter](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/docs/receivers-xbox-wireless-adapter.md), #134 at [8e34fa33](https://github.com/anderstvoss/controller-protocol-research/commit/8e34fa33240603a08c5a3d366878846d9545a5aa) | `rcv-xwa-association-queue`, `rcv-xwa-node-lifetime`, `rcv-xwa-slot-reuse`, `rcv-xwa-output-routing`, `rcv-xwa-failure-teardown`, `rcv-xwa-per-client-surfaces`, `rcv-xwa-mainline-xpad`, `rcv-xwa-sdl-linux`; `rcv-xwa-virtual-boundary` (inferred). |
| [Cross-receiver decision table](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/docs/receivers-lifecycle-decision-table.md), #135 at [117cf5c9](https://github.com/anderstvoss/controller-protocol-research/commit/117cf5c9eed414443ceab5e828e19b9fde7c34bb) | `rcv-pattern-ds4-dongle-presence`, `rcv-pattern-ds4-node-persistence`, `rcv-pattern-ds4-output-boundary`, `rcv-pattern-dj-receiver-children`, `rcv-pattern-dj-linkloss-unpair`, `rcv-pattern-dj-routed-output`, `rcv-pattern-dj-interface-failure`. |
| [DJ registry](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/controller/logitech-dj-receiver.json), #148 at [5df072bd](https://github.com/anderstvoss/controller-protocol-research/commit/5df072bd518504c81ed240c9c24090be4dfc1cf1) | Same four DJ IDs from #135, re-filed under `logitech-dj-receiver` without elevating provenance. Pattern only; not a gamepad profile. |

The decision table's Xbox draft wording is historical; both PRs are merged.
Puck consumers distinguish HID slot interfaces 2–5 from non-HID interfaces
0–1 and pogo interface 6 (not a fifth slot). SDL dispatches 0x46/0x79, not
0x7b, and input can imply presence; kernel status query and 0x79 are
per-interface. Consult the pinned [SDL Linux enumeration](https://github.com/libsdl-org/SDL/blob/531acc0a76cf910b7616b2c5f63de2c5b3a4ba0a/src/hidapi/linux/hid.c#L779-L830)
and [Triton binding](https://github.com/libsdl-org/SDL/blob/531acc0a76cf910b7616b2c5f63de2c5b3a4ba0a/src/joystick/hidapi/SDL_hidapi_steam_triton.c#L439-L456):
UHID bus metadata is not the actual USB parent/interface. For GPL-backed
consumer behavior use the pinned line-range references in the linked research
briefs, not excerpts. Do not adopt unclassified reports or watchdog values.

| Planned scope | Lifecycle / routing | Realization boundary / status |
| --- | --- | --- |
| Puck fixed four addresses | Native presentation removed on detach; new token/neutral on attach, receiver interfaces survive; per-slot battery, identity, outputs, explicit selected lizard roles | Synthetic WIP; reduced explicit experiments only (kernel/evdev); native SDL and Steam WIP, USB proof required. |
| Xbox 360 wireless receiver | Four-interface/pipe consumer model; per-interface presence and new evdev incarnation, independent FF/LED; host LED index not player or persistent identity | WIP later; vendor USB interfaces and interrupts required for xpad, SDL sees downstream evdev. No stock UHID binding or reduced equivalent claim. |
| Xbox Wireless Adapter | Dynamic identify/add/remove; 16 software entries not measured physical capacity; new incarnation on reuse, per-client output incl. optional separate audio subsystem | Unsupported until gates; xone separate from stock xpad. No fake GIP or standalone uinput equivalence. |
| DS4 dongle | One probe-created presentation retained through link loss; readiness/calibration gates active input; retained output channel not remote actuation | Deterministic fixture only. Parser-level UHID possibility is not dongle fidelity. |
| Logitech DJ | One-based indices 1–7, not measured concurrent capacity; heterogeneous pair-created children retained through link loss, removed on unpair; shared required interface loss removes all | Deterministic fixture only; single UHID child cannot prove shared USB grouping. |

Track native controls, motion/touch, identity/battery, presence, topology,
haptics/indicators, companions, required responses, slot/shared cleanup and
transport separately. Reduced targets must document/refuse unavailable
functions with regression tests. No promotion today: source, inference,
deterministic, virtual consumer, transport and physical evidence are separate.

## Ordered steps (one reviewable PR each; all tests and acceptance plans NOT RUN)

Paths below identify existing seams and proposed locations, not a frozen API or
mandatory file layout. Recheck main, evidence, authorization and dependencies
before every step; keep file diffs narrow and no compatibility aliases.

**29a. Core lifecycle model and regression oracle (Q1, Q2, Q4, Q5, Q7).**
Extend the shipped receiver aggregate fixture first, then extract a minimal
internal typed family-policy seam only where justified by the tests. Separate
address from occupant, association, availability and presentation incarnation;
model vacant, pending attach/identify, present, retained link-lost and terminal.
Checked generation and token allocation before IO, atomic publication of
multi-slot changes, deterministic preflight/preparation and reverse rollback.
No global four-slot assumptions or public receiver registry advertisement.
**Files:** `crates/gr-hid/tests/receiver_aggregate.rs`,
`crates/gr-curated-controllers/src/registry.rs` (internal metadata only as needed),
new crate-internal lifecycle module as justified; fixture spec
`docs/spec/receiver-aggregate-fixture.md`.
**Fake tests:** empty/one/four, larger/dynamic and one-based synthetic policy
bounds; invalid/duplicate address, overflow, no-op/coalesced changes,
batch rollback at every prepare stage, raw ownership arbitration not changing
token, DS4 initially absent and readiness/resume without token change,
DJ link loss vs unpair/re-pair (fresh token), stale handle rejection, neutral
resume and sibling survival. Keep old fixture regressions.
**Done:** no public snapshot/IPC change; lifecycle and fixture checks green.

**29b. Ownership, publication fences and service failure (Q2, Q4, Q5, Q7).**
Factor proven common scheduling/ownership primitives into production-internal
seams. Fencing uses receiver, address, occupant/presentation and original
session/request ordinal; retain immutable pending bytes and ordering.
Before reuse drain required original-session replies within bounded budget;
cancel definitely-unsent only where transport proves cancellation, otherwise
terminate owning boundary. Only DefinitelyUnsent retries same-route immutable
work; Submitted is submission, not actuation; Uncertain terminal, never replay.
Receiver-local status/query is distinct from controller mutation; absent
default refusal, evidence-backed native replies only, no invented one-way ACK.
Fair bounded round-robin, including empty receiver service; distinguish pressure
from terminal loss, preserve sibling progress and final diagnostics.
**Files:** receiver lifecycle internal module from 29a,
`crates/gr-hid/src/lib.rs`, `crates/gr-hid/src/group.rs`,
`crates/gr-hid/tests/receiver_aggregate.rs` (only necessary changes).
**Fake tests:** GET/SET and one-way absent requests; pending reply during
detach, ordering before reuse, same-route retry, explicit cancellation,
uncertainty with/without proven isolation, exhausted detach generation,
shared interface loss/reset while attach pending, sibling observations despite
slot error, reverse cleanup failures and repeated close/Drop. No lost required
completion at polling boundary; preserve queued bytes regression.
**Done:** no provider-owned protocol or public API change; bounded ownership
tests green.

**29c. Typed Puck package, fixed four slots and retained readback (Q1–Q3,
Q5–Q7).** After Steam VG-1b migration, implement receiver-specific address
policy and internal typed slot views over `SteamController2026Inputs`,
using the same final-candidate validator for inputs(), set_inputs, edit_inputs,
setters and neutralize; no metadata/clocks/output mutation via inputs.
Coherent retained receiver topology and generation advance once per effective
membership, availability, incarnation or role change, never duplicate/reject/
poll/eviction; readback and terminal reason survive close, with no wrap.
Proposed creation shape create_steam_puck(CreationOptions, SteamPuckOptions)
in curated/root layer **only once API exposure is authorized**; direct wired
constructor stays unchanged. Required selected companions preflight together,
release held keys/pointer state on loss, LizardOnly is controller policy not
RealizationId. Start on fake providers; reduced receiver-ID UHID is a
kernel/evdev experiment, optional independent wired UHID/uinput is reduced
and does not provide shared receiver identity. No reduced product target
without owner approval of exact metadata/refusals; keep API internal until
native package if not approved.
**Files:** `crates/gr-curated-controllers/src/registry.rs`,
`crates/gr-curated-controllers/src/steam_controller_2026.rs` and
`steam_controller_2026/{controller,definition,topology,evdev,lizard}.rs`
(share validator/policy, not wired protocol/identity), proposed adjacent Puck
receiver module, `src/controllers.rs` and relevant crate exports only upon
authorization; deterministic tests and receiver guide as needed.
**Fake tests:** empty listener, distinct four typed occupants, reject fifth,
invalid family/address, atomic invalid input, stale/closed/absent mutation,
fresh neutral reconnect, no metadata changes, companion capability refusal/
partial rollback, sibling service and lizard changes without token churn,
explicit target choice/no fallback. Reduced host experiment tests see 29f.
**Done:** retained readback first, no journal requirement for creation;
snapshot unchanged if internal, reviewed exact delta if public.

**29d. In-process per-slot host-output journal and optional lifecycle
observations (Q2–Q5, Q7).** Separate follow-up after VG-2 integration, not
prerequisite for the retained snapshot. Use ADR-0024 semantics at accepted
command, independently scoped by slot stream epoch; new presentation starts
new stream epoch, retained presentation alone does not prove continuity.
Loss with unproved continuity closes stream; reinitialize only on valid
protocol start. Distinguish receiver diagnostics, rejection, accepted commands
and latest host_outputs readback. Preserve closed owned ring/high-water/gap/
terminal metadata, bounded handoff or drain requirement; no unbounded
departed-occupant archive, stale cursor reuse or cross-slot total ordering.
Do not settle closed-view ownership or event API without owner review.
**Files:** internal receiver/Puck modules from 29c, existing VG-2 journal
seams in curated/core, receiver tests; approved API snapshots/migration only
if exposed. No USB/IP worker journal or IPC changes.
**Fake tests:** identical accepted writes get distinct sequences, retries
do not; GET/rejection not accepted command, retained output vs history,
bounded eviction/gaps/reader lag, old epoch terminal and drain, sibling
independence, optional observation loss recoverable via topology, no
application callback starvation of required replies.
**Done:** in-process only, separate generation/stream semantics proven.

**29e. USB-level Puck provider feasibility (#168, Q6a/b, Q7).** Independent
transport PR after source/descriptor evidence gates and 29c, not speculative
receiver bytes. Prove actual composite interface/control/interrupt transport
for the native receiver (USB/IP production; dummy_hcd secondary experiment).
Existing fixed HID gadget/f_hid is insufficient; provider carries transport,
not Puck lifecycle meaning. Use fake USB event/endpoint adapter first; stop
if protocol/version/worker wire changes require authorization. No worker
journal. Audit applies to shipping/default host setup, not each dev/test
change; preserve explicit disabled-by-default test provisioning.
**Files:** `crates/gr-usbip/src/{lib,control,profile,pending,worker}.rs`
only as evidence requires and existing transport tests; targeted #168
feasibility documentation. Do not change broker wire format without approval.
**Fake tests:** interface selection (including non-slot roles), endpoint
direction/type, real-control request routing including positive/negative
completion, immutable pending bytes, denied resource, partial creation,
reset/unplug and required interface failure, cancellation proof, no leaked
sessions. **Done:** record exact #168 proof separately; if missing, native
steps stay blocked, no fallback.

**29f. Native Puck protocol and receiver realization (Q1–Q8).** After 29e
and reviewed Puck descriptors/status/routing evidence; adopt only evidenced
identity, interfaces, attach/detach/feature/status and per-slot data/feedback
semantics, with typed creation/manifest/refusals. Four slot interfaces plus
reviewed non-slot roles, not five slots or four ordinary devices.
Use family-owned native input, output, battery, presence and companion
policy; no fabricated pairing, bond/identity values, unsupported reply
semantics or physical cadence. 29d journal integration when approved remains
in-process; not required to change worker wire.
**Files:** Puck module from 29c, shared Steam validator/role seams,
`crates/gr-curated-controllers/src/registry.rs`, `src/controllers.rs`,
reviewed fixture manifest/records at the adopted evidence pin, receiver
guide, affected API snapshots and migration.
**Fake tests:** exact reviewed report lengths/roles and presence, status
duplicate/coalesced and data-implied attach where evidenced, all supported/
rejected GET/SET and one-way requests, four distinct slot inputs/feedback,
independent attach/teardown, pending output during churn, denied setup,
shared fault and explicit companion rollback.
**Done:** deterministic native transport/protocol suite and explicit target
limitations; no support promotion yet. Reduced kernel/evdev experiments from
29c may run before native USB proof, with explicit reduced designation.

**29g. Puck acceptance and host audit (Q6a/b, Q7, Q8).** On recorded candidate
and host run the acceptance plan below; verify shipping/default host setup
audit amendment and owner sign-off before release or declaring the Puck host
audit complete. Fix defects at lowest seam with fake-provider regressions;
separate evidence/matrix PR after review. Do not claim SDL implies Steam.
**Files:** acceptance manifests/reports under existing `docs/` and acceptance
runner seams only where needed, receiver guide and `docs/CONTROLLER_SUPPORT.md`
only after approval; #258 audit documentation as applicable, no implicit
host-policy changes.
**Fake tests:** preserve all 29a–29f regressions; inject any discovered
lifecycle/transport defect before fixing it. **Done:** exact evidence and
negative results recorded, audit disposition explicit. Xbox 360 next only
after Puck and its host audit.

**29h. Xbox 360 wireless receiver (Q1–Q5, Q7, Q8).** Later distinct family
PR after 29g, reviewed evidence and #168 USB vendor transport proof, with
its own host audit as applicable. xpad binds vendor interfaces with two
interrupt endpoints, presence drives per-interface node lifecycle; four is
consumer model pending retail descriptor/capacity evidence. Use native
family inputs and distinct typed identity/LED/battery/output; no headset/
desktop companions by analogy, no unknown absent-output success.
**Files:** proposed Xbox 360 receiver module adjacent to existing Xbox 360
family, compiled registry, root facade/snapshots only for reviewed typed
surface, USB provider only for proven additional transport requirements,
family guide/tests.
**Fake tests:** listener with no devices, distinct virtual slot routes,
per-interface attach/remove/reuse, LED/FF isolation, stale work fencing,
required replies/one-way semantics as evidenced, denied setup, shared loss,
reverse close. **Done:** scoped deterministic support and independent
xpad/SDL acceptance plan below; preserve unknown physical capacity and
actuation. Explicitly reduced independent pads are not the receiver.

**29i. Xbox Wireless Adapter conditional scope (Q1–Q5, Q7, Q8).** Only after
29h and #168 proof plus owner-approved non-faked GIP path, otherwise leave
unsupported and do not open implementation PR. Dynamic bounded client
allocation/identification with fresh incarnation on removal/reuse; software
16-entry table not a physical count. xone is out-of-tree and separate from
stock xpad; SDL evdev downstream. No firmware inspection (needs private
analysis); no synthesized GIP or inferred pairing/optional audio support.
**Files:** proposed separate adapter family module, compiled registry and
approved typed API/snapshots, provider transport seam only as evidenced,
adapter guide/tests. **Fake tests if authorized:** bounded dynamic allocation,
duplicate and pending identification, failure before publication, removal/
reuse fencing, per-client output and explicit unsupported subsystem refusal,
sibling survival and shared reset/unplug, close/cleanup.
**Done:** defer unless all gates met; independent xone/SDL acceptance and
audit as needed. DS4 and DJ remain policy fixtures from 29a/b; no production
registry/API/host implementation without separately scoped owner work.

## Host setup and audit impact

Q6 amendments control: USB/IP production target, dummy_hcd experimental
secondary path as needed, neither established composite Puck support today.
No per-change owner sign-off for dev/test host setup; explicit optional,
reversible test setup only. A shipping release/default setup change needs
#258 amendment and owner sign-off, specifying exact mechanism, affected
users/required status, privileges, scope/persistence, real-device/security
impact, cleanup/reversibility, alternatives/default and evidence. Existing
#258 sign-off is not blanket approval for new surfaces. No broad hidraw ACL,
implicit module load, privileged CI, real-device unbind or broker/IPC change.
Do not change host defaults or release policy to make experiments pass.

## Snapshot, migration and acceptance consumers

Follow #322 alpha API policy and owner review: update only affected
`docs/api/ALPHA_API_SNAPSHOT.json` and
`docs/api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json` (or separately approved
family snapshots) for actually authorized/exposed surface, regenerate with
`scripts/check-alpha-api.py --write`, verify unchanged unrelated snapshots,
migrate compiling consumers in same PR and record exact delta in
`docs/ALPHA_API_MIGRATION.md`. Internal steps keep snapshots unchanged.
Proposed names and closed stream ownership are not pre-approved.
Future steps run ground-rules/AGENTS checks and corpus checks where relevant.

**Acceptance plan — NOT RUN (all steps):**
- Owner-operated sanitized Puck reference: actual descriptors/interfaces,
  attachment/status ordering, per-slot routing/identity scope, capacity and
  actual kernel/SDL node/instance lifetime; no private values or firmware
  analysis. Radio/pairing actions owner-only. Explicit open physical gaps
  remain (`rcv-puck-physical-slot-unknown`).
- Reduced kernel/evdev experiment with recorded target, kernel and consumer;
  check expected binding, representable controls, output/refusals, close;
  verify predicted native SDL exclusion on plain receiver-ID UHID, not just
  packet encoding. No claim of shared USB fidelity.
- Native Puck USB candidate after #168: record revision, transport, host/kernel,
  SDL version, commands/results; zero/one/four active slots, repeated and
  simultaneous churn with distinctive inputs/outputs, exact interface binding,
  neutral-first/reconnect, independent outputs, stale mutation refusal,
  node add/remove and instance lifetime independently observed rather than
  inferred from reused paths; verify optional companion and raw/native
  arbitration separately. Steam x86-64 #144 recognition, controls, output and
  lifecycle separate from SDL. No assertion of physical actuation from USB
  submission.
- Denied setup, unplug/reset during pending reply/output, isolated loss vs
  shared required interface failure, bounded teardown under churn, multiple
  concurrent receivers, no leaked resources; do not unbind real hardware.
  #166 first-valid-neutral, creation-ready/removal oracle/timing on recorded
  host, benchmark before choosing budgets; preserve failures and unknowns.
- Later Xbox 360 and conditional XWA: independently verify sanctioned retail
  descriptors/capacity, native xpad or xone bind, downstream SDL, ready/
  hotplug/remove, per-client controls/feedback and teardown. No physical
  promotion without reference; no XWA run if GIP or USB evidence missing.
- Promote only per-function cells with reviewed deterministic + virtual
  consumer evidence (and physical reference for physical tier) and owner
  approval. Record exact candidate, command, host/consumer versions,
  provenance, negative results and unmeasured dimensions separately.

## Open questions for Anders (not answered by Q1–Q8)

- Exact public receiver creation/options/slot handle names and surface;
  when to expose internal Puck API, and whether a reduced receiver-owned
  product target is desired (exact metadata, capability/refusal contract).
- Later optional lifecycle observation API and bounded handoff/ownership or
  drain rule for closed journal views. No worker journal absent new approval.
- Puck retail descriptors, precise absent/rejection/status behavior, attach
  order, routing, actual identity/battery/indicator/companion semantics,
  pairing, cadence and node observations; unknown behavior stays unsupported.
- Exact #168 provider feasibility and required interface/endpoint implementation;
  authorization for any worker/broker wire change; shipped setup audit scope,
  eligible hosts and owner-operated reference sessions. No per-change dev/test
  sign-off requirement.
- Xbox 360 retail simultaneous capacity and revision/descriptor/output gaps;
  XWA viable approved non-faked GIP path (firmware-dependent gaps need private
  analysis). Separate DS4/DJ production scope if ever wanted.

All tests above are future plans, not run. This brief changes documentation
only; no hardware observations, new support claims, code or API authorization.
