# Brief 21: Wii U Pro Controller package (#361)

Read [ground-rules.md](ground-rules.md) first, [brief 15](15-xbox-series-package.md),
[brief 17](17-gamecube-adapter-package.md), [brief 18](18-joycon-package.md),
#361 (including its starter findings), #322 (owner decision 6747067225),
[brief 13](13-wii-remote-package.md), the [Wii Remote guide](../../WII_REMOTE.md)
and the existing Wii Remote package (`crates/gr-curated-controllers/src/wii_remote.rs`
and `crates/gr-curated-controllers/src/wii_remote/`). This is an implementation
brief, **not authorization to start implementation**. The #322 alpha API policy
supersedes the snapshot freeze in the ground rules.

**Lane:** Controller. **Target:** Nintendo Wii U Pro Controller, research controller ID
`wii-u-pro`. **PR boundaries:** 21a–21g, one independently reviewable PR per step
(21a in the research repo; 21b–21g in virtualgamepad); no checkpoint branch
created by this brief.
**Blocked by:** VG-1b (`docs/spec/input-model-state-and-mutation.md` §9 steps a–e) complete; this brief merged to main, owner authorization, adopted research pin,
and an eligible Linux host for live acceptance. Retail descriptor/identity
fidelity and any new host privileges have independent gates below.

## Goal and decided context

Add a separate curated Wii U Pro family, with
`create_wii_u_pro(CreationOptions, WiiUProOptions)` at the curated and ordinary
root layers (C7). Recommend separate family, not a Wii Remote revision or
caller-visible Classic Pro attachment: the kernel uses an internal extension
decoder, not a removable expansion port. The native topology is one permanent
gamepad with 17 digital controls (A/B/X/Y, Plus/Minus/Home, four D-pad directions,
L/R/ZL/ZR, both stick clicks) and two analog sticks. ZL/ZR are digital, not native
analog triggers. Do not add Remote 1/2 buttons, IR, accelerometer, MotionPlus,
speaker, hotplug accessories or arbitrary gamepad remapping.

