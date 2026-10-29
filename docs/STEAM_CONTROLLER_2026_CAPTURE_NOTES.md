# Steam Controller (2026): capture research and protocol handoff

This document preserves the Steam Controller (2026) research recorded between
July 30 and August 7, 2026. The historical workstream calls this device **Steam
Controller 2** or **SC2**. It is independent of the original Steam Controller.

The research established direct-wired and official-Puck identities, a shared
HID descriptor length, native input framing, desktop fallback coexistence,
and connection-event shapes. It also produced capture tooling and experimental
software realizations on earlier branches. It did not establish a complete
bidirectional protocol, Bluetooth realization, or hardware-faithful Puck USB
implementation.

This is a sanitized consolidation of operator notes and historical source,
not a new hardware campaign. Raw recordings were not re-opened or independently
revalidated for this document. Counts below are transcribed observations;
historical code is identified separately from captured evidence. No serials,
account information, machine-specific device assignments, raw recordings, or
private host logs are included.

## 1. How to interpret the evidence

| Label | Meaning in this document |
| --- | --- |
| Capture-recorded | The local campaign notes record an observation from a physical controller or official Puck. The original raw artifact has not been revalidated here. |
| Descriptor-backed | A historical reviewed descriptor or evidence matrix declares a report shape. This does not prove a host sent that report or establish payload semantics. |
| Reference-backed | Historical OpenPuck research describes behavior; independent observation on the official device may still be missing. |
| Implemented historically | Earlier repository source implements or tests a behavior. This is evidence of software behavior, not physical-device fidelity. |
| Unresolved | The retained evidence is insufficient to make the corresponding protocol or host-compatibility claim. |

Historical fixtures use the label `capture-verified`. That label is preserved
when describing their contents, but it should not be read as a fresh review
of raw hardware traffic during this documentation task. In particular, an
output report's declared byte length is weaker evidence than an observed
host write followed by an observed device response.

There are three separate questions throughout this report:

1. What did the physical device emit in the recorded campaign?
2. What did the experimental encoder or state machine implement?
3. What did Steam or another host application demonstrably recognize?

Passing an encoder test answers the second question. It cannot by itself
answer the first or third.

## 2. Repository status and source provenance

At consolidation, the current branch is `wip/btvirt`. Its curated controller
set does not include SC2, and it has no `gr-cli` package or SC2 capture command.
The current compound-session specification mentions SC2/Puck as a future
consumer. The commands and APIs later in this document belong to historical
workstreams and are not runnable instructions for the current workspace.

| Historical source | Location or revision | What it preserves |
| --- | --- | --- |
| Capture and profile-era workstream | `codex/steam-controller-2`, inspected at `232f093` | Capture runner, manual gates, profile-era Puck routing, software owner, and operator probe |
| Native API migration | `codex/curated-controller-native-api`, inspected locally at `15d1f86` | Typed SC2 state, encoder, local UHID realization, Puck composition, and evidence matrix |
| Capture runner source | `crates/gr-cli/src/steam_controller_2_capture.rs` at `232f093` | Campaign selection, preflight, framed reads, summaries, conversion, and deterministic tests |
| Manual workstream guide | `docs/spec/implementation/manual-gates/steam-controller-2.md` at `232f093` | Capture checklist, operator gates, boundaries, and lab prerequisites |
| Native migration guide | `docs/spec/validation/STEAM_CONTROLLER_2_NATIVE_MIGRATION.md` at `15d1f86` | Typed migration boundary and software-only realization |
| Sanitized evidence matrix | `docs/spec/validation/fixtures/steam-controller-2-evidence.yaml` at `15d1f86` | Identities, report lengths, opaque fields, and unresolved items |

The local research records comprised direct-wired and Steam-free Puck campaign
notes dated July 31; Steam-active Puck notes dated August 3 with later handoff
annotations; OpenPuck research dated July 30; and software-owner/native migration
notes through August 7. They were local scratch documents, not tracked files.
This report makes their useful findings durable without publishing the scratch
directory or raw artifacts.

