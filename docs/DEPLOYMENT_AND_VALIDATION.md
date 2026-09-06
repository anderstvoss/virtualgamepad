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
allow_uid=1000
```

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
performs stale ConfigFS recovery. A manually invoked broker refuses to replace
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
