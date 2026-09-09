# ADR-0010: explicit compound association

Status: implemented native association and required protocol-group helpers;
DS4 isolated production acceptance remains pending.

`CompoundIdentity` holds separate 128-bit logical and creation tokens supplied
by the controller owner. Identity policy and entropy stay outside providers.
`ComponentId` is the package-owned role; deriving a component yields a stable
logical unique label and a fresh creation physical label. Application session
IDs are neither token. Callers must use a fresh creation token for every live
creation and preserve the logical token only under their persistence policy.

`CompoundSession::open_associated` prepares these labels before I/O. UHID gets
physical and unique labels; uinput gets the physical label. Duplicate roles fail
before opens, and partial opens roll back in reverse order. The compiled gadget
path rejects this API before any component opens because it cannot carry these
labels. Existing unassociated low-level construction remains available.

Diagnostics retain requested logical/creation/role association after close. A
provider may separately return an observed host path, valid only while open.
Uinput uses UI_GET_SYSNAME, validates the returned input name, and exposes its
sysfs path. Consumers must verify physical labels and host ancestry before using
child nodes. A matching display name alone is insufficient. Missing observed
metadata is represented by None, never a guessed path. Other providers currently
return None; their existing exact-session host inspection remains necessary.

NativeEvdevRealization gains optional physical_path metadata, validated as a
nonempty NUL-free label of at most 255 bytes. Uinput passes it through UI_SET_PHYS;
it is not a path to open. Ordinary curated evdev creation receives a fresh
process/creation label without deriving identity from application IDs.

Deterministic tests cover label derivation, maximum role, same logical identity
across creations, duplicate roles, every failed open position, actual prepared
labels and retained diagnostics. The ordinary-user uinput test queried the exact
kernel object, verified its physical label and confirmed removal/idempotent
cleanup. No permission/module/service changes were made. No compound host atomicity,
consumer association across reconnect or production DS4 split acceptance is
claimed. All support matrix cells remain WIP.

## Required protocol service ownership

`gr_hid::RequiredGroup` owns up to 32 components with distinct controller-supplied
roles. `ServicedComponent` is implemented by existing Runtime and may be implemented
by a controller-owned enum for heterogeneous personalities. The group visits each
component once per cycle, preserves its bounded transport budget, aggregates the
earliest deadline and per-component read/write interest, and keeps native editing
on the component type. It adds no threads or new dependencies.

Terminal service errors or a closed required component close the group in reverse
order. QueueFull, InvalidState and TimeReversed remain recoverable only while the
component is still open; siblings still receive service. Logical protocol removal
uses explicit group close. Consumer CLOSE/OPEN remains protocol-owned and does not
terminate the library. Request IDs and bounded retry/deadline authority remain in
each Runtime. GroupCycle returns prior component observations even if a later
component fails, with component-scoped failures. At most 128 optional observations
are retained per cycle with explicit omitted counts; no global hardware chronology
is inferred. Required work completes or is cancelled before callers observe them.

Creation rejection cleans every supplied component and returns cleanup failures.
Runtime now retains automatic-close failures so the group can report them after
a terminal request deadline or uncertain write. Close is idempotent, retains
failure diagnostics and removes scheduler interests; no cleanup retry or kernel
success is inferred. Fake-clock tests cover reused requests, all-role service,
consumer reopen, deadline boundary, uncertain delivery, prior observations and
cleanup failure. This helper is not yet a shipped multi-node controller profile.

### Single-component diagnostic review

Current curated handles expose `association()`: requested physical/unique labels,
controller-owned primary role and an optional host path observed once at creation.
The latter is cached to avoid provider I/O in frequent UI snapshots, and retained
with terminal diagnostics for cleanup investigation. It is historical evidence,
not authority to operate on a re-enumerated node. Callers must verify current
identity and host ancestry. Compiled gadget profiles expose no invented labels.
The test-only DS4 prototype now passes explicit logical/creation identity through
associated opens; its reused-ID test checks distinct creation paths and shared
role prefixes. This does not enable production DS4 split wiring.
