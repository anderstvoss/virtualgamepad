# Deployment and validation

The implemented realization IDs (`linux.uinput`, `linux.uhid.usb`, and `linux.dummy_hcd.usb-hid`) are peers. A controller is created only for the exact target selected by the application and declared by that controller. There is no target ordering and no fallback.

`Evdev` uses an already accessible `/dev/uinput`. `Uhid` uses an already accessible `/dev/uhid`. Neither provider changes permissions or host setup.

## Privileged attachment targets

`DummyHcd` requires the root-owned `virtualgamepad-broker` service. Unprivileged clients use the fixed group-restricted Unix socket `/run/virtualgamepad/broker.sock`. The broker independently authenticates every client with `SO_PEERCRED` and accepts only administrator-authorized UIDs from `/etc/virtualgamepad/broker.conf`:

The socket directory is traversal-only (`root:root 0711`) so members of the
socket group can reach its fixed pathname without gaining directory listing
access. The socket remains `root:virtualgamepad 0660`; both that group check
and the broker's UID authorization must succeed.

```ini
# One entry per local application UID.
allow_uid=<application-uid>
instance=default
allow_udc=<reserved-dummy-udc>
```

Replace the placeholders with the intended numeric UID and an explicitly reserved
name such as `dummy_udc.0`. The config and every parent directory must be
root-owned, not group/world writable, and not symlinks. Older UID-only configs
now fail closed: add the instance and UDC authorization before upgrading.

Administrator setup loads `libcomposite`, `usb_f_hid` (unless built in), and
`dummy_hcd` as needed, and prepares the existing ConfigFS gadget mount. Runtime
creation never invokes modprobe or mounts filesystems. Install the optional
`systemd/virtualgamepad-broker.tmpfiles.conf` with the broker and apply it to
create its private state directories. For a non-default instance, the
administrator creates its own root-owned mode-0700 state directory.

Install `systemd/virtualgamepad-broker.socket` and `systemd/virtualgamepad-broker.service`, install the broker executable at `/usr/libexec/virtualgamepad/virtualgamepad-broker`, then enable the socket:

```bash
systemctl enable --now virtualgamepad-broker.socket
```

The service intentionally has no network listener. Its protocol only permits
opening one of the compiled `DualSense`, `DualShock4`, `SwitchPro`, or
standard-HID `Xbox360` profiles, their exact fixed-size input reports, bounded
reverse-output polling, close, and diagnostics. Session handles are
broker-created, connection-bound, and invalid after disconnect or restart.
Clients cannot supply USB descriptors, identities, module names, ConfigFS
paths, UDC choices, or report schemas.

The broker validates systemd's `LISTEN_PID`/`LISTEN_FDS` contract before it
performs stale ConfigFS recovery. It also holds an exclusive cross-process
lock and recovers only that instance's journaled gadgets whose filesystem
identity and authorized UDC binding still match. Unjournaled legacy gadgets
require explicit operator review; a name prefix is not proof of ownership.
Records survive service restarts under `/run/virtualgamepad-state` and are
removed only after successful cleanup. A crash between directory creation and
record creation leaves an unjournaled directory for operator review. A manually invoked broker refuses to replace
an existing socket and never cleans up service-owned gadgets.

## Demo validation

For an interactive local check, run `cargo run -p virtualgamepad-demo`, select
one of the curated controllers and `USB / dummy_hcd`, then create it. The create panel
shows whether the broker socket is reachable; creation continues to provide the
authoritative authorization and host-preflight error. The DualSense, DualShock
4, and Switch Pro panels can exercise buttons, sticks, triggers, touch,
battery, and USB motion reports at 250 Hz. The Xbox 360 DummyHcd target is a
best-effort standard-HID USB attachment under the Xbox identity; it is not a
proprietary XInput/xpad implementation. Diagnostics and reverse-output
indicators show host activity.

## Host validation

Root-only integration tests are intentionally ignored in normal test runs.
DummyHcd validation covers USB enumeration, HID feature exchange, motion input,
reverse output, and resource-only cleanup. The Linux `dummy_hcd` module exposes
a finite virtual-UDC resource: run controller-specific root tests against a
fresh attachment, rather than concurrently. SDL/Steam recognition and gyro
behavior remain controller-specific acceptance criteria. Bluetooth realizations require the separate L/M gates and have no supported installation path yet.

## Stateful UHID service and current migration boundary

Applications must service `poll_output` on controller readiness and the next deadline even with unchanged semantic state. Required GET/SET requests are handled internally; user reply callbacks are not part of startup. A malformed request is rejected, and a transport whose delivery becomes uncertain is closed. Optional notifications can overflow only with an explicit dropped-event count.

The broker still uses its compiled startup feature path. Gate G must prove staged startup, control metadata/completion support, and latency before replacing it with unprivileged dynamic protocol handling. Do not infer gadget control-request parity from UHID tests. See [host prerequisites](architecture-overhaul/HOST_READINESS.md) and the [reviewable provisioning proposal](architecture-overhaul/HOST_PROVISIONING.md).

## Permission boundaries

Builds and deterministic tests need no device access or elevation. The Python
host inventory (`python3 scripts/host-preflight.py all`) only reads metadata and
configuration; it never opens device nodes or connects to the broker. Exit 1
means at least one prerequisite is missing, denied, mismatched, occupied, or
unvalidated. `available` is an observation, not authorization or reservation.
Run it for one complete realization ID to avoid unrelated prerequisites.

Direct creation access trusts all processes under the authorized identity to
create input devices, not only the curated gamepad profiles. Opt-in installations
may use the separate rules under `udev/` and matching groups
`virtualgamepad-uhid` and `virtualgamepad-uinput`; install only the selected
provider rule and explicitly enroll the intended user. Do not use world-writable
nodes or the broad `input` group. Existing distribution/session ACLs must be
reviewed rather than silently overwritten. The VM experiments use temporary
ACLs instead of persistent group enrollment.

Consumers need access to their session's hidraw/event nodes, not the creation
device or broker socket. SDL development files, Steam, capture tools, corpus
credentials, and physical reference devices are validation prerequisites only.
Audio/Bluetooth setup is deferred to its own realization gates.

The runtime service no longer has CAP_SYS_MODULE or ambient capabilities and
has ProtectKernelModules enabled. CAP_SYS_ADMIN is explicitly retained pending
the live Gate G reduced-capability test; its necessity is **not established**.
`systemd/virtualgamepad-broker-no-capabilities.conf` is an experimental drop-in,
not a validated default. Test it on the reserved UDC after administrator setup,
then remove CAP_SYS_ADMIN from the default only when that test passes. No
permission increase can restore missing gadget control metadata/completion.
