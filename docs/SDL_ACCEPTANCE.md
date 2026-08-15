# SDL and Steam Input acceptance

UHID controller targets are research-backed until a reference-device comparison
promotes their surface metadata. A familiar VID/PID alone is never a physical
controller-fidelity claim.

| Target | Current status | Scope of the claim |
| --- | --- | --- |
| Generic Gamepad / UHID | ResearchBacked | Project-defined input-only standard HID gamepad using provisional `1209:0001`; it is not an ID-allocation claim. |
| Xbox 360 / UHID | ResearchBacked | Local input-only HID identity and standard controls. XInput, xpad, rumble, and Xbox USB protocol fidelity require USB gadget work. |
| DualSense / UHID | ResearchBacked | USB-style numbered HID descriptor; forward sticks, triggers, buttons, touch, and IMU bytes; typed/raw decoding of output `0x02`. Descriptor/report parity and SDL-native advanced features remain subject to reference-device comparison. |

Before promoting any entry, compare its descriptor, neutral and full-state
reports, initialization exchanges, reverse reports, and observed SDL/Steam
behavior against a reference controller. `HostValidated` means the supported
host gate passed; `PhysicallyValidated` requires that comparison.

Each target must pass this supported-host gate:

1. Install current SDL3 development/runtime support and start Steam Input.
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
  sg input -c 'cargo test -p gr-curated-controllers \
    dualsense_steam_hidapi_opens_the_session_specific_controller \
    -- --ignored --nocapture'
```

Then compare Steam's most recent physical and virtual discovery blocks without
relying on their similarly named input devices:

```bash
scripts/steam-controller-ab-report.sh /path/to/console_log.txt \
  /dev/hidrawPHYSICAL /dev/hidrawVIRTUAL
```

Both blocks must select `SDL_JOYSTICK_HIDAPI_PS5 (ENABLED)` and show their own
hidraw path. The virtual gate also requires Steam to open the controller after
the exact session serial appears in the log.
