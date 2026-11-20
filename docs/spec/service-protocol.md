# VirtualGamepad service protocol (vgsp)

Status: Proposed companion to [ADR-0026](../adr/ADR-0026-broker-service-and-client-protocol.md).
No wire allocation is effective until owner acceptance; no implementation or
compatibility claim yet. This is the app-facing protocol, not the privileged
helper v1/v2, worker control or PCM protocol. All changes require Anders's
approval. MUST/SHOULD below describe the proposed implementation.

## 1. Transport and encoding

Linux AF_UNIX SOCK_SEQPACKET, filesystem /run/virtualgamepad/service.sock.
One packet = one frame; no stream fragmentation, compression, ancillary
fds or raw kernel descriptors in v1. Client and service authenticate peer
UID using SO_PEERCRED; client verifies configured service UID from trusted
installation, not HELLO. Server uses socket DAC and admin UID policy.
Nonblocking and CLOEXEC at creation/adoption; verify activated listener is
AF_UNIX, listening, SEQPACKET and exact configured path before accepting.
No abstract socket, remote transport or peer-provided path. Receive with
recvmsg and bounded control buffer; MSG_TRUNC, MSG_CTRUNC, unexpected
ancillary data, partial packet, inconsistent length or zero-length packet
are fatal (close any received descriptors on all paths). Send whole packet
or EAGAIN; ambiguous/partial send is terminal, never retry as new edit.
No reliance on actual default kernel socket-buffer sizes; qualify packet
capacity locally.

All integers unsigned fixed width LE unless explicitly signed. Header is
24 bytes:

| Offset | Width | Field |
| --- | --- | --- |
| 0 | 4 | ASCII VGSP (56 47 53 50 hex) |
| 4 | 2 | major (0 for bootstrap only) |
| 6 | 2 | minor (0 for bootstrap only) |
| 8 | 2 | message code |
| 10 | 2 | flags: bit 0 response, bit 1 optional extension; other bits zero |
| 12 | 4 | body length in bytes, exact packet size minus 24 |
| 16 | 8 | request ID (nonzero request/reply, zero unsolicited notice) |

Max packet 65,536 bytes, max body 65,512. No native alignment/padding,
repr, serde/bincode variant numbering, platform-sized fields or errno
values as codes. Primitive bool u8 is exactly 0 or 1. ID = u16 byte length
(1–96) then exact case-sensitive ASCII from compiled registry (lowercase
letters, digits, dot, hyphen, underscore); no NUL, normalization, path or
alias. Text = u16 length + valid UTF-8 up to 256 bytes; no authoritative
semantic meaning, no raw payload/secrets. Blob = u32 byte length + bytes,
max 48 KiB, type context defined below. Arrays = u16 count + elements;
caps checked before allocation, exact consumption, checked arithmetic.
Reserved fields must be zero. Unknown required fields/discriminants reject
before mutation; no ignored trailing bytes in known versioned structures.

Family typed values use V = schema:u16, revision:u16, blob. Schema numbers
are family-local documented constants, not enum ordinals; revision is
an explicitly negotiated compatible schema revision, not retained output
revision. Common typed structures use registered common schemas. Native
encodings are independently fixed-width LE, documented exact field order,
discriminants, domain bounds and golden vectors before tag; no raw Rust
layout, arbitrary HID descriptor, report injection, paths, binaries,
dynamic type loading or generic mutable event codes. Fixed frame grammar
does not by itself freeze native schemas: slice 1 must publish the
per-family/realization schema registry and fixtures for every advertised
operation before implementation acceptance. A server advertises only
compiled and mutually supported schemas; client never sends an unlisted
schema/revision. New family IDs can be added without wire version change
if they fit this grammar. Schema evolution is additive via a newly
negotiated revision (old layout and semantics retained).

## 2. Bootstrap and capabilities

