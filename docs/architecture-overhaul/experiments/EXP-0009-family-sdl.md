# EXP-0009 — Existing-family SDL acceptance

## Scope and apparatus

Reuse EXP-0008's private SDL 3.2.0 build (source revision
`535d80badefc83c5c527ec5748f2a20d6a9310fe`) and Linux 6.12.105 arm64 host.
Run only ordinary-user USB/UHID sessions, with scoped library lookup, exact
run-owned consumer selection, readiness/deadline servicing and a four-millisecond
semantic script. No host permissions, modules or shared services changed in these
runs. Machine-specific JSON records remain private.

Each family ran three sequential ten-second conditions with application IDs 7,
7 and 65543. Required observations include neutral state, all 15 SDL button
press/release transitions, six axis ranges and exactly one selected controller.
Every successful run reaped its consumer, removed its session nodes and closed
idempotently. Sequential ID reuse is not a concurrent-removal test.

## Results and fixes

| Family | Observed result | Limits |
| --- | --- | --- |
| DS4 | 3/3 pass; touch, both sensors (1523, 1546, 1650 distinct samples each), typed rumble and RGB feedback | No physical comparison; USB/UHID only |
| Switch Pro | 3/3 pass; gyro 1982/1982/1987, accelerometer 496/497/497 distinct samples | SDL rumble submission succeeded but compressed-rumble equivalence is not verified; no RGB requirement |
| Xbox standard HID | 3/3 pass after descriptor/encoder fix; neutral, all controls and cleanup | No sensors, touch, rumble, XInput or xpad claim |

Sensor observations had increasing timestamps and finite changing values. Counts
are acceptance evidence, not a scheduling or performance guarantee.

DS4 output decoding previously discarded the lightbar even though the HID
transport represented it. The personality now exposes optional RGB only when its
validity flag and complete field are present. The demo consumes that typed field;
rumble-only updates preserve its existing lightbar indicator. HID surfaces declare
RGB, while evdev explicitly documents the missing RGB transport.

The first Xbox run failed neutral acceptance while seeing every button and axis;
cleanup passed and later repetitions were stopped. Its descriptor declared all
six axes signed, but the encoder sent unsigned stick values and put the right
stick where Linux expects a trigger. The shared compiled descriptor now declares
unsigned bytes and the encoder follows X/Y/Z/Rx/Ry/Rz (left stick, left trigger,
right stick, right trigger). Deterministic descriptor and frame tests cover neutral,
endpoints and independent trigger positions. The nine-byte report shape remains.
The same descriptor is used by the compiled gadget profile, whose live acceptance
remains gated separately. The failed result is retained privately alongside the
three passing reruns.

## Assessment and remaining work

The shared test harness exercises the existing personality/service contract through
several consumers without adding controller semantics to providers. Production
changes are limited to DS4 typed RGB and Xbox axis correctness. The prior DualSense
lifecycle and transactional regressions remain in place.

These measurements do not close the full B/P gates or promote support levels.
DS4/Switch evdev force-feedback completion requires review: advertised uploads
must receive exact replies, not disappear as generic observations. Remaining
family concurrency/failure tests, evdev consumer matrices and Switch output
fidelity precede final core review. Steam, physical reference and Gate G remain
separate evidence requirements; extensions have not begun.
