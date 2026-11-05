# Steam Controller 2026 wired

C6 covers direct wired USB `28de:1302`. The Steam supporting module opens
the full wired UHID realization and a reduced uinput gamepad.

The approved Steam supporting API has its own reviewed snapshot,
[`api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json`](api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json),
checked by `python3 scripts/check-alpha-api.py` alongside the root and Wii APIs.

Research pin: `fc6964ac8799f0b7a38ffb9526361450f0b1b87a`, including the merged
fresh physical descriptor research PR 82 (`aeac7a5`), July notes and PR 81
research leads. The generated fixture
manifest records the exact revision and both fixture/record file digests.
Every `steam-c6-*` fixture used here is synthetic consumer-source evidence. The
`steam-s3-1302-report-descriptor` fixture is physical, imported without byte changes.
Kernel and SDL may share Valve source lineage; agreement alone is not faithful.

## Open gates

- **14a complete:** family/state metadata and the exact wired `28de:1302`
  descriptor are adopted. The generator verifies the 372-byte raw SHA-256
  `315a037c849ef9510d222ae520bb18c1b72552d26eb116c91b0750901a5c51fd`,
  imports the public hex fixture and writes a text Rust byte array for compile-time
  inclusion (the repository forbids binary artifacts). Rust tests bind those bytes to the hex fixture and parse every
  declared report ID/length. No descriptor item is edited, reordered or synthesized.
- **14a supporting API approved:** [owner decision](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6492166698)
  permits `SteamController2026State`, `SteamController2026Button`,
  `SteamTrackpadState` and `SteamMotionSample` in the supporting module.
  A [follow-up owner decision](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6492492101)
  approves its separate supporting snapshot/check, mirroring Wii's. The checker
  records the four native types, the supporting ID constant, and state
  declarations, fields, variants, methods and traits;
  changes fail until explicitly reviewed and refreshed with `--write` using
  fresh documentation builds. `--doc-root` infers the Steam supporting docs next
  to the root docs; `--steam-doc-root` can select them explicitly. Controller,
  surface and output types and a controller constructor remain for their authorized step.
  There are no root exports, creation-options type or broker changes. This
  [owner decision on the follow-up](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6502745647)
  keeps exactly these public items: `SteamController2026State`,
  `SteamController2026Button`, `SteamTrackpadState`, `SteamMotionSample` and
  `STEAM_CONTROLLER_2026_ID`, with the state methods including
  `SteamController2026State::input_topology()`. The `FamilySpec` constant stays
  internal wiring in `gr_controller_wire::registry::STEAM_CONTROLLER_2026`;
  the supporting module does not re-export it. Root and Wii
  snapshots remain byte-identical. For a Steam-only `--write` refresh, verify
  that the root and Wii snapshot diffs are empty before committing.
- **14c API:** the [owner decision](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6616502032)
  approves the controller, constructor, options constructor and retained haptic types.

## Realization differences

| Capability | Wired UHID | Reduced uinput |
| --- | --- | --- |
| Gamepad | Exact descriptor and compatibility `0x42` encoding | Kernel gamepad name, USB identity, keys, axis codes/ranges/flat and conventional FF_RUMBLE from `steam-c6-kernel-ibex-gamepad-evdev` |
| Axis fuzz and resolution | Determined by the bound kernel driver | Zero in the provider. Its axis specification supports range and flat |
| Motion | Native motion through `0x42` | No companion motion node. The provider cannot declare the accelerometer property or timestamp event |
| Touch | Native pad coordinates, click, pressure and touch flags | Pad click and contact-gated coordinates. The kernel gamepad mapping has no independent pressure, stick/grip touch or pad-contact key |
| Host output | Retained vendor haptics, reports `0x80`–`0x85` | Conventional rumble via reverse events. There is no vendor haptic payload for the six retained slots |
| Unit serial | Compatibility PID/counter feature string. Physical format: No evidence | No HID feature channel or evdev unique-ID setter |

