# Xbox Series controller

Use `create_xbox_series(CreationOptions::new(RealizationId::LINUX_UINPUT),
XboxSeriesOptions::default())` at the ordinary root. The curated package uses
the same two-argument shape with its supporting `CreationOptions`.

| Realization | Scope |
| --- | --- |
| `linux.uinput` | One gamepad with the Linux xpad evdev layout. It exposes A/B/X/Y, bumpers, View/Menu/Guide/Share, stick clicks, the D-pad hat, four signed stick axes and two 10-bit triggers. Conventional force feedback carries strong and weak main-motor playback. |
| `linux.uhid.bluetooth` | Local Bluetooth-bus HID for the measured `045e:0b13` report map. Qualified with SDL 3.4.18 over evdev and hidraw on stock `microsoft`: all standard controls, separate Guide/Share, final neutral, main rumble and independent virtual trigger-command reception. Physical trigger actuation and Steam Input remain unverified. Actual radio and pairing are tracked in [#164](https://github.com/anderstvoss/virtualgamepad/issues/164). |
| USB-bus UHID and `dummy_hcd` USB HID | Inapplicable to the wired GIP controller. GIP and USB/IP feasibility stay with [#168](https://github.com/anderstvoss/virtualgamepad/issues/168). |

The xpad [consumer ledger](https://github.com/anderstvoss/controller-protocol-research/blob/eba02ad610e264a2518a699394255cc7f1d670dd/docs/ledgers/xbox-series-l0.md)
covers the gamepad name, input ID, event codes, ranges and force-feedback
capabilities. The chosen input ID version is an observed Linux evdev value.
The provider sets stick flat to 128 and fuzz to zero. The stock xpad shape
has stick fuzz 16. Linux's xpad memoryless force feedback adds gain and periodic
effects. The uinput realization supports ordinary `FF_RUMBLE` with two
magnitudes and refuses independent trigger motors, gain, periodic effects and
button-triggered playback. Battery, headset audio and accessory traffic have no
channel on the selected surface.

## C9 Linux and SDL acceptance

Candidate `1fe3f4568edafb9df333ce1495fb1d91783395af` was tested from a clean
checkout on November 9, 2026 Pacific time. Host was the local x86-64 Linux
`7.3.0-1-mainline` D9 workstation, Intel Core i5-8265U, eight logical CPUs,
Rust 1.95.0. The host already exposed `/dev/uinput` to the caller and used
its existing active-seat evdev policy. No privilege or policy changes were
needed.

- The created node had Linux input bus `0003`, vendor `045e`, product `0b12`
  and the xpad Series name. Sysfs showed 12 keys, eight absolute axes and
  `FF_RUMBLE`. No hardware driver bound to the virtual input node. On the
  association-owned event node, `EVIOCGABS` agreed with the golden axis
  domains and flat. The four stick axes had zero fuzz as documented.
- The reviewed SDL probe opened exactly one matching path, using SDL 3.4.18
  (`SDL-release-3.4.18-0-g829a65d76`) with HIDAPI disabled. The probe reported
  all 16 mapped button presses and releases (mask `0xffff`) and all six
  mapped axes (mask `0x3f`). The mapping exposed Share as `misc1` and Guide
  separately. The probe observed neutral and ended with all buttons and axes
  neutral. The C source hash was
  `6a7dc315c6da303d2ce05138c47b92856c53bae8149ff157076171c80efd034c`.
- SDL submitted conventional rumble. The controller independently retained
  strong 16384, weak 32768 and one playback repetition. This is a virtual
  reverse-output observation. Steam Input and physical reference validation
  remain untested.

The reproduction commands use temporary output paths.

```sh
SDL_JOYSTICK_HIDAPI=0 cargo run --locked -p xtask -- acceptance \
  --controller virtualgamepad.xbox-series --realization linux.uinput \
  --cycles 1 --concurrency 1 --sdl --out /tmp/series-sdl.json
for concurrency in 1 4; do
  cargo run --locked -p xtask -- acceptance \
    --controller virtualgamepad.xbox-series --realization linux.uinput \
    --cycles 50 --concurrency "$concurrency" --out "/tmp/series-${concurrency}.json"
done
```

The timing runs observed all 250 creations, evdev consumer opens and owned
node removals with no service errors. Four sessions were live in each concurrent
cycle. The two timing runs had equal before/after descriptor, thread, input-node
and hidraw counts. The SDL run's immediate global input count briefly differed
after the owned event node disappeared. The next inventory returned to baseline.
Creation-to-open includes active-seat ACL readiness. The runner retries transient
access errors for up to two seconds on Series only.

| Concurrent | Temperature | Sessions | Create to evdev open p50 / p99 / max (ms) | Close to node gone p50 / p99 / max (ms) |
| --- | --- | ---: | ---: | ---: |
| 1 | Cold | 1 | 52.2 / 52.2 / 52.2 | 34.6 / 34.6 / 34.6 |
| 1 | Warm | 49 | 25.1 / 70.6 / 70.6 | 29.8 / 46.8 / 46.8 |
| 4 | Cold | 4 | 24.8 / 26.9 / 26.9 | 29.7 / 35.9 / 35.9 |
| 4 | Warm | 196 | 25.8 / 64.6 / 68.2 | 32.1 / 50.6 / 67.3 |

Use 100 ms per stage as a provisional Series uinput regression budget on
the same provisioned host, comparing the maximum of each populated
`*/consumer_open` and `*/node_gone` summary to `100000000` ns and
requiring zero missing samples. This host-local check excludes SDL probe
startup and is subject to #166 review. Run the same-host check after the
two timing commands.

```sh
python3 - /tmp/series-1.json /tmp/series-4.json <<'PY'
import json
import sys
from pathlib import Path

for path, count in zip(sys.argv[1:], (50, 200), strict=True):
    run = json.loads(Path(path).read_text())
    assert len(run["samples"]) == count
    assert all(not sample["errors"] for sample in run["samples"])
    for stage in ("consumer_open", "node_gone"):
        summaries = [run["summaries"][f"{temp}/{stage}"] for temp in ("cold", "warm")]
        assert sum(summary["n"] for summary in summaries) == count
        for summary in summaries:
            assert summary["missing"] == 0 and summary["max_ns"] <= 100_000_000
PY
```

SDL's neutral observation comes from its bounded control sweep. The runner
still has no timestamp oracle for the first valid consumer neutral state.

For the manual Steam Input check, close Dolphin first, then create one
product controller at a time. The owner-observed probe keeps it alive and
prints retained host output changes:

```sh
VIRTUALGAMEPAD_INPUT_LAB=1 cargo run --locked --example xbox_series_steam_probe -- uinput
# Repeat after closing that probe, selecting bluetooth instead.
```

Select the newly created virtual pad in Steam Settings → Controller. Record
whether Steam identifies it as Xbox Series. Enter `sweep-no-guide` in the probe to
cycle the other buttons, both sticks and both triggers. Guide can open Big
Picture; this mode deliberately leaves its Steam test-screen registration
unverified. The full `sweep` mode remains available for other consumers. Enter `neutral` before
Steam's rumble test; confirm that a nonzero retained output appears, rather
than treating a successful UI action as evidence. Enter `quit` and verify
removal. Ask for one owner observation at a time and retain the Steam build,
candidate, kernel and driver scope with the answers.

| Steam Input consumer | Status |
| --- | --- |
| `linux.uinput` | Owner confirmed Xbox Series identity and controls excluding Guide. Steam Ping produced retained 32767/32767 playback and stops. Guide test-screen registration remains unverified. |
| `linux.uhid.bluetooth` | Not listed during the inactive-seat lab run. Controls/rumble unverified; temporary active-seat visibility override prepared for retest. |

## Alpha 2 host audit

The [Alpha 2 host audit](ALPHA_2_HOST_AUDIT.md) covers the Series creator and
consumer permissions. Owner sign-off remains on [#258](https://github.com/anderstvoss/virtualgamepad/issues/258).

## Bluetooth UHID and wired audio checkpoint

See the [checkpoint findings and reproduction commands](XBOX_SERIES_CHECKPOINT.md)
for the physical Bluetooth report map, the local UHID consumer run, and the
physical Linux headset results through xone. Select
`RealizationId::LINUX_UHID_BLUETOOTH` with the same Xbox Series constructor.
The measured map carries input and rumble. Battery, LED and headset audio
have no report in this map. Native headset audio is tracked with USB GIP
feasibility in [#168](https://github.com/anderstvoss/virtualgamepad/issues/168).

## Native rumble units

Bluetooth HID report 0x03 carries magnitudes in its descriptor's logical
0 through 100 range. SDL's Xbox HIDAPI conversion divides a 16-bit API
magnitude by 655, so 16384 and 32768 become 25 and 50. Uinput retains the
16-bit evdev values. These are transport-specific units for the same API
request. `MotorRole::LeftTrigger` and `RightTrigger` identify the two
trigger channels separately from strong and weak main motors. Retained
duration/delay fields are 10 ms descriptor ticks; repeat is a raw count.

The evdev qualification covers controls and main rumble. Independent
trigger-motor actuation remains unverified. The original evdev runs could not exercise hidraw HIDAPI because access was
denied. A later dev-only raw-path probe with the stock microsoft driver
passed all controls and observed separate left-only and right-only trigger
commands. This qualifies a bounded virtual HIDAPI command path. Physical
trigger actuation remains unverified: the physical xpadneo bind rule prevented
stock microsoft binding, so no physical pulse was sent. Exact-candidate
virtual acceptance after the main merge passed.
The dev setup and failed competing-driver attempt are retained in the
checkpoint handoff.

## Bluetooth-bus UHID candidate qualification

See [the exact-candidate checkpoint](XBOX_SERIES_CHECKPOINT.md#rebased-product-candidate)
for the research pin and historical results. The [review follow-up](XBOX_SERIES_CHECKPOINT.md#review-follow-up-main-merge-failure-diagnosis-and-isolated-timing)
records the main merge, diagnosed harness readiness deadlock, final
acceptance, isolated 250-session timing and blocked physical test. The concurrent
Bluetooth open tail exceeded the C9 uinput budget; no Bluetooth timing budget
is qualified. First-valid-neutral remains open under #166.

For dev-only HIDAPI acceptance, use the installed input-lab policy and
xpadneo wrapper described in [deployment](DEPLOYMENT_AND_VALIDATION.md),
then run the ordinary product acceptance path:

```sh
VIRTUALGAMEPAD_INPUT_LAB=1 VIRTUALGAMEPAD_XBOX_HIDAPI=1 \
cargo run --locked -p xtask -- acceptance \
  --controller virtualgamepad.xbox-series --realization linux.uhid.bluetooth \
  --cycles 1 --concurrency 1 --sdl --out /tmp/series-hidapi.json
```

This mode requires the stock `microsoft` binding before and after SDL and
checks independently received left-only/right-only reports. It uses the
owned hidraw path with HIDAPI enabled. The generic probe's SDL 3.2 backend
classifier remains unknown on SDL 3.4.18; exact path selection, input and
reverse reports are recorded independently. It does not verify a physical
motor. Leave this mode unset for the evdev consumer run.