First request HELLO (header version 0.0, id 1). Body:
offer_count:u16 (1–8), repeated major:u16, min_minor:u16, max_minor:u16
(distinct nonzero majors, inclusive ranges); optional:u64, required:u64
(feature masks, disjoint); max_packet:u32 (4096–65536). Alpha 2 offer
1:[0,0]. Server chooses highest mutually supported major then minor from
its own supported ranges; no downgrade below an offered min. Required
features must all be supported at that version; unknown optional bits
ignored, unknown mandatory rejected before controller allocation.
WELCOME response (same id, header 0.0):
major:u16, minor:u16, selected_features:u64, max_packet:u32, connection_token:[16],
limits (below). Token fresh per connection, diagnostic only, not credential
or portable ID. Chosen packet limit is min of offered and server cap.
Selected bits subset of offered and server implemented. Client verifies
all selected values; mismatch closes. Bootstrap ERROR may precede close;
no valid HELLO within 3s closes. Pre-auth rejection may be bare EOF;
no allocation of controllers prior to negotiation.

| Feature bit | Meaning |
| --- | --- |
| 0 | Reserved: Service-mode PCM sample bridge (existing typed audio I/O only). Not advertised or accepted in 1.0; whether and how Service mode carries audio is an open owner decision for the final production broker build (ADR-0026 section 2) |
| 1 | Reserved: ADR-0024 journal reads (not advertised or accepted in 1.0) |
| 2 | Reserved: leases/transfer (not advertised or accepted in 1.0) |
| 3–63 | Unassigned; no private use in compatible production protocol |

Base lifecycle, capability listing, N-slot handle, typed edits, read views
and optional observations are mandatory, not separately negotiated. PCM
(bit 0) is not advertised in 1.0; if the owner later approves it, it may be
advertised only on reviewed family/realization/options combos after parity
qualification; absence is an explicit refusal. Capability
listing reflects actual service/profile availability, not physical acceptance.
Use LIST_CAPS after WELCOME, no CREATE before successful discovery.
Request start:u16 (zero initially), max:u16 (1–16); reply next:u16
(65535=end), count:u16, entries. Stable immutable capability snapshot per
connection; changes require new HELLO/reconnect, provider/preflight still
checks current availability. Max 256 entries; entry size <=3072 bytes;
reduce page count to fit negotiated packet; at least one entry must fit.
Entry:
family:ID, realization:ID, shape:u8 (1 standalone, 2 receiver),
max_slots:u16, address_policy:ID, hosted_count:u16 (<=32), hosted IDs,
option_flags:u64 (family schema-defined; zero reserved),
schema_count:u16 (<=64), entries {operation:u16, schema:u16,
min_revision:u16, max_revision:u16, max_blob:u32}, feature_mask:u64,
stability:u8 (1 stable, 2 preview; other values rejected).
Stable entries carry the section 9 compatibility promise. Preview entries
may change their schemas by a new revision in any minor, may be withdrawn,
and a client must opt in to each preview entry explicitly; the client
library never selects a preview entry implicitly. The owner names the
stable family/realization set at the tag (proposed: DualSense, DualShock 4,
Xbox 360 and Switch Pro on their existing realizations); all others start
as preview.
Schema entry keys are (operation, schema); capability list entries may not
repeat a key. A CREATE includes the chosen compatible revisions for every
schema needed by its requested options, including reply/notice schemas.
The service validates the selection before allocation and uses only those
revisions for the lifetime of the handle. A client lacking a required schema
cannot select that configuration; optional unsupported views/operations
are refused, not decoded as a different revision.
Schema ranges describe supported revisions, not implicit validation.
Address policy is a compiled identifier with published per-family bounds,
not a free-form client definition; actual addresses need not be contiguous.
Standalone has one slot at address 0; receiver can start with zero occupied.
Unknown family entries can be skipped by clients after structural validation.
Client selects compatible entries and schema revisions or refuses locally.
Examples of exact IDs: virtualgamepad.dualsense, virtualgamepad.xbox360,
linux.uhid.usb, linux.uinput, linux.uhid.bluetooth, linux.usbip.usb-audio.
No linux.dummy_hcd.usb-hid advertisement. UHID Bluetooth bus metadata is
not RF. No new family support inferred by listing syntax.

## 3. Requests, responses and scheduling

