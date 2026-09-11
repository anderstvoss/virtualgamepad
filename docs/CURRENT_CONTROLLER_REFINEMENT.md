# Current-controller refinement checkpoint

These are implemented functions and explicit restrictions, not support promotions.
Every controller/realization support cell remains WIP. Read selected component
surfaces for precise limitations and native types for semantic ranges/units.

| Function | DualSense | DualShock 4 | Switch Pro | Xbox 360 personality |
| --- | --- | --- | --- | --- |
| Buttons, sticks, D-pad, triggers | Typed setters/native readback; HID and uinput codecs | Same; newly explicit Cross/Circle/Square/Triangle setters | Native A/B/X/Y labels preserve Nintendo spatial positions; digital ZL/ZR, twelve-bit HID stick packing | Typed i16 sticks/u8 triggers; standard HID, not XUSB/XInput |
| Motion | USB/UHID protocol cadence; uinput lacks faithful Sony IMU presentation | USB/UHID motion; uinput surface explicitly restricts IMU exposure | USB/UHID mode/stream cadence; uinput explicitly lacks IMU exposure | Not a native feature |
| Touch | Two native contacts; realization-specific restrictions apply | Two contacts; combined evdev touchscreen classification and split-touch acceptance remain unresolved | Not a native feature | Not a native feature; compile-fail regression |
| Conventional rumble | Controller-owned evdev upload/play/stop/erase; native USB output retained | Same, plus native HID rumble/RGB output | Evdev conventional rumble differs from native compressed motor words | Evdev conventional rumble; standard-HID output is not a proprietary Xbox output claim |
| Rich native output | Flags, adaptive-trigger bytes, mute/player LEDs, lightbar and unknown payloads retained | Raw HID output and optional RGB changes retained | Encoded rumble plus subcommand/mode payloads retained; amplitude/frequency fidelity unvalidated | Native unknown payload retained; no advanced effects promised |
| Identity | Readable/generated/restorable Sony USB/UHID identity; fresh creation tokens | Same | Fresh creation; no persistence contract added | Fresh creation; no persistence contract added |
| Lifecycle | All four root handles: retryable dirty input, service while idle, independent removal, terminal close, retained cleanup errors | Same | Same | Same |

Native state retention and host exposure are distinct. For example, DS4/Switch
native motion state can be retained with evdev restrictions; this is not proof
of Linux/SDL motion exposure. DualSense feature operations retain their existing
explicit availability checks. Native input is not redefined by an SDL mapping.

## Deterministic evidence

- `src/controllers/tests.rs`: all-family public-handle lifecycle and exact evdev
  acknowledgements; native labels agree with existing spatial wire encoding.
- `src/output.rs` tests: Sony advanced output fields, Switch encoded words and
  unknown payloads survive application translation; required requests are errors.
- `gr-curated-controllers` tests: prior controller codecs, motion/touch, report
  classes, feedback ownership, retry/backpressure and native range regressions.
- `gr-controller-runtime` / `gr-hid` tests: compound lifecycle, persistent
  resources, mutable topology, bounded protocol servicing and exact acknowledgements.
- Demo tests: bounded edits, press/release ordering, autonomous service while UI
  is blocked, arbitrary selection/removal and lists beyond the viewport.

## Live evidence and remaining work

The existing demo UHID test passed on this batch: it creates two real DualSense
sessions through the root API, runs service workers, removes one independently,
and verifies final cleanup. This is virtual-device lifecycle evidence, not a
physical-reference comparison or the required user hands-on review. The existing
three-session DS4 UHID independence/cleanup test also passed without touch injection.
A root-only live identity test confirms stable restored Sony identity with fresh
creation tokens/transport labels and terminal closure.

Host preflight found UHID creation access ready; uinput registration and gadget
infrastructure were unavailable. No host configuration was changed. Those live
cases remain skipped. DS4 touch/compound isolation, physical reference parity,
Switch compressed-rumble fidelity and backend-specific consumer comparisons
remain named follow-ups, not fabricated passes.

The [active ledger](architecture-overhaul/PRE_ALPHA_STATUS.md) owns the refinement
checkpoint and release sequence. The final API/quality passes have not occurred.
