# Revised USB audio design: stock Linux requirement

Decision constraint: the maintainer requires stock Linux; do not maintain or
install a custom kernel/module to satisfy the audio milestone. EXP-0024 supersedes
the original dummy_hcd/FunctionFS transport assumption, not the creation-time audio
contract or broker security gates.

## Current acceptance ledger (2026-09-23)

The installed, connection-owned broker and dedicated unprivileged worker passed
the ordinary-user mixed/duplicate-session isolation and worker-death recovery
probe. The production USB PCM path completed three exact 60-second full-duplex
DualSense sample-access trials after warm-up. A subsequent DS4 trial failed
with a correlated approximately 200 ms host scheduling interruption and
observable silence/gaps. A fresh Xbox360 attempt passed three consecutive
60-second full-duplex trials with exact synthetic patterns. DS4 then passed
three consecutive trials at a test-only 12 ms microphone operating fill,
following another failed 8 ms-fill attempt. These findings are detailed below;
previous failures are retained as historical evidence. Public root USB creation, native-client
bridging, directional end-to-end latency measurements and the complete sustained
matrix remain open. Physical DualSense headset/grip channel order was confirmed
again, while speaker routing and microphone response remain unproven on this
host. No matching profile is enabled.

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

The staged session owner now validates the worker's exact readiness version and
generation, retains the child and kernel socket, and supports bounded, idempotent
cleanup. Child death closes that socket; rollback never detaches by port number.
Deterministic tests cover readiness rejection, child death, repeated cleanup and
preservation of unrelated sockets. This owner still needs wiring into the daemon's
attachment operation and ownership journal before ordinary-caller use is enabled.

Worker close acknowledgement now follows successful USB/PCM thread shutdown.
An acknowledgement therefore certifies processing has stopped and sample endpoints
are closed; cleanup failure terminates the control channel without a success reply.

## Staged daemon attachment and conservative recovery

The daemon now dispatches an explicit version-two audio connection to the fixed
VHCI factory when the administrator configures audio ports and worker credentials.
Version one remains available for existing realizations; switching versions within
a connection is rejected. USB-only configuration no longer requires ConfigFS
recovery. The public root USB realization remains disabled pending installation
and security acceptance.

The factory reserves a shared allowlisted port, starts the installed worker,
validates readiness, creates a durable ownership record and rechecks the port
immediately before attachment. Before handoff it checks the owned device identity,
high-speed status, selected configuration and all four compiled audio/HID interface
classes. Setup failures close only the owned socket and process. This path has not
yet received privileged live acceptance.

Provisioning must create `/run/virtualgamepad-state/<instance>.audio` owned by root
and not writable by other users. Records contain generated creation identity,
USB/IP device number and reserved port; none authorizes detach or process killing.
Successful owned-resource cleanup removes only the record with the saved inode.
A replaced or symlinked record is retained. Startup reports remaining records and
refuses new audio work for administrator review; it leaves unverifiable attachments
untouched. Automatic reconciliation of stale records remains deferred rather than
inferring ownership from a reused port or PID.

Regression tests cover durable retained records, duplicate identities, capacity,
replacement/symlink protection and enumeration identity mismatches. Installation,
privileged failure injection, public consumers and sustained latency acceptance
remain required. Physical tests requiring human observations must begin only after
the maintainer confirms readiness; mere device attachment is not that confirmation.

## Explicit audio-broker installation and ordinary-user probe

Build the fixed production binaries before installation:

```sh
cargo build -p gr-privileged-broker -p gr-audio-worker
python3 scripts/install-audio-broker.py --port 0 --port 1 --port 2
sudo /usr/bin/python3 -I scripts/install-audio-broker.py --apply --port 0 --port 1 --port 2
```