Branch names are historical locators, not a guarantee of remote availability.
At the publication preflight, `codex/steam-controller-2` was absent from the
remote, and the remote native-API branch did not contain the local SC2 additions.
Publishing this report does not publish those separate implementation branches.
Commit references may therefore require the original local clone to inspect.

## 3. Device identities and transport boundaries

| Target | Recorded USB VID:PID | Recorded HID name | Descriptor length | Evidence |
| --- | --- | --- | --- | --- |
| Direct wired SC2 | `28de:1302` | Valve Software Steam Controller | 372 bytes | Capture-recorded |
| Official Puck | `28de:1304` | Valve Software Steam Controller Puck | 372 bytes | Capture-recorded |
| OpenPuck compatibility personality | `28de:1142` | Reference implementation personality | Not independently established by the campaign counts | Reference-backed |

The official Puck identity and OpenPuck compatibility identity are distinct.
The physical capture corrected an earlier assumption that the compatibility
PID was the only valid Puck identity. Neither identity alone proves that a
device reproduces the required USB interface topology or host initialization.

The capture target and the physical controller link must also remain distinct:

- **Direct wired:** the controller itself is attached through USB.
- **Puck:** the controller reaches the host through the Puck; the monitored
  interface is on the USB-facing Puck.
- **Bluetooth:** a separate acquisition and device-role protocol, not covered
  by the wired hidraw campaigns.

Reproducing the USB-facing Puck contract does not require reproducing its
proprietary radio acquisition. Conversely, exposing one local UHID controller
does not reproduce a multi-interface USB receiver. USB bus metadata in a local
HID device is not evidence of USB attachment emulation, and Bluetooth bus
metadata is not evidence of pairing or radio transport.

## 4. Capture method and its limitations

### Preflight and operator confirmation

The historical runner was Linux-specific. It inspected the selected hidraw
device's identity and descriptor, required explicit target/link metadata, and
checked whether Steam was running. Steam-free campaigns required Steam stopped;
the Steam-active Puck campaign required Steam running and an explicit operator
confirmation that the controller was attached through the Puck with Steam Input
enabled.

Identity filtering was a guard against unrelated devices, not a substitute for
physical identification. In particular, the historical direct-target filter
was permissive within Valve identities. The recorded direct campaign's specific
`28de:1302` identity is stronger attribution than simply passing that filter.

Each action had a ten-second capture window. `READY` began a window. The
Steam-active campaign additionally required `DONE` after the window; the
Steam-free implementation treated completion as implicit once a READY-confirmed
window had been recorded. A skipped, empty, or unconfirmed window was
`inconclusive`, not proof that a report or behavior was absent.

The runner supported a one-based `--start-at` action index for targeted retries.
Retry observations should be attributed to the retry campaign, rather than
retroactively converting an incorrectly timed original window into success.

### Framing and byte-length terminology

The corrected capture format stores repeated records:

```text
u16 little-endian wire_length | wire_length bytes of one HID input report
```

The HID report ID is the first byte of the wire report. It is included in
`wire_length`; the two-byte recording prefix is excluded. A 54-byte native
report therefore has a 53-byte body and a length prefix of `36 00`.

The corrected runner read hidraw nonblockingly and framed each successful read
separately. This fixed the earlier loss of variable report boundaries. Treating
the recording as concatenated fixed-size packets would misparse a stream that
mixes mouse, keyboard, native, and connection reports.

The historical summary parser rejected truncated or empty framed records and
aggregated counts by report ID and wire length. However, the recording format
does not attach a timestamp to every report. Window counts cannot establish
precise inter-report timing, jitter, phase relationships, or the exact instant
Steam liveness expired.

### Directionality and attribution

This runner read **device-to-host input reports**. It did not record host
output writes or feature transactions. Seeing a Ping cause feedback cannot
identify the host report responsible for that feedback.

Likewise, a prompt asking an operator to exercise several controls establishes
the intended interaction, not a byte-by-byte semantic decoding for each control.
A multi-action window needs finer controlled captures before it can establish
individual bit positions, axis polarity, pressure scale, or sensor units.