Requests use strictly increasing connection-wide u64 IDs. HELLO consumes
ID 1; the first request after WELCOME uses ID 2. IDs never decrease,
no reuse/wrap; exhaustion closes. Responses echo ID and set response flag;
one final response per request, except handshake which uses chosen bootstrap
grammar. Message code identifies success shape; ERROR replies use common
envelope. Responses for independent handles can interleave/out of order;
within the same handle complete mutation and commit in request admission
order, with one outstanding mutating request per handle in the client.
No response can be applied to a different handle, slot, or incarnation;
unknown/stale response IDs or wrong response shapes are fatal at the client.
Unsolicited notices use id 0, no response flag. Unknown optional extension
notice may be ignored only when flagged optional, well-formed, bounded,
and not required to complete an outstanding transaction. Unsupported
requests (including a reserved code) receive typed UnsupportedOperation;
unknown required notice/flag closes. Implement no speculative dedup cache,
automatic retry on lost reply, or sequence wrap.

Service runs one authoritative owner per logical handle, services native
kernel readiness/deadlines even if caller is silent or IPC blocked. App
service() only pumps finite nonblocking frames, callbacks delivered on
app thread; socket fd is borrowed. Recompute wants_write and deadline
after pumping; shared connection readiness requires a single demultiplexing
owner so all typed handles may be serviced without a competing read.
Cache updates ordered by state revision; never overwrite newer accepted
state with old data. Synchronous setters/commit/creation/close wait only
on bounded correlated RPC; while waiting may pump/retain replies/observations
but never invoke arbitrary callbacks recursively. No locks across unrelated
provider waits. Client drop attempts bounded CLOSE/RELEASE for its own handle; on any
uncertain close or inability to enqueue cleanup, close the shared connection
and invalidate its sibling views. Explicit close retains terminal readback
until release/drop. Client deadline after sent request => close connection,
typed Uncertain, final last validated views; unsent EAGAIN can retry exact
packet within deadline or return WouldBlock with no mutation. Required
HID/FF replies get their own controller runtime priority/reserved slot,
never share application egress backpressure. No hard RT guarantee.

## 4. Handle, slot and data types

H = u64 monotonically allocated nonzero service-instance handle ID (never
reuse before service restart), bound to one connection. A foreign H can
never alias a different live connection's own H; lookup is still strictly
owner-scoped, not a bearer authorization. S = {address:u32, incarnation:u64}; incarnation nonzero for occupied
or retained child, zero for vacant address. Every child operation carries H
and S; receiver-level operations carry H alone (separate message shape);
slot address is not application player index. Shape = topology_generation:u64,
slot_count:u16, repeated {address:u32, incarnation:u64, presence:u8,
family:ID}; count <= advertised capacity and protocol cap 256. Presence
0 vacant, 1 pending, 2 present, 3 link-lost retained, 4 terminal; vacant
entry uses declared hosted family or receiver family if heterogeneous
unknown (not proof of occupant). Dynamic addresses may be omitted if
unallocated. Standalone always one slot address 0. Topology generation
advances on committed change; incarnation advances on recreation, not
retained link loss. Never confuse with stream epoch. No old edit replay
across address reuse; family-specific transaction and failure scoping per
ADR-0022. New occupant starts neutral.

Owner validates peer, H, S, schema, family and operation, then resources,
full native candidate and topology transaction before acceptance.
Metadata-only reads do not invent fields. State revision R:u64 is a
service-scoped monotonic snapshot publication revision (not input clock,
journal sequence or retained output revision); advances on change of
snapshot contents, never wraps. A snapshot bundle B = R, count:u16
(<=16), repeated {kind:u16,V}; kinds below. Full snapshot response
contains requested kinds; async update is a full value for listed kinds,
not an unvalidated field patch. Per-kind last accepted revision in client
cache prevents stale overwrite. Family schema status records explicitly
mark unsupported/unavailable. A request may select one kind to avoid
excess packet size; largest valid single value must fit advertised packet
or entry cannot be selected. Snapshot kinds: 1 inputs, 2 status,
3 retained outputs (ADR-0020 revision inside V), 4 diagnostics,
5 association, 6 surface/topology descriptors, 7 typed persistent resource
metadata, 8 audio metadata; 9–65535 reserved. Topology shape has its own
message. All payloads must fit negotiated limit, never silently truncate
accepted value. Terminal last validated caches may be incomplete after
disconnect and must report uncertainty, not claim final revision.

## 5. Message allocations and payloads

Hex codes below, all request/reply pairs use response flag only for replies.
Unassigned/reserved codes cannot be emitted in 1.0. OK response body empty
unless specified; ERROR may replace any response. Every request includes
only stated fields. H/S defined above; code ranges retain their meanings.

