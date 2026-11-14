# Alpha 2 release notes — DRAFT (not a release declaration)

> Prepared against `main` at
> [`f27e6114f742b5044b4ca735ae17b4d20807f654`](https://github.com/anderstvoss/virtualgamepad/commit/f27e6114f742b5044b4ca735ae17b4d20807f654).
> Merged VG-1b changes are listed **through this exact commit**;
> see [Alpha 2 release tracking #365](https://github.com/anderstvoss/virtualgamepad/issues/365) and
> [Anders's gate decisions](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358).
> **Not final:** remaining VG-1b d-family splits and e, the #304 research record, release-commit
> qualification and maintainer authorization remain outstanding. Preserve
> [Alpha 1's published notes](../ALPHA_RELEASE_NOTES.md); do not replace them.
> See the [release checklist](ALPHA_2_CHECKLIST.md).

## Summary

Alpha 2 is the next **Git-only** prerelease candidate after
[`0.1.0-alpha.1`](https://github.com/anderstvoss/virtualgamepad/releases/tag/0.1.0-alpha.1).
The four Alpha 1 families are joined by the Wii Remote, wired Steam
Controller 2026 and Xbox Series. C5–C9 are merged. The root application
surface was converged across the six pre-Series families and extended to the
seventh; whole-state input mutation and subsequent transactional input work
have begun. These are **implemented alpha surfaces**, not claims of complete
hardware equivalence. Per-realization capabilities and validation remain
bounded by the [support matrix](../CONTROLLER_SUPPORT.md), compiled
[family registry](../../crates/gr-curated-controllers/src/registry.rs),
[acceptance manifests](../../acceptance/README.md) and
[status page](../STATUS.md).

## Controllers and realizations

**Support category:** every family/transport cell in
[`CONTROLLER_SUPPORT.md`](../CONTROLLER_SUPPORT.md) remains **WIP**;
no `✓*` or `✓` cell has been promoted. The transport list below describes
declared/implemented paths, not a blanket acceptance of all native functions.

| Family | Alpha 2 declared paths and scope | Overall support |
| --- | --- | --- |
| DualSense | `linux.uinput` reduced evdev; `linux.uhid.usb` HID; separately opt-in USB/IP HID/audio worker | WIP |
| DualShock 4 | `linux.uinput` reduced evdev; `linux.uhid.usb` HID; separately opt-in USB/IP HID/audio worker | WIP |
| Switch Pro | `linux.uinput` evdev; `linux.uhid.usb` HID | WIP |
| Xbox 360 | `linux.uinput` evdev and `linux.uhid.usb` **standard HID** (not proprietary Xbox XInput); separately opt-in USB/IP worker | WIP |
| Wii Remote | `linux.uinput` reduced evdev with companion nodes; `linux.uhid.bluetooth` **local Bluetooth-bus metadata via UHID**, without radio or pairing | WIP |
| Steam Controller (2026), wired | `linux.uinput` reduced evdev; `linux.uhid.usb` wired USB HID with native protocol controls. The Puck receiver is **not implemented** | WIP |
| Xbox Series | `linux.uinput` reduced, xpad-style evdev only. Not wired GIP and not Bluetooth | WIP |

The Wii package includes explicit attachment/EEPROM and MotionPlus behavior
within its [function limits](../WII_REMOTE.md); it is not native Bluetooth
pairing. Wired Steam includes the opt-in Steam-client compatibility profile and
a reduced-uinput path with explicit unrepresentable-input refusals; the
[bounded D9 observations](../C6_D9_ACCEPTANCE.md) do not establish complete
client/game compatibility. Xbox Series has an
[explicit reduced-function manifest](../../acceptance/virtualgamepad.xbox-series.toml).
Experimental `dummy_hcd` USB HID creation is refused before allocation; it
is not a shipped functional realization.

## API changes since Alpha 1

The API remains **alpha-changeable** under the
[#322 owner policy](https://github.com/anderstvoss/virtualgamepad/issues/322#issuecomment-6747067225).
Existing consumers should use the root `virtualgamepad` crate rather than
supporting-crate SPI. Merged changes to review:

- [C5 #233](https://github.com/anderstvoss/virtualgamepad/pull/233),
  [C6 #301](https://github.com/anderstvoss/virtualgamepad/pull/301):
  Wii and wired Steam APIs/packages, with follow-on C7 root access.
- [C7 #329](https://github.com/anderstvoss/virtualgamepad/pull/329)
  (steps [#326](https://github.com/anderstvoss/virtualgamepad/pull/326),
  [#327](https://github.com/anderstvoss/virtualgamepad/pull/327),
  [#328](https://github.com/anderstvoss/virtualgamepad/pull/328)):
  `create_<family>(CreationOptions, <Family>Options)` shape; remove legacy
  `_with_identity` constructors; root Wii and Steam handles; identity API
  changes. See [the migration ledger](../ALPHA_API_MIGRATION.md).
- [C9 #342](https://github.com/anderstvoss/virtualgamepad/pull/342):
  Xbox Series as the seventh root/curated family.
- [Worker #350](https://github.com/anderstvoss/virtualgamepad/pull/350):
  USB/IP retained-output **snapshots** for the three worker-backed
  families; a current client refuses an older worker with reinstall guidance.
- [VG-1 #352](https://github.com/anderstvoss/virtualgamepad/pull/352):
  `set_inputs` for DualSense, DS4, Switch Pro and Xbox 360.
  [#348](https://github.com/anderstvoss/virtualgamepad/pull/348) was stacked
  and was carried to `main` through #352, not a separate `main` merge.
- [VG-1b/a #353](https://github.com/anderstvoss/virtualgamepad/pull/353):
  shared final-state validation and clock transitions.
  [VG-1b/b #363](https://github.com/anderstvoss/virtualgamepad/pull/363):
  family-typed transactional `edit_inputs`.
  [VG-1b/c1 #364](https://github.com/anderstvoss/virtualgamepad/pull/364):
  Steam reduced-uinput refusal parity.
  [VG-1b/c2 #366](https://github.com/anderstvoss/virtualgamepad/pull/366)
  (`b41c82da593f179ad2933098dedb4dd658e2aaf0`): Wii battery moved from input snapshots to
  controller-owned `WiiRemoteBattery` metadata with `battery()` and
  `set_battery(...)` accessors.
  [VG-1b/d DualSense #370](https://github.com/anderstvoss/virtualgamepad/pull/370)
  (`f27e6114f742b5044b4ca735ae17b4d20807f654`): split native `DualSenseInputs` from
  `DualSenseStatus`; `inputs()` and `status()` provide separate read views.
  Other step-d families and step e remain outstanding at this baseline.

**TODO(VG-1b e):** reconcile the remaining d-family splits and e,
including final merged PR links, API snapshot deltas, removed types,
updated constructors/methods and concrete migration examples. Copy verified
facts from the final `docs/ALPHA_API_MIGRATION.md` ledger *after* step e;
do not guess final names or promote unmerged code.

## Host setup and permissions

The [Alpha 2 host audit](../ALPHA_2_HOST_AUDIT.md) was
[owner-signed and #258 closed](https://github.com/anderstvoss/virtualgamepad/issues/258#issuecomment-6777536122)
against C9's merge SHA `69c2283`. This is the release authority for host
provisioning; the library itself does not install rules or elevate processes.

Creator access to `/dev/uinput` or `/dev/uhid` is distinct from
consumer `/dev/input/event*` and `/dev/hidraw*` access. Use existing
distribution/seat policy. Dedicated creator groups, optional module boot
files and privileged USB/IP installation are explicit administrator choices,
not defaults; never enroll users in the broad `input` group. Wii driver
handoff, LED write grants, `hid-wiimote` module unload and #299 input-lab
isolation are **test-only**. USB/IP audio is off by default as a privileged
WIP opt-in; actual Bluetooth provisioning is out of scope. A newer USB/IP
client may require reinstalling an older worker after #350, using the existing
administrator-run installation process.

## Known limitations

- **All support cells are WIP**; per-function virtual-host or partial
  physical correspondence is not blanket physical-controller equivalence.
  Physical fidelity #136 and many consumer combinations remain unqualified.
- **First-valid-neutral oracle (#166):** creation-performance validation and
  the first-valid-neutral timing oracle are still missing. Existing per-family
  N1/N4 lifecycle runs remain the available evidence; **#166 is not an
  Alpha 2 gate**, per [#365 owner decision](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358).
- **Audio:** sustained duplex continuity, latency, matching profiles and
  physical audio are not qualified (#116, #135, #143, #145); dropouts and
  earlier strict zero-loss failures remain recorded. USB/IP is opt-in WIP.
- **USB/IP host output:** #350 provides retained **snapshots** (live/final),
  **not a loss-aware host-output stream**. VG-2/#344 and ADR-0024 are assigned
  to the separate audio track; neither is an Alpha 2 gate.
- **Alpha 2 blocking gate — real kalux USB/IP #350 run:** **not yet run**
  as of this draft; socket-pair/synthetic tests are not a substitute. Per
  [#365 owner decision](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358), the local run must
  confirm live `host_outputs()` snapshots while in service, the FINAL
  snapshot after close, and stale-worker refusal with reinstall guidance.
  Do not mark the gate passed without its actual on-host evidence.
- **Xbox Series:** xpad-style uinput only. No Bluetooth pairing/HID report
  map, wired USB GIP, impulse-trigger motors, headset audio/accessories or
  other refused features recorded in its manifest (#160, #168, #164).
- **Steam Puck/receiver:** unsupported in production; wired Steam is not a
  Puck/receiver simulation. Real Bluetooth (including Wii radio/pairing)
  remains out of scope (#159, #164, #165).
- **USB HID via dummy_hcd:** ordinary-root creation is refused due to
  Linux `f_hid` GET_REPORT/negative-reply limitations. Standard-HID
  Xbox 360 is not XInput.
- **Consumer/GUI:** some reduced-uinput Steam paths did not list in the bounded
  Steam client run; broader interactive GUI and screen-reader acceptance are
  deferred (#126, #144). Windows/macOS CI does not establish functioning
  non-Linux providers.
- **Research closeout:** #304 kernel observations must be recorded in
  [research PR #122](https://github.com/anderstvoss/controller-protocol-research/pull/122)
  before phase 2 closes; it was open on the draft baseline.
- **Post-Alpha 2 families:** DualSense Edge #170, Switch 2 Pro #171,
  Xbox One/Elite/Nano #355–#359, Switch 2 GameCube #360,
  Wii U Pro #361 and Joy-Con 2 #362 are future targets, not included.

## Upgrade notes

1. Treat Alpha 2 as a **Git revision**: pin the eventual reviewed tag or
   full commit SHA, rather than assuming compatibility with Alpha 1.
   The current workspace version in [`Cargo.toml`](../../Cargo.toml)
   is `0.1.0`, with `publish = false`; **Anders decides** whether the
   version/tag changes. This draft does not make that decision.
2. Apply the C7 creation/identity changes and use explicit realization
   selection. No automatic transport fallback or privileged provisioning
   is introduced.
3. Update callers for VG-1 whole-state mutations and VG-1b transactional
   validation/refusal semantics; preserve explicit `commit()`/`service()`
   and do not infer host-visible atomic input edges.
4. **TODO(VG-1b e):** insert the final input-only type and metadata
   migration examples, removed names and snapshot references after
   the implementing agent's step-e ledger is merged.
5. Where USB/IP is explicitly enabled, reinstall an outdated worker to
   match the #350 worker-control protocol; use the existing signed-off
   host setup procedures. Review the final
   [Alpha 2 checklist](ALPHA_2_CHECKLIST.md) for qualification.