Select ports reserved by the administrator; this example authorizes three ports.
The installer creates a dedicated non-login worker account, root-owned binaries,
policy and runtime journals, and an audio-specific socket-activated service. It
refuses differing existing policy/service files and refuses to replace an active
broker. It does not load modules, grant sound-device access, route physical audio,
or install sudo delegation. Provision the stock VHCI module separately. The
socket uses the invoking user's primary group, while the daemon still authorizes
the exact numeric UID using peer credentials. No group-login refresh is needed.

The service permits credential dropping and the fixed VHCI attachment operation;
workers drop credentials, supplementary groups and capabilities before processing.
This configuration requires live privileged acceptance and is not a security
acceptance result by itself. Existing gadget-only service files are not silently
replaced by the audio installer; migration needs administrator review.

After installation, run the bounded ordinary-user probe:

```sh
cargo run -p gr-audio-worker --example broker_audio_probe
```

It creates each compiled family, reads worker diagnostics, verifies acknowledged
idempotent closure and checks endpoint EOF. It opens no physical audio device and
is not the sustained audio acceptance harness. The private broker client verifies
a root peer, protocol version, nonzero creation identity, exact descriptor handoff
and bounded creation/cleanup replies. Public root API integration remains gated
on installed-broker acceptance.

The first installed-service smoke run started the broker but creation returned EOF
before attachment; VHCI ports remained free and no worker survived. This is a
failed run, not installation acceptance. The launch path now uses `fexecve` on the
verified image descriptor instead of resolving a procfs descriptor pathname after
credential changes. A process regression exercises descriptor execution without
a usable command pathname. Version-two setup errors now return a bounded error
reply, with a regression for failure before handoff. Reinstallation and a fresh
installed-service probe are required to identify or confirm resolution of the
host failure. The physical DualSense is disconnected; physical acceptance remains
pending reattachment and explicit readiness confirmation.

