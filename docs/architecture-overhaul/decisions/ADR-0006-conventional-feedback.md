# ADR-0006 — Controller-owned conventional evdev feedback

Status: accepted for the curated evdev path; broader consumer parity remains gated.

## Problem

DS4 and Switch declared FF_RUMBLE but discarded upload/erase requests. DualSense
and Xbox exposed raw native effect bytes and relied on applications to reply later.
A callback that only logged output could therefore leave a consuming request
pending. The demo acknowledged uploads and treated them as motor activity, although
an upload only stores an effect.

## Decision

Keep transactional native input encoding. All four curated evdev sessions share
an explicit conventional-rumble policy in the controller layer. Uinput uses libc's
native ioctl structures to decode generic effect fields and executes exact replies;
it makes no controller acceptance decisions. No new runtime dependency is required.

The policy owns 64 indexed effects, matching the declared uinput capacity. It
accepts conventional rumble with replay duration/delay, rejects unsupported types
or automatic trigger fields with EOPNOTSUPP, and rejects invalid IDs or unknown
erasures with EINVAL. The provider passes kernel-assigned effect IDs; application
session IDs are not effect identities. Triggered playback and Switch HD-rumble
frequency encoding are explicit evdev limitations.

Every consumed upload/erase is answered in the same poll unless the transport
reports definitely-unsent WouldBlock. One reply and at most 32 consumed events
remain owned during retry; no further reads occur while that reply is blocked.
Eight deferred retries are allowed. Exhaustion, ambiguous write failure, removal,
read failure or batch overflow closes the session and cancels its requests.
Accepted state and completion observations become visible only after successful
reply delivery. Repeated close cannot reopen a session or replay a pending reply.

Typed Uploaded/Erased observations report the result; typed Playback commands
resolve stored magnitudes and include duration, delay and repetitions. A zero
repetition count means stop. These are replay commands, not a software mixer or
physical actuator simulation. Kernel stop-before-erase commands remain visible.
The demo shows playback activity and does not own acknowledgements.

## API migration and validation

Curated output enums now expose `ForceFeedback(ForceFeedbackEvent)` uniformly.
Remove calls to the former `reply_force_feedback_upload/erase` methods; they no
longer exist on curated controllers. Low-level provider clients still own explicit
ProviderFrame replies, while upload events now contain typed ForceFeedbackEffect
instead of ABI bytes. Existing HID behavior is unchanged.

Fake-provider tests cover all families, exact positive/negative replies, repeated
polls, playback/stop, all effect slots, updates, same-ID independent sessions,
backpressure, exhausted retries, partial reads, overflow, uncertain completion and
terminal cleanup. [EXP-0010](../experiments/EXP-0010-evdev-feedback.md) records the
separate live kernel evidence. This does not establish complete SDL evdev parity,
physical motor fidelity, trigger synthesis or a new realization.