| Request code / body | Success code / body | Purpose |
| --- | --- | --- |
| 0001 HELLO (section 2) | 0002 WELCOME | Negotiate |
| 0003 LIST_CAPS (section 2) | 0004 CAPS | Capability page |
| 0005 PING empty | 0006 PONG empty | Liveness |
| 0010 CREATE: family:ID, realization:ID, selection_count:u16 (1–64), {operation:u16,schema:u16,revision:u16} entries, common_options:V, family_options:V | 0011 CREATED: H, Shape | Prepared, not yet active |
| 0012 CREATE_CONFIRM: H | 0013 CONFIRMED: H | Commit client ownership |
| 0014 CLOSE: H | 0015 CLOSED: H, terminal:ErrorBody | Idempotent provider close, retains view |
| 0016 RELEASE: H | 0017 OK | Discard closed handle/readback, free memory |
| 0018 ENUM_OWNED: after:H (0 first), max:u16 (1–16) | 0019 OWNED: count:u16, {H,state:u8,family:ID,realization:ID,Shape} entries, next:H (0=end) | Owner-scoped page |
| 001a GET_TOPOLOGY: H | 001b TOPOLOGY: H, Shape | Retained current shape |
| 0020 SET_INPUTS: H,S,V | 0021 ACCEPTED: H,S,B | Validate and install complete native levels |
| 0022 COMMIT: H,S | 0023 COMMITTED: H,S,B | Explicit submission attempt |
| 0024 NATIVE_OP: H,S,op:u16,V | 0025 APPLIED: H,S,B | Typed non-level operation |
| 0026 RECEIVER_OP: H,op:u16,V | 0027 RECEIVER_APPLIED: H,Shape | Typed receiver lifecycle transaction |
| 0028 GET_VIEW: H,S,kind_mask:u64 | 0029 VIEW: H,S,B | Read validated current/terminal typed data |
| 002a RESOLVE_RESOURCE: H,S,disposition:u8 (0 discard,1 keep snapshot) | 002b RESOURCE: H,S,present:bool,[V if present] | Explicit terminal typed resource (ADR-0013) |
| 002c GET_HANDLE_VIEW: H,kind_mask:u64 | 002d HANDLE_VIEW: H,B | Receiver-wide typed metadata/diagnostics |
| 0040 PCM_READ: H,S,endpoint:u16,max_frames:u16 | 0041 PCM_DATA: H,S,endpoint:u16,first_frame:u64,count:u16,discontinuity:bool,blob | Existing selected playback sample I/O |
| 0042 PCM_WRITE: H,S,endpoint:u16,count:u16,blob | 0043 PCM_WRITTEN: H,S,endpoint:u16,accepted_frames:u16 | Existing selected microphone sample I/O |
| 0044 PCM_FLUSH: H,S,endpoint:u16 | 0045 PCM_FLUSHED: H,S,endpoint:u16 | Existing typed explicit flush |
| — | 0080 ERROR: ErrorBody (response flag) | Correlated rejection/failure |
| — | 0081 TERMINAL: scope:u8,H,S,ErrorBody | Unsolicited; scope 0 connection,1 handle,2 slot |
| — | 0082 VIEW_UPDATE: H,S,B | Optional retained snapshots |
| — | 0083 TOPOLOGY_UPDATE: H,Shape | Retained topology generation notification |
| — | 0084 OBSERVATIONS: H,S,total_dropped:u64,count:u16,V entries | Existing optional callback observations only |
| — | 0085 DRAINING: remaining_ms:u32 | Refuse allocation/mutation, bounded stop |
| 0100–01ff | Reserved | ADR-0024 journal access (section 8) |
| 0200–02ff | Reserved | Lease/transfer later |
| all other codes | Reserved | Future owner-approved use; no vendor private codes |

