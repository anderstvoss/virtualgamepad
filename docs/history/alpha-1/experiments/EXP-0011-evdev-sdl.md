# EXP-0011 — SDL evdev discovery, mappings and conventional output

## Scope and setup

Linux arm64 6.12.107, pinned private SDL 3.2.0 revision
`535d80badefc83c5c527ec5748f2a20d6a9310fe`. Existing ordinary-user uinput and
consumer access sufficed. No permission, module, service or kernel changes were
made. Private JSON records include the exact path, joystick/gamepad match counts,
SDL-selected mapping, control/sensor observations and separate cleanup records.

The shared harness uses a before/after inventory and requires one new virtual
input node with the expected family name. It disables HIDAPI only in each evdev
consumer process, uses the same semantic controls as the UHID script, and omits
unsupported evdev motion requests. The new `--gamepad-rumble-script` requires
neutral, 15 standard button transitions, six axis ranges and successful SDL
rumble submission. The parent independently requires typed playback with exactly
0x4000 strong and 0x8000 weak magnitudes. Upload/submission alone is insufficient.
Cleanup closes the controller before reaping a possibly blocked consumer.

## Results and corrections

| Family | Result | Meaning |
| --- | --- | --- |
| Switch Pro | Three ten-second runs pass after button-map correction | Standard controls and conventional rumble; no HD-rumble/motion claim |
| Xbox | Three ten-second runs pass | Existing conventional evdev controls/rumble; no new USB XInput behavior |
| DualSense | Three ten-second runs pass with native mapping profile 0x8111 | Standard controls/rumble; touch/auxiliary-button consumer association remains unvalidated |
| DS4 | Fails exact SDL gamepad discovery; cleanup passes | Combined BTN_TOUCH plus gamepad axes classify as touchscreen on this host |

Switch initially missed the guide button. Capture occupied a stick-button code
and shifted the SDL mapping. Capture now uses BTN_Z, stick presses use
BTN_THUMBL/BTN_THUMBR, and spatial west/north use the Nintendo kernel assignments.
Sony evdev mappings similarly separate touch-click from stick presses and expose
trigger-button state consistently with analog trigger state. Sony/Nintendo
west/north codes are corrected. Xbox deliberately retains xpad's legacy BTN_X/Y
assignment; a shared numeric face-button assumption would invert that profile.
Individual-control deterministic tests cover these mappings and releases.

DualSense initially selected SDL's older raw-HID-style mapping and failed neutral
because sticks occupied mapped trigger axes. Merely setting the native high bit
on 0x0110 was insufficient in the pinned database. The evdev presentation now
explicitly uses the supported 0x8111 mapping revision, and the resulting JSON
confirms the intended stick/trigger bindings. This identifies a virtual native
presentation, not a physical firmware version. UHID identity/wire behavior is
unchanged. DS4's version is retained while its presentation remains unresolved.

The initial negative records are preserved privately. The broad simultaneous
button sweep proves presence/transitions, not every physical label in isolation;
individual semantic-to-evdev regressions provide additional mapping coverage.
No physical-equivalence or full evdev touch claim follows from these passes.

## Current-kernel UHID recheck

The unchanged UHID profiles also passed three ten-second SDL repetitions each on
6.12.107: DualSense and DS4 controls/touch/sensors/typed rumble/RGB, Switch controls
and motion, and Xbox standard-HID controls. Exact selection, removal and repeated
cleanup passed for each repetition. These runs exercised the current rewrite;
the older baseline comparison and fault-injection records were not rerun here.
No full B/P closure, Switch compressed-rumble or Xbox HID rumble claim is added.

## References and next implementation

- [systemd v257 input classification](https://github.com/systemd/systemd/blob/v257/src/udev/udev-builtin-input_id.c): touch classification precedes joystick classification for combined absolute/touch capabilities.
- [Linux v6.12 Nintendo mappings](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-nintendo.c): capture and positional button assignments.
- [Linux v6.12 PlayStation driver](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-playstation.c): native mapping revision convention and controller-owned input presentation.
- [Linux v6.12 xpad](https://github.com/torvalds/linux/blob/v6.12/drivers/input/joystick/xpad.c): retained legacy Xbox button codes.
- [Pinned SDL mapping database](https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/SDL_gamepad_db.h).

Next, prototype DS4 as a controller-owned gamepad plus touch companion. Reuse
compound lifecycle ownership, validate second-open rollback, full-frame retry,
reverse-request routing, arbitrary removal and repeated cleanup. Prove joystick
classification and contact delivery on separate exact nodes before adopting it.
Do not remove touch capabilities or require a global udev classification override.
Gate E's broader compound UHID usefulness remains separate from repairing this
existing evdev presentation. Then complete family concurrency/failure and demo
scheduling review. Steam, physical references, audio, Bluetooth and Gate G remain
separate evidence requirements.
