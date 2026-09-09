# Compound controller sessions and reverse transactions

This specification supports curated controllers whose Linux presentation has a
primary component and optional controller-declared companions. It is not a
generic composite-controller or input-injection API.

## Migration status

The existing compound helper remains on the earlier frame runtime. The stateful HID runtime represents one component per instance and preserves submission history across partial delivery. Do not reuse the older full-frame retry rule for non-repeatable HID actions. A future compound wrapper must service these instances fairly, preserve component identities, and retain reverse-order rollback/close. Gate E controls advertising specific multi-UHID presentations, not the existence of lifecycle helpers.

Required HID GET/SET completion now belongs to personalities, independent of optional callback subscriptions. The subscription/token machinery below remains relevant to the older native and compound paths; it is not the completion owner for migrated UHID controllers.

## Components

A controller package supplies ordered component identifiers, prepared provider
requests, encoders, and reverse decoders. The runtime preflights every selected
component before opening any. A failed later open closes previous components in
reverse order and returns the primary error plus rollback diagnostics. Commits
contain every active component exactly once in deterministic order; a backpressured
send leaves the logical state dirty so retry resends the full repeatable frame set.
Every selected component is required for the lifetime of this helper. A provider
error other than `WouldBlock` closes the group before returning the original
component/error. Invalid caller frame sets or reply routing do not close it.

Companions are disabled by default. A concrete controller creation option may
enable only its declared typed companion roles. Keyboard and pointer companions
are capability-limited prepared uinput devices; no root-level generic
constructor, caller-provided key map, arbitrary device path, or host mutation
is permitted.

## Reverse delivery

The runtime provides bounded isolated callback subscriptions and typed
one-shot reply tokens. Controller packages convert raw provider records to
their native output/request types. Callback failure, cancellation, queue
saturation, duplicate reply, and closed reply state are diagnostics and
recoverable errors; they never close unrelated components or mutate input
state. Reply payload types remain controller-native.

### Component-scoped native completion

`CompoundSession::reply(component, frame)` routes only HID GET/SET and FF
upload/erase completions to an owned component. Request identifiers may repeat
across components. Invalid components, input frames and closed sessions fail
before provider I/O. Input commits still require the entire ordered frame set;
a reply neither resends inputs nor changes their dirty state.

The helper does not own a protocol request table or retry policy. The controller
must complete or cancel each delivered request within its service cycle, with
bounded retry only when the provider confirms backpressure. Provider errors retain
the component and original error. No implicit retry is safe for uncertain writes.
Reverse records stream to the caller even when a subsequent read fails; callers
must handle those records and the terminal error together. The group is already
closed on terminal provider failure, so delivered requests are cancelled by
transport teardown rather than replied to through a surviving sibling. A partial input send
is not atomic across devices; only repeatable full snapshots use full-set retry.

These are deterministic prerequisites for DS4 gamepad/touch composition. The
current DS4 production node is still combined and its SDL discovery limitation
remains. No new companion or realization is advertised by this runtime change.

## Steam Controller 2 and Dreamcast benchmark

A future Steam Controller 2/Puck package may use a native component and an
explicitly enabled desktop keyboard/pointer companion. Its lizard policy is a
controller-native presentation policy, not a realization target. `LizardOnly`
must suppress native presentation without changing retained input state.

Dreamcast is not an implementation target. Synthetic tests use a controller
with two attachable accessory components and a 48×32 one-bit, 192-byte display
framebuffer to prove attachment, reverse request/reply, retry, and close
behavior. No Dreamcast API, Maple target, VMU storage, or hardware claim is
created by this work.

The DS4 gamepad/contact prototype currently exists only in tests. An interrupted
live experiment coincided with a reported display-session crash; see
[EXP-0012](../../architecture-overhaul/experiments/EXP-0012-ds4-compound-interruption.md).
Shared-desktop contact injection must not be repeated as acceptance. Production
adoption requires isolated consumer validation of both nodes and their cleanup.

## Required-component failure policy

[ADR-0009](../../architecture-overhaul/decisions/ADR-0009-compound-terminal-failure.md)
defines fail-closed ownership for the existing native compound helper. Close
attempts run in reverse order even after cleanup errors; diagnostics preserve
those errors without replacing the initiating failure. Repeated close and all
later I/O are terminal. A failed close is not proof that host resources vanished.

Controller packages still interpret raw lifecycle events (including STOP versus
removal), own request deadlines and bounded backpressure retry, and explicitly
close on protocol-level terminal failure. This helper does not parse lifecycle
bytes or run a clock. Optional hot-removable components and identity-derived
association need a controller-owned model before production adoption. Steam
Controller development is deferred until the overhaul lands.
