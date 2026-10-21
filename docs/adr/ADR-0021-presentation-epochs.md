# ADR-0021: Logical controller and host presentation epochs

Status: Proposed; waits on D3. No implementation or ordinary-root API addition is approved here.

## Context

A controller's semantic state, identity and declared persistent resources can
outlive one host presentation. Steam lizard/native policy (#159, #165) and
8BitDo presentation modes (#172) need this distinction. Closing a transport and
recreating the whole logical controller loses resources or confuses terminal
shutdown with a mode transition. A realization ID describes a transport target,
not lizard/native desktop policy.

## Proposed decision

An opt-in controller-owned logical handle retains semantic state, identity and
persistent resources. Its presentation owns selected provider sessions, queues,
transport counters and diagnostics. A controller-native typed transition prepares
all required components, swaps presentations and closes the old presentation.
Reuse [compound preflight and rollback](../spec/implementation/COMPOUND_CONTROLLER_SESSIONS.md):
validate/preflight before opening anything, prepare every required component,
then publish the replacement. Failed preparation closes only newly opened
resources in reverse order and leaves the old presentation intact. Preserve
ordered rollback errors rather than silently falling back.

A successful replacement advances a checked monotonically increasing epoch.
Association metadata identifies the current presentation and epoch. A retained,
pollable counter follows [live topology](ADR-0019-live-topology.md), outside lossy
observations; no callback is required to discover a replacement. Epoch exhaustion
rejects before preparation. A replacement gets fresh transport counters and
queues; semantic values, identity and validated persistent resource ownership
survive. Accepted old commands must not be replayed through the new presentation.

Once publication occurs, failure closing the old presentation cannot roll back
an already visible replacement. Preserve that cleanup error diagnostically while
keeping ownership and the published epoch unambiguous. A shared terminal failure
closes the active group; logical close remains terminal and idempotent, including
during a transition. Step13a must prove the precise service/close sequence with
fake providers before implementation is accepted.

[Receiver aggregates](ADR-0022-receiver-aggregates.md) retain receiver-owned shared
failure and typed protocol-slot identity. Per-slot disconnect is not a whole
receiver presentation epoch, and a Puck protocol slot is not an application player
slot. LizardOnly suppresses native host presentation while retaining native
semantic state; it is explicit controller policy, not a new RealizationId.

## Rejected alternatives

- Treating every mode change as logical close/recreate loses continuity and makes
  explicit terminal resource disposition ambiguous.
- Provider-owned policy, generic mutable gamepad state, runtime profiles and
  silent realization fallback violate the curated controller-native boundary.
- Replaying an old full frame or non-repeatable command after uncertain delivery
  violates the HID delivery contract. Only DefinitelyUnsent retries immutable
  accepted bytes within the owning presentation.
- A mandatory epoch layer or resource field on all fixed controllers adds cost
  without a measured need; use opt-in ownership for families that require it.

## Consequences and first consumer

Steam wired native/lizard transitions are the first consumer, followed by Puck
receiver presentation policy and 8BitDo mode changes. Deterministic step 13a tests
must cover successful swap, failed prepare leaving the old presentation intact,
failure after publication, close during replacement, identity/resource continuity,
queue/counter reset, overflow and idempotent cleanup. No physical acceptance is
implied by those fixtures.

## API snapshot growth when a family ships

Potential additions are a retained presentation epoch and association metadata
plus family-specific typed mode actions. Exact signatures and public exposure
require D2 separately. This ADR changes neither the 110-item ordinary-root snapshot
nor broker/IPC formats. No public API is added before a consumer and approval.

## Open decision and implementation

D3 decides whether to accept this logical-controller/presentation split. While it
is open, merge only this Proposed contract and skip step 13a. Implementation:
[brief08a](../plans/pre-controller-buildout/08-epochs-and-receivers.md), step 13a;
controller-owned resources follow [ADR-0013](ADR-0013-persistent-resources.md).