### Local artifacts and sanitized publication

Historical artifacts included preflight metadata, a descriptor copy, numbered
framed input files, and action outcomes. Raw metadata could include device paths,
HID strings, timestamps, and other local details, so it was retained locally.

The summarizer emitted body-free report counts and lengths. A separate converter
accepted that summary and produced an opaque fixture candidate. It did not read
raw report bodies or infer semantics. Manual review was still required before
tracking a fixture. A sanitized summary reduces disclosure risk; it does not
automatically strengthen the underlying protocol evidence.

## 5. Direct wired campaign: July 31, 2026

The direct controller was connected by USB with Steam stopped. The recorded
identity was `28de:1302`; the descriptor was 372 bytes. All five confirmed
ten-second windows were non-empty.

| Window | Intended interaction |
| --- | --- |
| 1 | Every control neutral |
| 2 | Face, menu, Steam, quick-access, and rear controls, individually pressed and released |
| 3 | Both sticks through center, extrema, and a diagonal |
| 4 | Both triggers through rest, midpoint, and full pull |
| 5 | Both pads through touch, click, and pressure, followed by controller motion |

The notes record predominantly `0x42` input, with occasional `0x40`. After the
framing correction, the neutral stream began with 54-byte `0x42` wire reports.
The descriptor declared input IDs `0x40`, `0x41`, `0x42`, `0x43`, `0x44`,
`0x45`, `0x79`, and `0x7b`, plus output IDs `0x80..0x89`.

The important correction was that a prior `0x45` implementation could not be
treated as complete direct-wired support. The physical stream demonstrated
`0x42`, and the implementation needed to follow that observed framing.

The retained direct note does not provide per-control byte diffs or window
counts. It explicitly leaves detailed field mapping, feature replies, host
output, and Bluetooth unresolved. Later encoding code supplies a historical
mapping, documented below, but does not fill that missing capture provenance.

## 6. Official Puck with Steam stopped: July 31, 2026

The controller was attached through the official `28de:1304` Puck. Steam was
stopped. The descriptor was 372 bytes, and five confirmed ten-second windows
were non-empty.

| Interaction window | `0x40` | `0x41` | `0x42` | `0x43` | `0x79` | `0x7b` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Idle / neutral | — | — | 2661 | 3 | — | 20 |
| Lizard control | — | 8 | 2661 | 2 | — | 19 |
| Right-pad mouse | 432 | — | 2656 | 2 | — | 18 |
| Keyboard controls | — | 18 | 2657 | 3 | — | 20 |
| Disconnect / reconnect | 1 | 1 | 1990 | 2 | 2 | 17 |

A dash means the retained window summary listed no count for that report.
It is not a universal assertion that the report cannot occur in that mode.

Three findings materially changed the model:

1. Native `0x42` input continued without Steam. Desktop fallback was not an
   exclusive switch away from native controller reports.
2. Lizard mouse/keyboard reports appeared alongside native input when the
   corresponding controls were used. Enabling fallback must not implicitly
   suppress native input in a behavior-matching implementation.
3. Connection lifecycle used `0x79`; two lifecycle frames were recorded in
   the disconnect/reconnect window.

The native counts suggest sustained delivery during the full neutral/control
windows. They do not establish a guaranteed polling rate. The smaller lifecycle
count is consistent with part of that window being disconnected, but the
summary cannot identify the exact disconnected duration or report scheduling.

## 7. Steam-active Puck campaign: August 3, 2026

The official Puck was used with Steam running and Steam Input enabled at
preflight. The historical campaign defined seven actions:

