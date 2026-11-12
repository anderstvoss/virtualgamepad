# Xbox Series buildout checkpoint

Historical physical and scratch observations below were collected on
November 10–11, 2026 Pacific time. Research #125 merged as
`eba02ad610e264a2518a699394255cc7f1d670dd`; the product pins that commit.
The rebased product results are recorded separately at the end.

## Verified on Linux

| Check | Result |
| --- | --- |
| Wired xpad input and reconnect | All 12 keys, eight directions, four stick axes and two triggers passed. A, Guide and Share worked after USB reconnect. Physical stick fuzz was 16. C9 uinput uses 0. |
| Wired xpad outputs | Both conventional main-rumble pulses were felt. No controller LED or headset PCM surfaced under xpad. |
| Driver lab | Live switching and explicit module unload passed for xone-wired and xpad without per-test sudo. Socket activation survived reboot. The xone install blacklists xpad autoload, and the lab helper can explicitly load it. |
| Wired xone | Version 0.5.8 created distinct gamepad and headset GIP clients after the gamepad and headset modules were loaded and the device reconnected. The first boot binding exposed only the adapter. An xone white status LED node appeared; changing brightness was unverified. |
| Physical USB headset | S16_LE, 48 kHz stereo playback and 24 kHz mono microphone capture. The operator heard left then right. The concurrent repeat captured all 120,000 mic frames in five seconds without a reported xrun. The initial probe overran because it deferred draining its capture pipe. No PCM was retained. |
| Physical Bluetooth | BlueZ paired the reference as `045e:0b13` on bus 5. Linux bound `microsoft`. The 283-byte HID report map matched xpadneo's published map. The hidraw ACL prevented raw input capture. |
| Local Bluetooth UHID | The scratch device bound to `microsoft`. SDL 3.4.18 on the owned evdev path with HIDAPI disabled saw all 16 buttons and six axes, Share as `misc1`, final neutral, and consumer close/reopen. UHID received rumble start and stop. Destruction removed the owned node. |

The [wired physical ledger](../protocol-research/docs/ledgers/xbox-series-wired-physical.md)
links the sanitized evdev fixtures. The [Bluetooth ledger](../protocol-research/docs/ledgers/xbox-series-bt-reference.md)
links the captured report map and records output bytes and qualification limits.
Exact hardware model and installed firmware remain unconfirmed.

## Reproduce the bounded UHID experiment

On a Linux host with access to `/dev/uhid`, the active-seat joystick event
nodes and SDL 3.4.18, run from the repository root:

```sh
probe_dir=$(mktemp -d)
cc -std=c11 -D_POSIX_C_SOURCE=200809L scripts/sdl3-gamepad-probe.c \
  -o "$probe_dir/sdl-probe" $(pkg-config --cflags --libs sdl3) -lm
python3 scripts/probe-xbox-series-bt.py "$probe_dir/sdl-probe" \
  protocol-research/fixtures/xsphys-bt-descriptor.hex
```

The scratch probe uses a fresh synthetic physical path per run. Bluetooth
pairing supplied the physical reference map. The local UHID run itself uses
the kernel virtual bus and needs no adapter pairing.

## Integrated Bluetooth UHID acceptance

The ordinary root constructor created the Bluetooth target on Linux
7.3.0-1-mainline. The existing acceptance runner selected the owned evdev
node and SDL 3.4.18 with HIDAPI disabled. All 16 button presses and releases
and all six analog axes were seen, with Share mapped separately from Guide
and final neutral. The controller retained rumble mask 3, main magnitudes
25 and 50, and zero trigger magnitudes. Closing removed both associated
evdev and hidraw nodes. Five four-controller cycles observed 20 opens and
20 removals with no service errors. A separate uinput regression run kept
all control masks and retained conventional rumble at 16384 and 32768.

```sh
SDL_JOYSTICK_HIDAPI=0 cargo run --locked -p xtask -- acceptance   --controller virtualgamepad.xbox-series --realization linux.uhid.bluetooth   --cycles 1 --concurrency 1 --sdl --out /tmp/series-uhid-sdl.json
cargo run --locked -p xtask -- acceptance   --controller virtualgamepad.xbox-series --realization linux.uhid.bluetooth   --cycles 5 --concurrency 4 --out /tmp/series-uhid-n4.json
```

