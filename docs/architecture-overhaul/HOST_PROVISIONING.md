# Reviewable host provisioning proposal

No commands in this proposal have been applied. The approved plan requires review before changing modules, permissions, or host services. The current execution identity lacks UHID access and usable noninteractive sudo; uinput is already accessible.

## Minimal temporary UHID access

On the intended validation desktop, review and run:

```sh
sudo modprobe uhid
sudo setfacl -m "u:$(id -u):rw" /dev/uhid
```

This grants only the invoking validation identity access to the existing device node. Record the previous ACL outside Git before changing it, and restore that ACL after the experiment. Do not use globally writable device permissions. If setfacl is unavailable, have the host administrator provision equivalent scoped access.

Then run the existing process-owned UHID integration test and sequential B/P procedure from `docs/SDL_ACCEPTANCE.md`. An SDL3 development installation, working interactive Steam session, and reference-controller model/firmware alias are separate prerequisites. Merely finding a Steam launcher does not establish them.

## Gadget experiments

Confirm kernel support and existing gadget ownership before applying any setup. Required mechanisms are dummy_hcd, ConfigFS/libcomposite, and the HID gadget function. Review module loading, any ConfigFS mount, broker installation, UID authorization, and systemd socket activation against `docs/DEPLOYMENT_AND_VALIDATION.md`. Do not bind or unbind an existing gadget for this survey.

Gate G requires a deliberately reserved test UDC and scoped control-path/latency measurements. Missing or unsupported gadget metadata is not repaired by faster IPC. Audio and Bluetooth additionally need their own endpoint/topology and consumer criteria; this proposal does not authorize incidental driver rebinding or capture of unrelated devices.