ENUM state: 1 prepared, 2 active, 3 closing, 4 closed-owned.
Only release closed-owned handles, not active ones. Before CREATE_CONFIRM,
the client obtains all initial typed state/read views and prepares any
required client-side audio endpoint; failure closes the prepared handle or
the connection. If required views/shape, maximum valid replies or terminal
metadata cannot fit the negotiated packet ceiling, refuse CREATE before
opening a provider. Large optional responses are requested by individual
kind; validate response size before any mutation with required ACK data.
Kind_mask bit (kind - 1) selects each known kind (kinds 1–64 addressable
by mask; kinds above 64 need a separately approved additive message);
unknown requested bits are rejected. Count arrays cannot contain duplicate IDs/kinds. Cursor is ordered
by H; if concurrent changes occur it is not a snapshot of past objects.
Limit by count AND bytes; cursor continues strictly after last returned H.
For scope 0 terminal H,S zero; scope 1 S zero; scope 2 exact H,S.
For handle with receiver shared native commit, its typed receiver
operation invokes the owning transaction; never synthesize a child address.
NATIVE_OP family-local typed registry covers explicit neutralize,
refresh_motion, battery, attachment, other reviewed metadata/subsystems;
never generic mutable state and never an arbitrary callback. Op IDs
and payloads must be documented with schema registry per section 1;
unsupported op fails before mutation. SET_INPUTS is the only whole-input
replacement; closure/native input setter translates to validated candidate.
ACK/ACCEPTED after semantic acceptance not external delivery. COMMIT
returns WouldBlock for retryable pressure preserving dirty/pending bytes,
never advances motion clock again; uncertain delivery terminal.
Rejected native edits preserve inputs/status/dirty/output and no provider
I/O. All metadata reads from owner. Optional observations preserve FIFO
order within owning controller, bounded loss separately reported by
cumulative counter (count can be zero to report loss). Client includes its
own optional queue loss in diagnostics without double-counting service
loss; late subscriber cannot claim full history. These are existing
callback semantics only, not new typed haptic/speaker payload path.
Retained read updates may coalesce only replaceable views, not accepted
mutations, terminal or required reply. Shape can be read even if notice lost.

PCM (bit 0) is an explicitly selected existing audio sample endpoint,
not a host-output command stream. Endpoint IDs/format/channel count
are compiled and returned in typed audio metadata; max 128 frames per
batch; S16LE interleaved on current profiles, exact count * channels * 2
bytes, no raw arbitrary audio paths. PCM_READ count zero means no samples,
not EOF; first_frame scoped to endpoint lifetime, gap/discontinuity
explicit. PCM_WRITE accepted_frames <= requested, zero is backpressure,
only definitely unaccepted suffix may be resubmitted. Separate queues
and scheduling budget, bounded samples; required audio loss terminal
to selected compound. Only advertise tested typed sample modes; do not
send caller PipeWire paths, session credentials or raw provider fds.
Caller-side graph/bridge remains session-owned; USB/IP retained snapshots
operation 8 remains unchanged. Specific caller-session bridge integration
and supported mode matrix need local evidence before tag.

## 6. Errors

ErrorBody = code:u16, flags:u16, detail:Text, typed_present:bool,
[V if typed_present]. Optional family-typed detail must use the negotiated
error-detail schema for the originating operation (e.g. native validation
reason or cleanup diagnostics); not permitted before negotiation or for an
unowned handle. Typed error details are required where the facade needs
them to preserve a documented family error's discriminants/fields. Flags bit0 terminal,
bit1 cleanup_uncertain; rest zero. Code 0 OK only in CLOSED terminal
status, not ERROR. Unknown nonzero codes map to typed UnknownRemoteError
(code, flags, bounded text); unknown flags fail closed. Typed mapping to
existing family errors and ControllerError preserves domain and initiating
vs cleanup reason; additional typed transport variants are compatibility
surface. Text is optional (zero length), not parsed for recovery. No
foreign owner disclosure, private path/identity or arbitrary errno codes.

