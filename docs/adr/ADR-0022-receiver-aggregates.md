# ADR-0022: Receiver-owned presentation with typed controller slots

Status: Accepted in C8. C4 shipped deterministic evidence for the contract.

## Context

A receiver can present several logical controllers through one host device.
The Steam Puck is a candidate consumer (#159, #165), with four protocol slots;
Joy-Con pairs are a fixed two-slot case. Those slots are protocol addresses,
not GamepadManager player assignments or global application slot allocation.

## Decision

A receiver handle owns its host presentation, receiver protocol and typed slot
handles. Its compiled registry entry has kind `Receiver` and names the hosted
families. Each slot retains its own controller-native state, presence, feedback
routing, identity and teardown. Slot changes are validated transactionally as a
receiver-owned operation; invalid combinations leave the live aggregate intact.

Connecting a slot preflights all selected required components before opening any
resource. Partial creation rolls back in reverse order; rollback failures remain
diagnostic. Disconnecting a slot neutralizes/removes only that slot and its owned
companions. Siblings retain their state and presentation. Receiver-wide terminal
failure closes the entire group deterministically and idempotently, with no
implicit fallback to independent devices.

The receiver routes host requests to the named protocol slot, acknowledges every
supported or rejected request and preserves accepted required replies across slot
changes. Non-repeatable commands retain immutable accepted bytes: only
`DefinitelyUnsent` can retry; `Uncertain` terminates the owning presentation.
The older compound full-frame retry model is not sufficient for a live receiver
and must not be used to replay ambiguously delivered slot requests.

Optional keyboard/pointer companions are explicit required components when
selected. Their creation and removal share receiver preflight/rollback boundaries;
they are not silently added or degraded. Slot-local loss and receiver failure
remain distinct. The separate [presentation epoch proposal](ADR-0021-presentation-epochs.md)
is deferred.

## Alternatives and consequences

One independent host device per slot cannot express a faithful shared receiver
presentation, receiver-wide handshakes or shared failure. It is rejected as an
implicit receiver implementation. Such individual devices may remain explicitly
selected reduced realizations with limitations recorded on their surfaces.

Reusing application player slots for protocol routing is rejected because player
assignment can change independently of receiver attachment and pairing. Generic
provider-owned slot policy is also rejected: controller/receiver code owns report
meaning and valid transitions; providers carry transport operations only.

Step 13b shipped the [synthetic receiver fixture](../spec/receiver-aggregate-fixture.md)
with zero to four typed slots, required request acknowledgements, companion
rollback, per-slot failure isolation and receiver-wide terminal cleanup. The
compiled registry vocabulary includes `FamilyKind::Receiver`; the receiver
fixture and metadata test do not advertise a production realization.

The receiver handle outside tests, including any ordinary-root slot or aggregate
API, is deferred under [#165](https://github.com/anderstvoss/virtualgamepad/issues/165).
Puck and Joy-Con implementations require their own evidence and scopes.

## Proposed amendment (draft, not accepted)

See the [receiver lifecycle design](../spec/receiver-lifecycle-design.md) for
evidence, proposed API, validation and open owner questions. No status, existing
decision or implementation authorization changes until owner review.

- Retain one receiver-owned transport/protocol, typed independently scoped
  slots and transactional preflight/rollback; declare bounded address scheme,
  capacity and node-lifetime policy per compiled receiver family, not a global
  maximum of four. The Puck remains four fixed controller slots.
- Separate presence from child presentation lifetime. Destroy/recreate on
  detach/reconnect for Puck, xpad and xone; keep DS4 and DJ nodes through
  ordinary link loss, remove DJ children on unpair. Do not infer physical
  capacity or bond persistence from consumer tables.
- Record a receiver-scoped child presentation incarnation on first publication
  and allocate a new one only upon destruction and recreation, not retained
  link loss. Keep ADR-0021 deferred; no general presentation swap layer.
  Receiver association, protocol address and application player slot differ.
- Extend ADR-0019's retained topology pattern to receiver slot presence and
  incarnation; publish typed optional events and, where supported, ADR-0024
  per-slot accepted output journals. Stream epoch/sequence are independent
  of topology generation and presentation incarnation; USB/IP journal access
  remains refused pending separate approval.
- Fence admitted work by originating slot/session/incarnation before address
  reuse; preserve immutable pending bytes and required replies, retry only
  DefinitelyUnsent in the owning boundary. Uncertain is terminal, not a
  replay. Refuse absent-child mutation by default; no invented native errors,
  output actuation or queued replay on reconnect.
- Isolated child loss preserves siblings; primary/shared required transport
  loss (including a DJ required receiver interface) and uninterpretable
  shared delivery failure terminate the receiver. Retain diagnostic scope
  and idempotent reverse cleanup.
- Use typed receiver-family creation/options and input-only native slot
  mutation consistent with the post-VG-1b model; explicit capability-limited
  companions remain required when selected. Future heterogeneous paired
  children need their own evidence and scope, not implicit desktop devices.
- Develop deterministic Puck lifecycle first; reduced independent/HID targets
  are explicitly not shared USB fidelity. Native USB Puck and later Xbox
  receiver families require #168 feasibility and updated #258 host-audit
  approval as applicable. No GIP faking, firmware analysis or support-cell
  promotion from this proposal.
