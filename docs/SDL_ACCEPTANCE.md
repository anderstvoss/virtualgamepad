# SDL and Steam Input acceptance

UHID controller targets are research-backed until a reference-device comparison
promotes their surface metadata. A familiar VID/PID alone is never a physical
controller-fidelity claim.

| Target | Current status | Scope of the claim |
| --- | --- | --- |
| DualSense / UHID | ResearchBacked | Curated DualSense HID realization with project-owned descriptor and feature fixtures. |
| DualSense / DummyHcd | HostValidationRequired | Curated USB attachment realization; validate enumeration, feature exchange, motion, and output on a root-enabled host. |
| Xbox 360 / UHID | ResearchBacked | Local input-only HID identity and standard controls. XInput, xpad, rumble, and Xbox USB protocol fidelity require USB gadget work. |

Before promoting any entry, compare its descriptor, neutral and full-state
reports, initialization exchanges, reverse reports, and observed SDL/Steam
behavior against a reference controller. `HostValidated` means the supported
host gate passed; `PhysicallyValidated` requires that comparison.

Each target must pass this supported-host gate:

1. Prepare a pinned, private SDL3 development prefix and a usable interactive Steam Input session; record their versions.
2. Create one UHID controller at a time; do not leave the evdev sibling active.
3. Run `scripts/run-sdl3-gamepad-probe.sh /dev/hidrawN 1000`, substituting the
   session-specific hidraw path, and record the controller's SDL identity,
   GUID, gamepad type, sensor availability/rate, and sensor events. The path
   argument prevents a physical reference controller from being selected.
4. In Steam's controller test surface, verify detection and every standard
   control. For targets that declare HID output reports, exercise supported
   rumble/LED output and retain the typed reverse-event log.

The probe deliberately has no Cargo dependency on SDL. Steam Input validation
is interactive and must not be represented as an ordinary CI pass. The current
repository has no SDL3 development package installed, so this gate remains a
supported-host prerequisite rather than an ordinary automated test.

For the D9 host conclusions, exact udev rule inventory, installation modes and
future setup checks, see the
[host setup handoff](plans/pre-controller-buildout/HOST_SETUP_HANDOFF.md).

## Active Sony touch acceptance isolation

Active DualSense/DS4 touch acceptance requires the optional input-lab udev
rule. Install `udev/virtualgamepad-input-lab-seat.py` as
`/usr/lib/udev/virtualgamepad-input-lab-seat.py` (root:root, mode `0755`) and
`udev/71-virtualgamepad-input-lab.rules` as
`/etc/udev/rules.d/71-virtualgamepad-input-lab.rules` (root:root, mode `0644`),
then reload udev. The rule matches only Sony UHID nodes created by a test run
with `VIRTUALGAMEPAD_INPUT_LAB=1`; their test-only `HID_PHYS` path carries the
creating process ID. The udev rule carries a corresponding inactive seat from
the HID parent through the input device to its event nodes, which also receive
`LIBINPUT_IGNORE_DEVICE=1`. Only nodes carrying the helper-validated parent seat property receive the
lab hidraw `uaccess` tag; an active local logind session can grant SDL access. The live test verifies both event-node
properties before it sends active contacts.
Ordinary demo and Steam sessions do not set this opt-in and keep their existing
device identity. Keep Steam closed during the active-contact SDL test so it
cannot act as a competing consumer.

The installed rule changes needed for this isolation are:

1. Match the HID parent's `HID_PHYS` only under
   `virtualgamepad/input-lab/uhid/{dualsense,dualshock4}/p*-i*`, then call the
   helper to derive a process-specific inactive seat.
2. Import that seat from the HID parent into each `input*` node and set
   `ID_SEAT`; on `event*`, also set `LIBINPUT_IGNORE_DEVICE=1`.
3. On hidraw, import the helper-validated parent seat property and require it
   before adding `TAG+="uaccess"`. A broad lab namespace glob alone never
   grants hidraw access.

Matching the input node's `phys` did not work because it was empty, and matching
its `uniq` did not work because the PlayStation driver rewrites that value. The
HID parent `HID_PHYS` remains stable across those child events and carries the
test process ID. The exact installed rules are in
`udev/71-virtualgamepad-input-lab.rules`.