| Decimal code | Name | Meaning |
| --- | --- | --- |
| 0 | Ok | Orderly close only |
| 1 | MalformedFrame | Invalid header/body/shape; terminal if framing ambiguous |
| 2 | VersionMismatch | No overlap; terminal |
| 3 | UnsupportedFeature | Mandatory bit unavailable; terminal in HELLO |
| 4 | Unauthorized | Peer/policy denied; terminal at admission |
| 5 | UnsupportedFamily | Unknown/disabled registry family |
| 6 | UnsupportedRealization | Absent or refused exact target |
| 7 | UnsupportedOperation | Reserved/unsupported op/option/schema |
| 8 | NotOwned | Unknown or foreign handle; identical result |
| 9 | StaleSlot | Wrong incarnation or absent child |
| 10 | InvalidInput | Typed family validation rejected |
| 11 | Capacity | Quota denied before allocation |
| 12 | WouldBlock | No new admission/delivery; retry only definitively unsent work |
| 13 | Timeout | Proven pre-accept expiry; otherwise Uncertain |
| 14 | Uncertain | Acceptance/delivery unknown; terminal owning boundary |
| 15 | ProviderUnavailable | Required provider/host access not available |
| 16 | ProviderFailed | Required provider failure; owning boundary terminal |
| 17 | CleanupFailed | Terminal; preserve uncertainty flag if resource not proven free |
| 18 | Closed | Object already terminal for mutation |
| 19 | Overflow | Queue/resource overflow; terminal unless proven pre-admission rejection |
| 20 | DaemonStopping | New work refused; terminal for connection on drain |
| 21 | Disconnected | Client-local terminal transport loss; last view not proof of final state |
| 22 | ResourceUnavailable | Typed resource absent or not supported |
| 23 | HelperUnavailable | Privileged attachment not ready, no fallback |
| 24 | ProtocolViolation | Wrong ordering/ID/mandatory message; terminal |
| 25 | Internal | Unclassified internal failure; fail closed |
| 26 | EpochMismatch | Reserved for future journal feature |
| 27 | InvalidCursor | Reserved for future journal feature |
| 28–65535 | Unassigned | Never recycle published values |

Code 21 is client-local unless a service has an explicitly known underlying
disconnection to report; bare terminal EOF maps to Disconnected, or Uncertain
when an unacknowledged sent request exists. Overflow of existing optional
observations is *not* a terminal error; report loss counter. Capacity or
WouldBlock explicitly means request not admitted or commit retry state,
never silent loss of previously accepted input. Cleanup failure cannot be
turned into normal close by repeated CLOSE. Preserve last validated views
and diagnostics, including initiating and independently reported cleanup
error where possible.

## 7. State machines, lifetime and limits

Connection states:
- Accepted/auth pending: check peer, reserve admission. Denied closes; no
  handle. HELLO timeout/invalid closes.
- Negotiating: parse HELLO, send WELCOME within deadline; discover caps.
  Failure closes; no resource allocation. Duplicate HELLO invalid.
- Active: only negotiated messages, owner-scoped objects; independent
  controller servicing. No idle expiry: a connected, authenticated client
  keeps its controllers however long it goes without sending requests.
  PING/PONG is an optional client liveness probe.
- Draining: explicit service stop or terminal policy; reject new create/
  edit, attempt DRAINING and terminal notices, bounded cleanup. EOF,
  SHUT_WR (RDHUP), SHUT_RD causing send failure, HUP, SIGKILL last fd,
  invalid required messages and stalled egress take terminal cleanup path.
  Half-close is not a request to retain devices. No lease.
- Closed: all owned handles destroyed/released or external resources
  quarantined under verified owner record. Restart starts fresh, never
  recover app handles; connected fd delegation not supported in 1.0.

Handle states:
- Absent -> Preparing: validate all typed options, caps, quotas,
  reservation and required components. No publish to app before prepare.
  Open may temporarily be visible to OS; preflight failure no opens;
  partial failure reverse rollback and retain cleanup diagnostic.
- PreparedAwaitConfirm: CREATED enqueued and sent; handle only permits
  CREATE_CONFIRM, CLOSE, enumeration/read metadata. Confirm deadline
  begins on enqueue, not client receipt. No active inputs; service
  required host protocol even here. Lost reply/timeout/disconnect tears
  down; no retry/recreate automatically.
- Active: matching confirm accepted and CONFIRMED sent; client returns
  handle only on receiving it. Loss of final reply means client must
  terminate connection by deadline, daemon cleans on EOF/liveness.
  No finite protocol can prove final ACK receipt after peer crash;
  bounded uncertainty plus connection ownership is the contract.
  Failed confirm / undeliverable reply closes; duplicate confirm invalid.
- Closing: fence new mutations, neutralize where meaningful without
  delaying required replies, finish bounded definitive delivery or close
  via ADR-0004 uncertainty rules. Close required group in dependency
  order, retain final observed snapshots and original/cleanup diagnostics.
  Slot-local failure may close only isolated slot; shared required
  failure closes entire receiver.
