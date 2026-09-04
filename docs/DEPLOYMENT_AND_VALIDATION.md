# Deployment and validation

The three realization targets are peers. A controller is created only for the exact target selected by the application and declared by that controller. There is no target ordering and no fallback.

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
behavior remain controller-specific acceptance criteria. Bluetooth realization
is deferred to `wip/btvirt` and has no installation or validation path on this
branch.
