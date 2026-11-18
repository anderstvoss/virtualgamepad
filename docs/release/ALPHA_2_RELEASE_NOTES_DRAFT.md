# Alpha 2 release notes — DRAFT (not a release declaration)

> Prepared against `main` at
> [`12358d71d762fb438cbf01f5b120986279a8540d`](https://github.com/anderstvoss/virtualgamepad/commit/12358d71d762fb438cbf01f5b120986279a8540d).
> The VG-1b section is reconciled through #390 (`a0398b7`); other evidence
> retains this draft baseline. Step e awaits owner merge;
> see [Alpha 2 release tracking #365](https://github.com/anderstvoss/virtualgamepad/issues/365) and
> [Anders's gate decisions](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358).
> **Not final:** step-e owner merge, release-commit
> qualification and maintainer authorization remain outstanding. The #304
> kernel observations are recorded upstream in [research #122 merge](https://github.com/anderstvoss/controller-protocol-research/commit/261e7bec4351a6969563893a2092198a183ab8c9). Preserve
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

**Support category:** every implemented family/transport path in
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
| Xbox Series | `linux.uinput` reduced xpad-style evdev; `linux.uhid.bluetooth` local Bluetooth-bus HID with independent virtual trigger-rumble commands. No wired GIP or real Bluetooth radio/pairing | WIP |

The Wii package includes explicit attachment/EEPROM and MotionPlus behavior
within its [function limits](../WII_REMOTE.md); it is not native Bluetooth
pairing. Wired Steam includes the opt-in Steam-client compatibility profile and
a reduced-uinput path with explicit unrepresentable-input refusals; the
[bounded D9 observations](../C6_D9_ACCEPTANCE.md) do not establish complete
client/game compatibility. [#369](https://github.com/anderstvoss/virtualgamepad/pull/369) added Xbox Series local
Bluetooth-bus UHID beside uinput ([compiled Series definition](https://github.com/anderstvoss/virtualgamepad/blob/12358d71d762fb438cbf01f5b120986279a8540d/crates/gr-curated-controllers/src/xbox_series/definition.rs#L437-L447); [compiled registry](https://github.com/anderstvoss/virtualgamepad/blob/12358d71d762fb438cbf01f5b120986279a8540d/crates/gr-curated-controllers/src/registry.rs#L160-L168)).
[The Series guide](../XBOX_SERIES.md) records bounded SDL 3.4.18 evdev and
HIDAPI observations, including main rumble and independent virtual trigger-rumble
commands on UHID. Steam listed both realizations and exercised controls excluding
Guide; the UHID observation required the temporary active-seat dev setup.
See the [per-function manifest](../../acceptance/virtualgamepad.xbox-series.toml).
No physical effect or blanket support is inferred.
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
  standalone identity-restoration constructors; root Wii and Steam handles; identity API
  changes. See [the migration ledger](../ALPHA_API_MIGRATION.md).
- [C9 #342](https://github.com/anderstvoss/virtualgamepad/pull/342):
  Xbox Series as the seventh root/curated family.
- [#369](https://github.com/anderstvoss/virtualgamepad/pull/369): local Xbox Series Bluetooth-bus UHID, typed Bluetooth
  rumble outputs and HID lifecycle/raw-output events.
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
  [VG-1b/d DualShock 4 #371](https://github.com/anderstvoss/virtualgamepad/pull/371)
  (`031dd11696badb267e1c3c2b3e962316dfba7f1c`),
  [Switch Pro #374](https://github.com/anderstvoss/virtualgamepad/pull/374)
  (`8cd1000a308eac682eca43d766b0f1a1f71a6f1e`), and
  [Xbox 360 #375](https://github.com/anderstvoss/virtualgamepad/pull/375)
  (`26168b3f291090f9e44bc1e6abd7cd3431dffee9`) also landed input-only
  type/status splits. [Steam #387](https://github.com/anderstvoss/virtualgamepad/pull/387),
  [Xbox Series #389](https://github.com/anderstvoss/virtualgamepad/pull/389) and
  [Wii #390](https://github.com/anderstvoss/virtualgamepad/pull/390) complete step d.
  Step e reconciles consumers and documentation and awaits owner merge.

### VG-1b final merge ledger

| PR | Verified main merge SHA |
| --- | --- |
| [#353](https://github.com/anderstvoss/virtualgamepad/pull/353) | `2890223d79d6a5ec53778e8d588737e20e4a3fb6` |
| [#363](https://github.com/anderstvoss/virtualgamepad/pull/363) | `5bea7a097f66c9526fae45ea32b5a8a936029580` |
| [#364](https://github.com/anderstvoss/virtualgamepad/pull/364) | `77ec6e39457ddf123ef732ac0e28237307e05f8c` |
| [#366](https://github.com/anderstvoss/virtualgamepad/pull/366) | `b41c82da593f179ad2933098dedb4dd658e2aaf0` |
| [#370](https://github.com/anderstvoss/virtualgamepad/pull/370) | `f27e6114f742b5044b4ca735ae17b4d20807f654` |
| [#371](https://github.com/anderstvoss/virtualgamepad/pull/371) | `031dd11696badb267e1c3c2b3e962316dfba7f1c` |
| [#374](https://github.com/anderstvoss/virtualgamepad/pull/374) | `8cd1000a308eac682eca43d766b0f1a1f71a6f1e` |
| [#375](https://github.com/anderstvoss/virtualgamepad/pull/375) | `26168b3f291090f9e44bc1e6abd7cd3431dffee9` |
| [#387](https://github.com/anderstvoss/virtualgamepad/pull/387) | `685479bf69041d1da77452bc67f43acfbf6fb1ca` |
| [#389](https://github.com/anderstvoss/virtualgamepad/pull/389) | `edeea686d80ff38cabf7bb3547e250d6b71a57b9` |
| [#390](https://github.com/anderstvoss/virtualgamepad/pull/390) | `a0398b75010ba134839379f0c0dc87fba77cab1f` |

Step e has no new public API delta: the final snapshots contain root 172, Wii 23
and Steam 14 pages. Its actual PR/merge SHA is recorded in #322 closeout after
owner merge. The [final migration table](../ALPHA_API_MIGRATION.md#alpha1--alpha2-final-consumer-table)
is authoritative for removed combined input types and legacy constructors,
final native `<Family>Inputs`, and independent metadata/status reads. There are
no compatibility aliases or new input event queues. These source changes do not
qualify hardware, host permissions or the remaining release gates.

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

The #369 Xbox driver lab helper, xpadneo rule wrapper, Xbox additions to the input-lab rule and temporary Steam visibility rule are dev/test-only, not release defaults, and are not installed by default. Per the 2026-11-11 [owner classification recorded in #369's review](https://github.com/anderstvoss/virtualgamepad/pull/369#issuecomment-6806936162), no new #258 sign-off is needed for these development changes; selection of a beta+ subset is deferred.

## Known limitations

- **All implemented support paths remain WIP**; undeclared and deferred
  targets are not shipping realizations. Per-function virtual-host or partial
  physical correspondence is not blanket physical-controller equivalence.
  Physical fidelity #136 and many consumer combinations remain unqualified.
- **First-valid-neutral oracle (#166):** the deterministic oracle core landed
  in #372, but the host capture/consumer adapter and empirical first-valid-neutral
  timing and creation-performance qualification remain missing. Existing per-family
  N1/N4 lifecycle runs are lifecycle evidence only; **#166 is not an
  Alpha 2 gate**, per [#365 owner decision](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6792411358).
- **Audio:** sustained duplex continuity, latency, matching profiles and
  physical audio are not qualified (#116, #135, #143, #145); dropouts and
  earlier strict zero-loss failures remain recorded. USB/IP is opt-in WIP.
- **USB/IP host output:** #350 provides retained **snapshots** (live/final),
  **not a loss-aware host-output stream**. VG-2/#344 and ADR-0024 are assigned
  to the separate audio track; #373 landed the journal core without family
  wiring. Neither is an Alpha 2 gate.
- **Alpha 2 gate — real kalux USB/IP #350 run: PASS** on candidate
  `324d250bb01a5ddbfc1dd189513d4c9160429ee1`, per the [recorded #365 result](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-6969740517). All three verdicts passed:
  live `host_outputs()` snapshots while in service, FINAL retained snapshot
  after close, and stale-worker refusal with reinstall guidance. The candidate
  pair was restored and root API smoke passed; the report retains a separate
  lower-level raw broker smoke failure for investigation. This is scoped
  #350 evidence, not general streaming or final-release qualification.
- **Xbox Series:** uinput is limited to conventional main-motor rumble;
  Bluetooth-bus UHID has independent virtual trigger-rumble commands but no
  verified physical trigger actuation. SDL 3.4.18 evidence is for virtual
  commands; Steam package 1792618285 listed both targets and exercised controls
  excluding Guide, with the temporary active-seat dev setup for UHID. Steam
  Guide test-screen registration and Steam trigger-rumble support remain unverified. Real radio/pairing,
  wired GIP, battery, LED, audio and accessories remain unimplemented
  ([Series guide](../XBOX_SERIES.md), #160, #168, #164).
- SDL 3.2.10's Wii HIDAPI path can disconnect on a non-MotionPlus extension
  hotplug without reattaching the still-enumerated device ([#241](https://github.com/anderstvoss/virtualgamepad/issues/241),
  [W4 evidence](../WII_REMOTE.md#w4-fixed-extension-sdl-scope)).
- Linux SDL Wii MotionPlus gyro events were not observed in the tested SDL
  3.2.10 path; kernel MotionPlus gyro delivery was observed separately
  ([#244](https://github.com/anderstvoss/virtualgamepad/issues/244), [W5 evidence](../WII_REMOTE.md#w5-gyro-consumer-boundary)).
- DualShock 4's typed USB HID output event (`DualShock4HidOutput::UsbOutput`)
  omits a decoded blink field; retained `host_outputs()` carries the blink
  bytes. This is an Alpha 2 API event-model limitation, not a finding that host
  blink bytes are lost ([#287](https://github.com/anderstvoss/virtualgamepad/issues/287)).
- **Xbox 360 feedback (#282):** Linux uinput exposes conventional evdev
  `FF_RUMBLE`, but cannot create the `xpadN` LED-ring class device used by
  Linux xpad. Standard-HID USB UHID has no output report; it does not provide
  proprietary Xbox USB feedback. The uinput rumble observations are scoped
  consumer evidence; physical motor and LED-ring behavior remain unvalidated
  ([#282](https://github.com/anderstvoss/virtualgamepad/issues/282)).
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
- **Research closeout:** #304 kernel observations are recorded upstream by
  [research #122 merge](https://github.com/anderstvoss/controller-protocol-research/commit/261e7bec4351a6969563893a2092198a183ab8c9); the separate vg adoption decision remains open.
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
4. Use `controller.inputs()` and copyable `controller.status()`; `set_inputs`
   and `edit_inputs` take final native input types. Read battery/clocks through
   status, and use Sony `refresh_motion()` for an explicit equal-value sample
   tick. With Wii, use typed `battery()` / `set_battery(WiiRemoteBattery::new(raw))`
   and live `current_topology()`; `projected_input_topology(bool)` is for detached
   editors. To neutralize an attached extension, call `neutralize()` or clone
   accepted inputs, neutralize the clone, then `set_inputs`. Default Wii inputs
   describe no extension. For example:

   ```rust,no_run
   use virtualgamepad::{ControlError, WiiRemoteController};
   fn clear_attached(wii: &mut WiiRemoteController) -> Result<(), ControlError> {
       let mut neutral = wii.inputs().clone();
       neutral.neutralize();
       wii.set_inputs(neutral)?;
       let _live = wii.current_topology();
       Ok(()) // Commit and service explicitly in the caller.
   }
   ```

   See [the consumer table and historical removed-name ledger](../ALPHA_API_MIGRATION.md#alpha1--alpha2-final-consumer-table)
   and snapshots under `docs/api/`. Steam's source timestamp remains input;
   configured desktop support differs from accepted host intent. Step e awaits
   owner merge and does not close the other Alpha 2 gates.
5. Where USB/IP is explicitly enabled, reinstall an outdated worker to
   match the #350 worker-control protocol; use the existing signed-off
   host setup procedures. Review the final
   [Alpha 2 checklist](ALPHA_2_CHECKLIST.md) for qualification.