- ClosedOwned: terminal, no provider work, bounded immutable readback
  and explicit resource resolution while owned. Repeated CLOSE returns
  same cached CLOSED status. RELEASE discards, cannot allocate same H.
  Connection death releases all closed records including unsaved images.
  Successful closed status requires provider cleanup proof where available;
  failed helper release retains verified journal quarantine, not an app lease.

Daemon stop also stops accepting on activation socket (coordinate unit
ordering), closes live handles, attempts typed terminal delivery, then
closes client fds; systemd hard kill beyond budget is abrupt loss, not
successful drain. Kernel external resource recovery on next start
validates exact journal ownership and port free; no blind detach or
record clearance. Unavailable helper must not block ordinary startup.
Service-created controllers may be observed by unrelated host consumers;
owner-scoped RPC enumeration does not isolate host visibility.

Proposed engineering defaults (not benchmarked). Server advertises effective
limits in WELCOME in this exact order (u16 unless stated u32). Cap may be
lower by policy, never above protocol caps; client enforces advertised
effective limits, service independently checks. Quota counters charged
before allocation/open, include preparing/closing/closed-owned storage,
and return on rollback/release. All counts checked; on pressure reject
before acceptance or terminally close on inability to preserve accepted work.

| Limit field | Default / cap |
| --- | --- |
| max_connections, per_uid_connections | 32, 8 |
| max_handles, per_uid_handles, per_connection_handles | 64, 16, 8 (live + prepared + closed-owned) |
| max_pending_creates, per_uid_pending, per_connection_pending | 8, 2, 1 |
| max_slots_per_handle, max_slots_total | 256, 512 (family-specific actual capacity; includes reserved) |
| max_inflight_per_connection | 16 |
| queued_frames_per_handle, queued_frames_per_connection | 16, 64 |
| queued_bytes_per_handle:u32, queued_bytes_per_connection:u32, queued_bytes_per_uid:u32, queued_bytes_total:u32 | 262144, 1048576, 4194304, 16777216 |
| fds_per_handle, fds_per_connection, fds_per_uid, fds_total | 32, 128, 256, 1024 (all owned and transient, including helper handoff; reserve daemon listener/control fds separately) |
| memory_per_handle:u32, memory_per_uid:u32, memory_total:u32 | 2097152, 16777216, 67108864 (service-owned buffers including per-family journal and typed caches; separate measured kernel socket buffers/worker RSS) |
| pcm_frames_per_endpoint | 2048 |
| hello_ms:u32, request_ms:u32, prepare_ms:u32, confirm_ms:u32 | 3000, 1000, 10000, 5000 |
| egress_stall_ms:u32, drain_ms:u32 | 1000, 5000 |

Hard max counts beyond above: capabilities 256, slots 256/handle,
schemas 64/entry, observations <=32 per packet, typed views <=16;
parser never allocates from unchecked length. Both directions queue
accounting includes encoded bytes and metadata; charge conservatively
for buffers/caches and family memory, account transient fds and worker
resources separately in helper-specific quotas. Connection quota includes
idle/auth pending. Active connections have no idle or heartbeat expiry:
dead clients are detected by EOF/HUP on the local socket, so an app that
pauses its loop (loading, debugger, suspend) keeps its controllers and the
service keeps answering required replies. Only the HELLO deadline and the
egress-stall rule for required frames (below) close a live connection;
optional observations are evicted with loss counted, never a reason to
close. Quota held by a paused but connected app stays charged to its UID.
Service must provide fair cycle (e.g. one admitted mutation and bounded
IPC frames before native service, round-robin across ready handles,
earliest required retry deadline first); use independent bounded tasks for
blocking prepare/teardown. Reserve at least two frame slots and enough
bytes for error/terminal per connection outside optional egress, accounted
within totals. Full optional observation queue evicts oldest and reports
loss, reserve does not displace required protocol replies. Required
response or terminal notice undeliverable for egress deadline closes
connection. No unbounded thread, queue or tombstone archive.
Absolute request/prepare/confirm/close budgets never extended by partial
progress. CREATE uses prepare_ms rather than request_ms from admission to
CREATED; client waits no longer than prepare_ms plus the egress bound.
Confirm deadline applies from CREATED enqueue through confirmation; the
client uses the remaining conservative local budget for initial reads and
client-side setup or aborts. CONFIRMED must be delivered within the same
budget plus egress bound. CLOSE may use drain_ms (rather than request_ms); true uncertainty
reported if deadline or provider cleanup fails. Family journaling caps
remain ADR-0024-owned; charge full reserved capacity, reject creation if
budget cannot fit largest family record. Tune only with owner review and
local stress evidence before freeze.

