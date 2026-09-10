# EXP-0010 — Conventional evdev completion and replay

## Setup

Linux arm64 `6.12.107+deb13-arm64`, ordinary-user test binaries. Read-only
preflight found uinput unregistered. The installed, previously authorized helper
loaded only uinput; existing creation and consumer access sufficed. No ACL/group,
service, capture or gadget changes were made. Prior module/ACL/helper inventory
and raw results are stored privately. This running kernel differs from the
6.12.105 host recorded in the earlier UHID/SDL experiments; those measurements
are not automatically recertified on this kernel. This task did not replace the
kernel or reboot the VM. The newly loaded module is retained for
administrator review; both temporary helper leases remain inactive.

The ignored `evdev_feedback_live` integration test creates one controller at a
time, selects exactly one new virtual input event node, verifies its name again
after opening, and runs a separate bounded consumer process. The parent services
reverse events as an ordinary user. Ambiguous enumeration aborts. No SDL or root
compiler/test process is involved.

## Result

DS4, Switch, DualSense and Xbox each passed three effect cycles. Each cycle uploads
and updates a rumble effect, starts it with two repetitions, explicitly stops it,
and erases it. For each family the consumer completed six upload/update ioctls,
three start writes, three stop writes and three erase ioctls. The controller
observed six successful uploads, three starts, six stops and three successful
erasures, with exact updated magnitudes and replay timing. Every consumer was
reaped and every created node disappeared after repeated close.

A second condition killed each family's consumer process immediately after its
first accepted upload. All four runs observed kernel stop and erase requests,
completed them without application replies, reaped the killed child and removed
the session node after repeated close. This validates consumer-exit cleanup with
a stored effect; it is separate from controller-removal-during-ioctl evidence.

The initial DS4 apparatus run failed its expectation of three observed stops;
consumer ioctls and cleanup passed. Linux sends a second stop before each erase:
see [Linux v6.12 ff-core.c, erase_effect](https://github.com/torvalds/linux/blob/v6.12/drivers/input/ff-core.c#L161-L176).
The harness now checks that exact count, and a deterministic regression preserves
both stop commands. The initial negative record remains private. Subsequent
four-family execution passed. No production suppression of duplicate stops was
introduced to satisfy the test.

## Assessment and next work

[ADR-0006](../decisions/ADR-0006-conventional-feedback.md) defines bounded
controller-owned completion and the public API migration. Provider ABI parsing
is generic, and native libc structures replace fixed-size effect byte buffers.
The demo reports playback activity rather than inferring it from storage uploads.

This closes the measured Linux FF upload/update/play/stop/erase path. It does not
close full evdev input/touch consumer parity, SDL rumble observation, remaining
family concurrency/failure evidence, or physical output fidelity. The next batch
reuses the exact-device SDL apparatus for evdev controls and conventional outputs,
then completes family failure/concurrency and demo scheduling review. Gate G
still needs reserved setup and a supported metadata/completion interface; extension
production changes remain dependent on their own gates.
