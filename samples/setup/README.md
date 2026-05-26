# Linux Virtual-Device Host Setup

These samples help prepare a Linux developer host for the Phase 8
`gr-provider-linux-uinput` manual gate and the Phase 9
`gr-provider-linux-uhid` manual gate.

## Included files

- `99-virtualgamepad-uinput.rules`: grants the `input` group read/write
  access to `/dev/uinput`
- `99-virtualgamepad-uhid.rules`: grants the `input` group read/write
  access to `/dev/uhid`
- `install-linux-input-rules.sh`: installs both rules, reloads `udev`,
  and retriggers the live device nodes when present

## Recommended install path

Run:

```bash
sudo ./samples/setup/install-linux-input-rules.sh
```

The script copies the rules into `/etc/udev/rules.d/`, reloads the
rules, and attempts to trigger:

- `/dev/uinput` via `/sys/devices/virtual/misc/uinput`
- `/dev/uhid` via `/sys/devices/virtual/misc/uhid`

## Group membership

These rules assume the runner user belongs to the `input` group.

If your user is not already in that group:

```bash
sudo usermod -aG input "$USER"
```

Log out and back in before relying on the new group membership.

## Verify device access

Check the device nodes:

```bash
ls -l /dev/uinput /dev/uhid
id -nG
```

Expected result:

- both device nodes are group-owned by `input`
- both device nodes are mode `crw-rw----`
- your user lists `input` in `id -nG`

Then verify with the repo smoke surfaces:

```bash
cargo run -p gr-cli -- run-uinput-smoke generic-gamepad
cargo run -p gr-cli -- run-uhid-smoke dualsense --bus usb
cargo run -p gr-cli -- run-uhid-smoke dualsense --bus bluetooth
```

## Troubleshooting

If `/dev/uhid` or `/dev/uinput` still keeps older permissions after the
rule is installed:

```bash
udevadm info -q path -n /dev/uhid
udevadm info -q path -n /dev/uinput
```

If those commands resolve to device paths, reload and retrigger:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger --verbose --action=add /sys/devices/virtual/misc/uhid
sudo udevadm trigger --verbose --action=add /sys/devices/virtual/misc/uinput
```

If a VM or distro still refuses to apply the rule immediately, you can
temporarily repair the live node and continue manual validation:

```bash
sudo chgrp input /dev/uhid /dev/uinput
sudo chmod 0660 /dev/uhid /dev/uinput
```

That fallback is session-local; persistent behavior should come from the
installed `udev` rules.