| Window | Requested action | Retained outcome |
| --- | --- | --- |
| 1 | Idle with Steam active | Confirmed baseline |
| 2 | Open Steam menu, dismiss, return to neutral | Confirmed interaction |
| 3 | Exercise native controls and motion | Confirmed interaction |
| 4 | Trigger one user-visible output action | Ping caused operator-observed haptic/sound feedback |
| 5 | Exit Steam, wait approximately seven seconds, exercise fallback | Original window inconclusive: Steam exited before the timed window began; targeted retry confirmed |
| 6 | Relaunch Steam, confirm recognition, repeat input and Ping | Targeted retry confirmed |
| 7 | Disconnect and reconnect through Puck | Targeted retry confirmed |

The original body-free summary recorded:

- Windows 1 and 4: `0x42` at 54 bytes, `0x43` at 15 bytes, and `0x7b` at
  13 bytes.
- Windows 2 and 3: the above set plus six-byte `0x40` reports.
- Original incomplete window 5: `0x40` and nine-byte `0x41` alongside
  `0x42`, `0x43`, and `0x7b`.

Because Steam exited before the original fallback window, that window could
not prove the requested transition timing. A retry beginning at action 5
completed all three remaining confirmations. It recorded six-byte mouse
reports, nine-byte keyboard reports, continued native input during fallback,
and `0x79` disconnect/reconnect bodies `01`, then `02`.

The historical workstream guide also records a six-byte `0x44` observation
in active/fallback work. The shorter local campaign note is less specific
about where it occurred. Preserve that report shape as a recorded historical
observation with unresolved semantics; do not assign it a particular control
or timing transition from this consolidation alone.

Ping is evidence that the physical setup produced feedback when requested.
It is not a decoded haptic command. The hidraw input recording cannot show
the causative output write, feature exchange, actuator selection, intensity,
or acknowledgement.

## 8. Consolidated input report inventory

All lengths below distinguish the report ID from the body.

| Report ID | Wire bytes | Body bytes | Retained interpretation | Evidence limit |
| --- | ---: | ---: | --- | --- |
| `0x40` | 6 | 5 | Lizard mouse | Shape recorded; complete movement/button semantics need controlled fixtures |
| `0x41` | 9 | 8 | Lizard keyboard | Shape recorded; complete mapping and release behavior need controlled fixtures |
| `0x42` | 54 | 53 | Native controller input | Presence and size recorded across modes; final eight body bytes remain opaque |
| `0x43` | 15 | 14 | Unknown vendor/state report | Shape recorded; semantics unresolved |
| `0x44` | 6 | 5 | Unknown vendor/state report | Historical guide records observation; semantics unresolved |
| `0x45` | Unresolved for these physical campaigns | Unresolved | Reference native-state format | Descriptor presence and an OpenPuck format do not prove use in these campaigns |
| `0x79` | 2 | 1 | Connection lifecycle | Disconnect `01`, reconnect `02` recorded |
| `0x7b` | 13 | 12 | Periodic status | Presence/length recorded; contents opaque |

The OpenPuck research described a 46-byte `0x45` native format. That reference
must not replace the independently observed 54-byte `0x42` format or be treated
as direct-wired verification. A declared report may be valid yet unused in a
particular campaign, mode, or firmware path.

## 9. Historical native `0x42` encoder layout

The native migration implemented the following body layout in
`crates/gr-controllers/src/lib.rs`. Offsets are zero-based **within the 53-byte
body**; add one to locate a field in the wire report. Multibyte values were
encoded little-endian.

This table documents the encoder inspected at `15d1f86`. It is not a claim
that each field was independently correlated against physical captures. The
retained research still calls for detailed direct-controller field fixtures.

| Body offsets, inclusive | Width | Historical encoding |
| --- | ---: | --- |
| 0 | 1 | Sequence byte |
| 1–4 | 4 | Button bitmask |
| 5–6, 7–8 | 2 each | Left and right trigger, unsigned 16-bit |
| 9–10, 11–12 | 2 each | Left stick X and Y, signed 16-bit |
| 13–14, 15–16 | 2 each | Right stick X and Y, signed 16-bit |
| 17–18, 19–20, 21–22 | 2 each | Left pad X, Y, pressure, signed 16-bit |
| 23–24, 25–26, 27–28 | 2 each | Right pad X, Y, pressure, signed 16-bit |
| 29–32 | 4 | Motion timestamp, unsigned 32-bit |
| 33–34, 35–36, 37–38 | 2 each | Acceleration X, Y, Z, signed 16-bit |
| 39–40, 41–42, 43–44 | 2 each | Gyroscope X, Y, Z, signed 16-bit |
| 45–52 | 8 | Opaque extension, preserved without semantic interpretation |

