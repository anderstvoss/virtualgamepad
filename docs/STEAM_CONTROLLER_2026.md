# Steam Controller 2026 wired: S3 groundwork

Scope: checkpoint C6 steps 14a and 14b, direct wired `28de:1302` only.
The private `gr-curated-controllers` module is compiled and deterministically
tested, but has no public API, registry entry, descriptor or creation path yet.
It cannot create a virtual Steam Controller. Ordinary-root and Wii supporting
API snapshots remain unchanged. No Steam supporting snapshot is introduced.

Research pin: `7922acef3966aa8da7770878a528d80085c5dddd`, which contains
`b9c6369` and research PRs 27, 32 and 73 through 76. The generated fixture
manifest records the exact revision and both fixture/record file digests.
Every `steam-c6-*` fixture used here is synthetic consumer-source evidence.
Kernel and SDL may share Valve source lineage; agreement alone is not faithful.

## Open gates

- **14a descriptor:** the physical report descriptor and serial-filtered USB
  configuration/interface descriptors are requested on [#159](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6491735187).
  No substitute descriptor is synthesized. A research experiment, fixture,
  hash and claims must land before descriptor integration. Multiple HID
  interfaces or a disagreement with the unified-interface claim stops that work.
- **14a supporting API:** [the concrete proposal](https://github.com/anderstvoss/virtualgamepad/issues/159#issuecomment-6491766605)
  awaits approval before any public item or Steam API snapshot is added.
  The private native state is groundwork, not a reviewed application API.
- **14c/14d and C6 checkpoint PR:** held for step 16 on the owner's kernel
  7.3 or newer machine. No kernel, SDL, Steam, physical or timing acceptance
  follows from these deterministic tests. An optional SDL run on the 6.12
  generic HID path still needs the owner's separate OK.

No acceptance manifest is added while the family is unregistered: existing
manifest validation requires exactly the compiled families. Future cells must
carry the classification and experiment below in their `reason` field.

## Faithful versus compat behaviour

All implemented rows are **compat**. There are no faithful protocol claims in
this groundwork. The tests check byte layouts and synchronous request completion,
not the physical controller or a real consumer.

| Behaviour | Basis and choice | Experiment to promote or retire |
| --- | --- | --- |
| Primary input `0x42`, 54 bytes | `steam-c6-input-42`, `steam-c6-disagreement-47-kernel/sdl`; native signed LE fields, no consumer-axis transforms | Wired input capture plus independently decoded kernel/SDL controls |
| Deliberate alternate `0x45`, 46 bytes | `steam-c6-input-45`; private encoder only | Per-transport report histogram and differential input decoding |
| SDL-only `0x47`, 46 bytes | `steam-c6-input-47`; explicit encoder argument used only by tests, never selected by normal protocol input | Version-pinned SDL timestamp/trackpad capture and decoding |
| Buttons, pad position/pressure, stick/pad/grip touch, IMU | `steam-c6-input-buttons`, `steam-c6-input-scaling`; raw native fields retain signedness; touch and pressure are independent | One-control-at-a-time wired capture, physical sensor and pressure correlation |
| Optional battery `0x43`, 15 bytes | `steam-c6-battery-43`; encoder accepts caller data, no automatic status or made-up voltage/cadence | Physical wired battery/status capture |
| Feature 1 SET/GET, 64-byte host buffer | `steam-c6-feature-kernel-exchange`, `steam-c6-feature-registration`; serial and attributes return empty structural fixture shapes | Physical GET readback; step 16 good/short/missing optional-GET host gates |
| Kernel and SDL open orders | `steam-c6-feature-kernel-lizard`, `steam-c6-feature-sdl-lizard`, `steam-c6-feature-imu`; neither requires the other's GETs | Both opening orders with kernel bound/unbound, with settings readback |
| Retained lizard/mapping/IMU/watchdog intent | Same feature claims; unknown initial/default values remain unknown; restore-defaults discards assumed settings rather than fabricating values | Physical initial/default-settings readback and mode transition trace |
| Haptics `0x80` through `0x85` | `steam-c6-output-80-81`, `steam-c6-output-82-85`; exact lengths and typed fields retained separately, no input ACK | Host-write bus capture correlated to observed physical actuators |
| Raw rumble gain bytes | `steam-c6-disagreement-gain-kernel/sdl`; all byte values accepted without signed reinterpretation, kernel 2/0 and SDL 0/0 tested | Physical per-motor gain sweep; controlled tests use shared nonnegative range |
| Unknown feature commands | Explicit unsupported SET completion, followed by short command-echo GET if attempted; `steam-c6-feature-kernel-exchange` short-payload `-EPROTO` path | Step 16 optional-GET error injection and physical unknown-command readback |

## Judgement calls and No evidence

- The attachment explicitly permits 14b first while 14a's descriptor is pending.
  Private native state and fixture pin adoption accompany that groundwork so
  it is buildable and testable without prematurely publishing the 14a API.
- Mapping restoration (`0x85`) and setting restoration (`0x8e`) are separate.
  The latter retains unknown defaults, not guessed lizard/IMU/firmware values.
- Malformed settings reject atomically, and every SET supersedes the prior
  feature exchange; an invalid command cannot leak an earlier serial reply.
- The normal stream emits only `0x42`. Battery and alternate reports are explicit
  encoders. There is **No evidence** for an automatic wired cadence; the private
  protocol uses explicit state updates, without inventing a timer.
- There is **No evidence** for rumble timeout/auto-stop. No such timer exists.
  `steam-c6-sdl-rumble-resend-40ms` describes host resend; its roughly 50 ms
  timeout comment needs a physical lapse/resend sweep. Kernel 20 Hz coalescing
  and SDL 4032 microsecond sensor interval are consumer assertions, not device
  scheduling requirements.
- There is **No evidence** for wired feature 2, `0x44` ACK/flow control, streaming
  outputs `0x86` through `0x89`, retail serial/attributes/calibration/firmware,
  lizard watchdog duration or real-device command responses. These are refused
  or left unknown. No wired `0x79` is emitted.
- The recovered July operator notes are historical reference only until their
  research import merges. Descriptor/report-mix/lifecycle/output-length rows
  await that import; section 9's encoder is not capture-correlated. HHD review
  is still missing. No byte here cites those notes as evidence.
- Puck `1304`, receiver `1305`, Bluetooth `1303`, desktop companions, LEDs and
  audio remain outside S3. No broker or host policy changes are needed.

## Deterministic checks

`cargo test --locked -p gr-curated-controllers --lib steam_controller_2026`
compares every input fixture, all nine consumer feature requests, both optional
replies, all six distinct-field haptic fixtures, raw gain domains, malformed
lengths, report classes, retained outputs, sequence wrap, stop/restart and two
independent fake sessions with per-cycle request completion and idempotent close.
Fixture regeneration also checks independent fixed lengths and hashes.