The helper rejects malformed, overlong, non-Sony and noncanonical numeric
identities before producing any seat. The new rules cover UHID only; uinput
active-contact tests still require separately reviewed process-owned isolation.
See [deployment instructions](DEPLOYMENT_AND_VALIDATION.md#optional-sony-uhid-input-lab-isolation)
for the opt-in install.

Run the ignored active-contact test as the ordinary user, with the private SDL
prefix scoped to this command:

```bash
VIRTUALGAMEPAD_INPUT_LAB=1 \
VIRTUALGAMEPAD_SDL_PROBE="$PWD/scripts/run-sdl3-gamepad-probe.sh" \
PKG_CONFIG_PATH="$HOME/.local/opt/sdl3-<shortrev>/lib/pkgconfig" \
LD_LIBRARY_PATH="$HOME/.local/opt/sdl3-<shortrev>/lib" \
  cargo test --locked -p gr-curated-controllers --test dualsense_uhid_live \
    dualsense_sdl_observes_motion_and_returns_output -- --ignored --nocapture
```

Use the actual private SDL prefix in place of `<shortrev>`. The test refuses to
send contacts if the opt-in is absent or any created event node lacks the
process-owned seat and libinput-ignore properties.

## Steam physical/virtual A/B

With Steam running and its `console_log.txt` available, run the opt-in virtual
Steam gate with a session-specific controller:

```bash
VIRTUALGAMEPAD_STEAM_CONSOLE_LOG=/path/to/console_log.txt \
  cargo test -p gr-curated-controllers \
    dualsense_steam_hidapi_opens_the_session_specific_controller \
    -- --ignored --nocapture
```

Then compare Steam's most recent physical and virtual discovery blocks without
relying on their similarly named input devices:

```bash
scripts/steam-controller-ab-report.sh /path/to/console_log.txt \
  /dev/hidrawPHYSICAL virtualgamepad-dualsense-session-408
```

Both blocks must select `SDL_JOYSTICK_HIDAPI_PS5 (ENABLED)`. The virtual session
serial, rather than its hidraw path, identifies it after teardown because the
kernel can reuse hidraw node numbers. The virtual gate also requires Steam to
open the controller after the exact session serial appears in the log.

## Stateful-session regression comparison

Compare the baseline revision with the migrated controller on the same provisioned host, using sequential session-specific runs. Both use USB UHID bus metadata; no bus-setting fix is claimed. Include creation-time feature requests, an unchanged-state interval serviced at the documented deadlines, sustained output, consumer CLOSE/OPEN, per-controller removal, and final cleanup. Record kernel/driver/SDL/Steam versions and each evidence axis separately. `gr-hid` and fake-provider tests establish deterministic behavior only; B/P remain blocked until these live comparisons run.

## Scoped development setup

The producer needs only its selected creation node; the consumer needs only the
created controller's hidraw/event nodes. Inspect existing user-session access
first. If additional access is required, an administrator grants a temporary
ACL to the exact verified session nodes and restores previous ACLs afterward.
Re-enumeration requires identity verification again; node numbers are reusable.
Do not enroll the test user in the general input group or grant all hidraw nodes.

Pin SDL source revision and build options in the experiment record. Install to
a private user-owned prefix; scope PKG_CONFIG_PATH and LD_LIBRARY_PATH to the
probe command, never a shell profile or system loader configuration. The probe
builds into a fresh private temporary directory and removes it on exit, including
compiler/probe failure. It is never run as root. Missing Steam or reference
hardware blocks only that evidence axis; neither is a library dependency.

## Linux startup prerequisite probe

On an already prepared, isolated UHID host, run as the ordinary user:

```bash
cargo test -p gr-curated-controllers --test dualsense_uhid_live -- --ignored
```

This services the production personality, checks its process-owned playstation
binding and input children, and verifies removal after repeated close. It does
not launch SDL or Steam, alter permissions, or substitute for the acceptance
steps above. Three repetitions passed on the recorded host; see
[EXP-0006](history/alpha-1/experiments/EXP-0006-dualsense-live-startup.md).

A test-only USB/virtual-bus baseline comparison is also available via
`cargo test -p gr-curated-controllers controlled_bus_startup_probe -- --ignored --nocapture`.
Its Linux enumeration results are documented in
[EXP-0007](history/alpha-1/experiments/EXP-0007-bus-baseline-comparison.md).
It submits changing motion but does not measure SDL sensor events or Steam behavior.

## Automated consumer assertions

The probe now requires an exact device path and duration in milliseconds
(100–60000). It returns nonzero for missing/ambiguous selection or failed required
observations and emits schema-version-1 JSON. `--require-motion` requires both
motion sensors; `--dualsense-script` additionally validates the standard control
sweep and output submission. Successful submission alone is not feedback evidence:
the ignored Rust acceptance runner verifies typed rumble/RGB output and device
cleanup separately. Consumer close in probe JSON does not mean the virtual device
has been destroyed.

See [EXP-0008](history/alpha-1/experiments/EXP-0008-sdl-core.md) for the
baseline/rewrite measurements, private build recipe and fault tests. These results
do not promote Steam or physical-fidelity status.

## Existing-family profiles

The private probe also accepts `--motion-gamepad-script` for Switch controls and
both motion sensors, and `--gamepad-script` for standard-HID Xbox controls. Both
require observed neutral state, every standard button transition and all six axis
ranges. They do not require Sony touch/RGB capabilities. The original
`--dualsense-script` profile also covers DS4; its Rust harness checks typed rumble
and RGB feedback separately from successful SDL submissions.

The shared ignored integration harness in `dualsense_uhid_live.rs` selects the
exact run-owned hidraw node for Sony/Switch and event node for standard-HID Xbox.
It performs three sequential ten-second runs and verifies repeated close and
removal after each. See [family results](history/alpha-1/experiments/EXP-0009-family-sdl.md).
No SDL dependency is added to ordinary Cargo builds.

The evdev `--gamepad-rumble-script` profile requires standard controls and rumble,
with the Rust parent verifying typed playback magnitudes. It scopes
`SDL_JOYSTICK_HIDAPI=0` to that child and records SDL's actual mapping and joystick
match counts to explain discovery failures. DS4 currently fails gamepad discovery
because its combined touch node is classified as a touchscreen; do not suppress
touch capabilities or change global udev policy to count a pass. See
[EXP-0011](history/alpha-1/experiments/EXP-0011-evdev-sdl.md).
