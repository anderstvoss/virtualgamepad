# Revised USB audio design: stock Linux requirement

Decision constraint: the maintainer requires stock Linux; do not maintain or
install a custom kernel/module to satisfy the audio milestone. EXP-0024 supersedes
the original dummy_hcd/FunctionFS transport assumption, not the creation-time audio
contract or broker security gates.

## Candidate: local USB/IP VHCI with an unprivileged device worker

Use the stock `vhci_hcd` virtual host controller and a compiled userspace USB
personality. Host ALSA sees a real virtual USB Audio interface alongside HID.
Userspace implements fixed UAC2 emulated descriptors and class/control requests;
controller-matching UAC1 remains separately evidence-gated. This replaces kernel
UAC2 gadget composition for the virtual-host path.

The [v6.12.107 VHCI receive implementation](https://github.com/gregkh/linux/blob/v6.12.107/drivers/usb/usbip/vhci_rx.c)
handles isochronous packet descriptors and completion status. Unlike FunctionFS
OUT reads, [USB/IP submit/return framing](https://docs.kernel.org/usb/usbip_protocol.html)
separates receipt of a complete OUT request from its returned status. The worker
can therefore consult the unprivileged controller personality before completing a
SET request. The initial source-backed feasibility finding is now supported by the scoped
DualSense VHCI/ALSA run in [EXP-0025](experiments/EXP-0025-local-usbip-audio.md).
Full transport and broker acceptance remain open.

The host kernel configuration includes `CONFIG_USBIP_VHCI_HCD=m`. The module was
prepared by the maintainer previously, but is absent after the latest host restart.
The subsequent lab run confirmed anonymous local stream sockets: [VHCI attach](https://github.com/gregkh/linux/blob/v6.12.107/drivers/usb/usbip/vhci_sysfs.c)
requires a stream socket and reserves only an unused port. Do not add a network
listener, remote USB export/import service or arbitrary-device forwarding.

## Ownership and security

The broker authenticates the application connection and allocates an
administrator-allowlisted, unused VHCI port. Replace UDC allowlisting with explicit
VHCI port allowlisting for this realization. No caller chooses a socket, device
ID, USB identity, descriptor, path or port. Retain the quotas/deadlines and
connection-owned cleanup already required by the audio plan.

A trusted worker drops privilege before enumeration and owns the compiled USB
personality. Production workers require a dedicated identity distinct from every
application client; running under the client UID would permit same-UID process
inspection or tampering and would not protect the compiled-profile boundary. Application clients never receive the kernel-side USB/IP socket:
that would let them replace compiled descriptors with an arbitrary device.
Keep fixed descriptors and standard USB/audio control processing in the worker;
controller-specific HID report decisions remain unprivileged and typed. Bound
all URB lengths, packet counts, pending transfers, completion deadlines and cancel
races before interacting with kernel-facing state.

PCM travels over the dedicated worker data path into bounded stream queues, never
through the broker's small privileged command channel. Native host clients use
ALSA/PipeWire on the virtual USB device. Library/native caller ownership remains
exclusive per stream group. No physical microphones or speakers are auto-connected.

Attach is staged: worker ready, descriptors/profile validated, session ownership
record durable, then port attach. Any failure drops sockets and cleans only the
verified owned port. Restart recovery must verify the active port/device/session
record; it must never detach an unrelated attachment. No broad sound ACL changes.

## Next proof before enabling a public realization

1. Bounded framing and request/cancel/completion regression tests.
2. Isolated VHCI probe with compiled HID plus UAC2 descriptors and an unprivileged
   worker; verify the anonymous-socket transport and exact SET failure replies.
3. Host ALSA playback/capture, packet timing, silence on capture underrun, explicit
   discontinuities and channel isolation; measure scheduling limits.
4. Broker port ownership, client death, malformed input, restart and cleanup tests.
5. Three-family root consumer/lifecycle and sustained audio acceptance, followed
   by native-client acceptance and physical DualSense comparison.

If live feasibility fails, report that result and revisit the transport decision;
do not substitute companion endpoints or call enumeration alone audio support.


## Bounded feasibility harness

Build and validate the worker without privileges:

```sh
cargo build -p gr-usbip --example usb_audio_probe
python3 scripts/validate-usb-audio-worker.py --worker target/debug/examples/usb_audio_probe
```

The process test covers all three compiled families, using anonymous sockets and
synthetic samples. It exercises the existing controller personality, invalid SET
completion, timed shutdown and PCM channel order. It opens no host USB device.

An administrator can explicitly run one bounded kernel probe after preparing the
stock modules. Run from the checkout, select an unused high-speed port, and keep
the command running while the unprivileged lab client examines the virtual device:

```sh
sudo python3 -I scripts/prepare-audio-lab.py --transport usbip --apply
sudo python3 -I scripts/run-usb-audio-lab.py \
  --worker target/debug/examples/usb_audio_probe \
  --profile dualsense --port 0 --seconds 240
```

The example supplies a deterministic quiet microphone pattern and drains playback
into counters; it records no physical audio. Device paths for ALSA tests must be
resolved from the newly attached device, never a card number remembered from a
previous session. Do not route a physical microphone or change desktop defaults.

The supervisor checks the chosen port twice, with the kernel's occupied-port
rejection resolving attach races. It drops worker privileges before enumeration,
clears supplementary groups and environment, prevents privilege gain, and bounds
readiness, memory, descriptors, diagnostic output and lifetime. Supervisor death
terminates the child. Cleanup shuts down the owned socket rather than detaching a
port that another session could have reused. No ACL/sudoers changes are made.
This manually authorized lab executable is not the ordinary-caller broker; compiled
worker installation, connection ownership, quotas and restart/security acceptance
remain required before the public USB realization is enabled.


While an explicitly attached lab profile is running, the unprivileged live checker
can run in a second terminal (use the matching profile and reserved port):

```sh
python3 scripts/validate-usb-audio-live.py --profile dualsense --port 0 --seconds 60 --trials 3
```

It prints a start message and one result per trial, checks synthetic microphone
patterns, and reports ALSA errors. It resolves the current card through VHCI
ancestry and refuses a different session between trials. Run only one checker on
a lab device. It retains no raw recordings and does not equate playback submission
with exact worker-side sample acceptance.

## Worker scheduling and PCM channel increment

The USB worker now waits for socket readiness or absolute capture/completion
deadlines instead of sleeping a fixed millisecond each cycle. A preallocated,
32-entry completion FIFO retains exact partial-write offsets; delivery has a
one-second absolute deadline and queue exhaustion terminates the connection.
Pending-request ownership is also preallocated. The safe readiness wrapper uses
`rustix` 1.1 (`event` feature), already present transitively in the workspace.
See the [rustix poll API](https://docs.rs/rustix/1.1.4/rustix/event/fn.poll.html).

Implementation-only PCM IPC now provides separate anonymous stream channels and
queue/socket pumps with fixed buffers. Packets validate protocol version, compiled
profile, direction, generation, frame count and position; declared gaps survive
queue transfer. Normal packets contain at most 128 frames. Partial I/O preserves
its offset, and pending consumer blocks preserve their unaccepted suffix. Invalid
input, expired partial transfers and one-second consumer stalls close the channel
and its owned queue. These implementation limits are not public tuning options.

Deterministic tests cover short writes, backpressure, FIFO wrap/reuse, deadlines,
wrong generations/formats, malformed packets, replayed frames, all profile/channel
layouts, propagated discontinuities and terminal closure. The three-family lab
process checks passed after the scheduler change. This does not establish kernel
latency acceptance or explain the historical Xbox interruption.

The separate `gr-audio-worker` executable now connects PCM channels, curated HID
personalities, native-state transactions, reverse outputs and diagnostics. Its
process harness exercises all three families without privileges or host devices.
It is not yet installed or connected to a broker attachment operation; root USB
creation and the full security/live matrix remain open.

Native-state snapshot codecs now preserve the complete DS4, DualSense and Xbox360
state, including controller-specific sequence/sensor fields, constrained touch
coordinates and battery metadata. Their private version/family tags are validated
before replacing state. The transaction ledger binds updates to a generation,
requires consecutive sequence numbers, acknowledges an identical last retry
without applying it twice, and rejects conflicting/stale retries. Invalid
snapshots leave accepted state and sequence unchanged. These are worker-side
primitives integrated into the separate worker; installation and ordinary-caller
USB creation remain outstanding.


## Production launch and descriptor boundary

The broker launch helper accepts only a compiled profile and administrator-owned
worker at `/usr/libexec/virtualgamepad/gr-audio-worker`. It validates ownership,
permissions and parent directories, then executes the verified open inode. Before
exec it clears the environment and supplementary groups, drops UID/GID, disables
privilege gain, bounds resources and requests termination on broker death. Worker
channels use broker-selected inherited descriptors; applications do not select
these descriptors or receive the kernel-facing socket.

Version-two framing is available alongside strict version-one compatibility.
Successful audio descriptor handoff is a separate bounded message containing
exactly three distinct connected Unix stream sockets. Wrong counts, duplicate
sockets, files and unexpected ancillary data are rejected with received resources
closed. Ordinary request frames still reject descriptor transfers. These helpers
are not yet an enabled daemon operation or evidence of privileged-launch acceptance.

The process harness validates both low and high inherited descriptor numbers for
all three families:

```sh
cargo build -p gr-audio-worker
python3 scripts/validate-production-audio-worker.py --worker target/debug/gr-audio-worker
```

These checks cover enumeration, exact bidirectional sample patterns, diagnostics
and closure. They do not substitute for the installed-worker security tests or
sustained kernel latency and continuity acceptance.