The button mask implemented these assignments:

| Bits | Historical assignments in increasing bit order |
| --- | --- |
| 0–3 | A, B, X, Y |
| 4–9 | QAM, R3, View, R4, R5, right bumper |
| 10–13 | D-pad down, right, left, up |
| 14–19 | Menu, L3, Steam, L4, L5, left bumper |
| 20 | Right stick touch |
| 21–23 | Right pad touch, right pad click, right trigger click |
| 24 | Left stick touch |
| 25–27 | Left pad touch, left pad click, left trigger click |
| 28–29 | Right grip touch, left grip touch |
| 30–31 | Not assigned by this encoder |

The sequence byte should not be confused with a verified scheduling algorithm.
Similarly, integer storage widths do not establish sensor scale, axis frame,
calibration, physical pressure units, timestamp frequency, or wrap behavior.
Those require controlled capture correlation. Unknown extension bytes must
remain opaque rather than being discarded or assigned invented meanings.

## 10. Host output and feature transactions

The retained descriptor/evidence matrix records the following host-output
body lengths. Wire lengths assume a one-byte report ID.

| Output ID | Body bytes | Wire bytes |
| --- | ---: | ---: |
| `0x80` | 9 | 10 |
| `0x81` | 7 | 8 |
| `0x82` | 3 | 4 |
| `0x83` | 9 | 10 |
| `0x84` | 8 | 9 |
| `0x85` | 3 | 4 |
| `0x86` | 3 | 4 |
| `0x87` | 63 | 64 |
| `0x88` | 63 | 64 |
| `0x89` | 63 | 64 |

The historical YAML calls these `capture-verified-length-only`; the workstream
guide specifically attributes the sizes to the captured descriptor. This
report therefore treats them as shape constraints, not a bidirectional traffic
transcript. Historical routing validated lengths and retained payloads as
opaque reverse events. No supported haptic/LED semantics follow from that.

The July 30 reference research also listed feature report IDs `0x01` and
`0x02`, and management command values `0x83` (attributes), `0xae` (string
attributes), `0xb4` (slot connection state), `0xad` (pairing enable), `0xa2`
(write/clear slot), and `0xa3` (read slot). These are reference-backed research
leads. A command value inside a feature transaction must not be confused with
an identically numbered HID report ID. Official-device request/reply bytes,
error acknowledgements, and persistence behavior remain to be captured.

## 11. Lizard behavior, slot state, and software realization

The original reference model used output/settings IDs `0x80..0x89` as a
Steam-alive signal and a seven-second watchdog before desktop fallback.
Historical software implemented that policy. The physical campaigns recorded
fallback coexistence and reactivation, but did not capture the exact host
liveness writes or prove the watchdog's exact timing on the official device.

The software policy exposed two separate decisions:

- Whether desktop mouse/keyboard fallback was enabled.
- Whether native reports remained visible during fallback.

`HardwareFaithfulEnabled` was the historical desktop policy name for retaining
native `0x42` alongside fallback. That name described one behavior; it did not
certify USB topology or overall hardware fidelity. The opt-in `LizardOnly`
policy suppressed native delivery during fallback while retaining controller
state for later reactivation.

Historical Puck routing modeled four logical slots, initially disconnected.
It implemented connection/status reports, retained input state, bounded opaque
reverse queues, malformed-output rejection, and cleanup. The software owner
composed one local direct-SC2 UHID child per connected slot plus a uinput
keyboard/mouse companion. A fake child/companion factory enabled deterministic
failure and lifecycle testing.

