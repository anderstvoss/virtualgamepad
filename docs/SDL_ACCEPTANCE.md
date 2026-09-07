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
[EXP-0006](architecture-overhaul/experiments/EXP-0006-dualsense-live-startup.md).

A test-only USB/virtual-bus baseline comparison is also available via
`cargo test -p gr-curated-controllers controlled_bus_startup_probe -- --ignored --nocapture`.
Its Linux enumeration results are documented in
[EXP-0007](architecture-overhaul/experiments/EXP-0007-bus-baseline-comparison.md).
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

See [EXP-0008](architecture-overhaul/experiments/EXP-0008-sdl-core.md) for the
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
removal after each. See [family results](architecture-overhaul/experiments/EXP-0009-family-sdl.md).
No SDL dependency is added to ordinary Cargo builds.
