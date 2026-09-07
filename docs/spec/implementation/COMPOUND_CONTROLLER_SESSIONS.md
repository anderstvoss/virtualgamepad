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
contain every active component exactly once in deterministic order; a failed
send leaves the logical state dirty so retry resends the full frame set.

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
