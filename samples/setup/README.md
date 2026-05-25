# `uinput` Host Setup

These samples help prepare a Linux developer host for the Phase 8
`gr-provider-linux-uinput` manual gate.

## Udev rule

Install `99-virtualgamepad-uinput.rules` to:

```bash
/etc/udev/rules.d/99-virtualgamepad-uinput.rules
```

Then reload the rule and trigger it:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger /dev/uinput
```

## Group membership

The sample rule grants the `input` group read/write access to
`/dev/uinput`.

If your user is not already in that group:

```bash
sudo usermod -aG input "$USER"
```

Log out and back in before relying on the new group membership.

## Notes

- This repo does not install the rule for you.
- Without this setup, Phase 8 manual validation may require running the
  smoke/demo commands with `sudo`.
