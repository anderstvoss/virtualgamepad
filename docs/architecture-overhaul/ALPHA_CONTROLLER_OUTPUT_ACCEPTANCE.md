# Controller output acceptance ledger

Gate 2 remains open. This ledger separates already inspected/accepted boundaries
from the process-level USB observations added during the sequential review.
A process socket fixture is not kernel-consumer or physical output acceptance.
See the canonical [gate criteria](ALPHA_GATE_CRITERIA.md).

| Family and implemented fields | HID / evdev boundary | USB validation boundary and remaining evidence |
| --- | --- | --- |
| DualSense: native report 2; three validity flags, compatible motors, two 11-byte trigger effects, mute LED, microphone mute, four audio path modes, speaker/microphone volume, speaker preamp, player indicators and RGB | Controlled UHID outputs and typed decoding have prior acceptance; evdev represents conventional rumble, not Sony-native report fields. Audio-control observations do not change PCM or establish physical actuator response. | Production worker process tests now verify both interrupt OUT and SET_REPORT acknowledgements, exact ordered raw observations across start/update/stop and four audio paths, no duplicate events and zero output drops. Typed field decoding remains covered by existing controller tests. Live USB consumer/callback observations and final lifecycle matrix remain required. |
| DS4: native report 5; motors and validity-gated RGB; unknown bytes retained | Controlled UHID outputs, evdev rumble and two-contact frames/releases have prior acceptance. Associated gamepad/contact nodes preserve ownership. | Same process-level validation verifies both output paths and start/update/stop observations. Live USB output/callback evidence remains required. Partial second-node creation has `second_open_failure_rolls_back_gamepad` deterministic coverage; complete live rollback/isolation disposition remains required. |
| Switch Pro: raw report 1/0x10 motor words, implemented subcommand/player acknowledgements and protocol state | Prior controlled UHID evidence includes exact player-command success acknowledgement and encoded rumble observations. Compressed amplitude/frequency physical interpretation remains unvalidated. Evdev conventional rumble is separate. | No compiled USB/audio worker profile exists for this family. Do not invent or infer native USB support from HID evidence. Available realization rejection/documentation must remain explicit. |
| Xbox 360: evdev conventional rumble; generic HID raw observations and host-request/lifecycle representation | Evdev rumble has prior acceptance. The HID realization is a local generic HID identity, not the physical xpad/XInput transport. | The compiled `Xbox360HidEmulated` descriptor implements generic inputs, and its controller output setter is unsupported. Both USB output paths must stall and enqueue no output observation. This is tested with start/update/stop-shaped packets; it is not Xbox USB rumble emulation. |

## Confirmed USB interrupt-output defect and correction

At preceding source `668fc11f0b76dbed12a2f5958c2f763521bee92f`, the production
worker accepted a malformed two-byte DS4 report `[5, 0]` on interrupt OUT with
status zero and actual length two. The identical payload sent as SET_REPORT
returned EPIPE (`0xffffffe0`) and length zero. The interrupt handler called the
permissive native HID observation hook instead of the controller setter policy.
This also acknowledged outputs the Xbox generic HID profile cannot represent.

`crates/gr-audio-worker/src/session.rs` now routes interrupt outputs through
`Protocol::request(RequestKind::Set(...))`, accepts only a successful setter
reply and enqueues its event once. It reuses the controller's policy without
copying report lengths, IDs or codecs into the worker. Native UHID unknown/raw
observation behavior is unchanged. The USB protocol fixture follows the same
policy. No ordinary-root signature or supporting-trait signature changes.
Downstream USB callers now receive EPIPE for malformed/unsupported outputs that
previously received misleading success; valid report observations remain intact.

The lowest-seam Rust regression tests all three compiled families, malformed
IDs/lengths, empty reports, supported Sony reports, unsupported Xbox reports,
setter/interrupt parity, exact events, no duplicates and zero loss. The expanded
production worker validator repeats all three families using descriptor slots
3/4/5 and 64/65/66, verifies both endpoint paths, empty queues after each event,
unsupported output GET rejection and normal PCM/diagnostics/shutdown behavior.
Five Python regressions cover fixture fields, rejection, generation/order and
incorrect acknowledgements. Both validators refuse optimized Python execution
rather than silently disable their existing assertions.

All six production-process cells and all three protocol-fixture cells passed.
The five mandatory repository checks, forced Rust 1.85 check and all 329 Python
tests passed for this increment. Raw before/after receipts remain outside Git.
These results close only the tested process cells, not the whole gate.

## Invalidation and remaining gates

The production worker image changed. Gate 3's pass remains pinned to its recorded
candidate; provider acceptance must be revalidated on the corrected image before
retaining a whole-gate pass for the newer candidate.

The normally built GUI-soak image also changed from its previously accepted
byte hash because the supporting worker crate changed. Do not reuse the old
soak for this new executable. Repeat the separate two-hour neutral soak on the
final byte-pinned GUI image after further relevant changes settle, without
compilation, Steam or competing audio trials. Interactive keyboard/accessibility,
live USB outputs, complete isolation/rollback, routing, Steam and sustained audio
acceptance remain required. No dependencies were added and no loss/latency
threshold was relaxed.
