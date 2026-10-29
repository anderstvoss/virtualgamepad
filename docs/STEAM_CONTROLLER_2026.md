# Steam Controller 2026 wired: S3 reports and supporting state

Scope: checkpoint C6 steps 14a and 14b, direct wired `28de:1302` only.
The public `gr_curated_controllers::steam_controller_2026` supporting module
exposes native state, button, trackpad and motion types. The report/feature/output
protocol remains private and deterministically tested. A private compiled staging catalog
records the family and two planned targets with explicit 14c/step 16 restrictions;
the advertised registry is unchanged.
The exact physical wired descriptor is compiled in, but there is no provider,
driver or controller creation path; this cannot create a virtual Steam Controller.
Ordinary-root and Wii supporting API snapshots remain unchanged.
The approved Steam supporting API has its own reviewed snapshot,
[`api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json`](api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json),
checked by `python3 scripts/check-alpha-api.py` alongside the root and Wii APIs.

Research pin: `7a887f43fe4ce25a652c8381a5f962bb2dc927b0`, including the merged
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
  records the native types, supporting family metadata constants, and state
  declarations, fields, variants, methods and traits;
  changes fail until explicitly reviewed and refreshed with `--write` using
  fresh documentation builds. `--doc-root` infers the Steam supporting docs next
  to the root docs; `--steam-doc-root` can select them explicitly. Controller,
  surface and output types and a controller constructor remain for their authorized step.
  There are no root exports, creation-options type or broker changes. This
  scope now includes `STEAM_CONTROLLER_2026`, `STEAM_CONTROLLER_2026_ID` and
  `SteamController2026State::input_topology`; these supporting additions are
  named for review and included in the refreshed Steam snapshot. Root and Wii
  snapshots remain byte-identical. For a Steam-only `--write` refresh, verify
  that the root and Wii snapshot diffs are empty before committing.
- **14c/14d and C6 checkpoint PR:** held for step 16 on the owner's kernel
  7.3 or newer machine. No kernel, SDL, Steam, physical or timing acceptance
  follows from these deterministic tests. An optional SDL run on the 6.12
  generic HID path still needs the owner's separate OK.

No acceptance manifest or cell is added. The private staged registry contains
metadata only; public registry advertisement remains 14c work. Existing registry,
demo and test-runner assertions continue to require constructor adapters for
every advertised family and are unchanged. Full/reduced staged tiers are planned
levels, not completed realizations. The internal metadata contract requires only
the definition contract, without requiring a driver implementation. This keeps
14a registration separate from 14c and adds no fake provider or constructor.

The native topology covers buttons, sticks, digital/analog triggers, pads,
raw IMU, six D2 `CapacitiveTouch` kinds and two `PressureButton` kinds. Coordinate
ranges/extents are source field domains, not physical resolution or calibration.
Actual uinput mapping/limitations remain 14c work; native schema registration
must not be mistaken for an evdev realization.

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
| Primary input `0x42`, 54 bytes | Length: **faithful (fresh physical capture)**, `steam-s3-1302-report-shapes`; corroborates the historical July claims. Field semantics: **compat**, `steam-c6-input-42`, `steam-c6-disagreement-47-kernel/sdl`; native signed LE fields, no consumer-axis transforms | Fresh wired input capture plus independently decoded kernel/SDL controls |
| Deliberate alternate `0x45`, 46 bytes | Declared length **faithful (fresh physical capture)**; fields **compat** `steam-c6-input-45`, private encoder only | Per-transport report histogram and differential input decoding |
| SDL-only diagnostic `0x47`, 46 bytes | **Not declared** by the fresh wired descriptor (`steam-s3-1302-sdl47-descriptor-conflict`). `steam-c6-input-47` is a source decoder, not physical declaration. Wired encoder rejects it; normal protocol only emits 0x42 | Version-pinned SDL timestamp/trackpad capture and decoding |
| Buttons, pad position/pressure, stick/pad/grip touch, IMU | `steam-c6-input-buttons`, `steam-c6-input-scaling`; raw native fields retain signedness; touch and pressure are independent | One-control-at-a-time wired capture, physical sensor and pressure correlation |
| Optional battery `0x43`, 15 bytes | Declared length **faithful (fresh physical capture)**; field meaning **compat** `steam-c6-battery-43`; encoder accepts caller data, no automatic status or made-up voltage/cadence | Physical wired battery/status capture |
| Feature 1 SET/GET, 64-byte host buffer | `steam-c6-feature-kernel-exchange`, `steam-c6-feature-registration`; serial and attributes return empty structural fixture shapes | Physical GET readback; step 16 good/short/missing optional-GET host gates |
| Feature 2 on wired | Declaration **faithful (fresh physical capture)**: 63 body bytes (`steam-s3-1302-feature2-declaration`). Wired requests/replies remain **No evidence** (`steam-c6-no-wired-feature2`); GET and SET explicitly complete Unsupported | Separately authorized physical feature transaction capture; no fabricated reply |
| Kernel and SDL open orders | `steam-c6-feature-kernel-lizard`, `steam-c6-feature-sdl-lizard`, `steam-c6-feature-imu`; neither requires the other's GETs | Both opening orders with kernel bound/unbound, with settings readback |
| Retained lizard/mapping/IMU/watchdog intent | Same feature claims; unknown initial/default values remain unknown; restore-defaults discards assumed settings rather than fabricating values | Physical initial/default-settings readback and mode transition trace |
| Haptics `0x80` through `0x85` | **faithful (fresh physical capture)** declared body lengths: `0x80=9`, `0x81=7`, `0x82=3`, `0x83=9`, `0x84=8`, `0x85=3`, per `steam-s3-1302-report-shapes`; wire length adds the ID byte. Field meanings/retained intent remain **compat** (`steam-c6-output-80-81`, `steam-c6-output-82-85`), no input ACK | Fresh host-write bus capture correlated to observed physical actuators |
| Unknown feature `0x87` setting IDs | **Compat rejection**: the whole SET completes `Unsupported`, no partial settings retention or opaque acceptance. Known consumer keys are 9, 48 and 71; other meanings are unproven. No creation-time toggle | Planned **steam-s3-steam-client-settings** run, Steam-only and SDL/Steam opening orders on the step 16 x86-64 host: capture exact feature writes/completions and whether rejection prevents open/input/haptics; propose opaque acceptance only from that evidence |
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
  Opaque acceptance and a creation-time compat toggle are deferred; neither is
  required to complete 14b for the pinned kernel/SDL requests.
