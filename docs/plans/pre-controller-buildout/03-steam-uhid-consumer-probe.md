# Brief 03: Steam UHID consumer probe (cleanup step P, #159)

Read `ground-rules.md` first.

**Lane:** Lab / research. Merges no virtualgamepad code.
**Blocked by:** P0 is free to run now (source reading only). P1 needs D1 and D9 (which host runs it).
**Decides:** whether Steam Controller's playable tier is `linux.uhid.usb` or needs USB/IP, which decides whether #168 (USB function list) is a Steam prerequisite.

## Question

Does a UHID device with Valve IDs get accepted by the consumers that matter (`hid-steam`, SDL HIDAPI, Steam client), or do they need real USB topology? UHID nodes have no USB parent and no interface number. Source evidence so far (#159): kernel `hid-steam` on master `0fcb05b72344` matches the 2026 wired controller `28de:1302` with `HID_USB_DEVICE` as **one unified interface**; the Puck `28de:1304` has 7 interfaces (0 to 1 non-HID, 2 to 5 slots, 6 pogo) and conflicts with OpenPuck's 4.

## P0. Source pass (cloud session is fine)

1. Pin the **first kernel release** that carries `STEAM_CONTROLLER_IBEX` (`28de:1302`). Find the commit that added it to `drivers/hid/hid-ids.h` and the first tag containing it. The probe host must run that kernel or newer.
2. Read `hid-steam.c`'s interface filter at that release: does it consult `intf->cur_altsetting->desc.bInterfaceNumber` or the USB parent for the wired IBEX case? A UHID device has no `usb_interface`, so any `to_usb_interface()` call on the hid parent is a likely reject or crash path. Record file:line.
3. Read SDL's HIDAPI Steam driver(s) at a pinned SDL3 commit: does it support `0x1302`, and does it filter on `interface_number` (hidapi reports `-1` for non-USB) or usage page?
4. Record findings in `controller-protocol-research` as source records plus consumer-evidence claims, and link them from #159.

P0 alone may answer the question (for example, if `hid-steam` requires a USB interface). Say so on #159, but still run P1 unless Anders says the source answer is enough.

## P1. Host experiment

- **Scope:** a throwaway probe on a branch or under `examples/` with `required-features = ["experimental"]`. Do not merge it. Steam family wiring still waits for brief 02.
- **Host:** a prepared x86-64 Linux host with `/dev/uhid` access and the kernel from P0, outside CI. The Steam client part uses the #144 host. No `.github/` changes.
- **Cases:**
  1. Wired controller: UHID, USB bus, `28de:1302`, descriptor and a neutral input report built from source evidence (kernel `hid-steam`, SDL).
  2. Puck: `28de:1304` as a single UHID node, to document what one interface cannot express.
  3. Control: the existing DualSense UHID realization under the same probes.
- **Observe for each:** which driver binds (`hid-steam` or `hid-generic`) from sysfs; whether hidraw appears; what hidapi reports for interface number, usage page and bus; whether SDL3 HIDAPI opens it as a Steam controller (`scripts/run-sdl3-gamepad-probe.sh` with the HIDAPI hints); whether the Steam client lists it; whether one input report reaches each consumer; whether `hid-steam` turns lizard mode off on open.

## Decision rule

- `hid-steam` or SDL accept the wired case: Steam's playable tier is `linux.uhid.usb`. The wired Steam family needs only brief 02 to start.
- Rejected because of the missing interface number or USB parent: Steam's playable tier moves to USB/IP; #168's generalized USB function list becomes a Steam prerequisite and gets its own brief before Steam package work.
- The Puck stays a faithful-tier USB/IP item either way.

## Evidence and reporting

Record P1 as an experiment in `controller-protocol-research` (no raw captures or serials; synthetic hex fixtures only). Link it from #159, tick step P on #176, and state the tier decision in one line on both.