That composition was a local multi-device software presentation. It did not
establish the official Puck's USB configuration, four-controller hardware
routing, radio bonds, or Bluetooth device-role behavior. Earlier profile-era
code preserved slot addresses in composite frames and explicitly rejected
unsupported composite realizations rather than flattening them into one HID
device.

The native rewrite replaced profile/planner-based APIs with typed compiled
controllers. Its handoff explicitly warned against transplanting old profile
adapters into the new runtime or coupling SC2 to Steam Controller 1. Later
current-branch architecture changes mean these historical APIs require review
before reuse.

## 12. Historical tooling and regression coverage

The following command names identify earlier work; none is available in the
current workspace merely because this document exists:

| Historical command | Purpose and evidence boundary |
| --- | --- |
| `capture-steam-controller-2` | Operator-confirmed Steam-free direct/Puck or Steam-active Puck input capture |
| `summarize-steam-controller-2-capture` | Parse framed inputs into body-free per-window ID/length counts |
| `convert-steam-controller-2-fixture` | Convert a body-free summary into an opaque candidate requiring review |
| `validate-steam-controller-2 --mode ci` | Deterministic descriptor, encoder, and Puck state checks without hardware |
| `validate-steam-controller-2 --mode local-observe` | Historical host/Steam/Puck prerequisite inventory |
| `validate-steam-controller-2 --mode local-software` | Local UHID/uinput readiness and software-only boundary |
| `validate-steam-controller-2 --mode steam-gated` | Explicit operator gate; not an unattended acceptance test |
| `validate-steam-controller-2 --mode lab-bidirectional` | USB monitoring, device-controller, and gadget prerequisite checks |

Historical capture tests covered campaign/link parsing, Steam-active preflight,
one-based retry bounds, framed variable-length report preservation, summary
aggregation, truncated-frame rejection, opaque candidate conversion, and
coverage of native/lizard/lifecycle prompts. Later SC2 tests covered typed
encoding, sequence/extension preservation, exact realization selection,
software-Puck failure policies, and separation from legacy native controls.

These tests are preserved in historical commits. They were not restored or
executed as part of publishing this document. Passing the current workspace's
tests validates the current workspace; it does not execute missing SC2 code
or renew a previous hardware acceptance result.

## 13. Implementation chronology and recovery anchors

| Date | Commit | Recorded increment |
| --- | --- | --- |
| July 31 | `65bd980` | Workstream and capture evidence plan |
| July 31 | `c5a4db7` | Wired capture runner |
| July 31 | `bc5c195` | Mandatory physical controller-link metadata |
| July 31 | `d8c682c` | Preserve report boundaries with length-prefixed framing |
| July 31 | `81dfca0` | Align Puck reports with captured hardware |
| August 3 | `540484b` | Steam-active Puck campaign gates |
| August 3 | `d36e637` | Targeted campaign retries |
| August 3 | `a117b25` | Correct captured lizard report length |
| August 3 | `084a547` | Validate opaque Puck output lengths |
| August 3 | `d4daf84` | Deterministic validation feedback loop |
| August 3 | `bc8e8da` | Fixture and lab workflow guards |
| August 3 | `5487d29` | Software-only Puck presentation policy |
| August 3 | `c74ac56` | Local uinput lizard companion |
| August 4 | `8a387a6` | Software Puck owner session |
| August 4 | `ab219dd` | Fake Puck device boundary |
| August 4 | `232f093` | Sanitized software probe outcome recording |
| August 7 | `8c8ee34`, `a9a3fde` | Independent typed state and native encoder |
| August 7 | `3925f8d`, `32896d5` | Local UHID realization and public typed handle |
| August 7 | `ca0fa2d`, `d9e0cf4` | Native Puck state machine and software session |
| August 7 | `1d3bc36` | Sanitized protocol evidence matrix |
| August 7 | `15d1f86` | GUI SC2 creation entry |

For a clone that contains these objects, inspect historical files without
changing branches:

