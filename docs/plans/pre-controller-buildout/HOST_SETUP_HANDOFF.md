# Coordinator handoff: Steam development host

Status checked 2026-11-02. This document records this host's setup and observed
virtual-consumer results; it does not promote physical fidelity or gameplay.

## Conclusions and published evidence

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
  CI, CodeQL and Secret scan all passed as of this handoff; PR review/merge
  remains outstanding.

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

These are the installed artifacts verified on this host. Review existing
destination files before installing elsewhere. The optional lab source files
are currently local changes and must be carried into a reviewed change before
another clone can rely on obtaining them from main.

| Repository source / package | Installed destination | Owner / mode | Purpose |
| --- | --- | --- | --- |
| `udev/70-virtualgamepad-uhid.rules` | `/etc/udev/rules.d/70-virtualgamepad-uhid.rules` | root:root / 0644 | `/dev/uhid`: group `virtualgamepad-uhid`, mode 0660 |
| `udev/70-virtualgamepad-uinput.rules` | `/etc/udev/rules.d/70-virtualgamepad-uinput.rules` | root:root / 0644 | `/dev/uinput`: group `virtualgamepad-uinput`, mode 0660 |
| `udev/71-virtualgamepad-input-lab.rules` | `/etc/udev/rules.d/71-virtualgamepad-input-lab.rules` | root:root / 0644 | Optional opt-in Sony touch-test isolation and lab hidraw access |
| `udev/virtualgamepad-input-lab-seat.py` | `/usr/lib/udev/virtualgamepad-input-lab-seat.py` | root:root / 0755 | Validate Sony lab identity and derive inactive per-process seat |
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

The complete optional lab rules are:

```udev
SUBSYSTEM=="hid", ENV{HID_PHYS}=="virtualgamepad/input-lab/uhid/*/p*-i*", PROGRAM=="/usr/lib/udev/virtualgamepad-input-lab-seat.py %E{HID_PHYS}", ENV{VG_INPUT_LAB_SEAT}="%c"
SUBSYSTEM=="input", KERNEL=="input*", IMPORT{parent}="VG_INPUT_LAB_SEAT", ENV{ID_SEAT}="%E{VG_INPUT_LAB_SEAT}"
SUBSYSTEM=="input", KERNEL=="event*", IMPORT{parent}="VG_INPUT_LAB_SEAT", ENV{ID_SEAT}="%E{VG_INPUT_LAB_SEAT}", ENV{LIBINPUT_IGNORE_DEVICE}="1"
SUBSYSTEM=="hidraw", IMPORT{parent}="HID_PHYS", ENV{HID_PHYS}=="virtualgamepad/input-lab/uhid/*/p*-i*", TAG+="uaccess"
```

The helper accepts only `dualsense` and `dualshock4` families in that test
namespace, and outputs `seat-vg-alpha-p<hex-process-id>`. The hidraw rule uses
the reserved lab namespace glob. The rule file alone is insufficient: the
executable helper and `VIRTUALGAMEPAD_INPUT_LAB=1` in the test process are also
required. Ordinary demo/Steam runs do not enable the lab identity.

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
   hidraw node. If needed, grant a temporary ACL only to that verified node.
5. Before active Sony contacts, require the expected process-specific `ID_SEAT`
   and `LIBINPUT_IGNORE_DEVICE=1` on every owned event node. The live test gates
   contacts on these checks. Keep Steam closed for this isolated SDL test.
6. Remove the controller and verify owned nodes disappear. Node numbers can
   be reused; never reuse an ACL command without verifying the new identity.

## Coordinator next actions and workspace disposition

1. Review research PR #90 and its explicit limits; merge under the project's
   policy. Step 16 remains unchecked pending that review/merge. Assess its
   unknown delivery cells before treating the entire consumer objective as
   accepted; D9 itself is already cleared.
2. Preserve root working changes. At the end of the host work, the original checkout remained on `main`, base
   `8f60dcce577ab02bd8ed8b3426349729d417393c`, with research submodule still
   pinned to `719befa86bbff044cb0cdc524d66085a4b610800`. No root commit or
   submodule update was made during that host work. Research work used a separate checkout and
   branch `codex/steam-uhid-host-closeout`.
3. Split the reusable input-lab identity, udev helper/rules, regression tests
   and setup documentation into a reviewed root change. Keep the throwaway
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
