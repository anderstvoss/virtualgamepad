# Wired Steam Controller 2026 LED controls

Draft dependency: [research PR #109](https://github.com/anderstvoss/controller-protocol-research/pull/109)
contains the sanitized LED records below. They must merge and be adopted by the
research pin/acceptance cell before this implementation is ready to merge.

The candidate default Steam compatibility profile retains host LED intent on wired UHID.
The supporting `SteamController2026Outputs` fields are:

| Field | Meaning |
| --- | --- |
| `led_brightness: Option<u16>` | Last accepted raw setting 45 value; percent-like physical observations, no calibrated transfer function |
| `led_override: Option<bool>` | Last accepted setting 37 value, restricted to 0 or 1 |
| `led_rgbw: Option<[u8; 4]>` | Last accepted C5 bytes in red/green/blue/white order, independently of override |

`None` means unknown/unobserved. Restore-settings invalidates brightness and
override knowledge, without inventing factory defaults. It retains the separate
last C5 intent; this does not assert physical reset persistence. Repeated equal
writes do not increment the retained revision. Invalid requests update none of
these fields. A sibling controller remains independent.

The reduced uinput realization has no numbered HID feature channel, brightness
axis or RGBW output representation. These fields remain `None` there and raw
feature application is refused. This restriction is regression-tested.

## Physical evidence

- `steam-d9-wired-led-brightness`: passive Steam slider capture identifies
  command 87/key 45 with visible dimming/brightening. Nearby repeated writes
  prevent an exact UI-value or persistence claim.
- `steam-d9-wired-rgbw-override`: owner-authorized C5 basis stimuli at raw 25
  produced red, green, blue and white only with key 37 enabled. Zero colour
  produced off between trials; restoring key 37 to 0 returned normal wired green.
- `steam-d9-wired-rgbw-mixing-observation`: pairwise 25-level mixtures, with
  and without white 10, visibly changed hues. Equal raw commands are neither
  calibrated optical powers nor sRGB coordinates.
- `steam-d9-led-colour-readback-quantization`: E9 returned four zero bytes
  during visibly active stimuli at the tested values 25 and 10. The cause and
  higher-value behavior are untested.

The compatibility getter returns the observed four zero bytes in a zero-padded
63-byte numbered feature reply. This is a bounded compatibility policy; it does
not predict physical readback at higher raw values.
It does not substitute the getter result for retained raw C5 intent. Command 89
reads only key 37, with a zero default as explicit compatibility policy; this
default does not establish observed retained intent. Other setting reads reject.

The latest photograph reading is central LED1 flanked by D20 and D4. Which
packages emit light, their die construction and allocation to these channels
remain unclear. Four logical controls do not resolve physical packaging.
No spectral, full-range, calibrated brightness, reset/restart persistence or
wireless claim is made. No physical named-store commit or firmware action was used.

## Regression and consumer checks

Focused protocol tests cover exact ED/C5/E9/89 replies, atomic settings, repeated
queries with fresh padding, strict-profile rejection, restore semantics, sibling
isolation and idempotent shutdown. The fake runtime answers every request within
its polling cycle. Negative triggers are rejected on both realizations before
encoding, without consuming the input sequence or a pending feature query.

`steam-d9-virtual-led-consumer` records two fresh owned UHID candidates, each
with 77 feature requests and 77 submitted replies, zero unmatched requests and
zero write failures. All basis/mixed raw intent checks, repeated exact getters,
invalid requests and owned-node cleanup passed. Physical devices and competing
clients were absent. This validates virtual protocol behavior, not optical output.

Linux UHID maps every nonzero feature reply error to raw-client EIO. The checks
confirm internal Invalid acknowledgements and client errno 5 separately; they
do not claim exact physical USB STALL propagation. No shared provider/API change
is included.
