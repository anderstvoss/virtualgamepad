# Deployment and validation

The four realization targets are peers. A controller is created only for the exact target selected by the application and declared by that controller. There is no target ordering and no fallback.

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

The service intentionally has no network listener. Its protocol only permits opening compiled DualSense sessions, fixed-size input, bounded reverse-output polling, close, and diagnostics. Session handles are broker-created, connection-bound, and invalid after disconnect or restart.

## Host validation

Root-only integration tests are intentionally ignored in normal test runs. DummyHcd validation covers USB enumeration, HID feature exchange, motion input, reverse output, and resource-only cleanup. SDL/Steam recognition and gyro behavior remain controller-specific acceptance criteria. Bluetooth realization is deferred to `wip/btvirt` and has no installation or validation path on this branch.