- **First, `linux.uhid.bluetooth`:** faithful-path target (local Wii-family
  HID protocol, driver and sysfs behavior), playable once an actual consumer
  observes the controls. `BUS_BLUETOOTH` is only kernel metadata. It is not
  radio advertising, pairing, bonding or reconnect (#164). Start from a
  source-backed consumer compatibility model, not a retail-device claim.
- **Then, `linux.uinput`:** separately selected reduced playable realization,
  one Pro evdev node with the evidenced key/axis surface and conventional FF
  mapped to the native single binary motor. No HID parent, addressed memory,
  raw protocol or kernel Wii LED/battery/calibration interfaces. No silent
  fallback or extra generic inputs.
- **USB:** no native wired USB data/USB HID path is evidenced; cable/charge
  flags do not establish such a transport. USB-bus UHID cannot bind hid-wiimote.
  Do not target a counterfeit USB personality. If later evidence justifies a
  distinct USB-level realization, apply brief 17's owner precedent: USB/IP
  production target, experimental `dummy_hcd` secondary as needed, each its own
  PR, permission audit amendment and owner sign-off. Neither is a dependency
  here. Real Bluetooth remains separately deferred.

**Post-VG-1b shape:** family-native `WiiUProInputs` contains only accepted
button and stick levels. Propose validated four 12-bit positions (nominal
source-model center 2048; no measured retail calibration). Preserve separate
directions and stick clicks. `set_inputs`, per-field setters and
`edit_inputs` use one family-specific transactional final-state validator;
`inputs()` is the accepted read view. A rejection leaves desired values,
dirty state and pending work intact; no-op adds no new dirty work. Neutralize
only changes caller inputs; preserve already pending deliveries and explicit
`commit()`/`service()`. Callers commit between meaningful edges; no generic
edge queue, masks, compatibility aliases, provider-owned protocol or generic
mutable state. Battery/charging/connection metadata, identity, protocol clocks,
fixed topology and retained host outputs are separate typed views. The brief
**assumes VG-1b steps d and e are finished**; do not extend the interim Wii
combined state or touch the local agent's branches.

Identity is caller-specified at creation, unique per-instance default (#156,
#162 and C7); use a validated typed identity, distinct association/physical
correlation for each live device, stable for its promised lifetime. Document
which fields each target represents. UHID can carry synthetic uniq and matching
HID identity; uinput has no uniq setter (use association and typed handle
identity). Do not infer physical serial/version or publish identifying values.
Choose a consumer-match VID/PID only after the evidence gate; distinguish
compatibility choice from retail identity. Options are immutable at creation
for any compatibility shim; keep strict and compatibility test arms explicit.

## Evidence and function matrix

Research PR #137 is merged. At research revision
`73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f` read the
[Pro registry](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/records/controller/wii-u-pro.json),
[Remote registry](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/records/controller/wii-remote.json),
[Pro starter](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/docs/wii-u-pro-starter.md),
[Pro L0 ledger](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/docs/ledgers/wii-u-pro-l0.md)
and [Remote L0 ledger](https://github.com/anderstvoss/controller-protocol-research/blob/73a6fc6724c3fbf9b6cbbf5aaa21822978c42c4f/docs/ledgers/wii-remote-l0.md).
The Remote ledger expressly excludes the Pro. Registry IDs are both parentless;
the shared code recommendation is inferred, not a protocol equivalence claim.
At vg base `12358d71d762fb438cbf01f5b120986279a8540d` the
`protocol-research/` gitlink is `eba02ad610e264a2518a699394255cc7f1d670dd`;
the Pro registry lookup at that pin returns not found. Adopt a reviewed
descendant containing the required records before implementation; this brief
does not bump the pin.

| Native function / consumer boundary | Source claim IDs | Bluetooth-bus UHID | uinput |
| --- | --- | --- | --- |
| Identity and driver binding | `wiiupro-pa2-kernel-bluetooth-match`; `wiiupro-pa2-kernel-pro-classification`; `wiiupro-pa2-physical-identity-gap` | WIP; source evidence target for Pro classification, physical identity unknown | WIP; source evidence target for evdev shape only; no HID binding/uniq |
| 17 digital controls including ZL/ZR and stick clicks | `wiiupro-pa2-kernel-pro-input`; `wiiupro-pa2-sdl-pro-mapping` | WIP → source evidence | WIP → source evidence; SDL mapping independently checked |
| Two sticks, native packing and neutral | `wiiupro-pa2-kernel-pro-input`; `wiiupro-pa2-kernel-pro-stick-calibration`; `wiiupro-pa2-sdl-pro-report`; `wiiupro-pa2-sdl-pro-mapping`; `wiiupro-pa2-physical-calibration-gap` | WIP → source evidence, physical range/calibration unknown | WIP → source evidence, transformed to kernel units; fuzz deviation |
| Internal Pro classifier, register/init, rate/mode and report 0x3d | `wiiupro-pa2-kernel-pro-mode`; `wiiupro-pa2-kernel-pro-classification`; `wiiupro-pa2-sdl-pro-detection`; `wiiupro-pa2-sdl-pro-report` | WIP → source evidence for verified subset; exact startup/tail/cadence unknown | Not supported (no HID command/report channel) |
| Binary rumble, four player LEDs | `wiiupro-pa2-kernel-pro-output-power` | WIP → source evidence, physical effect unmeasured | WIP → source evidence for binary FF; LED class/output Not supported |
| Battery/charging/connected flags and power status | `wiiupro-pa2-kernel-pro-output-power`; `wiiupro-pa2-sdl-pro-power`; `wiiupro-pa2-physical-calibration-gap` | WIP → source evidence for consumer interpretations, not physical voltage or charging | Not supported on host; typed handle metadata only if meaningful |
| Calibration and persistence | `wiiupro-pa2-kernel-pro-stick-calibration`; `wiiupro-pa2-physical-calibration-gap` | WIP for kernel first-packet heuristic and pro_calib; retail EEPROM/storage unknown | Not supported for driver calibration interface; nominal conversion only |
| Motion, IR, removable extensions, speaker | `wiiupro-pa2-kernel-pro-mode`; `wiiupro-pa2-kernel-pro-output-power` | Not supported (not the Pro control topology) | Not supported |
| Exact-owned lifecycle, #166 timing, SDL acceptance | `wiiupro-pa2-first-uhid-bluetooth`; `wiiupro-pa2-separate-family-shared-code`; `wiiupro-pa2-virtual-acceptance-gap` | WIP; source evidence target, virtual host not run | WIP; source evidence target, virtual host not run |
| Cemu native Wiimote / Dolphin | `wiiupro-pa2-cemu-native-pro-exclusion`; `wiiupro-pa2-virtual-acceptance-gap` | Native Cemu path Not supported at cited revision; generic paths WIP/unknown | Native Cemu Wiimote path Not supported; generic paths WIP/unknown |

All listed Pro claims are presently source-supported, inferred or unknown
research, not implementation acceptance. Categories are **Not supported**,
**WIP**, **source evidence**, **physical evidence**; the first supported
target is source evidence, with no physical evidence in the Pro ledger.
Record deterministic synthetic and live virtual-host tests separately from
retail observations; neither is physical evidence. In the future support
matrix retain repository WIP / N/A / accepted-source / accepted-physical
status semantics and evidence links, not an automatic promotion from code
or a source claim. Actual Bluetooth is deferred, USB realizations not supported
for present scope; no whole-controller promotion.

### Pinned consumer behavior (restate, never copy excerpts)

- Linux v6.12 (tag resolves to
  `adc218676eef25575469234709c2d87185ca223a`):
  `wii-gap-dolphin-attach-kernel-core-v612`,
  `wii-gap-dolphin-attach-kernel-modules-v612`,
  `wiiupro-pa2-linux-hid-ids-v612`.
  [BT match](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-core.c#L1869-L1889)
  and [IDs](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-ids.h#L985-L987)
  cover Nintendo vendor 0x057e, products 0x0306 and 0x0330, not an independently
  proven retail Pro PID. [Classifier](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-core.c#L809-L855)
  selects Pro by extension type or name Nintendo RVL-CNT-01-UC.
  [ID read](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-core.c#L432-L455)
  tests a six-byte extension identifier at register 0xa400fa using suffix
  0x01 0x20. The other four identity bytes are not settled by this test.
  [Module selection](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-core.c#L594-L621)
  and [dispatcher](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-core.c#L1594-L1618)
  route Pro extension payload rather than Remote motion/IR. Pro evdev open
  requests extension reporting ([open and FF](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-modules.c#L1764-L1810)).
- Linux [Pro decoder](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-modules.c#L1572-L1761):
  the report 0x3d payload has 21 extension bytes, with first 11 interpreted.
  Four low-12-bit positions are in LX, RX, LY, RY order; the host subtracts
  the nominal center on X and inverts Y. Buttons are active low. First-packet
  center correction is per axis near neutral (absolute offset strictly less
  than 500); host recalibration is a driver operation, not device EEPROM
  ([sysfs](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-modules.c#L1813-L1860)).
  [evdev declarations](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-wiimote-modules.c#L1862-L1929)
  give ABS_X/Y/RX/RY, advertised limits -1024..1024, fuzz 4, flat 100,
  17 digital keys and FF_RUMBLE. Decoder output may differ from advertised
  limits; do not clamp silently or equate the declared bounds with raw range.
  Two conventional rumble magnitudes collapse to one binary motor.
- SDL `wiiupro-pa2-sdl-wii-877399b2` at
  `877399b2b2cf21e67554ed9046410f268ce1d1b2`:
  [VID/PID gate](https://github.com/libsdl-org/SDL/blob/877399b2b2cf21e67554ed9046410f268ce1d1b2/src/joystick/hidapi/SDL_hidapi_wii.c#L182-L190),
  [extension detection](https://github.com/libsdl-org/SDL/blob/877399b2b2cf21e67554ed9046410f268ce1d1b2/src/joystick/hidapi/SDL_hidapi_wii.c#L354-L443),
  [mode and calibration](https://github.com/libsdl-org/SDL/blob/877399b2b2cf21e67554ed9046410f268ce1d1b2/src/joystick/hidapi/SDL_hidapi_wii.c#L550-L603)
  and [mapping](https://github.com/libsdl-org/SDL/blob/877399b2b2cf21e67554ed9046410f268ce1d1b2/src/joystick/hidapi/SDL_hidapi_wii.c#L926-L1054)
  identify Pro type 0x0120, exclude MotionPlus, select report 0x3d and map
  15 buttons and six axes (four sticks plus digital ZL/ZR as trigger axes,
  not analog device controls). Its normalization differs from Linux. Its
  [power decoding](https://github.com/libsdl-org/SDL/blob/877399b2b2cf21e67554ed9046410f268ce1d1b2/src/joystick/hidapi/SDL_hidapi_wii.c#L509-L548)
  interprets coarse battery and charging/connection fields with uncertainty.
  Verify the continuous-mode flag and service expectations at the actual
  build, not an assumed physical reporting cadence.
- Cemu `wii-gap-consumer-cemu-hidapi`:
  [native Wiimote discovery](https://github.com/cemu-project/Cemu/blob/29f2f1fae4631e2317977debd9eacd55cc28dd7d/src/input/api/Wiimote/hidapi/HidapiWiimote.cpp#L5-L48)
  excludes the Pro product string. This is a negative control, not a claim
  about the generic SDL/gamepad route. Dolphin's Pro-specific support and
  generic Cemu/Dolphin routes need separate pinned records before asserting
  an affirmative path. Linux GPL sources are consumer expectations; do not
  reproduce code or comments.

### Reuse boundary and explicit research gaps

The existing source-backed Remote implementation is the closest reference,
not a Pro device fixture. Its `codec.rs` owns the report envelope, request
admission/length checks, ACK/status, deadline and bounded reply queue;
`memory.rs` owns addressed reads, chunk/error replies and Remote-specific
register routing/resources; `controller.rs` owns creation/session selection,
retained outputs, close/disposition and live topology; `realizations.rs`
owns the interim synthetic descriptor and separate evdev role maps.
`extensions.rs` packs a six-byte Remote accessory packet, `topology.rs`
exposes dynamic accessories, and `native.rs` manages their child nodes.
None of these proves a Pro report or a Pro accessory port.

Extract the smallest private Wii-family helpers for length/framing,
transactional admission, status/ACK/read reply construction, bounded
addressed read progression, LED mask and shared rumble bit, and use existing
common session/retained-output/lifecycle/resource primitives. Keep family
input types, status flag policy, report mode selection, register address
routing and error policy parametrized only where a concrete common behavior
is evidenced; no universal Wii memory trait or shared mutable resource.
Remote evidence entry points: `wii-kernel-report-modes`,
`wii-kernel-status-ack-flow`, `wii-kernel-extension-init`,
`wii-kernel-outputs-battery`, `wii-c5-report-layouts`,
`wii-c5-memory-errors`, `wii-dolphin-memory-routing` and
`wii-memory-size-hypothesis`. These scopes are Remote only; reuse a primitive
only after Pro consumer evidence covers the corresponding behavior.

Pro-specific: new 21-byte report 0x3d encoder (not the six-byte accessory
packer), validated sticks/active-low buttons and power fields, internal
extension identification and initialization, status presence semantics,
power policy, fixed topology and single evdev node. Do not reuse a Remote
revision label, its attach/detach or MotionPlus map, IR/camera/speaker
registers, its EEPROM size hypothesis or accessory calibration format.
Use independent Pro state per instance. No copied source fixtures, retail
memory contents, real device dumps or identifying data.

Explicit gaps: full retail HID descriptor, actual VID/PID/version/SDP,
full internal ID and required init sequence, exact startup/status/ACK
and mode transitions incl. idle consumer reopening, uninterpreted report
bits/tail, calibration persistence, charge/battery semantics and physical
cadence/actuation. The three explicit unknown claims are
`wiiupro-pa2-physical-identity-gap`, `wiiupro-pa2-physical-calibration-gap`
and `wiiupro-pa2-virtual-acceptance-gap`. Where Pro lacks a source record,
add a research PR before implementation; reused Linux core/modules source
locators focus on MotionPlus, so confirm/add Pro-specific line coverage in
that PR rather than treating every cited line as a current Pro claim.
If firmware investigation would be required, mark **needs private analysis**
and stop on that gap. Do not invent bytes, measured calibration or timing.

## Ordered steps (one reviewable PR each; all tests proposed, not run)

**21a. Research completion, before code (research repo).** Add a research PR
with Pro-scoped records for the exact kernel and SDL boot/open paths,
full ID/register probe and initialization, supported status/ACK/read/write
report lengths and error replies, neutral and boundaries at the consumer
decoders, output and power semantics, complete evdev name/key/code/range
table, and descriptor/identity availability. Distinguish source-backed
fields from explicit synthetic compatibility choices (including unused
report bytes/ID prefix), and document whether continuous mode is actually
requested. Record generic SDL evdev mapping and affirmative emulator paths
only if verified; otherwise retain unknowns. If behavior still lacks evidence,
refuse it or block dependent features rather than invent it.
**Done when:** source/claim schema, publication guard, corpus tests and
reviewed source line/revision checks pass; each implemented constant/request
has a Pro-scoped claim, and outstanding gates are explicit. Adopt the merged
research pin at 21b, not in this brief.

**21b. Compiled family, native contracts and post-VG-1b inputs (vg main).**
Add a separate `virtualgamepad.wii-u-pro` compiled family using the research
ID, fixed topology, input-only `WiiUProInputs`, typed metadata and outputs,
realization restrictions, explicit identity policy and creation options.
Declare staged unavailable targets honestly; no provider opens until its
implementation lands. Retain the native binary motor and four LED bits,
rather than widening output type for conventional FF. Define a nominal
synthetic neutral and model ranges, separate from any device calibration.
**Done when:** independent buttons, digital triggers/clicks and four
axes, domain boundaries, exact neutral, state/field/closure parity,
late-failure rollback incl. dirty flags, no-op pending-work preservation,
closed-handle rejection, identity isolation and unsupported options are
covered. No generic inputs or Remote attachment operation.

**21c. Private Wii-family reuse without Remote changes in behavior.**
Extract only concrete common Wii framing/reply/read-cursor/LED/rumble
helpers from the current Remote code after VG-1b, and use existing common
resource/retained/lifecycle infrastructure. Keep the Remote's memory map,
topology, calibration, speaker/IR and MotionPlus logic local. Avoid modifying
public Wii API merely to make reuse convenient.
**Done when:** the pre-existing Remote codec, memory, extension, compat,
lifecycle and fake-provider tests pass unweakened; byte-exact synthetic
Remote reply sequences, errors, queue bounds and stop/start behavior are
unchanged. Pro-focused helper tests prove independent instance state and
no Remote-specific assumptions. No expected public API change in 21c.

**21d. Pro protocol and internal register model.** Add Pro codec and
fixed internal ID, report 0x3d (21-byte payload, first 11 decoded),
mode select and source-backed initialization; evaluate the common Wii
host requests 0x10/0x11/0x12/0x15/0x16/0x17 and replies 0x20/0x21/0x22
against 21a before declaring them. Model only the bounded Pro register
windows actually justified by evidence, including the A4 internal
extension detection path; no inherited MP or external attach. Use separate
power metadata and binary motor/LED retained intent. Define source-backed
read/write behavior, exact negative replies for unsupported spans, and
explicit rejection of unsupported modes and feature requests. If persistent
Pro resource evidence later establishes a need, add a separate researched
PR using #162's validated import-before-open and terminal keep/discard
contract; no fabricated retail EEPROM map or default factory data.
**Done when:** deterministic neutral, press/release, per-button bit and
stick-order tests independently check Linux/SDL interpretations; cover
each admitted request and required response, negative/malformed requests,
missing/wrong ID, split/straddling reads, unsupported registers/modes,
queue saturation, rollback/retry, repeated outputs, retained output despite
observation loss, stop/start, repeat open and close. Every declared host
request gets its exact reply within the required poll cycle. No invented
rate guarantee; a bounded synthetic scheduling policy, if required by a
consumer, is named, documented, measured only on virtual host and switchable
at creation.

**21e. Bluetooth-bus UHID realization and converged root wiring.** One
owned UHID with tested Pro-compatible HID identity and report descriptor,
typed session, association, lifecycle, service, registry selection, demo
and root/curated constructors. Default per-instance identity and allow
validated caller identity before open; no serial or pairing data fixtures.
If owner approves a synthetic descriptor/identity profile, explicitly
select it at creation, mark **WIP compatibility**, and never label it retail
faithful; replace with an evidenced descriptor when available. No implicit
fallback, no fabricated unsupported report declarations. Reject invalid
options before obtaining provider resources.
**Done when:** descriptor/report lengths and numbered report declarations
match the synthetic/source fixtures; root/curated compile and snapshots pass;
fake-provider lifecycle, exact GET/SET success or refusal, output, retries,
creation rollback, independent association and idempotent cleanup pass.
On an eligible host verify wiimote binding, Pro classification, single Pro
evdev node rather than base Remote or MP nodes, hidraw and LED/power
interfaces, correct open-mode and first neutral; if blocked, record missing
host/descriptor evidence and keep WIP. No automatic driver rebind.

**21f. Reduced uinput realization.** Add one independently selectable
Pro evdev device matching the source-backed 17-key/four-axis capability
table and conventional FF_RUMBLE collapsed to one binary native output.
Keep ZL/ZR digital; SDL may project them as axes, not new native fields.
Document provider's USB bus metadata and inability to express uniq or
nonzero fuzz, absent HID/LED/battery/driver calibration and any necessary
mapping or value conversion differences. No generic pad impersonation or
silent success for missing capabilities; expose each realized limitation
and typed refusal where applicable. If FF path is blocked, do not advertise
FF or mark its cell accepted; deliver only after evidence-backed FF test.
**Done when:** evidence-derived golden capabilities compare to fake-provider
ones including each documented discrepancy; tests cover every control,
axis sign/center/boundary conversion without hidden clipping, independent
stick clicks, binary FF nonzero/zero, failed open, repeated create/remove
and cleanup. On an eligible host compare exact evdev shape and SDL evdev
mapping, retaining differing/raw paths independently. Explicit uinput
selection and no automatic fallback are regression-tested.

**21g. Acceptance, #166 timing and closeout.** Add
`acceptance/virtualgamepad.wii-u-pro.toml` and `docs/CONTROLLER_SUPPORT.md`
per-function/realization evidence. Include new family examples, docs and
app migration; coordinate #166 consumer oracle integration with its owner,
not its in-flight code. Record selected and independently attributed
consumer paths, exact candidate/pin, OS/kernel, consumer versions/build
hints, commands, test IDs/results, permissions, cold/warm N=1 and
concurrent N>=4 create/remove distributions, resource counts, failures
and cleanup. Device creation, kernel node existence, SDL discovery,
probe completion, first report, first valid neutral and removal-to-node-gone
are distinct observations. Test non-neutral first packet and recalibration
as a separate qualification, not a substitute for a neutral timing run.
**Done when:** the manifest validates and every row has supported evidence or
an explicit untested/unavailable reason; kernel, SDL and supported
output paths have scoped virtual-host results on an eligible host; if the
#166 oracle or host is not ready, preserve missing observations and leave
the corresponding cells open. No universal latency limit or inferred
physical acceptance.

## Host setup and audit impact

Use existing #258 signed-off creator access to dev nodes, distro/seat
consumer access and ordinary kernel binding. Verify a Linux host with
`/dev/uhid` writer access, hid-wiimote present/bindable, exact-owned
evdev and hidraw read/write as appropriate, plus `/dev/uinput` for the
second target. Record creator and consumer permissions separately,
kernel driver, sysfs ownership and actual node attribution; the already
documented Wii contiguous-memory allocation failure is a diagnostic
possibility, not proven on this host. No automatic module loading,
global ACL, group input, system broker or sandbox policy changes.

**New Pro surfaces to inventory as dev changes before use:** the Pro-only
`pro_calib` sysfs read and optional recalibration/write (who can read,
administrator-mediated exact-object write if needed; state local to
kernel instance; removal resets it), Pro LED-class/power-supply ACLs,
and exact-node Pro hidraw grants and any new input-lab family rule.
Test read-only first; do not write calibration or alter another device.
For a direct SDL HIDAPI vs bound-kernel conflict, first diagnose and
test separately; any new Pro per-device hid-generic handoff, replaced
hidraw ACL or sandbox override needs a test/dev audit amendment and
owner approval. Handoff loses wiimote-specific evdev/sysfs while active;
record loss, restore binding and revoke temporary grants at cleanup.
Do not inherit Wii's prior test-only privileges automatically. Avoid
global unload, blacklist, memory compaction or elevated daemons;
if necessary, owner reviews separately as system-wide dev actions.

For every new provisioned scope amend `docs/ALPHA_2_HOST_AUDIT.md` in
its own reviewed implementation/setup PR with mechanism, users,
required/optional, privilege, scope/persistence, risks to other devices,
cleanup/crash recovery, safe release default and evidence; get new
Anders sign-off (#258 comment 6777536122). USB/IP worker/attach,
USB bus ACLs, and `dummy_hcd` module/configfs/gadget setup are distinct
heavy-permission surfaces if ever justified; each independently requires
a new audit amendment and sign-off (brief 17, #296 comment 6807419275).
Do not change host policy or introduce a privileged realization in
this docs PR. Keep real Bluetooth within #164.

## Snapshot, migration and acceptance consumers

This docs PR adds one file only; **no code, snapshots, submodule pin,
acceptance files or host policy changes**. Each future implementation PR
updates affected root/supporting snapshots, `docs/ALPHA_API_MIGRATION.md`
and lists exact deltas in its PR body under #322. If extracting private
helpers unintentionally changes the Wii supporting snapshot, address
it explicitly; no compatibility aliases. Keep broker/IPC wire format,
security, license, release files and workflow changes owner-gated.
Maintain source pin/claim traceability, regression tests, compiled
in-repo consumers, demo and examples in the PR that introduces the
public surface.

Kernel hid-wiimote evdev and sysfs, SDL evdev, SDL Wii HIDAPI and generic
SDL mappings are mandatory separate acceptance paths; verify each exact
version and hints, individual controls, FF and output where available,
battery/LED/status separately. Cemu native Wiimote is a pinned expected
exclusion; Cemu generic SDL and Dolphin generic input are independent
research/acceptance candidates, never inferred from Wii Remote support.
RetroArch generic gamepad and Steam Input are supplementary distinct
manual/untested paths until exercised. Physical reference tests would
be separate owner-authorized experiments with sanitized summaries only.
No host or hardware run was performed for this brief.

## Open questions for Anders

1. **Separate family or Wii Remote variant?** Recommend a separate
   `wii-u-pro` family with private Wii-family primitives and independent
   topology/identity/API. Linux and SDL's internal extension representation
   is not a reason for the public handle to behave like a Remote.
2. **Provisional descriptor/identity and compatibility defaults?** Recommend
   an explicitly selected, creation-toggleable consumer-compatibility
   profile after 21a documents its evidence and synthetic choices; default
   to strict/source-backed behavior, refuse fidelity requests without a
   sourced descriptor/identity, and never label the provisional profile
   retail faithful. Do not remove or weaken rejection tests.
3. **Calibration/storage and power scope?** Recommend nominal synthetic
   stick neutral plus separate typed power metadata for the evidenced
   consumer model; test kernel auto-centering and optional sysfs recalibration
   separately. No Pro EEPROM import/persistence until Pro-specific evidence
   establishes its map and need; no physical power accuracy claim.
4. **Host and hardware availability?** Recommend authorize an eligible D9
   host with existing creator permissions for bound-kernel UHID, SDL direct
   and evdev, and uinput runs; separately approve any exact-device
   recalibration/LED/rebind dev change if needed. If a retail Wii U Pro is
   available later, authorize a separate evidence effort within public
   sanitization constraints; otherwise keep physical cells unknown. Do not
   block deterministic implementation solely on unavailable lab hardware.
