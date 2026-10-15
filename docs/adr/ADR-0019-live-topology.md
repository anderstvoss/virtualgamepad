# ADR-0019: Retained generations of controller-owned live topology

Status: Proposed. No ordinary-root API addition is approved by this document.

## Context

A family's static `InputTopology` describes its maximum shape. Accessories can
change which of those controls are present in a live logical controller. Wii
extensions are the first consumer (#158); headset presence and other accessories
may later use the same distinction. A lossy output queue cannot reliably describe
the current shape after many attachment edges.

## Decision

The controller owns a current topology that is a subset of its compiled maximum,
and a retained monotonic generation. A successful edit that changes the active
shape advances the generation once. A rejected edit, a repeated request for the
same shape, polling or observation eviction does not advance it. Generation and
current shape remain readable after terminal close; edits fail after close.

Protocol configuration changes use the transactional `Runtime::update_protocol`
seam from [ADR-0014](ADR-0014-protocol-edits.md). Candidate validation includes the
controller's complete attachment combination and current semantic state. Accepted
input and required reply bytes keep their delivery order and are not rewritten
by a later topology edit. Host STOP is an explicit lifecycle cancellation, not a
caller topology edit; it follows the existing runtime lifecycle contract.

For uinput, the first Wii implementation keeps a static maximum capability
superset for the lifetime of the node. Removing an accessory neutralizes its
controls in subsequent reports and makes caller edits to absent controls fail
with `UnavailableInRealization`; it does not recreate the kernel node. This
preserves identity and sibling sessions. The retained current topology tells
applications which advertised controls are active. HID presentations keep their
controller-native accessory handshake and status edges.

The generation and active shape are retained independently of the bounded lossy
observation queue. Events can report edges, but consumers recover by reading the
retained generation/current shape. No provider owns attachment policy, and there
is no arbitrary runtime topology or descriptor injection.

## Alternatives and consequences

Recreating uinput nodes on every accessory change would change identity, require
consumer rediscovery and complicate rollback. It is rejected for the initial Wii
path. The static-superset policy requires explicit neutralization and honest
active-topology reporting rather than claiming every advertised axis is present.

Inferring the current shape from observation events is rejected because events
can be evicted. Treating the static maximum as the live shape would silently
permit edits to missing controls. A controller must validate the current subset
and retain its generation outside the queue.

Step 8 implements the seam crate-internally and proves it with the synthetic
fifth family, including attach/detach, absent edits, close and uinput policy.
When a real consumer family ships, proposed root additions are read-only
`topology_generation()` and `current_topology()` accessors on its handle. Their
exact public signatures and snapshot changes require separate owner approval;
this Proposed ADR and C2 do not grow the 110-item snapshot.
