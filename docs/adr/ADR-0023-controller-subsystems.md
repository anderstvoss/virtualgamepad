# ADR-0023: Controller-owned bounded in-band subsystems

Status: Proposed; adopted by the owner under D4 (2026-10-21). No implementation or
ordinary-root API addition is approved here.

## Context

Wii ADPCM speaker commands and future displays, NFC or IR-camera traffic are
controller-native protocol data. They differ from out-of-band host components
such as a UAC endpoint or an explicitly selected keyboard/pointer companion.
Putting bulk data through service callbacks or the 32-entry observation queue
would couple payload lifetime to optional observation delivery and hide loss.

## Proposed decision

A compiled controller family declares supported in-band subsystems and exposes
explicit opt-in creation options only when a measured family needs them. The
controller owns protocol semantics, bounded buffering, validation and shutdown;
providers carry transport operations. Use a typed borrowed handle, analogous to
controller.audio(), rather than a generic mutable extension API.

Creation validates selected options and preflights required resources before any
provider opens. Report unavailable operations on the selected realization with
the technical reason; no silent fallback, optional degradation or automatic
companion creation. Out-of-band endpoints remain selected components following
[ADR-0017](ADR-0017-primary-realization-and-audio-components.md) and compound
preflight/rollback. A subsystem does not become a new realization target.

Buffers have explicit bounded capacity and a typed acceptance/backpressure
result. Required protocol traffic retains immutable accepted bytes and bounded
servicing; DefinitelyUnsent may retry, Uncertain terminates the owning
presentation. Bulk data never flows through observation callbacks, and lossy
observations cannot be used to recover authoritative subsystem state.

Close remains terminal and idempotent. Stop accepting new writes before cleanup;
required work and uncertain delivery follow the established HID/compound
contracts. Diagnostic errors retain cleanup ordering. Persistence follows
[ADR-0013](ADR-0013-persistent-resources.md): only registry-declared typed resources
support explicit validated import and terminal retain/discard; subsystem presence
never implies that payloads persist or that Drop writes files.

## Rejected alternatives

- A universal bulk payload/state field, runtime profiles, plugins or provider-owned
  interpretation would erase controller-native semantics and validation.
- Treating in-band speaker/NFC traffic as UAC components misstates the actual
  transport and independent lifecycle.
- Unbounded queues or callbacks as payload ownership obscure backpressure,
  accepted-write disposition and shutdown behavior.
- An in-process/library-only tier is a possible future ADR, not decided here.

## Consequences and first consumer

Wii Remote speaker support is the first prospective consumer; VMU-class displays,
NFC and IR camera are later examples, not commitments. A family proposal must
supply evidence-backed formats, bounded limits, typed operations, realization
limitations and deterministic validation/backpressure/cleanup tests. This draft
implements no Wii feature, decoder, transport or physical acceptance claim.

## API snapshot growth when a family ships

Potential public additions are family-specific subsystem selection and typed
borrowed access with explicit result/error semantics. Exact signatures and root
snapshot growth require D2 separately. Nothing changes the 110-item snapshot or
broker/IPC format here; existing audio APIs are examples, not permission to
expand them.

## Adoption and implementation

D4 adopted this ADR on 2026-10-21 as the shared shape for controller subsystems
in later checkpoints. Implementation still needs a separate approved family scope;
[brief 04](../plans/pre-controller-buildout/04-contract-adrs.md) supplies the design
questions only.
