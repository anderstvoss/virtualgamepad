# C6 D9 acceptance and timing

Run date: 2026-11-03 (Pacific). Linux `7.3.0-1-mainline`, x86-64, Intel Core i5-8265U, rustc 1.95.0. Tracked-clean executable source `bbf6b961390f3aad6985ec979f775df4bb9dca6f`; research pin `fc6964ac8799f0b7a38ffb9526361450f0b1b87a`. These are virtual consumer observations, not physical-device evidence.

## Timing (#166)

[Tracker observation](https://github.com/anderstvoss/virtualgamepad/issues/166#issuecomment-6624965663).

Ten cycles at N1 and N4 for each realization: 100 total samples. The first cycle is cold; the rest warm. N controllers coexist before removal. All operation error lists were empty; every node-gone stage completed. FD, thread, input-node and hidraw-node counts returned to baseline. Missing consumer-open observations retain EACCES. First-valid-neutral has no runner oracle; SDL was not requested for timing.

| Realization | N | Warm samples | API p50 ms | Close p50 ms | Close max ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| linux.uinput | 1 | 9 | 0.774 | 14.774 | 24.874 |
| linux.uinput | 4 | 36 | 0.415 | 14.190 | 19.548 |
| linux.uhid.usb | 1 | 9 | 0.168 | 83.287 | 94.593 |
| linux.uhid.usb | 4 | 36 | 2.223 | 99.533 | 202.708 |

Cold UHID N4 close had one 5507.519 ms sample. The warm table does not hide this tail. Concurrent build and research-validation workloads were active; this is not an isolated-host performance baseline. No latency budget or throughput claim is made. API return measures preparation/open; removal measures close through node disappearance with 1 ms polling. Kernel initialization and peer servicing can affect close duration. These historical measurements are not relabelled as final-head performance.

Reproduce with `cargo run --locked -p xtask -- acceptance --controller virtualgamepad.steam-controller-2026 --realization <linux.uinput or linux.uhid.usb> --cycles 10 --concurrency <1 or 4> --out <local-json>`. Keep raw host output local; it contains transient device paths.

## Historical 14d functional observations

A separate one-cycle UHID run used private SDL `e0232c5b9d972ad394d597ce858185cd15a0f1a7` with command-scoped library/pkg-config paths and `SDL_JOYSTICK_HIDAPI_STEAM=1`. It observed an exact associated hidraw device, `hid-steam` binding, neutral, all six axis bits, changing gyro and accelerometer samples, and 37 touch events. The probe structured touch verdict remained missing because it tracks first-pad contacts only. It saw button down/up mask `0x300000`, not the earlier 14c full `0x3fffff` mask. Steam was running during this rerun; no causal attribution is established. Earlier #298 observations are retained as historical results, not substituted for the rerun. Full per-button live acceptance remains No evidence on this run.

SDL advertised rumble and submitted a rumble call. Controller-side output delivery/actuator effect is No evidence. The client listing/input test remains manual and untested.

Sysfs capability masks were observed, including the gamepad key/axis masks and hid-steam FF bits beyond FF_RUMBLE. Event decoding and ABS ioctl ranges/fuzz/flat/resolution remain No evidence: direct evdev opens returned EACCES. Driver binding and sysfs capability agreement do not demonstrate packet decoding. The [input-lab extension #299](https://github.com/anderstvoss/virtualgamepad/pull/299) is a separate owner-reviewed proposal; nothing was installed.

## Missing evidence and restrictions

This list describes the C6 checkpoint. The post-C6 follow-up below adds scoped
kernel decoding and ABS ioctl observations without relabelling historical runs.

- Both realizations: per-control kernel events, ioctl ABS attributes, controller-side output delivery, Steam client listing/input/gameplay, identity reuse, physical serial format, physical effects/timing/calibration/firmware.
- UHID: feature 2 transactions, 0x44 ACK/flow control, output 0x86–0x89 streaming, real-device unknown-command responses, bootloader attach sequence, automatic input cadence, rumble auto-stop and lizard watchdog duration.
- Reduced uinput cannot expose motion companion properties/timestamp, independent pad pressure/contact and grip/stick touch metadata, vendor haptics/features or host-visible serial; axis fuzz/resolution are provider-limited. These are explicit restrictions, not evidence of equivalence.

## Local record digests

Raw files are retained locally; these SHA-256 digests identify the measured records without publishing host paths.

- `uinput-n1`: `4a0dd8e53456369295dff14d04208877fb459c95c329eac1dcda8c5db6505fb5`
- `uinput-n4`: `2a080cfa02cfa040579a4b12b779f9156c396e69a64bae029a6e440ef945a85d`
- `uhid-n1`: `8c0f4d5d9b658ce2453656844297191e45aad6cd371b2768ccd42d4481ad20d8`
- `uhid-n4`: `61c976085f03e794aecd30cff8ffc74e2e04e0b2241ef692360ca54986cf8fd5`
- `sdl`: `a7d9b29ea08c6909ec57dcc114b3f4768a317a24ec8165b82d27ea56c99fedf9`

## Manual Steam client procedure (owner)

1. Use the reviewed C6 head on the prepared D9 host. Record commit, kernel,
   Steam build/channel and controller support settings. Close SDL probes and
   remove earlier virtual controllers; keep real input available for recovery.
2. Launch `cargo run --locked -p virtualgamepad-demo`. Select
   **Steam Controller 2026**, choose **linux.uhid.usb**, and press **Create**.
   Leave the demo running so its worker services feature requests.
3. Open Steam → Settings → Controller. Report whether a Steam controller is
   listed, its displayed name, and whether opening its input test succeeds.
   A missing listing is a result; do not change system policy to force it.
4. In the demo, press/release every face, D-pad, shoulder, trigger click,
   stick click, Menu/View/Steam/QAM and L4/L5/R4/R5 control individually.
   Sweep both sticks and triggers; move/touch/click both pads separately,
   then change raw gyro/accelerometer fields. Report each visible response,
   missing control, stuck state, axis polarity/range and unsupported display.
   A test screen without a motion display provides no motion evidence.
5. If Steam offers an identify/rumble action, invoke it and report the demo's
   retained/logged output request and any errors; no virtual hardware actuator
   effect is claimed. Do not send firmware/bootloader commands or use capture
   tools without separate authorization.
6. Remove only this controller in the demo. Confirm Steam removes its listing;
   create it again and report discovery/cleanup behavior. Then close the demo.
   Report leftovers and errors with sanitized summaries, never real serials,
   private paths or raw captures. Repeat reduced **linux.uinput** as a separate
   case, explicitly reporting its missing native HID controls/features.

The step 16 IBEX stub was not listed by Steam. That stub returned short synthetic
identity/attribute/settings replies. The current codec returns descriptor-sized
64-byte feature-1 frames: a per-instance synthetic or validated caller-set serial
for 0xae, the structural empty attribute response for 0x83, and a descriptor-sized
malformed command echo for unknown GETs after explicit unsupported SETs. Known
settings are retained, unknown setting IDs reject atomically, feature 2 remains
Unsupported, and no 0x47 input is emitted. These changes do not establish Steam
client acceptance or real-device reply fidelity.

## Puck feasibility (out of C6 scope)

C4's receiver/epoch infrastructure landed in #215; this generic dependency is
closed. Its synthetic 0-to-4-slot fixture is not a Puck implementation or evidence
for real receiver identities, slot protocol, pairing, reconnect or feedback routing.
Those device-specific gaps remain open. A direct wired 1302 controller uses one
unified HID interface; a single UHID node cannot be called faithful Puck USB.

USB receiver feasibility #168 is still open. It must establish whether USB/IP or
another provider can expose the required enumeration, interface topology, control
requests and exact success/error replies, interrupt endpoints and clean teardown
without falling back. UHID kernel binding for one fabricated Puck interface does
not answer that question. Step 16's single-node Puck had no matching SDL joystick,
despite exact-node access. Keep that negative result and the wired controller's
positive kernel/SDL tier decision separate.

The interface-count conflict is unresolved: OpenPuck describes four interfaces,
while the kernel consumer describes seven (0–1 non-HID, 2–5 slots, 6 pogo).
Consumer source agreement alone cannot settle physical topology. A separately
authorized physical descriptor/topology investigation and receiver protocol
research must settle transport/version differences and interface roles before a
faithful Puck plan. No receiver, Bluetooth, bootloader or streaming implementation
is introduced by this checkpoint.

## Before-merge review follow-up (2026-11-04)

Steam was shut down normally, and `pgrep -x steam` and
`pgrep -x steamwebhelper` both returned no processes before and after the probes.
The original checkpoint head `55df060` with the original harness returned
`0x3ffffe` down/up, axes `0x3f`, 37 touch events and changing valid sensors.
The missing A bit exposed a separate startup race: a fixed 350 ms producer delay
could start the first button before SDL finished compilation/open/neutral polling.
The measured `bbf6b96` baseline returned `0x3fffff`; runtime codec/state/runner/probe
content is identical across the relevant interval (demo-only changes do not
feed xtask). The historical successful baseline does not exclude a timing race. Bisection of the original
14c commit line tested the main merge `6388aea` and demo timestamp commit
`eb5d4c3`; both returned full down/up masks. Retesting the nominal endpoint
`9e9df6f` also returned full masks. Its only delta was a demo test, so the
initial bad endpoint was not stable; attributing the loss to that commit would
be incorrect. The neutral-ready regression targets the shared timing defect.

The harness now waits for the probe's flushed neutral-ready marker before
starting its single-button transitions. A deterministic clock test covers
arbitrary opening delays through five seconds, preserves the first A transition
and the final trigger, and checks repeated polling without premature samples.
The diagnostic probe also retains independent down/up/motion masks for both
pads, while preserving its legacy first-pad fields. Fake SDL regression tests
exercise both pads and normal close. No descriptor, public API or snapshot changed.

With Steam still fully closed, the corrected one-cycle wired UHID probe using
private SDL `e0232c5` and command-scoped `SDL_JOYSTICK_HIDAPI_STEAM=1` returned:

- Button down **0x3fffff**, button up **0x3fffff** (all 22 bits).
- Axes **0x3f** (all six).
- Pad 0 and pad 1 each: down mask **1**, up mask **1**, motion **true**.
- Gyro and accelerometer each: present/enabled, **167** distinct samples,
  valid timestamps/values and no invalid-sample flag.
- No operation errors; owned nodes disappeared and fd/thread/input/hidraw
  counts returned to baseline.

Steam running explains the bulk suppression in the historical 14d result
`0x300000`; the closed-Steam check additionally identified and fixed the
independent first-button startup race. Keep the original #298 and 14d runs
as history, not current failures or replacement measurements. These are SDL
decoding observations, not kernel evdev decoding, physical calibration or
controller-side output acceptance. #299 is merged as `17936b7` but is not
installed: kernel-evdev-decoding and evdev-abs-ioctl remain No evidence.

The default serial is now compat synthetic `v<pid-hex>-<counter-hex>`, unique
across concurrent process IDs and within each process. A checked 44-bit counter
fits the 21-byte bound with the maximum PID and refuses exhaustion. Physical
serial format and live host serial readback remain No evidence.

Follow-up raw-record SHA-256 (retained locally, not published captures):
- `steam-closed-baseline`: `35a48ca668e10d51f5677f0310cb9996e032f69bec486a6a164c4c9c8007b593`
- `bbf-baseline`: `39ca5b75fc62285078c629f1e52cde5a12556ec173900d35e37e00f1431bb277`
- `final`: `51a39765491ecd2d0acace6ae1267dd8c250bf488cd3d785e2d3e400666bfbd0`

A final rerun from tracked-clean step source `a2ce0b5` (including current main)
confirmed the same full masks and both-pad results, with **168** distinct valid
samples from each sensor. Steam/helper absence was checked before and after.
This rerun also removed all owned nodes and restored resource baselines.
Raw-record SHA-256: `af227e9e190182f3fdf5ce04f04e58f91f250653cda4904e82287c60661026a8`.

## Post-C6 kernel evdev follow-up (2026-11-04)

Executable source: merged C6 `ad80ae6`; kernel `7.3.0-1-mainline`, x86-64.
The local producer uses the unmodified C6 controller packages. A separate local
reader associates nodes by the complete per-creation physical label and reads
Linux input events and `EVIOCGABS`; raw output stays local. Each case creates one
controller with `VIRTUALGAMEPAD_INPUT_LAB=1`, changes one control at a time,
services reverse events throughout, neutralizes, closes twice and verifies that
all owned event/hidraw nodes disappear. No descriptor, public API or protocol
implementation was changed. No firmware/bootloader commands were submitted.

Anders installed #299. The installed rule/helper bytes match both `17936b7`
and C6, `udevadm verify` passes, and ownership/modes are root:root 0644/0755.
Every owned event node has the expected process-specific inactive lab seat,
`LIBINPUT_IGNORE_DEVICE=1`, transport group and mode 0660; ordinary-user read/write
opens pass. UHID includes a generic HID input node alongside the gamepad and
motion nodes; the generic node has no absolute axes and emitted no control events.

### Consumer isolation and kernel mode

Dolphin opened the exact layered Steam hidraw node during the first attempt;
the kernel gamepad and sensor nodes were removed. That attempt is inconclusive,
not a decoding failure. Steam and Dolphin were absent for the isolated reruns.
Opening a layered hidraw client and kernel evdev acceptance are separate cases.

With the normal kernel lizard-mode setting, newly created IBEX gamepad/motion
nodes initially emit no events, even though their capabilities and ABS ioctls are
available. A **650 ms Menu press/release** enables gamepad mode on that virtual
instance. No module parameter or system policy was changed. This is consistent
with the pinned hid-steam mode-switch handler and its input/sensor gates, not a
codec change. Both the initial-mode silence and subsequent decoding are retained.
Each later individual Menu test is shorter than the mode-switch threshold.

### Per-control decoding

All 22 digital controls produced matching press=1 and release=0 on both
realizations. The numeric Linux EV_KEY mapping is:

| Native controls | Kernel key codes, in the same order |
| --- | --- |
| A, B, X, Y | 304, 305, 307, 308 |
| QAM, right-stick click, Menu | 294, 318, 315 |
| R4, R5, right shoulder | 549, 551, 311 |
| D-pad down, right, left, up | 545, 547, 546, 544 |
| View, left-stick click, Steam | 314, 317, 316 |
| L4, L5, left shoulder | 548, 550, 310 |
| Right trigger click, left trigger click | 313, 312 |

Both realizations also produced these independent responses:

| Change | Observed EV_KEY / EV_ABS response |
| --- | --- |
| Left/right pad click, with or without touch | Key 289 / 290, press=1 and release=0 |
| Left stick X/Y | ABS 0 / 1; X unchanged, Y inverted |
| Right stick X/Y | ABS 3 / 4; X unchanged, Y inverted |
| Left/right trigger | ABS 21 / 20; positive raw values unchanged |
| Left pad touched X/Y | ABS 16 / 17, native polarity |
| Right pad touched X/Y | ABS 18 / 19, native polarity |
| Pad position while untouched | No coordinate event |
| Pad contact | No independent contact key; contact gates stored coordinates |
| Pad pressure, stick touch, grip touch | No independent evdev event |

Stick samples included -32768, -10000, 0, 10000 and 32767; the emitted lower
endpoint was -32767. Pads were moved independently through -10000, 0 and 10000.
Click and touch were tested separately, including clicks without contact.

After the mode switch, UHID motion changes arrived only on the sensor device.
Native accel X/Y/Z map to ABS X / negative ABS Z / ABS Y; native gyro X/Y/Z map
to ABS RX / negative ABS RZ / ABS RY. Positive and negative 10000 and neutral
were tested separately on all six components. Every one of the 154 control
samples also produced EV_MSC/MSC_TIMESTAMP. Reduced uinput creates no sensor
companion and motion changes produce no events, as its target restriction states.

### ABS ioctl attributes

All tuples below are `(minimum, maximum, fuzz, flat, resolution)`, excluding the
transient current value. Every axis was queried, not inferred from sysfs masks.

| UHID device / axes | Observed tuple |
| --- | --- |
| Gamepad ABS_X/Y/RX/RY (0,1,3,4) | (-32767,32767,0,0,6553) |
| Gamepad ABS_HAT0X/Y/HAT1X/Y (16-19) | (-32767,32767,256,0,1638) |
| Gamepad ABS_HAT2X/Y (20,21) | (0,32767,0,0,5461) |
| Sensor ABS_X/Y/Z (0-2) | (-32768,32768,128,0,16384) |
| Sensor ABS_RX/RY/RZ (3-5) | (-32768,32768,0,0,16) |

These match every attribute in `steam-c6-kernel-ibex-gamepad-evdev` and
`steam-c6-kernel-ibex-sensor-evdev`. The sensor property mask is 0x40
(INPUT_PROP_ACCELEROMETER); the gamepad has no property bits.
Reduced uinput matches all ten gamepad minima, maxima and flat values, but all
ten axes have fuzz=0 and resolution=0.

### Disagreements and limits

- **Reduced-uinput ABS metadata versus golden:** pad fuzz is 0 rather than 256;
  stick/pad/trigger resolutions are 0 rather than 6553/1638/5461. The native
  evdev axis contract exposes only range and flat, and the provider sets the
  remaining attributes to zero. This is the documented provider limitation.
- **Negative trigger input:** for synthetic raw -10000, UHID emits -10000 on
  ABS_HAT2Y/X despite the advertised minimum of zero. Reduced uinput clamps
  negative trigger input to zero. Positive trigger values match. The golden
  registration record specifies attributes, not clamping of incoming values;
  this observation does not change its minimum or justify a codec transform.
- **Kernel evdev versus SDL controls:** the kernel exposes no independent pad
  contact/pressure or stick/grip-touch events. Prior closed-Steam SDL evidence
  includes both-pad contact/motion; it is a different consumer surface. Pad Y
  is native in evdev; the source-backed SDL transform normalizes and inverts
  it (`steam-c6-input-scaling`). No new SDL numeric scaling measurement is claimed.
- No golden ABS disagreement was found for UHID. Initial kernel-mode gating and
  layered-hidraw consumer interference must remain explicit prerequisites.
- This is virtual kernel-consumer evidence. It does not establish physical
  calibration, output delivery, serial fidelity, or manual Steam acceptance.

Both isolated cases completed 154 one-control samples with no operation errors
or provider write failures. All owned nodes were removed, including the UHID
layered hidraw and generic input nodes. Repeated close succeeded.

Raw record SHA-256 (kept locally, no raw captures or host paths published):
- UHID initial-mode silent run: `a932f450ca867a490696c30b2dbb2d6e45b165c824dca98dfc91b3c1c626f374`
- UHID after Menu mode switch: `eeb78787a9598e2d1d80bc105e47bcfe49f73b9ab473d9bfd498bb1cf3311392`
- Reduced uinput isolated rerun: `9db537b643a4378ba90939714bab34c8a9b72a1685929ab9795e1011645b870b`
- Owner's independent reduced-uinput run: `e0fdfdb79545477131036e2f92364674c2442ae7ccd110fe74d27948ce2507b0`

The kernel/ABS cells retain `wip` with these scoped observations because the
schema has no virtual-live status and no formal research run record is adopted.
Source evidence and historical runs remain intact. The manual Steam results
below are separate owner observations, not kernel or physical input acceptance.

### Manual Steam baseline (owner screenshots, 2026-11-04)

Stable Client, Steam version `1792618285`, API `SteamClient023`.
The About dialog shows client build October 21, 2026 and web build October 19,
2026. Xbox, Switch Pro and generic Steam Input support are enabled; PlayStation
Controller Support is Enabled. Guide-button focus and guide-button chords are
enabled, idle shutdown is 15 minutes, and turning off controllers on Big Picture
exit is disabled. No controllers are listed before virtual-controller creation.
This is the pre-creation baseline, not a negative result for either realization.
The post-creation observations below distinguish both realizations from this empty-list baseline.

### Manual wired UHID: initial owner observation

With the baseline settings above, the owner created one Steam Controller 2026
using `linux.uhid.usb` in the unmodified C6 demo and left its worker running.
Steam's Controller settings did **not list** the controller. Therefore its input
test could not be opened and no per-control or output acceptance is claimed.
A read-only exact-node check found the Steam process holding the layered hidraw
node. Access/opening alone does not establish discovery, input decoding or a
reason for the missing listing. This is a new C6 result, separate from step 16's
short-reply stub. The owner removed the controller and closed the demo without errors; its owned HID nodes disappeared.

### Wired diagnostic repeat and boot-sequence review

The owner repeated creation with a local C6 demo containing opt-in diagnostic
logging only in the Steam request handler. The logger changes no request/reply,
state, descriptor or API behavior, and records only report class/ID, body length,
inner command and setting IDs. It is not part of this PR. The controller was
again **not listed**. The owner then removed it; a read-only ancestry check found
zero remaining owned UHID nodes. No Steam input test or control/output delivery
could be observed in either attempt.

The completed virtual-only diagnostic log contains 371 feature GET requests and
442 feature SET requests, all report ID 1; every SET has 63 body bytes (64 with
report ID). Inner SET command counts are 0xae:24, 0x83:364, 0x85:7, 0x8e:7 and
0x87:40. The settings requests identify key 50 seven times and key 9 thirty-three
times. No feature-2 request or output-report request was logged. These counts
include kernel initialization and userspace traffic; the logger does not identify
request origin or record completion status. They must not all be attributed to
Steam or treated as a request/acknowledgement trace.

C6's existing policy rejects unknown setting 50 atomically. This is a possible
compatibility lead, **not an established cause** of the missing listing. The
0x83 response remains structural rather than retail attribute data. No setting
acceptance, attribute values or protocol behavior was changed to chase listing.
Local diagnostic patch SHA-256:
`a5b94b48351d1af4bfe371e224a888740d2407f16edaebf68205a7c446b348c4`.
Completed diagnostic-log SHA-256:
`9d15d121f97dbdcb4a8affab7b0f9a6ec72b82c66f31c46a62722531f47b517f`.

After the virtual negative result, the owner connected the real controller and
confirmed its Steam listing/input-test availability. The screenshot identifies
it as **Steam Controller**, connection **USB**, with **Begin Test** available.
The owner also mentioned a Puck; no receiver, pairing or Bluetooth result is
inferred. A passive sysfs-only check of the real wired 28de:1302 found one HID
interface and the same 372-byte report descriptor SHA-256 as C6:
`315a037c849ef9510d222ae520bb18c1b72552d26eb116c91b0750901a5c51fd`.
No real serial, firmware attribute or physical feature query was collected.
Physical reattachment occurred during the diagnostic session, so its effect on
virtual-device rescanning is not isolated. Only the virtual codec was logged.

The owner's bootloader-first attach report remains separate from the main-stage
descriptor result. The pinned physical capture plan records bootloader add →
removal → main 28de:1302 as an operator observation, with boot-stage identity,
timing and automatic-versus-host-triggered transition still unmeasured. C6 creates
the main-stage UHID device directly, following the owner decision on #159
(comment 6492587335). Main-stage descriptor equality does not establish a matching
boot/USB lifecycle, and the virtual negative listing does not prove Steam requires
the transient stage. No pre-attach observation was running during this physical
reconnection. No bootloader experiment or emulation was performed.

### Manual reduced-uinput observation

The owner disconnected the physical wired controller and Puck, kept Dolphin
closed, left the demo without a virtual controller and confirmed Steam's list
was empty. One Steam Controller 2026 was then created using `linux.uinput`.
It was **not listed**, so no Steam input test, per-control decoding, identify or
output delivery is claimed. A read-only exact-node check found the ordinary
(non-lab) Steam Controller event node at mode 0660, group input, with an
ordinary-user read open succeeding. This ordinary desktop case is separate from
the dedicated-group input-lab kernel runs. Uinput exposes no HID feature channel;
there can be no equivalent codec feature-request diagnostic for this case.
The owner requested a fresh demo launch for the repeat; the preceding demo
instance and its uinput node were already gone. The new uinput instance was
also **not listed**. The owner then removed it and closed the demo normally
without errors. A final read-only check found no remaining virtual uinput event
nodes. An idle owned demo process remained and was stopped by the agent only
after node cleanup was verified; normal GUI close terminating every process is
not independently established. There is no listing-disappearance observation
to make when neither instance was listed. Controls, motion, identify and output remain unobserved
in Steam for both realizations; no physical equivalence is claimed.

## Recorded feature profile investigation (2026-11-04)

Owner decision #159 comment 6645358188 permits reviewed non-identifying model
attributes/settings; serials remain synthetic. The bool creation option was
approved in comment 6646269189, and comment 6646272685 subsequently made the Steam
supporting API mutable. Root/Wii/core API restrictions are unchanged.

### Fresh controller-log comparison and physical sessions

With no physical controller/Puck or SDL/Dolphin client, unchanged C6 was again
not listed. Its bounded controller-log window contained 306
`CGetControllerInfoWorkItem::RunFunc: Read failure` messages, six failed-info
results and no successful controller-info record. After virtual removal, the
real wired controller listed with no such failures, HWID 72 and firmware build
timestamp 0x6ACFF323. Raw logs remain private; account/path/serial data is omitted.

Anders installed wireshark-cli and loaded usbmon himself. The agent performed
no root operation or host-policy change. A 45-second kernel-only capture and a
90-second Steam-start/reattach/idle/close capture contain six and 218 paired
feature exchanges respectively. Their firmware attribute is 0x6ACFF323;
bcdDevice 0404 is separately a USB version marker. No update prompt or other
issue was reported. Raw captures stay outside both repositories. The physical
experiment, reviewed frames and all sanitized exchanges are in research records
`steam-d9-kernel-feature-capture`, `steam-d9-steam-feature-capture`,
`steam-d9-1302-model-attributes`, `steam-d9-steam-setting-acceptance` and
`steam-d9-feature-response-framing` at the adopted research revision.

Physical 0x83 replies contain IDs 1/2/10/4/9/11, values 0x1302/0/0x68D2F92E/
0x6ACFF323/0x48/4000. All observed GETs delivered 63 bytes including report ID 1;
C6's former attribute reply delivered 64 and had no entries. Steam accepted
sixteen setting keys, including 50=900; fourteen are outside C6's 9/48/71 allowlist.
No physical retention, ranges or effects follow from successful completion.
Serial contents and unreviewed feature bytes are withheld. All observed control
requests were paired, but a capture-drop counter was not supplied.

### Compat-on virtual retest

A local caller created one virtual wired controller with
`steam_client_compat=true`, default synthetic identity, no lab flag and continuous
reverse-event servicing. The real controller/Puck were disconnected and
SDL/Dolphin absent. The profile replays the recorded attribute frame and retains
captured setting intent; it changes no descriptor, serial identity or report
cadence. Steam logs V1 HID protocol via USB, device opened, and successful
HWID 72/FWTimestamp 0x6ACFF323 with **zero controller-info failures**.

Anders nevertheless reported **not listed**, both initially and after a fresh
neutral report was sent following Steam's open. No Steam input test could be
opened. Per-control, motion, Identify/rumble and gameplay acceptance remain
unobserved. The controller retained eight Pulse output observations during
opening; its last retained Pulse had side 1 and zero on/off/count parameters.
This proves request receipt only, not an owner-triggered Identify operation or
an actuator effect. Provider diagnostics ended with zero write failures.
The caller closed twice and every owned HID/input node disappeared.

### Differences after the first replay attempt

The profile establishes progress in the controller-info handshake, not the sole
cause of UI discovery. Physical DC/E2 SET and ED SET/GET complete successfully
where the codec still returns Unsupported; physical C1 SETs stall. Those
semantics are not invented. Synthetic serial behavior remains intentional.
No new unmeasured successful reply was added to chase listing.

One possible next hypothesis is Steam needing USB-level enumeration
or metadata absent from UHID, to investigate under #168. The separate Session E
bootloader-first hypothesis remains unmeasured. Neither is proven by this result,
and remaining feature differences must be considered before assigning causality
to transport. No bootloader emulation, USB provider, receiver or Bluetooth work
was started. Findings are on #159 comment 6646427928. The steam-client cell stays
`wip` with this negative result; physical feature evidence is not virtual listing
or controls acceptance.

### Listing after DC/E2 acknowledgement

The same opt-in attribute/settings profile was run with only the captured DC/E2
SET acknowledgements added. ED replay, a caller serial override, and global
63-byte GET framing were **not** in the running binary. After an initial
negative answer, Anders corrected the result to **listed**, displayed name
**Steam Controller**, with **Begin Test** opening. A read-only target check
found no other wired 1302 and no physical controller during this virtual run.
The log window has zero controller-info failures and two successful info records.
This establishes listing in that run, not that DC/E2 alone caused the change:
a KDE Remote Control permission dialog for Steam was also shown, and its
selection/timing relative to listing has not been established.

Manual control acceptance is unresolved. The initial held-button sequence
closed the tester on B. A separate short synthetic A tap was reported as
“B then A”. These observations are not passing control results. The owner
requested a labelled physical input capture before changing the encoder.
The virtual caller was neutralized and closed twice, with zero write failures;
all owned HID nodes disappeared before the real wired controller was attached.

Running-binary SHA-256:
`c1b2d64414a8d582ec65f6a7a8f5d97f4e16f998713de153018c7fd215388811`.
Sanitized-window source-log SHA-256 (raw remains private):
`1eecce8ae1cc832806c5ba32d86fdf4eb22c071fe3fcfce0f9e45c1a9fddff2f`.

### Review follow-up: isolate remaining protocol differences

Research PR #101 review requested five smaller checks before assigning the
remaining discovery failure to USB topology: acknowledge the captured DC/E2
SETs, review/replay the one-byte ED reply, compare C1 failure status, try a
synthetic serial with the physical character pattern, and match 63-byte GETs.
The listing result above preceded the ED/serial/global-framing trials. Input
acceptance remains unresolved; the physical reference capture is in progress.

Private capture reinspection corrects the ED summary: the three GET replies
declare lengths 1, 24 and 0 after SET lengths 24, 9 and 11. Only the first
meaningful byte (`00`) is reviewed for replay. Opaque tails and other ED bodies
remain withheld; compat zero padding must not be called exact physical replay.
DC/E2 bodies are respectively `01 02` and `01 20`, both declared length 2.
The opt-in profile acknowledges those exact bodies without retaining state.

C1 does not fail identically: the physical trace has 100 USB EPIPE
completions (`-32`). The generic provider encodes Unsupported as `-95`, but
Linux UHID converts **any nonzero** SET reply error to EIO. A direct ioctl on
only the owned virtual node confirmed errno 5 (`EIO`), without physical
feature injection. A shared Stall enum variant would not fix this, so the
proposal was withdrawn and the shared API remains unchanged. See
[Linux v7.3-rc1 UHID](https://github.com/torvalds/linux/blob/v7.3-rc1/drivers/hid/uhid.c#L288-L291).
The deterministic fake-provider sequence checks explicit C1 failure on each
poll and cleanup; it does not claim to reproduce a USB-level STALL.

The same owned-node check requested a 64-byte attribute GET and received
exactly 63 bytes, with prefix `01 83 1e`. This confirms UHID supports the
observed physical response length; matching other GET replies is a separate
profile change yet to be tested.

The physical serial strings have 13 characters, a declared inner length of 20,
and selector-1 character classes `UUUDDDDDDDUUD` (uppercase ASCII / decimal
digit). Actual characters remain private. A caller-set made-up serial can test
that pattern without changing the public API or copying physical identity.

Local retest digests (raw artifacts not published):
- controller-log window: `40580043fae8325541459f9bdf783bb16edcf267336f74980aa421bc40da4a00`
- local probe source: `c03a757528b6f39c2ce0526a39a5ab9eab30a856bf32f33ceb829097d3f61df6`
- local probe binary: `5df8b8b0c808cf8e0dbdaf32f1f5d74256e0b9e0c9875b3c10c12ddb3706c695`


### Native reference and manual Steam checkpoint

The owner completed labelled real wired input captures, plus Steam Ping and
separate brief SDL rumble channels. [Research PR #105](https://github.com/anderstvoss/controller-protocol-research/pull/105)
proposes `steam-d9-native-input-output-reference` and linked fixtures preserving selected
non-identifying input/output packets. Physical button/axis/touch offsets agree
with the encoder. Motion fields froze in the Steam-only motion batch; enabling
sensors through the provided SDL build produced changing accel/gyro/quaternion.
This is a raw reference with documented missed actions, incidental contacts and
uncalibrated motion, not a universal firmware or force-calibration claim.

The owner later showed per-side Steam **Trackpad Lockout** settings. Treat the
observed same-side pad suppression as configuration-dependent, not an immutable
hardware rule. No automatic stick/pad exclusion was added to the encoder.

A fresh virtual compat-on instance (attributes/settings, DC/E2 and reviewed ED
reply; default synthetic identity) listed and opened the input tester. Periodic
streaming stayed **off** throughout this checkpoint. Desktop bindings initially
opened the keyboard on X and entered space on Y, as also seen with the physical
controller. The owner replaced the desktop profile with the generic **Gyro**
profile and verified physical inputs were reachable there; virtual X/Y/B then
registered without keyboard interference. A is also confirmed. The UI latches
press history; release is separately verified from reports on the exact owned
hidraw node held by Steam. No account/layout identifier is recorded.

Manual observations on this instance:

- D-pad up/down/left/right, both shoulders and both stick clicks registered in
  order. Rear L4/L5/R4/R5 reports match the physical bits, but the GUI side/order
  labels were unclear; retain that qualification for later independent demo use.
- Both triggers were confirmed right then left at `0 -> 32767 -> 0`.
- Both sticks moved left/right/up/down and centered. The first right-stick
  observation was missed; repeats with touch and then without touch worked.
  There is no demonstrated requirement to force stick touch for axis output.
- Stick/grip touch indicators were observed. Reverse-view grip indicators are
  visible; their exact GUI presentation remains for the owner's independent
  demo check. One decoded binary grip-touch flag exists per side; two drawing
  elements do not establish a second wire axis.
- Both pads displayed touch/pressure at 1000 and 16000, returned to zero, and
  registered the higher-pressure press indication. Both pad X/Y tests matched
  `(-20000,0)`, `(20000,0)`, `(0,20000)`, `(0,-20000)` with six-second holds.
- View, Options/Menu and QAM were individually detected with the tester staying
  open. Steam was last and navigated to Steam's main interface, which the owner
  accepted as receipt; no latched Steam-button checkbox result is claimed.
- **Gyro remains unresolved.** In the separate gyro view, raw X/Y/Z pulses of
  +1024 (about 62.5 degrees/s under the pinned scaling) produced no visible
  change. The owned-node capture contains the exact three signed gyro vectors
  and zero vectors on release. The experiment used change-driven reports;
  sensor mode, fresh-sample pacing and the UI's interpretation need isolation.

The virtual controller is still running at this checkpoint; final removal and
shutdown for this instance have not yet been recorded. Earlier owned-node cleanup
results remain scoped to their own runs. No gameplay or complete motion acceptance
is claimed, and individual discovery changes remain causally unisolated.


### Gyro follow-up: valid pose (2026-11-05)

The owner showed the live Gyro Calibration view with software calibration
selected. Repeating raw +1024 on each gyro axis with continuous reports still
showed zero. A further repeat advanced the caller's sensor timestamp throughout
each hold; owned-node observations verified 24 distinct timestamps per active
vector, but the owner again reported zero on all axes.

With acceleration `(0, 0, 16384)` (approximately 1 g) and quaternion
`(32767, 0, 0, 0)` supplied alongside the gyro samples, the owner observed:

- raw X +1024: positive pitch, approximately 62.5 degrees/s;
- raw Y +1024: positive roll, approximately 62.5 degrees/s;
- raw Z +1024: negative yaw, approximately 62.5 degrees/s.

This is a positive manual gyro observation with a nondegenerate pose, not a
calibration or accuracy measurement. Acceleration and quaternion were introduced
together; their separate necessity is unisolated. The successful trial also used
continuous reports, so change-driven delivery with this pose is a separate check.
No production clock, input cadence, motion default or exclusivity rule changed.

The subsequent valid-pose X-only trial with continuous streaming **off** also
showed positive pitch. This confirms visible gyro delivery through the existing
change-driven path for that input; a production cadence change is not justified
by these observations. Return-to-zero packets were sent; no separate visual
release confirmation was requested for the Y/Z observations.


### Reverse outputs and final cleanup (2026-11-05)

One owner-triggered virtual Identify/Ping produced output `85 05 0c 00`, exactly
matching the physical Ping reference. The API retained
`Script { side: 5, script: 12, gain_db: 0 }`; raw selector 5 is preserved without
inventing actuator semantics. No physical sound or vibration is claimed from a
virtual device.

After the owner closed Steam, the same pinned SDL helper completed both
250-ms rumble API trials at 4096/65535 and both explicit zero stops. The virtual
provider observed seven `0x80` writes for left speed 4096, seven for right speed
4096, and two zero stops, matching the physical reference. The final retained
rumble speeds were zero. Separate kind-1 output writes around Steam shutdown are
not attributed to the helper's labelled trials. The helper exited successfully.

The virtual instance then neutralized, committed, and closed twice. Diagnostics
reported Closed, 33896 frames sent, 3803 reverse events, zero write failures and
no last error. Its owned hidraw/input nodes were absent after shutdown. This
closes the instance previously described as still running; manual GUI label
qualifications and the unisolated discovery/pose requirements remain.

Streamed audio and MIDI were not tested. The physical Ping chirp and its matched
script request establish neither a streaming path nor MIDI semantics.

The implementation pin adopts merged research main, including feature evidence
from #101. The newer input/output reference in #105 is a separate review; no
unmerged fixture is required by the production attribute/settings generator.