The default unit serial is a synthetic `v<pid-hex>-<counter-hex>` label,
allocated once per instance. The separated PID makes concurrent processes distinct;
a checked 44-bit counter makes instances within a process distinct. Its maximum
encoded length is 21 bytes; exhaustion refuses creation instead of wrapping or
truncating. PID reuse after process exit is not a persistent identity guarantee. Caller values must contain 1–21 printable ASCII bytes. The limit
comes from Linux v7.3-rc1 [hid-steam.c lines 399–414 and 667–685](https://github.com/torvalds/linux/blob/cee9395acd8043be0644b25c34bfa86623f2b935/drivers/hid/hid-steam.c#L667-L685).
The kernel uses a 22-byte buffer with NUL termination and does not validate
a character class. Printable ASCII is this project's compatibility policy.
Pinned [SDL Triton lines 453–471](https://github.com/libsdl-org/SDL/blob/e0232c5b9d972ad394d597ce858185cd15a0f1a7/src/joystick/hidapi/SDL_hidapi_steam_triton.c#L453-L471)
register wired controllers without querying the feature serial, so that driver
imposes no additional wire bound.

## Demo editor

The demo includes a native Steam editor for both realizations: buttons, sticks,
trigger axes and clicks, both trackpads, independent capacitive contacts and pad
pressure, and raw motion. Edits use the bounded worker queue and native state;
per-target restrictions remain visible. No additional public API is introduced.

## 14d acceptance and timing

The [C6 D9 report](C6_D9_ACCEPTANCE.md) records the new N1/N4 measurements,
separately qualified SDL rerun, post-C6 kernel decoding and ABS ioctl results,
manual Steam procedure and Puck feasibility note. The manifest retains
source-only evidence levels and explicit missing live cells; none is promoted
to physical acceptance.

## D9 wired acceptance

The shared acceptance runner used one cycle per realization on Linux
`7.3.0-1-mainline`, with the private SDL `e0232c5` build and Steam HIDAPI
enabled for the UHID run.

- Both realizations exposed associated nodes and removed them on close.
- UHID bound to `hid-steam`. Sysfs showed the golden gamepad and motion-node
  names, USB IDs, key and absolute-axis bits, and sensor property and
  timestamp bits. The kernel gamepad exposed additional FF bits alongside
  FF_RUMBLE. The uinput gamepad matched its key and axis masks and exposed
  conventional FF_RUMBLE.
- SDL opened the associated hidraw device, observed neutral state, button
  and axis changes across its 22-button and six-axis masks, touch events,
  and changing accelerometer and gyro samples. This is SDL decoding evidence
  for the scripted `0x42` stream.
- Direct evdev opens returned permission denied. Kernel `0x42` event decoding,
  ioctl-level axis attributes, output delivery and Steam client behavior
  have no acceptance evidence from this run.

The isolated 2026-11-04 follow-up in that report now verifies direct evdev opens,
per-control decoding and every ABS attribute. UHID requires a long Menu
press/release to enable the kernel's per-instance gamepad mode; layered hidraw
clients such as Dolphin must be absent for kernel-node testing. All UHID ABS
attributes match the golden gamepad/sensor records. Reduced uinput decodes its
representable controls but retains zero fuzz/resolution, an explicit provider
limitation. Historical EACCES results above remain unchanged. No physical tier
acceptance follows from these kernel observations. In the separate owner manual
checks, neither wired UHID nor reduced uinput was listed by Stable Steam
1792618285, including repeat creation; cleanup passed. No Steam input/output
acceptance is claimed. The report lists the sanitized virtual feature-request
summary and its limits; no protocol behavior was changed to chase listing.

## Faithful versus compat behaviour

The descriptor bytes, declared report shapes and single USB-interface topology
are **faithful (fresh physical capture)**, grounded in
`steam-s3-1302-report-shapes` and `steam-s3-1302-usb-interface`. The capture went
through VM USB passthrough. It says nothing about the **bootloader-first USB
attach sequence** the owner observes; boot sequence and firmware tracking remain
open. No firmware value is recorded. A USB `bcdDevice` value, if discussed later,
is a version marker only, not proof of firmware.

Field semantics and feature/error reply policy remain **compat**. Haptic effects,
physical cadence and boot behavior remain compat hypotheses or **No evidence**.
Descriptor declarations do not establish that reports are transmitted, commands
have physical effects or consumers accept a virtual device. July notes remain
historical/raw-not-revalidated corroboration; the fresh fixture is separate.

| Behaviour | Basis and choice | Experiment to promote or retire |
| --- | --- | --- |
| Primary input `0x42`, 54 bytes | Length: **faithful (fresh physical capture)**, `steam-s3-1302-report-shapes`; corroborates `sc2-july-wired-input-42`, **physical (historical capture, raw not revalidated)**. Field semantics: **compat**, `steam-c6-input-42`, `steam-c6-disagreement-47-kernel/sdl`; native signed LE fields, no consumer-axis transforms | Fresh wired input capture plus independently decoded kernel/SDL controls |
| Deliberate alternate `0x45`, 46 bytes | Declared length **faithful (fresh physical capture)**; fields **compat** `steam-c6-input-45`, private encoder only | Per-transport report histogram and differential input decoding |
| SDL-only diagnostic `0x47`, 46 bytes | **Not declared** by the fresh wired descriptor (`steam-s3-1302-sdl47-descriptor-conflict`). `steam-c6-input-47` is a source decoder, not physical declaration. Wired encoder rejects it; normal protocol only emits 0x42 | Version-pinned SDL timestamp/trackpad capture and decoding |
| Buttons, pad position/pressure, stick/pad/grip touch, IMU | `steam-c6-input-buttons`, `steam-c6-input-scaling`; raw native fields retain signedness; touch and pressure are independent | One-control-at-a-time wired capture, physical sensor and pressure correlation |
| Optional battery `0x43`, 15 bytes | Declared length **faithful (fresh physical capture)**; field meaning **compat** `steam-c6-battery-43`; encoder accepts caller data, no automatic status or made-up voltage/cadence | Physical wired battery/status capture |
| Feature 1 SET/GET, 64-byte host buffer | `steam-c6-feature-kernel-exchange`, `steam-c6-feature-registration`; serial returns virtual identity and attributes retain the empty structural shape | Physical GET readback; step 16 good/short/missing optional-GET host gates |
| Feature 2 on wired | Declaration **faithful (fresh physical capture)**: 63 body bytes (`steam-s3-1302-feature2-declaration`). Wired requests/replies remain **No evidence** (`steam-c6-no-wired-feature2`); GET and SET explicitly complete Unsupported | Separately authorized physical feature transaction capture; no fabricated reply |
| Kernel and SDL open orders | `steam-c6-feature-kernel-lizard`, `steam-c6-feature-sdl-lizard`, `steam-c6-feature-imu`; neither requires the other's GETs | Both opening orders with kernel bound/unbound, with settings readback |
| Retained lizard/mapping/IMU/watchdog intent | Same feature claims; unknown initial/default values remain unknown; restore-defaults discards assumed settings rather than fabricating values | Physical initial/default-settings readback and mode transition trace |
| Haptics `0x80` through `0x85` | **faithful (fresh physical capture)** declared body lengths: `0x80=9`, `0x81=7`, `0x82=3`, `0x83=9`, `0x84=8`, `0x85=3`, per `steam-s3-1302-report-shapes`; wire length adds the ID byte. Historical corroboration: `sc2-july-wired-descriptor-report-shapes`, **physical (historical capture, raw not revalidated)**. Field meanings/retained intent remain **compat** (`steam-c6-output-80-81`, `steam-c6-output-82-85`), no input ACK | Fresh host-write bus capture correlated to observed physical actuators |
| Unknown feature `0x87` setting IDs | Default **compat rejection** is atomic. The opt-in Steam client profile additionally retains the captured accepted keys, without asserting their effects; see the profile below and `steam-d9-steam-setting-acceptance`. | Settings readback, persistence and physical effects remain unproven |
| Output `0x86` | **Unsupported** semantics; `steam-s3-1302-report-shapes` declares 3 body bytes, **faithful (fresh physical capture)** shape only | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, `0x44` return traffic and actuator response |
| Output `0x87` | **Unsupported** semantics; same fresh capture declares 63 body bytes, **faithful** shape only. This is an output ID, distinct from feature 1 command `0x87` | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, buffer/flow control and actuator response |
| Output `0x88` | **Unsupported** semantics; same fresh capture declares 63 body bytes, **faithful** shape only | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, buffer/flow control and actuator response |
| Output `0x89` | **Unsupported** semantics; same fresh capture declares 63 body bytes, **faithful** shape only | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, buffer/flow control and actuator response |
| Raw rumble gain bytes | `steam-c6-disagreement-gain-kernel/sdl`; all byte values accepted without signed reinterpretation, kernel 2/0 and SDL 0/0 tested | Physical per-motor gain sweep; controlled tests use shared nonnegative range |
| Unknown feature commands | **Compat kernel-path fit, not device behaviour**: explicit unsupported SET completion, followed by a descriptor-sized malformed command-echo GET if attempted; `steam-c6-feature-kernel-exchange` short-payload `-EPROTO` path | Step 16 optional-GET error injection and physical unknown-command readback |

## Judgement calls and No evidence

- The attachment explicitly permits 14b first while 14a's descriptor is pending.
  Native state and fixture pin adoption accompany that work. The approved
  supporting API is exposed, and 14a now adopts the merged physical descriptor.
  The report/feature-exchange objective of 14b remains independently tested.
- Mapping restoration (`0x85`) and setting restoration (`0x8e`) are separate.
  The latter retains unknown defaults, not guessed lizard/IMU/firmware values.
- Malformed settings reject atomically, and every SET supersedes the prior
  feature 1 exchange; an invalid command cannot leak an earlier serial reply.
  Unknown setting IDs keep the existing atomic rejection instead of claiming
  that successful storage alone fulfils undocumented Steam-client settings.
  The later opt-in profile accepts only captured setting keys, retaining intent
  without assigning unproven semantics; the default path keeps this rejection.
- The old short error echo violated the fresh feature body size. It now returns
  63 body bytes with an intentionally impossible message length, preserving the
  kernel `-EPROTO` fit without a short HID frame. This is compat error framing,
  not measured device behaviour. The regression failed at 2 versus 63 bytes
  before the fix, and passes with the descriptor-sized response.
- The normal stream emits only `0x42`. Battery and alternate reports are explicit
  encoders. The D9 physical reference observed an approximately 4 ms input
  completion interval; that observation does not establish a required virtual
  timer. The protocol uses explicit state updates, and manual Steam input and
  valid-pose X gyro worked with periodic streaming off.
- There is **No evidence** for rumble timeout/auto-stop. No such timer exists.
  `steam-c6-sdl-rumble-resend-40ms` describes host resend; its roughly 50 ms
  timeout comment needs a physical lapse/resend sweep. Kernel 20 Hz coalescing
  and SDL 4032 microsecond sensor interval are consumer assertions, not device
  scheduling requirements.
- There is **No evidence** for wired feature 2, `0x44` ACK/flow control, streaming
  semantics for outputs `0x86` through `0x89` (their declared lengths are known),
  identifying serial/calibration data, firmware behaviour or lizard watchdog
  duration. Only the explicitly captured non-identifying attributes, feature
  replies and output requests described below are adopted. Other responses are refused
  or left unknown. No wired `0x79` is emitted.
- July notes are now imported through research PR 79. The two cited July
  claims support only historical observed input length and descriptor-declared
  report shapes. **The historical encoder layout is not evidence**: its Menu
  and View bits are swapped relative to consumers. Keep Menu at bit 6 and View
  at bit 14 (`steam-c6-input-buttons`); field semantics stay compat. HHD review
  is still missing. Fresh descriptor bytes now come from the separate merged
  experiment/fixture, not the historical encoder or a reconstructed descriptor.
- D9 Sessions 1 and 3 captured the wired kernel/Steam feature exchange; the
  profile below adopts only the reviewed fields. The physical input/output
  follow-up captured Ping and brief SDL rumble, not streamed audio or MIDI.
  **steam-s3-wired-output-streaming** remains a planned session: streaming
  commands, buffer/flow control and physical output effects need a separately
  authorized bidirectional capture. No MIDI interface or command semantics are
  established by the Ping chirp. These runs do not authorize an audio/streaming
  implementation.
- Puck `1304`, receiver `1305`, Bluetooth `1303`, desktop companions, LEDs and
  audio remain outside S3. No broker or host policy changes are needed.

## Deterministic checks

`cargo test --locked -p gr-curated-controllers --lib steam_controller_2026`
compares every input fixture, all nine consumer feature requests, both optional
replies, all six distinct-field haptic fixtures, raw gain domains, malformed
lengths, report classes, retained outputs, sequence wrap, stop/restart and two
independent fake sessions with per-cycle request completion and idempotent close.
Fixture regeneration also checks independent fixed lengths and hashes.

## Opt-in recorded Steam client profile

`SteamController2026CreationOptions::steam_client_compat` is a per-instance bool,
default **false**. Enable it through `create_steam_controller_2026_with_options`
with `SteamController2026CreationOptions { steam_client_compat: true,
..Default::default() }`. The caller must continue servicing the controller.

On wired UHID, the profile returns the exact recorded 63-byte attribute reply
from `steam-d9-1302-model-attributes`, scoped to HWID 0x48 and firmware build
0x6ACFF323, and accepts the sixteen setting keys in
`steam-d9-steam-setting-acceptance` alongside the existing known settings.
Captured writes establish transport acceptance only. The private model retains
last accepted intent atomically, including last-write-wins for duplicate keys;
restore-settings invalidates known intent without inventing factory defaults.
Malformed settings and unreviewed keys still fail atomically. Serial identity
remains synthetic or validated caller-set; its reply behavior is unchanged.
The profile also acknowledges the exact observed DC/E2 two-byte SET bodies
without retaining state. For the observed ED SET declaring length 24, it returns
the reviewed one-byte `00` value in a 63-byte report with fresh zero padding.
Other ED forms remain unsupported, and opaque reply tails are never replayed; the three
physical replies have different declared lengths. These choices cite
`steam-d9-feature-response-framing` and assert no command semantics.
C1 remains an explicit failure. Linux UHID converts all nonzero SET errors
to EIO, so it cannot reproduce the physical USB EPIPE/STALL exactly; see the
D9 report for the direct owned-node ioctl result and fake-provider regression.
The descriptor and supported input/output reports are unchanged. Uinput has no
HID feature channel; the option has no effect on its controls or outputs.

This is a firmware-scoped compat profile, not a retail identity or a promise of
Steam listing across arbitrary clients. The initial attribute/settings-only
attempt corrected the controller-info failure but remained unlisted. Later
compat runs with DC/E2 acknowledgement listed and opened the input tester.
The current profile also includes the reviewed ED reply. Manual buttons, axes,
pad position/pressure and touch observations are recorded in the D9 report,
with GUI qualifications and a changed desktop-profile test condition. With acceleration `(0, 0, 16384)` and quaternion `(32767, 0, 0, 0)`,
gyro +1024 showed +pitch/+roll/-yaw at approximately 62.5 degrees/s; X also
worked with periodic streaming off. All-zero pose trials showed zero, so the
separate acceleration/quaternion requirements remain unisolated. Virtual Ping
and both SDL rumble channels matched the physical reference. No calibrated
motion or gameplay acceptance is claimed. The profile does not hard-code Steam's
configurable Trackpad Lockout policy or add periodic input streaming.
See [the D9 report](C6_D9_ACCEPTANCE.md) for exact scope and remaining differences.

In the demo, select Steam Controller 2026 and enable **Steam client compatibility**
in Advanced options before creating the controller. The checkbox defaults to off
and applies to newly created instances. It has no HID feature effect on uinput.