```bash
git show 232f093:crates/gr-cli/src/steam_controller_2_capture.rs
git show 232f093:docs/spec/implementation/manual-gates/steam-controller-2.md
git show 15d1f86:docs/spec/validation/fixtures/steam-controller-2-evidence.yaml
git show 15d1f86:crates/gr-controllers/src/lib.rs
```

These references establish source provenance. This documentation commit does
not make unrelated historical objects reachable from its branch.

## 14. Remaining capture and validation work

| Gap | Evidence needed before claiming resolution |
| --- | --- |
| Detailed direct-input mapping | Isolated press/release and axis-extrema captures, correlated byte diffs, negative controls, and reviewed sanitized fixtures |
| Touch, pressure, and IMU interpretation | Independent contact/click/pressure actions, axis orientation experiments, scaling/calibration evidence, timestamp and wrap tests |
| Opaque `0x42` extension | Controlled changes tied to the eight unknown bytes, including tests that preserve all bytes before semantics are assigned |
| `0x43`, `0x44`, `0x7b` semantics | Event-correlated observations and length/content fixtures across idle, active, fallback, and reconnection |
| Exact Steam liveness | Timestamped bidirectional host/device traffic correlated with Steam start/exit and fallback transitions |
| Haptics and LEDs | Recorded host write/feature request, device response or acknowledgement, and observed output effect for each decoded operation |
| Feature protocol | Every declared report type, exact success/error replies, unsupported requests, repeated polling, and timeout cleanup |
| Official Puck USB topology | Configuration/interface/endpoint descriptors, ordering, independent slots, and observed Steam/SDL discovery on the actual realization |
| Four-controller behavior | Independent concurrent controllers, addressed input/output, per-slot removal, reconnect, and unaffected surviving slots |
| Bluetooth | Advertising/service metadata, pairing, encryption, report framing, reconnect, and cleanup from a separate device-role campaign |
| Host acceptance | Observed Steam recognition, mapping, motion, fallback, reverse effects, and reconnect on each claimed target |

The historical lab handoff reported unavailable USB-monitor tooling and
peripheral-mode gadget prerequisites on the capture host. Those were historical
environment blocks, not protocol failures and not a statement about the present
machine. A new lab campaign must recheck monitoring access, an appropriate USB
Device Controller or validated attachment environment, and required kernel
interfaces independently.

A useful continuation sequence is:

1. Recover historical source and review it against the current compiled
   controller/runtime contracts before restoring any implementation.
2. Re-establish physical identity and descriptor provenance, then collect
   isolated direct-input fixtures with preserved framing and explicit outcomes.
3. Collect a bidirectional Puck transcript for initialization, Ping, liveness,
   fallback, reactivation, and disconnect/reconnect. Include monotonic timing.
4. Decode one output/feature behavior at a time and test exact acknowledgements,
   malformed requests, and cleanup before advertising the corresponding feature.
5. Validate topology and independent slot routing, then observe Steam and SDL
   acceptance on the claimed realization.
6. Run Bluetooth as a separate workstream with its own evidence and tests.

Future fixes need deterministic tests at the lowest practical seam, including
creation, host probe/reverse requests, repeated polling, per-controller removal,
application shutdown, and idempotent cleanup. No host request should remain
pending across a polling cycle. Provider limitations must be explicit and
tested; user-facing controls and outputs must be preserved wherever a provider
can represent them.

## 15. Conclusions supported by the retained record

The repository contains substantial SC2 acquisition research: two Steam-free
physical campaigns, a Steam-active Puck campaign with targeted retries, a
framed Linux capture runner, sanitized summaries, a reviewed report-shape
matrix, and historical encoder/state-machine implementations.

The strongest retained observations are the official identities, 372-byte
descriptor length, 54-byte native `0x42` wire report, continued native delivery
during lizard fallback, six-/nine-byte mouse/keyboard shapes, and two-byte
connection transitions. Detailed physical field semantics, reverse behavior,
USB topology fidelity, multi-controller hardware routing, and Bluetooth still
need evidence. Those boundaries should remain visible in any future API,
support matrix, or host-compatibility claim.
