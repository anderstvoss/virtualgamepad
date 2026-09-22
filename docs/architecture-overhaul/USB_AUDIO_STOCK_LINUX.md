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