## 8. Reserved journal and lease extensions (not implemented)

Reserve codes 0100 JOURNAL_READ / 0101 JOURNAL_BATCH, bit 1. Proposed
semantic envelope for future minor (not a normative 1.0 payload):
H,S, opaque epoch token, next_sequence:u64, max_records:u16,
max_bytes:u32; response has same epoch, oldest_available:u64,
next_sequence:u64, next_cursor, optional SequenceGap
[first_missing, first_available), bounded typed records in ascending
sequence (record discriminator incl accepted/rejected/lifecycle/feedback-
scheduled, sequence, optional monotonic sample and retained revision,
family-local V), optional typed terminal/high-water. Sequence starts at 1,
no wrap, stale epoch or future cursor rejected. Terminal read only while
closed handle still owned; no lossless claim after gap. Journal owned by
controller under ADR-0024, no service copy/archive, subscription,
mandatory reader ACK, raw output passthrough or alternate haptic/speaker
path. No USB/IP stream even if feature implemented on UHID/uinput; worker
op8 is retained snapshots only. Exact epoch width, record/batch bytes,
limits and additional codes frozen only by separately approved minor;
currently reject any request and required feature before allocation.
No feature advertised in Alpha 2; in-process VG-2 and haptic/Wii gates separate.

Reserve 0200–02ff and bit 2 for explicit lease/transfer: later separately
authorized policy must define scoped unguessable token, expiry, owner
reauthentication, neutral safety, revocation and terminal generation;
no lease state or parser in 1.0. D-Bus/per-user/portal mediator can future
delegate an authenticated session with bounded scope, not claim credential
equivalence of a forwarded socket; keep this framing and negotiate any
new mandatory authorization features. C ABI can wrap wire and typed
schemas with opaque handles; no C ABI promise in 1.0.

## 9. Compatibility and validation

At tag freeze major 1, 1.0 grammar, errors, existing registry ID meanings,
the schemas of capability entries marked stable, and the documented typed broker client API.
Highest compatible minor negotiation; additive operations/features only,
never reinterpret an old numeric value; new family IDs do not require a
protocol bump. Unknown optional capability ignored, required rejected.
Deprecate with replacement, migration notes, >=2 stable minor releases
and >=6 months (both), no removal within major 1. Breaking change requires
new major and Anders approval; coexistence period and explicit negotiation,
no silent translation or fallback to legacy private v1/v2. Security fail
closed openly with advisory and migration. Direct-only unrelated APIs,
experimental SPI, implementation details remain alpha-mutable until beta.

Cloud acceptance: encode/decode goldens including HELLO, max length, unknown
feature rules, malformed/truncated packet, bad credentials simulation,
caps bounds, schema mismatch, unknown required/optional messages, correlation
ID reuse/wrap, ordered mutations, independent queues, slot incarnation,
create rollback and lost confirmation/reply, repeat close, terminal loss,
EOF/half-close, stalled admission and fair scheduling, fake required
HID/FF requests under saturation (100ms existing retry policy unchanged),
retained snapshots and loss accounting, journal reservation and USB/IP
refusal. Fuzz bounded decoder and check no allocation before validation.
Tests for helper stale records/free-port proof and duplicate fd teardown
must preserve all existing regressions. External pinned Rust consumer
fixture with at least two families both modes; exact stable API inventory
and native schema golden fixtures published by implementation. Local
kalux/systemd negative permissions, peer identity, SIGKILL, abrupt
restart, owned node/fd/port removal, independent consumer recognition,
audio smoke and saturation benchmarks per ADR. Not run in this docs PR.

See [ADR-0026](../adr/ADR-0026-broker-service-and-client-protocol.md) for
owner decisions, host audit impacts and ordered implementation slices.
