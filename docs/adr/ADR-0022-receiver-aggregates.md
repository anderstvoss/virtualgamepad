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