- The old short error echo violated the fresh feature body size. It now returns
  63 body bytes with an intentionally impossible message length, preserving the
  kernel `-EPROTO` fit without a short HID frame. This is compat error framing,
  not measured device behaviour. The regression failed at 2 versus 63 bytes
  before the fix, and passes with the descriptor-sized response.
- The normal stream emits only `0x42`. Battery and alternate reports are explicit
  encoders. There is **No evidence** for an automatic wired cadence; the private
  protocol uses explicit state updates, without inventing a timer.
- There is **No evidence** for rumble timeout/auto-stop. No such timer exists.
  `steam-c6-sdl-rumble-resend-40ms` describes host resend; its roughly 50 ms
  timeout comment needs a physical lapse/resend sweep. Kernel 20 Hz coalescing
  and SDL 4032 microsecond sensor interval are consumer assertions, not device
  scheduling requirements.
- There is **No evidence** for wired feature 2, `0x44` ACK/flow control, streaming
  semantics for outputs `0x86` through `0x89` (their declared lengths are known), retail serial/attributes/calibration/firmware,
  lizard watchdog duration or real-device command responses. These are refused
  or left unknown. No wired `0x79` is emitted.
- July notes are now imported through research PR 79. The two cited July
  claims support only historical observed input length and descriptor-declared
  report shapes. **The historical encoder layout is not evidence**: its Menu
  and View bits are swapped relative to consumers. Keep Menu at bit 6 and View
  at bit 14 (`steam-c6-input-buttons`); field semantics stay compat. HHD review
  is still missing. Fresh descriptor bytes now come from the separate merged
  experiment/fixture, not the historical encoder or a reconstructed descriptor.
- **steam-s3-steam-client-settings** and **steam-s3-wired-output-streaming** are
  planned session names, not existing research records or completed runs. Both
  need the owner's separate host/capture authorization. The settings run uses
  the step 16 bound/unbound matrix and the #144 Steam-client host. The output
  session uses direct wired physical `1302`, pinned Steam client, bidirectional
  USB capture with sanitized per-report summaries, and actuator observations;
  input-only hidraw cannot settle these commands. It is haptic/flow-control
  research, not authorization to implement audio or streaming in S3.
- Puck `1304`, receiver `1305`, Bluetooth `1303`, desktop companions, LEDs and
  audio remain outside S3. No broker or host policy changes are needed.

## Deterministic checks

`cargo test --locked -p gr-curated-controllers --lib steam_controller_2026`
compares every input fixture, all nine consumer feature requests, both optional
replies, all six distinct-field haptic fixtures, raw gain domains, malformed
lengths, report classes, retained outputs, sequence wrap, stop/restart and two
independent fake sessions with per-cycle request completion and idempotent close.
Fixture regeneration also checks independent fixed lengths and hashes.
