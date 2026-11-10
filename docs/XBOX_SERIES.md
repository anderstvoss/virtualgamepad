# Xbox Series controller

Use `create_xbox_series(CreationOptions::new(RealizationId::LINUX_UINPUT),
XboxSeriesOptions::default())` at the ordinary root. The curated package uses
the same two-argument shape with its supporting `CreationOptions`.

| Realization | Scope |
| --- | --- |
| `linux.uinput` | One gamepad with the Linux xpad evdev layout. It exposes A/B/X/Y, bumpers, View/Menu/Guide/Share, stick clicks, the D-pad hat, four signed stick axes and two 10-bit triggers. Conventional force feedback carries strong and weak main-motor playback. |
| `linux.uhid.bluetooth` | **No evidence** for a Series report map with a qualified consumer path. Actual radio and pairing are tracked in [#164](https://github.com/anderstvoss/virtualgamepad/issues/164). |
| USB-bus UHID and `dummy_hcd` USB HID | Inapplicable to the wired GIP controller. GIP and USB/IP feasibility stay with [#168](https://github.com/anderstvoss/virtualgamepad/issues/168). |

The xpad [consumer ledger](https://github.com/anderstvoss/controller-protocol-research/blob/e1a059125d8c435d1575638360ab787a14136f19/docs/ledgers/xbox-series-l0.md)
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
startup and is subject to #166 review. SDL's neutral observation comes from
its bounded control sweep. The runner still has no timestamp oracle for the
first valid consumer neutral state.

For the manual Steam Input check, create one Series uinput controller, select
the exact virtual pad in Steam's controller tester, verify Share, Guide, both
triggers, all axes and D-pad directions, then remove the controller and verify
that Steam stops listing it. Record the Steam build and observed result with
[#160](https://github.com/anderstvoss/virtualgamepad/issues/160).

## Alpha 2 host audit amendment for #258

The C9 entries below extend the C8 audit's creator and consumer surfaces. They
cover the Series-specific evidence and can be folded into the corresponding
rows when the C8 audit is on main.

| What | Who needs it | Privilege | Scope and persistence | Implications and cleanup | Proposed release default and user steps | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| `/dev/uinput` creator access for Series | Required for the Series root, demo and Linux runner | Existing ordinary-user access, with an administrator only for initial node registration or an explicit access grant | System creation node. Persistent creator access depends on installed policy. Each Series node is tied to its owning descriptor. | The creation grant permits input injection beyond Series. Removal closes the owning descriptor. C9 required no new group, module load, broker or rule. | Use existing uinput access. If unavailable, follow the existing opt-in deployment policy and verify ordinary-user creation. | [C9 Linux acceptance](#c9-linux-and-sdl-acceptance) and [permission boundaries](DEPLOYMENT_AND_VALIDATION.md#permission-boundaries) |
| Association-owned Series `/dev/input/event*` access | Required for SDL and evdev acceptance. Steam Input remains manual. | Existing active-seat joystick access, separate from creator permissions | Per event node and active seat. Access and the node end when the device is removed. | Sysfs identity can be visible before the consumer ACL is effective. Confirm the current association before opening, and close the consumer descriptor and owning controller when done. | Keep distro/seat policy. Allow ACL readiness to settle, and record access denials without widening event permissions. | [C9 SDL and timing evidence](#c9-linux-and-sdl-acceptance) |

The Bluetooth and USB paths above have no C9 provisioning. Owner review and
sign-off remain on [#258](https://github.com/anderstvoss/virtualgamepad/issues/258).
