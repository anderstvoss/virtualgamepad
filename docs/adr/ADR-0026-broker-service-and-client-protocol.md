# ADR-0026: Broker service and client protocol

Status: Proposed. Acceptance by Anders on the PR is required before any new
broker/IPC wire format is implemented. This record and its companion
[vgsp wire specification](../spec/service-protocol.md) are a design, not an
implementation or host acceptance result.

## Context

[#396](https://github.com/anderstvoss/virtualgamepad/issues/396) targets
downstream native applications and service libraries after Alpha 2. The
[scope decision](https://github.com/anderstvoss/virtualgamepad/issues/396#issuecomment-6970959833)
makes the full upgrade a tag gate; the
[API/topology decision](https://github.com/anderstvoss/virtualgamepad/issues/396#issuecomment-6971199482)
makes the client API and wire compatibility-managed at the tag and requires
generic IDs and aggregate slots. The
[latest output decision](https://github.com/anderstvoss/virtualgamepad/issues/396#issuecomment-6971295220)
makes VG-2, typed haptics and Wii speaker separate Alpha 2 gates; only reserve a
journal read extension here. It supersedes earlier post-Alpha-2 VG-2 scheduling
in the assessments.

The [b1 audit](https://github.com/anderstvoss/virtualgamepad/issues/396#issuecomment-6971108978)
pins main at 324d250bb01a5ddbfc1dd189513d4c9160429ee1. Ordinary callers currently
own typed handles and creator access; the root broker is a privileged attachment
owner, not a general controller service. Its v1 gadget protocol is refused and
v2 audio owns one attachment per connection; worker control v1 and PCM v1 are
separate private protocols. Exact-version blocking exchanges, text errors,
UID admission and implicit connection cleanup are not a reusable app API.
The audit identifies startup blockage by stale records, no observed free-port
proof on close, idle connection starvation and missing orderly drain.

The [b2 survey](https://github.com/anderstvoss/virtualgamepad/issues/396#issuecomment-6971086342)
supports a typed feeder/device split (ViGEm), owned daemon targets
(InputPlumber), per-client slots (Sunshine), and negotiated pollable IPC
(libei). It does not establish per-app isolation, latency superiority,
or a ready-made portal for host-visible gamepads. No external dependency or
implementation is adopted. Evidence is static; live kernel/service results
remain local.

## Decision

### 1. One typed facade, two owners

Keep the existing root family constructors, common and family options, typed
errors, input-only state, status, retained output, topology, explicit commit
and caller-driven service/readiness/next_service_in contract. Add explicit
backend selection (proposed Direct default, Service opt-in), not a realization
ID or automatic fallback. The exact API spelling is for the implementation PR
and owner review. Direct uses the existing in-process owner; Service delegates
to the same compiled family implementation in the daemon. No generic normalized
mutable state, raw reports, runtime profiles, IUCM, or callbacks across IPC.

[ADR-0025](ADR-0025-input-model.md) remains authoritative: edit closures run
once locally over a detached candidate; the server validates the whole native
candidate and owns acceptance, clocks and topology. Only an acknowledged edit
updates the client's accepted snapshot. Rejection leaves both unchanged;
timeout after send is uncertain, closes the owning connection and is never
retried as a fresh edit. Explicit commit is separate, with existing retryable
dirty work semantics and no claim of host observation. Typed metadata and
non-level operations (e.g. attachment, refresh_motion, explicit resource
disposition) remain distinct. No implicit save, per [ADR-0013](ADR-0013-persistent-resources.md).

Preserve synchronous fallible control methods using bounded request/response
waits on the caller, never on the daemon's controller-service path. This is
not a zero-latency or universally nonblocking facade; an app with a strict
nonblocking executor must schedule such calls off its critical loop. Service
mode service() pumps bounded IPC with no blocking wait, delivers optional
callbacks on the caller, and exposes a borrowed client socket fd and write
interest/deadlines. It never owns the provider fd. Cache read-only views from
validated responses/updates; retained data may lag until service, and stale
data cannot authorize edits. Direct also retains its existing provider-specific
limits. Report unsupported operations/targets explicitly, never degrade.

### 2. Service and privilege split

Run virtualgamepad-service as a dedicated non-login non-root system identity
with no effective, permitted, bounding or ambient capabilities, granted only
necessary UHID/uinput creator access. Systemd socket activation opens a
filesystem AF_UNIX socket, proposed /run/virtualgamepad/service.sock, root
owned, 0660, dedicated trusted-client group (separate from creator groups);
verify inherited listener, nonblocking/CLOEXEC, SO_PEERCRED on both ends and
configured service UID on the client. No peer-supplied UID, no raw fd exposure.
No source-device reads, broad input grants, arbitrary file opens or network
listener. Systemd hardening and node ACLs are additional boundaries, not
substitutes for compiled policy. The service schedules its own controller
readiness and monotonic deadlines independently of app IPC, bounded fairness
per owner/component; [ADR-0004](ADR-0004-synchronous-hid-session.md) required
HID/FF replies and exact retry/terminal rules take priority over IPC, callbacks,
slow clients and all journal reads. [ADR-0007](ADR-0007-demo-service-ownership.md)
does not require one thread per controller; isolate blocking prepare/cleanup
and socket work from live deadlines.

Keep the existing root virtualgamepad-broker as a narrow USB/IP attachment
helper, called by the service or an authorized Direct client. It owns
allowlisted VHCI attachment, worker launch where necessary, detach and
ownership recovery; no general controller RPC, descriptor injection,
arbitrary worker or gadget creation. Existing v1/v2 helper, worker control
and PCM protocols are private, independently versioned; any change requires
owner review. Retain explicit WIP opt-in and #203 retained snapshots only.
Reduce helper privileges experimentally; no claim CAP_SYS_ADMIN is removable
until kalux evidence (b1 reports distinct installer capabilities). No
CAP_SYS_ADMIN in general service. Avoid circular root/service dependencies by
extracting private shared owner implementation if needed.

Audio is not silently lost. USB/IP worker remains the protocol owner and
private PCM transport; service may bridge only typed existing PCM sample
operations on explicitly selected NativeClient endpoints, bounded independently
of HID. Caller-session PipeWire cannot be accessed by a system UID by default:
keep the session graph/bridge on caller side, with a typed sample bridge and
required-component failure propagation, not access to another user's socket or
a daemon-wide sound grant. This is a proposed integration requiring local
validation, not existing parity. If an explicit audio mode cannot be supported
safely, report that capability refusal and obtain owner agreement on scope
before tag; Direct never depends on the service. No new raw haptics/PCM command
output path: in-band typed haptic/speaker commands belong to VG-2.

### 3. Negotiated vgsp wire

Use AF_UNIX SOCK_SEQPACKET with explicit LE encoding, finite packet length,
nonblocking sends/receives, correlation IDs, typed errors and HELLO major/minor
and feature negotiation. Packet boundaries simplify truncation/rejection and
remove partial stream reads, not the need for budgets. Validate actual Linux
packet/socket-buffer behavior locally. A single connection may own multiple
handles; each handle has N slots (one for a standalone controller; a receiver
may have zero occupied). Generic compiled registry string IDs and family-local
versioned typed value encodings avoid opcode churn for new families.
[ADR-0018](ADR-0018-compiled-broker-allowlist.md) forbids caller descriptors,
paths, executables and runtime registrations. Capability listing describes
exact permitted realization/topology and schemas, not tested fidelity.
See the companion spec for bounds, ordering, operation and error allocations.

[ADR-0022](ADR-0022-receiver-aggregates.md) and the
[receiver lifecycle design](../spec/receiver-lifecycle-design.md) own slot
policy: family capacity/address scheme, child incarnation fences, retained
topology generation, slot versus shared terminal failure. Do not conflate
protocol slots with player assignment or impose four slots globally. No
new receiver/GameCube/Joy-Con family implementations (#165/#296/#297),
pairing or presentation swaps; [ADR-0021](ADR-0021-presentation-epochs.md)
remains Deferred.

Reserve an optional journal message range and feature bit for
[ADR-0024](ADR-0024-host-output-stream.md): scoped epoch/cursor, bounded batch,
SequenceGap and terminal metadata. No service-created journal, competing
stream, read implementation or USB/IP ordered output access in Alpha 2.
Existing optional service observations and ADR-0020 retained snapshots keep
their meanings; neither reconstructs journal commands. Required replies
cannot await a reader. #203 worker operation 8 is snapshots only.

### 4. Ownership and failure

Peer credentials authenticate a connection; every handle and slot lookup is
connection scoped, not UID scoped. Same UID on another connection receives
the same not-owned error as an unknown ID. Enumerate only owned objects; no
admin/user-global enumeration RPC. CREATE validates/preflights, reserves all
required quotas and prepares atomically with reverse rollback. Delivery of
CREATED alone is insufficient to prove receipt, so use an explicit bounded
CREATE_CONFIRM (client accepts returned ID), otherwise tear down. Confirmation
failure or lost final ack terminates connection/cleanup; no unconfirmed input.
This is transactional ownership, not atomic host enumeration. Client must
close on timed-out request; no durable exactly-once across disconnect.

Explicit close fences input, neutralizes where supported, services required
replies to their bounded deadline or terminates by their transport rules,
closes required components in reverse dependency order, reports cleanup errors
and preserves last validated terminal views while owned. Repeated close is
idempotent; RELEASE disposes the closed handle. EOF, half-close, protocol fault,
SIGKILL (last socket reference), liveness timeout and overflow terminate the
connection or specified handle; no assumption that killing one PID closes an
inherited fd. Client sockets CLOEXEC; sharing/duplication is outside trusted
contract. Queue saturation cannot silently coalesce accepted edits/commits or
drop terminal status: reserve control/terminal egress, close if undeliverable,
client maps EOF to typed uncertainty. Optional observations can be lost only
with explicit cumulative loss; latest retained snapshots are not history.
Daemon stop refuses new work, drains bounded cleanup, attempts terminal notices,
then closes. Restart creates new connection/handle generations, no transparent
continuity, auto replay, persistent ownership or automatic resource export.
Lease/transfer is reserved, not supported.

USB/IP helper: classify stale journals against current kernel ownership;
prove free-port and worker/endpoint release before clearing matching record
and returning successful close. If ownership cannot be verified, quarantine
the record/port, fail closed for affected attachment, report operator recovery;
never detach by remembered port or sweep foreign resources. Separate ordinary
UHID/uinput service startup from helper unavailability. Bound idle admission,
partial setup, worker termination, and stop/drain. Diagnose the existing
kalux "owned endpoint remained open" failure
([#365](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6969844490),
[#350](https://github.com/anderstvoss/virtualgamepad/issues/350#issuecomment-6969735929)):
trace all broker/worker/client descriptor duplicates including ancillary handoff;
consume any buffered final frame then observe EOF/HUP; independently verify
worker exit, matched kernel port free, node removal, record reconciliation and
quota release. A buffered frame alone is not proof of leak or successful close.
Preserve failure evidence and inject corresponding deterministic regression
before claiming fixed.

### 5. Quotas and Alpha 2 threat boundary

Trusted local native applications in the socket group, additionally subject to
administrator-configured UID policy. Bound per-UID, connection, controller and
service aggregate pending creates, live/terminal handles, slots, frames/bytes,
fds, audio and journal memory; reserve before allocation/open, release on
rollback/teardown, quarantine only bounded uncertain external resources.
Spec limits are proposed engineering defaults, locally stress-tested before
freeze; lowering admission may reject new work but never revoke silently.
Group members can create host-visible controllers and may affect other games;
SO_PEERCRED is not per-app or seat identity. Untrusted same-UID applications,
Flatpak/portal, multi-seat and headless accounts have no isolation guarantee and
are out of Alpha 2 supported deployment. Consumers remain ordinary OS
consumers, governed independently by current distro policy.

[#258](https://github.com/anderstvoss/virtualgamepad/issues/258) impact
(supplement to the [host audit](../ALPHA_2_HOST_AUDIT.md), not an edit or
release approval). Every new host change is explicit dev-only now; beta/default
shipping requires Anders's sign-off for the exact resulting host policy.

| Change | Who / privilege / scope | Cleanup and security | Disposition |
| --- | --- | --- | --- |
| Dedicated service user | Admin creates system non-login account; persistent | No login/home; stop service and revoke groups before removal | Dev opt-in; beta sign-off |
| Dedicated client group / allowed UIDs | Admin, persistent enrollment/policy | Grants curated creation not raw node access; revoke and terminate existing connections | Dev opt-in; beta sign-off |
| Service socket and runtime directory | Admin installed socket unit, root 0660 group socket, non-writable parent | Stop socket then service, remove owned path only; peer-check both ends | Dev opt-in; beta sign-off |
| Service unit / sandbox | Admin installs root-owned unit; non-root execution; no caps, AF_UNIX only, device allowlist, no home | Stop/drain, disable/remove; prove no broad device or real-input access | Dev opt-in; beta sign-off |
| udev creator grant | Admin enrolls only service account in existing narrow creator groups; new rule only if needed | Raw creation rights broad for service itself; revoke grant, reload policy; no client input group | Dev opt-in; beta sign-off |
| Helper unit/socket/UID policy/recovery | Admin adjusts existing root helper for explicit service UID and approved Direct callers; reserved ports only | Stop and verify owned detach; remove owned policy only, preserve unrelated ports and workers | Dev opt-in; beta sign-off |
| Optional module registration | Admin, only if missing, existing uinput/UHID/VHCI mechanisms | Remove owned boot config; no unloading in-use real hardware | Dev opt-in; beta sign-off if changed |
| Client-side audio bridge | Ordinary authorized session graph, no service raw sound grant | Close owned endpoints/links; no physical routes automatically | Dev opt-in; beta sign-off for new host policy |

No automatic host policy changes, blanket hidraw/input/uinput grants to clients,
polkit grant, Flatpak override or real-device rebind.

### 6. Client library and compatibility

Propose a private protocol/codec crate (gr-service-wire) without dependency on
root or provider, and a Rust client module inside vg's ordinary typed facade
(not a second competing app SDK). Service binary can link Direct-side vg or
shared private owners; never recurse into Service backend. Stable surface
at Alpha 2 is the documented broker client contract, selected common/family
types and errors necessary to call it, plus vgsp major 1 and published family
codec schemas. Rust ABI is not frozen. A type shared with Direct is stable
insofar as needed for the broker contract; unrelated Direct-only APIs,
experimental SPI, internals and unapproved family/feature work remain
alpha-mutable until beta. Specify the exact API inventory/snapshots in the
implementation PR. Additive minors negotiate capabilities and never redefine
old IDs, layouts or semantics. Deprecate supported broker APIs/features with
replacement and migration notes for at least two stable minor releases and
six months (whichever longer); no removal within major 1. Breaking security
containment fails closed explicitly, with owner-reviewed advisory/migration,
not silently reinterpreting bytes. All broker/IPC wire changes need Anders's
approval, regardless of minor/major. No C ABI at Alpha 2.

Later per-user frontends can authenticate session identity then delegate a
scoped connected socket without changing frame layouts. D-Bus may mediate
discovery/consent or bind typed methods to the same wire; bus sender loss must
close its owned vgsp session. A Flatpak portal would require a new approved
gamepad mediator with revocable delegated fd/session credentials; RemoteDesktop
and EI do not provide host gamepads. A C ABI can wrap versioned typed opaque
handles and explicit memory ownership on vgsp; neither Rust ABI nor raw
provider handles cross that boundary. No promise these are already isolated
or implemented; use negotiated optional features and new separately reviewed
authorization where necessary.

### 7. Acceptance gate for Alpha 2

Full implemented upgrade, not acceptance of this document, gates the tag.
Pin external Rust consumer and lockfile to candidate vg revision, verify
same typed source in both backends on at least DualSense UHID and Xbox360
uinput (or owner-approved equivalent), independent host input/output
recognition, neutralization and actual removal. Cover all advertised
family/realization/options operations by deterministic parity tests, including
explicit refusals, resource retain/discard and selected audio component
rollback. Local/systemd/kalux evidence includes two simultaneous clients,
same UID foreign ID denial, separate UID quota, no creator access for
Service client, wrong service UID, denied socket, actual unit capabilities,
stop/restart/socket activation and failed USB/IP smoke resolution.
Cloud tests cover handshake/schema/limits, malformed packets/ancillary,
lost create reply/confirm, EOF/half-close, duplicate close, saturation,
stalled caller, status/optional losses, inherited fd behavior model,
slot fences, required reply priority and deterministic cleanup failures.
Local tests exercise SIGKILL and actual fd/node/port accounting, host
visibility/permission and opted-in audio where supported. No support-cell
promotion or hardware fidelity inference. Benchmark local, not run: p50/p95/p99
native input request/ack and commit-to-independent-consumer latency at
representative 250/1000 Hz, idle/bursts and concurrent controllers; RSS/fd
plateau and deadline misses at saturation; record kernel, hardware, revisions,
sample size, CPU, limits and baseline Direct. Approve latency/budget thresholds
before qualification; do not invent them here.

### 8. Ordered implementation slices (separate PRs)

| Slice | Depends on | Scope / evidence |
| --- | --- | --- |
| 1 | Accepted ADR | Cloud: codec, family schema inventory, golden packets, malformed and negotiated version tests; no provider |
| 2 | 1 | Cloud: service core with fake providers, connection ownership, create confirm, quotas, fairness/deadlines, typed lifecycle and shutdown |
| 3 | 1–2, stable family input seams | Cloud: typed client backend, facade parity, cache/ack and readiness; snapshot and migration updates in implementation PR |
| 4 | 2–3 | Cloud: narrow privileged helper split, deterministic journal recovery/cleanup, worker lifecycle/fd regression; separately approve any private wire change |
| 5 | 4 | Local (kalux): systemd/udev/installer, actual privilege reduction, stale port/record and smoke diagnosis; selected PCM/session bridge validation; deterministic fixes upstream |
| 6 | 3–5 | Cloud: external pinned consumer fixture and CI parity; local: same fixture actual host, systemd, pressure, SIGKILL, benchmarks and remaining negative cases |
| 7 | 6 and separate VG-2 gates | Cloud: final docs, public broker compatibility inventory, migration and evidence ledger; local/owner: review gate, host results and beta sign-off boundary |

VG-2 journal V2a (codex/vg2-v2a), typed audio/haptics and h2 docs refresh
(lane/alpha2/docs-refresh) are parallel owners; do not edit their files or
change their APIs here. Synchronize semantics/compatibility inventory before
tag; receiver work remains separately scoped.

## Alternatives

- Root all-in-one service: rejected; b1 shows attachment needs differ from
  ordinary node access. Non-root general service is achievable in principle,
  but verify actual provisioning/capabilities; do not infer from root unit.
- Direct-only or per-user-only: Direct retained, but downstream clients would
  need broad raw rights; per-user alone doesn't solve same-UID app isolation.
- SOCK_STREAM: viable portable/private precedent; adds partial frame handling,
  timeouts and buffering. Choose SEQPACKET for Linux v1 bounded message unit;
  no performance claims. Reconsider only if actual size/portability tests
  contradict choice, with owner approval. D-Bus is credible later mediation,
  not proven faster or per-object lifetime by default (b2 InputPlumber);
  avoid imposing its runtime or session policy now.
- Fully asynchronous new handle API: avoids synchronous RPC waits but breaks
  existing typed caller contract. Keep synchronous bounded mutation and
  nonblocking pump; add explicit async adapters later if needed.
- Raw descriptors, arbitrary callbacks, normalization, lease-by-default,
  fire-and-forget create or global registry: rejected for reviewed family
  boundary, orphan/uncertainty and cross-owner leakage risk.
- Second raw output stream or USB/IP journal via #203: rejected by owner;
  existing PCM sample I/O is distinct, not a host-command journal.

## Consequences

A real app gains a stable typed integration surface with reduced creator
privilege, independently serviced required deadlines and explicit connection
lifetime. IPC round trips, cache latency, finite queues, schema maintenance
and a system service add costs. A compromised trusted app can still inject
approved controller inputs; the service itself has powerful creator access.
A host stop/crash loses volatile handles and history; uncertain external
cleanup needs operator review. No physical timing or support claims follow.

## Validation

Docs-only: no runtime, snapshot, test, dependency or host changes. CI is the
first full validation of this PR. All implementation and local acceptance
above remain not run. Owner decisions requested: transport/name and schema
framing; sync mutation/confirmation and terminal policy; quota/time defaults;
non-root host/helper grants; audio scope/bridge and failure behavior;
compatibility/deprecation window and exact stable API inventory; acceptance
matrix and benchmark thresholds. The ADR remains Proposed until owner accepts.
