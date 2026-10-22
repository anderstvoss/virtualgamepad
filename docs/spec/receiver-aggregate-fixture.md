# Synthetic receiver aggregate lifecycle evidence

Step 13b exercises the existing HID `Runtime` and `RequiredGroup` ownership seams
with a test-only receiver. It does not implement or advertise a Steam Puck,
Joy-Con, pairing protocol, Linux provider, keyboard mapping API or radio device.
All report IDs, identities, values and capabilities are sanitized synthetic data.

The receiver owns one shared protocol runtime and four typed protocol addresses
A–D. Slot addresses are unrelated to GamepadManager player assignments. Each
slot has independent presence, identity, native input and retained feedback.
The controller owns attachment validation and an explicit keyboard/pointer
selection; fixed synthetic capability limits reject unsafe edits. Selected
companions are required, serviced fairly and removed only with their owning slot.
A shared presentation failure closes every owned component.

`FamilyKind::Receiver` names its hosted compiled family IDs. A metadata-only
receiver test uses the existing fifth test family; it has no real surfaces,
manifest targets, USB profile or production `FAMILIES` entry. This proves the
compiled metadata shape without advertising unsupported hardware.

## Transaction and delivery boundaries

The fixture prevalidates configuration and neutralized input before preflighting
all new companions. It opens them only after every preflight succeeds, rolls back
new resources in reverse order on failure and preserves rollback diagnostics.
Pure, prevalidated state/protocol installation follows the existing curated
`ControllerSession::update_configuration` contract. Removed controls become
neutral; unchanged sibling state and identity survive. Cleanup failures remain
diagnostic after a committed removal. Generation exhaustion rejects before I/O.

Every HID request receives the exact success/error reply or terminal cancellation.
Accepted replies and queued input bytes keep their immutable contents across slot
changes. Only `DefinitelyUnsent` retries; `Uncertain` closes the shared boundary
instead of applying the older compound full-frame retry rule. No migration of
that older runtime is needed for this fixture: required replies remain owned by
individual stateful HID runtimes and companions use `RequiredGroup`.

LizardOnly suppresses newly generated native-slot reports while retaining native
semantic values. Already accepted immutable reports keep their delivery order.
This is synthetic controller policy, not a realization ID or implementation of
presentation epochs. D3 continues to defer step 13a.

## Deterministic evidence

[receiver_aggregate.rs](../../crates/gr-hid/tests/receiver_aggregate.rs) covers:

- Zero through four slots, simultaneous attachment changes, independent identities,
  native inputs and feedback routing.
- All selected preflights before opens, partial-open reverse rollback, rollback
  close diagnostics and unchanged accepted state/generation/dirty status.
- Invalid or duplicate identities, absent controls, unsafe companion edits,
  overflow and repeated transitions.
- Slot-local disconnect/reconnect, sibling survival, LizardOnly retention and
  independent receiver instances.
- Exact primary and companion replies for supported and rejected report classes;
  busy siblings get service in the same bounded cycle without any callback.
- Pending required reply and native-frame immutability across changes,
  DefinitelyUnsent retry and uncertain shared-delivery termination.
- Constructor failure, close with a pending request, shared companion failure,
  cleanup errors, reverse explicit shutdown and idempotent close/drop.

No existing test or core delivery implementation changes. A real receiver package
must provide evidence-backed bytes, identities, slot validation and its declared
realizations; these fixtures do not supply physical or consumer acceptance.

## Slot failure boundary

A failed required companion group disconnects and neutralizes only its owning
slot. The service call returns the slot error after servicing siblings; a caller
may continue polling the surviving receiver. Accepted primary replies and sibling
state stay owned, and failed-slot cleanup diagnostics survive reconnect and close.
A primary transport failure is receiver-wide. Failure to install a validated
slot disconnect (for example generation overflow) also terminates the receiver.
This remains a synthetic fixture; no crate receiver handle is implemented yet.