The next installed probe returned `EPERM`. The running root broker exposed
`CAP_SYS_ADMIN` and `CAP_SETGID` as effective capabilities but lacked `CAP_SETUID`,
despite all three being in its bounding set. The installer now explicitly carries
these three capabilities in its ambient set, following the distinction described
in [systemd's execution documentation](https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml).
The worker explicitly clears effective, permitted and inheritable sets after
credential dropping. Regressions check missing effective launch capabilities and
zero worker capability sets across exec. A fresh installed run is still required;
the precise cause of the original effective-set omission is not established.
The installer migrates only the exact prior generated service template; differing
administrator configurations still require manual review.

After the capability update, installed creation reached VHCI attachment but the
broker rejected enumeration with an identity mismatch. Linux `port_show_vhci`
reports zero speed/device/socket fields until `VDEV_ST_USED`; the intervening
`VDEV_ST_NOTASSIGNED` state therefore cannot be checked against the final device
identity. The parser now waits through canonical 004/005 rows, validates identity
only at 006, and rejects malformed pending rows, wrong hubs, and terminal/unknown
states. The regression covers the full 004→005→006 sequence plus failures. This
fix needs installation and a fresh probe; the previous run remains a failure.

## Installed production path: lifecycle and initial duplex evidence

With the assignment-state fix installed, the ordinary-user probe passed all three
families: creation, worker diagnostics, native-state updates, acknowledged repeated
closure and endpoint EOF. The isolation probe also passed duplicate/mixed sessions,
three-port exhaustion, independent middle removal, recreation, abrupt client
closure and malformed-generation worker termination. These are scoped lifecycle
results, not the complete privileged crash/restart/security matrix.

The new `validate-broker-audio-live.py` uses the installed broker's PCM channels
and host ALSA simultaneously. It resolves the current owned card, waits for udev
settlement and disables only that virtual card's PipeWire profile for the direct
ALSA trial (`save=false`); the owned device is removed afterward. It does not
select physical devices or modify desktop defaults. Initial direct-ALSA failures
were a udev readiness race and session-manager contention; both remain recorded
as failed attempts, and the harness now prepares explicit exclusive access.

Three-second post-warm-up duplex trials passed exact playback/microphone patterns
for DS4, DualSense and Xbox360. Longer DualSense trials with 4 ms operating fill
failed with 48 silence frames. At 8 ms fill, DualSense trial 0 passed all 2,880,000
post-warm-up microphone frames and 2,976,000 total playback frames, but trial 1
failed with 48 silence frames. DS4 trial 0 failed with 96 silence frames. Playback
remained exact in these longer runs. Command interval and worker scheduling
lateness are not end-to-end latency measurements; no sub-20 ms acceptance is claimed.

The refill feedback had used USB completion counters, which lag per-packet queue
consumption. A new private worker operation exposes actual microphone frames
removed from the queue, excluding underrun silence. A deterministic multi-packet
request regression proves consumption advances before completion and silence does
not advance consumption. The new control client validates native transactions,
outputs, counters and acknowledgements; stale replies terminate it. Reinstall the
updated worker before rerunning the revised consumption-based harness:

```sh
cargo run -p gr-audio-worker --example broker_audio_isolation
python3 scripts/validate-broker-audio-live.py --profile dualsense --seconds 60 --trials 3
```

The 8 ms value is test operating fill, not a public API buffer-size promise. Whether
the corrected feedback eliminates the observed underruns remains a live-test gate.
Public root USB integration, native-client bridging, measured directional latency,
full acceptance matrix and remaining physical comparisons are still outstanding.

## Consumption feedback and idle-worker recovery follow-up

The consumption-credit worker was installed and tested through the production
broker. DualSense and DS4 each passed their first two consecutive 60-second
post-warm-up duplex trials: 2,880,000 exact microphone frames and 2,976,000 exact
playback frames including warm-up, with no gaps or silence. Their third trials
failed: DualSense had 144 microphone silence frames and DS4 had 384. Playback
remained exact throughout, without device disappearance or IPC errors. Workspace
validation overlapped parts of these runs; causation is not established. These
are failed three-trial acceptance attempts, not sustained acceptance passes.
Xbox360 then passed three consecutive 60-second post-warm-up duplex trials
with the same exact counts, zero gaps, silence, IPC errors, or device loss. Its
refill-timing field in that run included inactive intervals after capture ended;
the later harness correction excludes those intervals. The result demonstrates
continuity for one installed Xbox sample-access path, not the required latency
or native-client acceptance.

The harness now retains the eight largest caller refill intervals, credit reply
durations, and associated microphone consumption positions. Storage stays bounded;
these observations diagnose scheduling and refill behavior, not end-to-end audio
latency. A deterministic test covers retention bounds and frame correlation.
Microphone credit replies also validate their generation and cannot exceed frames
submitted by that client.

An independent lifecycle audit found that worker death while a client remained
idle could retain the broker's reservation and ownership record until the client
closed its connection. The broker now checks its owned child while waiting for
requests, with a 100 ms idle check interval; partial requests still have the
existing absolute one-second deadline and ancillary rejection. Failure unwinds
the connection-owned session and admission permit. A deterministic regression
retains all client descriptors and verifies cleanup and quota release on worker
death alone. The installed isolation probe now also requires all three ports to
be reusable while the failed client's broker connection remains open. Installation
of the updated broker and a fresh isolation run remain necessary for live proof.

After the broker and worker update, installed file hashes matched the tested
builds. The ordinary-user isolation probe passed again, including the new case
where a malformed-generation request kills one worker while its client keeps the
broker connection open: all three allowlisted ports became reusable, and the
other sessions remained independent. A three-second DualSense duplex run with
the new PCM-pump counter passed exact samples in both directions, zero
microphone silence and zero abandoned capture frames. Its maximum recorded USB
audio and PCM-pump lateness were 1.390 ms and 1.786 ms respectively. A longer
counter-correlated run was then completed; the short observation alone does not
close the continuity or end-to-end latency gates.

The installed DualSense USB sample-access path passed three consecutive
60-second full-duplex trials after a two-second warm-up. Each trial captured
2,880,000/2,880,000 exact microphone frames and the worker accepted the
2,976,000 playback frames including warm-up; no playback-invalid frames,
playback gaps, microphone silence, abandoned capture frames or IPC errors were
reported. The maximum recorded USB audio deadline lateness was 4.179, 3.213
and 2.581 ms; maximum PCM-pump lateness was 3.256, 4.026 and 3.785 ms.
The largest client refill intervals were 5.940, 7.590 and 5.934 ms. These
are correlated process/host counters, not measured end-to-end latency.
Prior failed runs below remain part of the evidence; this pass supports
continuity under the measured conditions without proving all access modes or
families stable.

The next isolated DS4 attempt failed trial 0. At measured frame 2,215,152
the host received silence; 20,528/2,880,000 microphone frames were silent.
Playback had two observed position gaps, though its nonzero frames had the
exact channel pattern. The worker reported 20,528 microphone-silence frames,
zero abandoned capture frames, and maximum USB-audio/PCM-pump lateness of
200.751/198.493 ms. The caller's largest microphone refill interval was
259.102 ms. This is a correlated scheduling interruption, not evidence that
the DS4 profile itself has a different stream mapping. It is still an
acceptance failure; an 8 ms operating fill cannot cover a 200 ms host stall
while satisfying the sub-20 ms design target.

At the original 8 ms test-client fill, the DS4 retry passed trial 0 but failed
trial 1 with 576 microphone silence frames. Playback remained exact and
gap-free. The maximum recorded USB/PCM-pump lateness was 8.147/8.128 ms and
the largest caller refill interval was 10.255 ms. The harness now accepts an
explicit, bounded 1–16 ms test-only microphone fill and records the selected
value. At 12 ms, DS4 passed three consecutive 60-second full-duplex trials:
each had 2,880,000/2,880,000 exact measured microphone frames, 2,976,000
exact playback frames including warm-up, and zero reported silence, playback
invalid frames, gaps, abandoned capture frames or IPC errors. Maximum USB-audio
lateness was 3.219, 3.445 and 4.115 ms. This supports a larger normal fill
for that host/profile under those conditions; the earlier failures remain
valid. The 12 ms fill's directional end-to-end latency remains unmeasured, so
this is not a sub-20 ms latency acceptance result.

A fresh installed Xbox360 run then passed three consecutive 60-second
full-duplex trials. Each trial had 2,880,000/2,880,000 exact measured
microphone frames and 2,976,000 exact playback frames including warm-up,
with zero reported microphone silence, playback-invalid frames, playback gaps,
abandoned capture frames or IPC errors. Maximum USB-audio lateness was 1.815,
3.926 and 2.684 ms. This is sustained continuity evidence for USB sample
access, not an end-to-end latency measurement or native-client acceptance.

A second DualSense three-trial attempt ran without concurrent workspace builds.
Trial 0 passed exactly; trial 1 failed with 96 microphone silence frames at
captured positions 1,602,864–1,602,959. Playback remained exact and no device or
IPC failure was reported. The largest caller refill interval was 9.196 ms at a
later consumption position, so that observation does not explain the gap. The
harness now captures worker transfer, microphone-silence, stall, frame,
abandonment and maximum scheduling-lateness counters after its PCM client
threads stop. A short three-second check confirmed that diagnostics path; the
longer counter-correlated run subsequently passed trials 0 and 1, then failed
trial 2 with 96 silence frames at positions 1,631,712–1,631,807. The worker
reported exactly 96 microphone silence frames, zero abandoned capture frames,
and maximum USB audio lateness of 2.066 ms. The largest caller refill interval,
8.610 ms, occurred at a later consumption position. This narrows the gap to the
worker's microphone supply at capture time; neither counter establishes the
PCM pump thread's scheduling history. A separate private worker operation now
reports maximum excess over that thread's 500 µs nominal pump interval. The
installed worker must be updated before a longer run can use this counter. The
failure remains unresolved, and no end-to-end latency claim follows from these
scheduling counters.