## Virtual USB GIP audio requirements

The physical xone run is the baseline for [#168](https://github.com/anderstvoss/virtualgamepad/issues/168).
A virtual GIP profile must provide the USB vendor interface and interrupt
control path that xone probes, the alternate audio interface and isochronous
IN/OUT, and the gamepad/headset enumeration and audio negotiation messages.
The existing USB/IP worker can carry isochronous transfers for compiled
HID/UAC profiles. GIP needs its own profile and device protocol, including
acknowledgements and rollback when the host changes alternate settings or
disconnects. Preserve the observed 48 kHz/24 kHz asymmetry until more
negotiated formats are measured.

The next feasibility gate is to bind xone to an owned virtual USB device,
observe separate gamepad and headset clients, then pass marker-based ALSA
playback/capture and hotplug/reconnect tests with no stale nodes. A bounded,
filtered raw GIP exchange from the physical reference would help pin the
negotiation. The current lab socket authorizes driver operations and does
not grant USB monitoring. USB/IP audio broker and VHCI were inactive on
this boot.

The Bluetooth report map has no audio interface or PCM reports. Any
caller-session PipeWire approximation needs explicit routing and a separate
compatibility label. Remaining Bluetooth checks include exact-product-candidate HIDAPI,
Steam Input, sustained and concurrent lifecycle qualification, and
independent physical trigger-motor actuation. The scratch HIDAPI follow-up
below qualifies the virtual command path only. Real virtual radio pairing is tracked
in [#164](https://github.com/anderstvoss/virtualgamepad/issues/164).

## Dev-only hidraw HIDAPI follow-up

After installation of the dev udev access update and a virtual-label guard
around xpadneo's forced-rebind rule, the scratch UHID reference stayed bound
to the stock `microsoft` driver. SDL 3.4.18 opened exactly its readable
hidraw node with HIDAPI enabled, observed button press/release masks
`0xffff`, axis mask `0x3f`, final neutral, and close/reopen. This used
the physical 283-byte report map and the same codec used for prior scratch
qualification.

Separate `SDL_RumbleGamepadTriggers` calls used left API magnitude
`0x4000` with right zero, then right `0x8000` with left zero. Both
were accepted. The independent controller-side observer saw numbered output
`03 0f 19 00 19 32 ff 00 eb` followed by
`03 0f 00 32 19 32 ff 00 eb`. The first two magnitude bytes are left
and right trigger, while the following main-motor values remained 25/50.
A zero-magnitude stop was also observed. The owned UHID node disappeared
after destruction. Physical left/right trigger actuation remains unverified.

The original new hidraw attempt encountered xpadneo's rebinding and product
rewrite during enumeration, yielding incompatible traffic and no control
sweep. It remains a failed consumer setup observation. The corrected run
verified the driver again after the consumer finished. The stock-driver
probe used an exact hidraw path; the generic probe's older SDL 3.2 backend
classifier left its backend label unknown for this newer SDL version.

The installed driver-lab guard also needed to accept xpadneo's rewritten
input identity `028e` when the original immutable HID device name still
identifies bus 5, vendor 045e, product 0b13. A regression test preserves the
single-reference bound-device restriction with that identity rewrite.

## Rebased product candidate

Tested commit: `888cce49f789dce8c1c8251dbd537ebd0d4105cd`, based on
product main `b41c82d`, including #353, #363, #364 and #366. Research pin:
`eba02ad610e264a2518a699394255cc7f1d670dd` (merged #125, including the
tracking thread's descriptor-link pin). Old unpublished corpus records
were preserved locally outside the committed submodule tree.

All runs used Linux `7.3.0-1-mainline`, SDL 3.4.18
(`SDL-release-3.4.18-0-g829a65d76`). Bluetooth runs bound to stock
`microsoft`; uinput has no hardware driver binding. Input-lab policy and
the dev xpadneo rule wrapper were installed. These are host-local results.

| Run | Result |
| --- | --- |
| Bluetooth evdev, one session | Exact owned event path, HIDAPI disabled, button down/up `0xffff`, axes `0x3f`, separate Guide/Share, final neutral, close/reopen; retained main rumble 25/50. |
| Bluetooth HIDAPI, one session | Exact owned hidraw path, HIDAPI enabled, same control masks and final neutral, close/reopen; `microsoft` verified before and after. Both trigger API calls accepted and independent reports received. |
| Uinput regression, one session | Same control masks and final neutral, close/reopen; retained strong 16384, weak 32768, repetition 1. |
| Bluetooth concurrent lifecycle | Five cycles of four controllers: 20 consumer opens and owned-node removals, zero service errors. |
| Bluetooth timing | 50 single-controller sessions plus 50 four-controller cycles (200 sessions), zero service errors; no missing open/removal samples. |
| Physical/virtual coexistence | Passed on `e3c2152667956c92180e3b513b0ffa14bc39b4a1` (documentation-only successor). Both evdev and HIDAPI saw two joysticks and exactly one owned match. Physical xpadneo and virtual microsoft bindings remained distinct; all four physical input nodes kept their identities throughout virtual creation, use and removal. |

All five run groups returned descriptor/thread/input/hidraw resource counts
to baseline. The timing harness observes an evdev open and owned-node
removal; it does not infer first-valid-neutral. That oracle remains #166.
SDL was installed at the version above; timing arms did not launch SDL.

| Concurrent | Temperature | Sessions | Create to evdev open p50 / p99 / max (ms) | Close to node gone p50 / p99 / max (ms) |
| --- | --- | ---: | ---: | ---: |
| 1 | Cold | 1 | 87.09 / 87.09 / 87.09 | 30.13 / 30.13 / 30.13 |
| 1 | Warm | 49 | 78.21 / 91.74 / 91.74 | 34.30 / 41.10 / 41.10 |
| 4 | Cold | 4 | 72.63 / 79.67 / 79.67 | 31.19 / 36.53 / 36.53 |
| 4 | Warm | 196 | 192.40 / 564.38 / 1312.67 | 36.58 / 85.49 / 91.02 |

The warm concurrent open tail exceeds the C9 uinput 100 ms provisional
budget. Bluetooth has no qualified budget. These runs included concurrent
validation load; its contribution was not isolated. This remains a timing
limitation for review, rather than a passing latency-budget claim.

Numbered HIDAPI reports received by the product (report ID followed by
payload) included:

- Main: `03 0f 00 00 19 32 ff 00 eb`.
- Left-only trigger: `03 0f 19 00 19 32 ff 00 eb`.
- Right-only trigger: `03 0f 00 32 19 32 ff 00 eb`.
- Stop: `03 0f 00 00 00 00 ff 00 eb`.

The SDL calls used left `0x4000`/right zero, then left zero/right `0x8000`,
with 200 ms API durations and separated submissions. The trigger probe now
waits beyond consumer startup: an earlier immediate short call was accepted
by SDL but did not yield an observed independent left report. Acceptance
requires both controller-side reports, not API return values alone.
Magnitudes are logical 0–100 (`SDL magnitude / 655`); duration/delay fields
are 10 ms ticks, repeat is a raw count. Physical trigger actuation and Steam
Input remain owner-observed checks. No battery, LED or audio report exists
in this Bluetooth map. Radio/pairing stays #164; GIP/audio stays #168.

Required Rust fmt/check/clippy/test, fresh default/all-feature API comparison,
Ruff, the 237-test Python suite, Gitleaks and research fixture regeneration
passed. The ordinary API adds the retained Bluetooth rumble type and field,
HID lifecycle/output variants and two trigger motor roles. The input-only
Series migration should migrate both realizations after rebasing on this
work; no open Series migration PR was found at this checkpoint.

### Connected physical reference follow-up

The owner connected the physical pad after draft #369 opened. On candidate
`e3c2152667956c92180e3b513b0ffa14bc39b4a1`, Linux `7.3.0-1-mainline` and
SDL 3.4.18, both product evdev and HIDAPI sweeps passed while the physical
reference remained bound to xpadneo v0.10.5. The virtual controller bound to
stock `microsoft` before and after. A concurrent observer sampled physical
binding and all four physical input-node identities through each run;
none disappeared or changed. SDL enumerated two joysticks, matched only the
owned virtual path, and observed all control masks and final neutral.
Virtual cleanup removed its owned nodes while the physical nodes survived.
Before/after resource counts matched for both runs. No physical rumble
commands were sent and no physical trigger-actuation claim is added.

### Portable constructor validation and final repeat

CI on macOS exposed a constructor-order regression: unavailable targets
returned platform-preflight errors before target rejection. The constructor
now rejects unsupported realizations first, preserving the prior contract,
with a platform-independent regression test. Runtime candidate
`63c35dcec6e5b850186e28b725dd6187e6a797e5` repeats the required 250 timing
sessions, 20-session concurrency check, uinput controls/16384–32768 rumble,
and both Bluetooth SDL paths with the connected physical reference.
All timing samples and owned cleanup completed without service errors;
resource counts returned to baseline. Coexistence passed with physical
xpadneo and virtual microsoft, two enumerated joysticks and one exact match.

| Concurrent | Temperature | Sessions | Create to evdev open p50 / p99 / max (ms) | Close to node gone p50 / p99 / max (ms) |
| --- | --- | ---: | ---: | ---: |
| 1 | Cold | 1 | 90.03 / 90.03 / 90.03 | 35.22 / 35.22 / 35.22 |
| 1 | Warm | 49 | 121.27 / 167.92 / 167.92 | 32.26 / 42.88 / 42.88 |
| 4 | Cold | 4 | 101.07 / 124.41 / 124.41 | 30.51 / 43.77 / 43.77 |
| 4 | Warm | 196 | 191.87 / 584.42 / 898.16 | 35.43 / 83.03 / 90.66 |

Concurrent validation load was again present. These measurements supersede
the earlier runtime candidate for the final repeat and still do not qualify
a Bluetooth timing budget. One evdev SDL sweep immediately after timing
exited unsuccessfully; the runner recorded no detailed probe observation
on that failure. A complete repeat of both paths passed on the same commit.
The intermittent consumer failure remains unexplained; the successful
repeat does not erase it. Physical nodes and binding survived both attempts.

The corrected candidate passed required Rust fmt/check/clippy/test and the
fresh API comparison. The additional pin-workflow suite passed all 15 tests.

## Review follow-up: main merge, failure diagnosis and isolated timing

Main `f27e611` (#370, DualSense input/status split) was merged into the
existing PR branch as `659b1f7`. The migration-guide conflict preserves both
entries; the merged API inventory was checked against fresh documentation.
No signed-off Alpha audit row changed.

The diagnostic harness candidate `109469a` recorded failure step, last
selected input event, expected missing controls/endpoints, elapsed time
since creation and whether the owned node still existed. Its first five
250-session timing runs (1,250 sessions) and all five immediate evdev sweeps
passed. Three of 20 standalone sweeps failed. Each failure had no input
events, all four stick axes at SDL's initial minimum, no observed neutral,
and a present node. This identified a producer/consumer readiness cycle:
the harness waited for neutral before sending its first input transition.
An experimental host-OPEN snapshot refresh still failed one of 20 standalone
sweeps and was reverted. It is not part of the final protocol.

Final runtime/harness candidate:
`8b42170810bb95e42f94cac845c543690d425552`. The Series sweep readiness
signal now means consumer-open plus a bounded settle interval. It reports
whether initial neutral was actually observed separately. It does not
claim neutral delivery or bypass the eventual control/neutral assertions.
Other exact-state modes retain their neutral prerequisite. A deterministic
fake-SDL test starts with non-neutral axes, verifies producer readiness,
and verifies that readiness alone cannot pass the control assertions.
The sanitized diagnostic retention also has a regression test.

After this fix, the 250-session timing run, immediate evdev sweep and all
20 standalone sweeps passed (21 sweeps, zero failures). This resolves the
reproduced harness deadlock; first-valid-neutral remains open under #166.
All these runs used Linux `7.3.0-1-mainline`, SDL 3.4.18 and virtual stock
`microsoft`. Xpadneo was kept unloaded. The physical reference was initially
disconnected and was present but unbound during the final isolated run.
No concurrent validation work ran during these timing arms.

| Final isolated arm | Sessions | Create to first evdev open p50 / p95 / p99 / max (ms) |
| --- | ---: | ---: |
| Single controller | 50 | 84.62 / 130.73 / 148.38 / 148.38 |
| Four concurrent controllers | 200 | 109.33 / 145.24 / 152.59 / 168.29 |

For the final concurrent arm's slowest open, observed phases since creation
were: provider create return 1.43 ms, hidraw appearance 3.47 ms, observed
kernel binding 3.53 ms, input-node appearance 3.48 ms, first open attempt
3.94 ms, successful open 168.24 ms. The open encountered 130 permission-denied
responses. These are polled upper bounds, not kernel trace timestamps.
The dominant observed tail is access-policy readiness after node appearance;
no library lock or serialized library wait accounts for that interval.
No kernel/udev workaround or Bluetooth timing budget is introduced.

Before the final fix, the five isolated single/concurrent maxima were
156.13/199.77, 181.33/224.99, 146.62/198.85, 159.83/342.96 and
174.69/301.76 ms. The worst observed access had 220 permission denials;
its input node appeared at 20.85 ms and open succeeded at 342.91 ms.
The earlier loaded-host 898 ms result remains historical, rather than an
isolated-host latency claim.

### Physical trigger check blocked before pulses

The owner confirmed reconnection with USB unplugged. With xpadneo unloaded,
the physical HID device was present but unbound. The installed physical
xpadneo `60-xpadneo.rules` unbinds stock microsoft and tries to bind xpadneo;
its dev wrapper exempts only the validated virtual lab device. No usable
physical hidraw node appeared, so the stock-driver permission gate could
not be reached. No pulse was sent and no owner actuation observation was
requested. Physical trigger actuation remains unverified; no passing-result
research PR is warranted. Reloading xpadneo restored the physical binding,
which was checked explicitly.

Udev also automatically reloaded xpadneo on new HID creation after a simple
module unload. For the isolated run the owner installed a temporary runtime
modprobe blacklist. This is dev/test only; cleanup is tracked in the GitHub
handoff. The driver lab helper's allowlist was not broadened.

### Latest main integration and final combined-head acceptance

Main advanced during the review to `031dd11696badb267e1c3c2b3e962316dfba7f1c`
(#371, DualShock 4 input/status split). Merge `11fa0b52ebcbd010fbab1c67efa4692e0068ace5`
resolves the application-guide conflict by keeping Series Bluetooth output
semantics and upstream DualShock 4 input/status documentation. The Series
protocol and sweep fix are unchanged. The fresh API inventory matches root
167 items, Wii 22 and Steam 13. Required Rust checks, Ruff and the 239-test
Python suite pass on this combined head.

Exact-head host acceptance on `11fa0b52ebcbd010fbab1c67efa4692e0068ace5`
repeated evdev and HIDAPI controls/neutral/reopen, separate virtual trigger
commands, uinput controls and retained 16384/32768 rumble, 20 four-controller
lifecycle sessions and 250 isolated Bluetooth timing sessions. All passed;
resource counts returned to baseline. Linux/SDL versions were unchanged,
virtual binding was microsoft, and xpadneo stayed unloaded during timing.
The physical reference was not usable on stock binding as recorded above.

| Latest isolated arm | Sessions | Open p50 / p95 / p99 / max (ms) |
| --- | ---: | ---: |
| Single | 50 | 96.41 / 106.66 / 110.48 / 110.48 |
| Four concurrent | 200 | 97.88 / 118.85 / 135.02 / 247.83 |

In the slowest concurrent sample, provider creation returned at 0.15 ms,
binding was observed at 2.67 ms, input appeared at 2.60 ms, open was attempted
at 5.16 ms and succeeded at 247.78 ms after 209 permission denials. The
phase attribution remains host access readiness. No Bluetooth budget is
claimed. Xpadneo was reloaded after this run.

The first main-merge commit `659b1f7` passed all 19 GitHub checks, including
pinned research and Windows/macOS tests. Later heads require fresh CI review.
The temporary runtime blacklist cleanup and Steam owner observations remain
pending in the GitHub handoff. No physical actuation claim or research
follow-up is added.
