# Coordinator handoff: Steam development host

Status checked 2026-11-02; host changes since then are in
[Host updates after 2026-11-02](#host-updates-after-2026-11-02) (updated 2026-11-08).
This document records this host's setup and observed virtual-consumer results;
it does not promote physical fidelity or gameplay.

## Conclusions and published evidence

- Full ignored suite passed on `3f214762f5c1af924e9c9907d31e8fedcd93c064`, 2026-11-02 (Pacific): 22 passed, 0 failed, cleanup verified; [run record](https://github.com/anderstvoss/virtualgamepad/issues/176#issuecomment-6595738774).
- Host setup is complete. The owner designated the prepared EndeavourOS x86-64
  machine as D9; the tracker checkbox is closed. See the
  owner decision in #176 (comment 6590315813)
  and readiness report in #159 (comment 6508766392).
- Running kernel: `7.3.0-1-mainline`; IBEX alias present; required UHID/uinput,
  HIDRAW, Steam, PlayStation, Wiimote and xpad configuration verified. Stock
  kernel and initramfs remain installed. No additional reboot is required.
- Dedicated-group UHID/uinput access and ordinary-user write opens pass.
  `/tmp` resolves to ext4; `tmp.mount` is masked. DummyHcd remains deliberately
  unprovisioned/unsupported under D5.
- Private SDL: `e0232c5b9d972ad394d597ce858185cd15a0f1a7`, Release, HIDAPI
  enabled, examples/tests off. Required ancestor and Triton driver checked.
  Keep its environment scoped to each command.
- [Research PR #90](https://github.com/anderstvoss/controller-protocol-research/pull/90),
  commit `9db1a78cac606bc2326bc559e9bc1627950542b8`, publishes sanitized
  six-run observations, commands, source/binary/patch hashes and limitations.
  CI, CodeQL and Secret scan passed. Research PR #90 merged as
  [`c9262690e0fb217802bd72dda66c605e7049e8f7`](https://github.com/anderstvoss/controller-protocol-research/commit/c9262690e0fb217802bd72dda66c605e7049e8f7).

| Case | SDL alone, Steam closed | Steam alone, SDL helper absent |
| --- | --- | --- |
| IBEX `28de:1302` | `hid-steam` bound; exact node accessible; SDL opened and observed neutral | Operator reported no controller listed |
| Single-node Puck `28de:1304` | `hid-steam` bound; exact node accessible; SDL found no matching joystick | Operator reported no controller listed |
| Production DualSense control | SDL opened and observed neutral | Operator confirmed DualSense listed |

All six runs verified owned-node removal. Valve request accounting ended with
zero pending requests. Separate isolated DualSense touch acceptance passed three
create/use/close iterations with touch, motion, output feedback and cleanup.

**Tier conclusion:** `linux.uhid.usb` meets brief 03's wired kernel-or-SDL
criterion through observed IBEX kernel binding and SDL opening. Steam client
compatibility is a separate acceptance axis. Puck's faithful USB topology is not represented by one
UHID node.

## Interpretation limits and corrections

- Earlier feature counters used raw type `2` and are invalid. UHID defines
  feature/output/input as `0/1/2`. Corrected probe tests cover types, framing,
  exact replies, polling without pending requests and consumer failure cleanup.
- Puck's earlier permission explanation was unverified. Corrected runs
  successfully opened its exact node read/write; do not blame permissions for
  the observed SDL discovery failure.
- IBEX/DualSense neutral-only SDL diagnostics returned nonzero for missing
  changing sensor samples. Their opened/neutral observations are valid, but
  those diagnostic runs are not full controls/motion acceptance passes.
- Exact report delivery inside SDL/Steam is unmeasured. A submitted report or
  initial neutral state is not proof that a particular packet was decoded.
- The throwaway probe returns short synthetic identity/attribute/settings
  feature replies, unlike the padded identity/attribute corpus fixtures.
  Negative Steam listing applies to this limited stub, not a complete wired
  implementation. It does not establish that USB topology is required.
- Lizard-disable detection does not identify the command's origin; userspace
  can issue it through UHID too. Lifecycle counts also include access checks.

## Exact udev and module inventory

These are the installed artifacts recorded on this host. The reusable lab
helper, rules, identity code and regression tests accompany this setup in the
repository. Review existing destination files before installing elsewhere; the
root-helper opt-in requires owner review and is not a D10 self-merge. The
hardened source must not be assumed installed merely because its PR is ready.
The uinput extension is an owner-reviewed design: both Sony component labels
are prepared before open, retaining exact physical association metadata.

| Repository source / package | Installed destination | Owner / mode | Purpose |
| --- | --- | --- | --- |
| `udev/70-virtualgamepad-uhid.rules` | `/etc/udev/rules.d/70-virtualgamepad-uhid.rules` | root:root / 0644 | `/dev/uhid`: group `virtualgamepad-uhid`, mode 0660 |
| `udev/70-virtualgamepad-uinput.rules` | `/etc/udev/rules.d/70-virtualgamepad-uinput.rules` | root:root / 0644 | `/dev/uinput`: group `virtualgamepad-uinput`, mode 0660 |
| `udev/71-virtualgamepad-input-lab.rules` | `/etc/udev/rules.d/71-virtualgamepad-input-lab.rules` | root:root / 0644 | Optional Sony UHID/uinput touch-test isolation, Steam lab event access (#299) and scoped consumer access |
| `udev/virtualgamepad-input-lab-seat.py` | `/usr/lib/udev/virtualgamepad-input-lab-seat.py` | root:root / 0755 | Validate Sony or Steam lab identity and derive inactive per-process seat |
| `modules-load.d/virtualgamepad-uhid.conf`, plus host's uinput boot entry | `/etc/modules-load.d/virtualgamepad-uhid.conf` | root:root / 0644 | Installed file loads both `uhid` and `uinput`; repository template loads only `uhid` |
| `steam-devices` package | `/usr/lib/udev/rules.d/60-steam-input.rules` | Package managed | Valve/Sony consumer access; left unchanged |

For future installations, use the separate opt-in
[`virtualgamepad-uinput.conf`](../../../modules-load.d/virtualgamepad-uinput.conf)
template when `CONFIG_INPUT_UINPUT=m`; install it as root:root mode 0644 at
`/etc/modules-load.d/virtualgamepad-uinput.conf`. Keep the UHID template scoped
to `uhid`. The recorded D9 host currently loads both modules from its existing
combined file; this documentation does not alter that installed configuration.
With `CONFIG_INPUT_UINPUT=y`, a uinput modules-load entry is unnecessary. See
[deployment instructions](../../DEPLOYMENT_AND_VALIDATION.md) and
[SDL acceptance](../../SDL_ACCEPTANCE.md) for the access and consumer gates.

The two creation rules are:

```udev
KERNEL=="uhid", SUBSYSTEM=="misc", GROUP="virtualgamepad-uhid", MODE="0660"
KERNEL=="uinput", SUBSYSTEM=="misc", GROUP="virtualgamepad-uinput", MODE="0660"
```

The complete optional lab rules, as on `main` after #299, are:

```udev
SUBSYSTEM=="hid", ENV{HID_PHYS}=="virtualgamepad/input-lab/uhid/*/p*-i*", PROGRAM=="/usr/lib/udev/virtualgamepad-input-lab-seat.py %E{HID_PHYS}", ENV{VG_INPUT_LAB_SEAT}="%c"
SUBSYSTEM=="hid", ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*", ENV{HID_PHYS}=="virtualgamepad/input-lab/uhid/steam-controller-2026/p*-i*", ENV{VG_INPUT_LAB_STEAM}="1"
SUBSYSTEM=="input", KERNEL=="input*", ATTR{phys}=="virtualgamepad/input-lab/uinput/*/p*-i*", PROGRAM=="/usr/lib/udev/virtualgamepad-input-lab-seat.py %s{phys}", ENV{VG_INPUT_LAB_SEAT}="%c"
SUBSYSTEM=="input", KERNEL=="input*", IMPORT{parent}="VG_INPUT_LAB_SEAT"
SUBSYSTEM=="input", KERNEL=="input*", IMPORT{parent}="VG_INPUT_LAB_STEAM"
SUBSYSTEM=="input", KERNEL=="input*", ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*", ENV{ID_SEAT}="%E{VG_INPUT_LAB_SEAT}"
SUBSYSTEM=="input", KERNEL=="event*", IMPORT{parent}="VG_INPUT_LAB_SEAT"
SUBSYSTEM=="input", KERNEL=="event*", IMPORT{parent}="VG_INPUT_LAB_STEAM"
SUBSYSTEM=="input", KERNEL=="event*", ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*", ENV{ID_SEAT}="%E{VG_INPUT_LAB_SEAT}", ENV{LIBINPUT_IGNORE_DEVICE}="1"
SUBSYSTEM=="input", KERNEL=="event*", ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*", ATTRS{phys}=="virtualgamepad/input-lab/uinput/*/p*-i*", GROUP="virtualgamepad-uinput", MODE="0660"
SUBSYSTEM=="hidraw", IMPORT{parent}="VG_INPUT_LAB_SEAT"
SUBSYSTEM=="hidraw", ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*", TAG+="uaccess"
SUBSYSTEM=="input", KERNEL=="event*", ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*", ENV{VG_INPUT_LAB_STEAM}=="1", GROUP="virtualgamepad-uhid", MODE="0660"
```

The helper accepts only the `dualsense`, `dualshock4` and `steam-controller-2026`
families in that test namespace for `uhid` or `uinput`, and outputs `seat-vg-alpha-p<hex-process-id>`. It bounds the entire
ASCII label to 63 bytes, validates a positive numeric PID through `0x7fffffff`,
and rejects malformed/noncanonical IDs. The hidraw rule imports the
helper-validated seat property; the namespace glob alone grants no access.
Import rules must precede their guards as separate rules because udev evaluates
ENV matches before IMPORT within one rule. The rule file alone is insufficient: the
executable helper and `VIRTUALGAMEPAD_INPUT_LAB=1` in the test process are also
required. It runs Python in isolated mode and performs no input-dependent
file/network access or command execution. Ordinary demo/Steam runs do not
enable the lab identity. See the [opt-in install](../../DEPLOYMENT_AND_VALIDATION.md#optional-sony-input-lab-isolation).

Matching child input `phys` failed because it was empty; input `uniq` changed
under hid-playstation. Importing the stable HID-parent `HID_PHYS` and carrying
the seat through HID → input → event resolved isolation. Do not replace this
with broad input-group membership or blanket hidraw access.

The installed Valve package additionally has this HID-path rule:

```udev
SUBSYSTEM=="hidraw", KERNELS=="000[356]:28DE:*", MODE="0660", TAG+="uaccess"
```

Unlike its USB-parent attribute rules, this pattern can cover a USB-bus UHID
Valve HID node. Inspect the package's actual rules on each future host; do not
assume a product-specific `1302` entry is needed. `uaccess` depends on an active
local logind session; successful rule parsing alone does not prove access.

For a future setup, after approved group/rule/module installation:

1. Reload udev; log out/in after adding dedicated-group membership. Recreate
   the test device to ensure its properties came from the current rules.
2. Run `udevadm verify` on the installed rule files and compare installed lab
   rule/helper bytes with the reviewed sources. A `cmp` success on this host
   confirmed both lab artifacts match their sources.
3. Check `/dev/uhid` and `/dev/uinput` groups/mode and verify ordinary-user
   opens. Kernel modules alone do not grant access.
4. Start only one owned test device. Identify all nodes by its session ancestry,
   not cached node numbers. Check ordinary-user read/write access to its exact
   hidraw node for UHID. For uinput, verify all exact event nodes belong to the
   dedicated `virtualgamepad-uinput` group at mode 0660 and are openable by
   the ordinary user. This grant also requires the validated seat marker and
   a uinput lab parent label; UHID event permissions remain unchanged.
5. Before active Sony contacts, require the expected process-specific `ID_SEAT`
   and `LIBINPUT_IGNORE_DEVICE=1` on every owned event node. The live test gates
   contacts on these checks. Keep Steam closed for this isolated SDL test.
6. Remove the controller and verify owned nodes disappear. Node numbers can
   be reused; never reuse an ACL command without verifying the new identity.

## Host updates after 2026-11-02

- **Steam lab event access (#299) installed, 2026-11-04.** The installed lab
  rules and helper were compared byte for byte with `main` and `udevadm verify`
  passed. A Steam UHID lab node came up with the process seat, group
  `virtualgamepad-uhid` and mode 0660, and opened as the ordinary user. The
  kernel evdev and ABS results that used it are in #304.
- **Close Dolphin before live tests.** A running Dolphin opens a new Steam
  hidraw node within about 50 ms of creation. hid-steam then removes the
  "Steam Controller" gamepad and motion evdev nodes, so only the generic
  keyboard/mouse node is left. Other hidraw consumers can do the same.
- **Original checkout.** It is still on base
  `8f60dcce577ab02bd8ed8b3426349729d417393c` with the preserved uncommitted
  host-work changes. Run new work in a fresh worktree off current `origin/main`. Leave the old checkout alone until
  the owner has reviewed what it holds.
- **Bluetooth.** The host has a built-in Bluetooth adapter. Nothing is
  provisioned for captures yet. Steam Controller Bluetooth (`28de:1303`) work
  is deferred; when it starts it needs BlueZ, a paired physical controller and
  `btmon` from `bluez-utils`, and it gets its own #258 row.
- **Reading a physical Wii Remote's EEPROM (private, read-only).** After the
  Remote pairs through BlueZ, `hid-wiimote` binds it and its hidraw node stays
  root-owned. No persistent rule is added. For one session the owner grants
  access to that one node only, after checking that it belongs to the physical
  Remote (bus `0005`, vendor `057e`, product `0306` or `0330`, not a
  `virtualgamepad` phys):

  ```bash
  udevadm info --query=property --name=/dev/hidrawN | grep -E '^(HID_ID|HID_NAME|HID_PHYS)='
  sudo setfacl -m "u:$USER:rw" /dev/hidrawN   # gone when the node is removed
  ```

  Dumped content is private (see the Wii EEPROM handoff); never commit it or
  post values or hashes from it. This grant is a #258 row.

## Coordinator next actions and workspace disposition

1. Research PR #90 is merged; retain its exact qualification when closing the
   tracker: `linux.uhid.usb meets brief 03's wired kernel-or-SDL criterion;
   Steam listing stub-limited, report delivery unmeasured.` D9 is cleared;
   missing delivery observations do not become full consumer acceptance.
2. Preserve root working changes. At the end of the host work, the original checkout remained on `main`, base
   `8f60dcce577ab02bd8ed8b3426349729d417393c`, with research submodule still
   pinned to `719befa86bbff044cb0cdc524d66085a4b610800`. No root commit or
   submodule update was made during that host work. Research work used a separate checkout and
   branch `codex/steam-uhid-host-closeout`.
3. Owner-review the reusable input-lab identity, hardened helper/rules,
   CI-executed Python regressions and setup documentation as a root-helper
   change; it must not self-merge under D10. Keep the throwaway
   `steam_uhid_probe` example, its Cargo registration/dev dependency and the
   metadata helper out of merged production changes as brief 03 requires.
4. Treat future Steam compatibility work as a consumer/protocol task: improve
   the source-backed feature exchange and instrument exact report delivery if
   required. No additional host package/rule change is justified merely by
   the limited probe's negative listing. Keep faithful Puck topology separate.

Root fmt/check/Clippy/workspace tests, four probe regressions, API snapshots,
C helper compilation and secret scans passed. Research publication/schema
checks and all 167 tests passed; all PR workflow runs subsequently passed.
No new third-party project dependency was introduced.
