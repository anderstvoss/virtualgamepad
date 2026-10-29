# Steam Controller 2026 wired: S3 reports and supporting state

Scope: checkpoint C6 steps 14a and 14b, direct wired `28de:1302` only.
The public `gr_curated_controllers::steam_controller_2026` supporting module
exposes native state, button, trackpad and motion types. The report/feature/output
protocol remains private and deterministically tested. There is no registry
entry, descriptor or controller creation path; this cannot create a virtual
Steam Controller. Ordinary-root and Wii supporting
API snapshots remain unchanged. No Steam supporting snapshot is introduced.

Research pin: `bae43833979c9a253b0285a0115aae81af303c04`, which contains
`b9c6369` and research PRs 27, 32, 73 through 76 and 79 (July capture notes). The generated fixture
manifest records the exact revision and both fixture/record file digests.
Every `steam-c6-*` fixture used here is synthetic consumer-source evidence.
Kernel and SDL may share Valve source lineage; agreement alone is not faithful.

## Open gates

- **14a descriptor:** the physical report descriptor and serial-filtered USB
  configuration/interface descriptors are requested on [#159](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6491735187).
  No substitute descriptor is synthesized. A research experiment, fixture,
  hash and claims must land before descriptor integration. Multiple HID
  interfaces or a disagreement with the unified-interface claim stops that work.
- **14a supporting API approved:** [owner decision](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6492166698)
  permits `SteamController2026State`, `SteamController2026Button`,
  `SteamTrackpadState` and `SteamMotionSample` in the supporting module.
  The existing API checker needs no change to expose this module; no separate
  Steam supporting snapshot/check is needed or added. Controller, surface and
  output types and a controller constructor remain for their authorized step.
  There are no root exports, creation-options type or broker changes. This
  approval does not complete 14a while the descriptor gate remains open.
- **14c/14d and C6 checkpoint PR:** held for step 16 on the owner's kernel
  7.3 or newer machine. No kernel, SDL, Steam, physical or timing acceptance
  follows from these deterministic tests. An optional SDL run on the 6.12
  generic HID path still needs the owner's separate OK.

No acceptance manifest is added while the family is unregistered: existing
manifest validation requires exactly the compiled families. Future cells must
carry the classification and experiment below in their `reason` field.

## Faithful versus compat behaviour

Field semantics and served behaviour remain **compat**. Report lengths alone
have the narrow class **physical (historical capture, raw not revalidated)**
where indicated below. Research PR 79 transcribes operator notes; the raw capture
has not been revalidated. Descriptor declarations are shape constraints, not
observed host output writes or proof of command semantics. Deterministic tests
check byte layouts and request completion, not a physical device or real consumer.

| Behaviour | Basis and choice | Experiment to promote or retire |
| --- | --- | --- |
| Primary input `0x42`, 54 bytes | Length: **physical (historical capture, raw not revalidated)**, `sc2-july-wired-input-42`, `sc2-july-wired-descriptor-report-shapes`. Field semantics: **compat**, `steam-c6-input-42`, `steam-c6-disagreement-47-kernel/sdl`; native signed LE fields, no consumer-axis transforms | Fresh wired input capture plus independently decoded kernel/SDL controls |
| Deliberate alternate `0x45`, 46 bytes | `steam-c6-input-45`; private encoder only | Per-transport report histogram and differential input decoding |
| SDL-only `0x47`, 46 bytes | `steam-c6-input-47`; explicit encoder argument used only by tests, never selected by normal protocol input | Version-pinned SDL timestamp/trackpad capture and decoding |
| Buttons, pad position/pressure, stick/pad/grip touch, IMU | `steam-c6-input-buttons`, `steam-c6-input-scaling`; raw native fields retain signedness; touch and pressure are independent | One-control-at-a-time wired capture, physical sensor and pressure correlation |
| Optional battery `0x43`, 15 bytes | **Compat** `steam-c6-battery-43` layout; encoder accepts caller data, no automatic status or made-up voltage/cadence | Physical wired battery/status capture |
| Feature 1 SET/GET, 64-byte host buffer | `steam-c6-feature-kernel-exchange`, `steam-c6-feature-registration`; serial and attributes return empty structural fixture shapes | Physical GET readback; step 16 good/short/missing optional-GET host gates |
| Kernel and SDL open orders | `steam-c6-feature-kernel-lizard`, `steam-c6-feature-sdl-lizard`, `steam-c6-feature-imu`; neither requires the other's GETs | Both opening orders with kernel bound/unbound, with settings readback |
| Retained lizard/mapping/IMU/watchdog intent | Same feature claims; unknown initial/default values remain unknown; restore-defaults discards assumed settings rather than fabricating values | Physical initial/default-settings readback and mode transition trace |
| Haptics `0x80` through `0x85` | **physical (historical capture, raw not revalidated)** body lengths: `0x80=9`, `0x81=7`, `0x82=3`, `0x83=9`, `0x84=8`, `0x85=3`, per `sc2-july-wired-descriptor-report-shapes`; wire length adds the ID byte. Field meanings/retained intent remain **compat** (`steam-c6-output-80-81`, `steam-c6-output-82-85`), no input ACK | Fresh host-write bus capture correlated to observed physical actuators |
| Unknown feature `0x87` setting IDs | **Compat rejection**: the whole SET completes `Unsupported`, no partial settings retention or opaque acceptance. Known consumer keys are 9, 48 and 71; other meanings are unproven. No creation-time toggle | Planned **steam-s3-steam-client-settings** run, Steam-only and SDL/Steam opening orders on the step 16 x86-64 host: capture exact feature writes/completions and whether rejection prevents open/input/haptics; propose opaque acceptance only from that evidence |
| Output `0x86` | **Unsupported** semantics; `sc2-july-wired-descriptor-report-shapes` declares 3 body bytes, **physical (historical capture, raw not revalidated)** shape only | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, `0x44` return traffic and actuator response |
| Output `0x87` | **Unsupported** semantics; same July record declares 63 body bytes, historical physical shape only. This is an output ID, distinct from feature 1 command `0x87` | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, buffer/flow control and actuator response |
| Output `0x88` | **Unsupported** semantics; same July record declares 63 body bytes, historical physical shape only | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, buffer/flow control and actuator response |
| Output `0x89` | **Unsupported** semantics; same July record declares 63 body bytes, historical physical shape only | Planned **steam-s3-wired-output-streaming** capture: correlate host writes, buffer/flow control and actuator response |
| Raw rumble gain bytes | `steam-c6-disagreement-gain-kernel/sdl`; all byte values accepted without signed reinterpretation, kernel 2/0 and SDL 0/0 tested | Physical per-motor gain sweep; controlled tests use shared nonnegative range |
| Unknown feature commands | **Compat kernel-path fit, not device behaviour**: explicit unsupported SET completion, followed by short command-echo GET if attempted; `steam-c6-feature-kernel-exchange` short-payload `-EPROTO` path | Step 16 optional-GET error injection and physical unknown-command readback |

## Judgement calls and No evidence

- The attachment explicitly permits 14b first while 14a's descriptor is pending.
  Native state and fixture pin adoption accompany that work. The approved
  supporting API is exposed; 14a remains incomplete because its descriptor is
  still missing. The report/feature-exchange objective of 14b is independent.
- Mapping restoration (`0x85`) and setting restoration (`0x8e`) are separate.
  The latter retains unknown defaults, not guessed lizard/IMU/firmware values.
- Malformed settings reject atomically, and every SET supersedes the prior
  feature 1 exchange; an invalid command cannot leak an earlier serial reply.
  Unknown setting IDs keep the existing atomic rejection instead of claiming
  that successful storage alone fulfils undocumented Steam-client settings.
  Opaque acceptance and a creation-time compat toggle are deferred; neither is
  required to complete 14b for the pinned kernel/SDL requests.
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
  is still missing. The historical 372-byte descriptor length supplies no
  descriptor bytes and does not clear the fresh descriptor gate.
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
