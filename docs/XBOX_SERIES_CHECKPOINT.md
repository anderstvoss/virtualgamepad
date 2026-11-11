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
| Physical/virtual coexistence | Awaiting a connected physical pad for this exact candidate; earlier pre-rebase coexistence is historical only. |

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
