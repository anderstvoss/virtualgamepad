# C6 D9 acceptance and timing

Run date: 2026-11-03 (Pacific). Linux `7.3.0-1-mainline`, x86-64, Intel Core i5-8265U, rustc 1.95.0. Tracked-clean executable source `bbf6b961390f3aad6985ec979f775df4bb9dca6f`; research pin `fc6964ac8799f0b7a38ffb9526361450f0b1b87a`. These are virtual consumer observations, not physical-device evidence.

## Timing (#166)

[Tracker observation](https://github.com/anderstvoss/virtualgamepad/issues/166#issuecomment-6624965663).

Ten cycles at N1 and N4 for each realization: 100 total samples. The first cycle is cold; the rest warm. N controllers coexist before removal. All operation error lists were empty; every node-gone stage completed. FD, thread, input-node and hidraw-node counts returned to baseline. Missing consumer-open observations retain EACCES. First-valid-neutral has no runner oracle; SDL was not requested for timing.

| Realization | N | Warm samples | API p50 ms | Close p50 ms | Close max ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| linux.uinput | 1 | 9 | 0.774 | 14.774 | 24.874 |
| linux.uinput | 4 | 36 | 0.415 | 14.190 | 19.548 |
| linux.uhid.usb | 1 | 9 | 0.168 | 83.287 | 94.593 |
| linux.uhid.usb | 4 | 36 | 2.223 | 99.533 | 202.708 |

Cold UHID N4 close had one 5507.519 ms sample. The warm table does not hide this tail. Concurrent build and research-validation workloads were active; this is not an isolated-host performance baseline. No latency budget or throughput claim is made. API return measures preparation/open; removal measures close through node disappearance with 1 ms polling. Kernel initialization and peer servicing can affect close duration. These historical measurements are not relabelled as final-head performance.

Reproduce with `cargo run --locked -p xtask -- acceptance --controller virtualgamepad.steam-controller-2026 --realization <linux.uinput or linux.uhid.usb> --cycles 10 --concurrency <1 or 4> --out <local-json>`. Keep raw host output local; it contains transient device paths.

## Functional observations

A separate one-cycle UHID run used private SDL `e0232c5b9d972ad394d597ce858185cd15a0f1a7` with command-scoped library/pkg-config paths and `SDL_JOYSTICK_HIDAPI_STEAM=1`. It observed an exact associated hidraw device, `hid-steam` binding, neutral, all six axis bits, changing gyro and accelerometer samples, and 37 touch events. The probe structured touch verdict remained missing because it tracks first-pad contacts only. It saw button down/up mask `0x300000`, not the earlier 14c full `0x3fffff` mask. Steam was running during this rerun; no causal attribution is established. Earlier #298 observations are retained as historical results, not substituted for the rerun. Full per-button live acceptance remains No evidence on this run.

SDL advertised rumble and submitted a rumble call. Controller-side output delivery/actuator effect is No evidence. The client listing/input test remains manual and untested.

Sysfs capability masks were observed, including the gamepad key/axis masks and hid-steam FF bits beyond FF_RUMBLE. Event decoding and ABS ioctl ranges/fuzz/flat/resolution remain No evidence: direct evdev opens returned EACCES. Driver binding and sysfs capability agreement do not demonstrate packet decoding. The [input-lab extension #299](https://github.com/anderstvoss/virtualgamepad/pull/299) is a separate owner-reviewed proposal; nothing was installed.

## Missing evidence and restrictions

- Both realizations: per-control kernel events, ioctl ABS attributes, full independent button decoding in the rerun, controller-side output delivery, Steam client listing/input/gameplay, identity reuse, physical serial format, physical effects/timing/calibration/firmware.
- UHID: feature 2 transactions, 0x44 ACK/flow control, output 0x86–0x89 streaming, real-device unknown-command responses, bootloader attach sequence, automatic input cadence, rumble auto-stop and lizard watchdog duration.
- Reduced uinput cannot expose motion companion properties/timestamp, independent pad pressure/contact and grip/stick touch metadata, vendor haptics/features or host-visible serial; axis fuzz/resolution are provider-limited. These are explicit restrictions, not evidence of equivalence.

## Local record digests

Raw files are retained locally; these SHA-256 digests identify the measured records without publishing host paths.

- `uinput-n1`: `4a0dd8e53456369295dff14d04208877fb459c95c329eac1dcda8c5db6505fb5`
- `uinput-n4`: `2a080cfa02cfa040579a4b12b779f9156c396e69a64bae029a6e440ef945a85d`
- `uhid-n1`: `8c0f4d5d9b658ce2453656844297191e45aad6cd371b2768ccd42d4481ad20d8`
- `uhid-n4`: `61c976085f03e794aecd30cff8ffc74e2e04e0b2241ef692360ca54986cf8fd5`
- `sdl`: `a7d9b29ea08c6909ec57dcc114b3f4768a317a24ec8165b82d27ea56c99fedf9`

## Manual Steam client procedure (owner, untested)

1. Use the reviewed C6 head on the prepared D9 host. Record commit, kernel,
   Steam build/channel and controller support settings. Close SDL probes and
   remove earlier virtual controllers; keep real input available for recovery.
2. Launch `cargo run --locked -p virtualgamepad-demo`. Select
   **Steam Controller 2026**, choose **linux.uhid.usb**, and press **Create**.
   Leave the demo running so its worker services feature requests.
3. Open Steam → Settings → Controller. Report whether a Steam controller is
   listed, its displayed name, and whether opening its input test succeeds.
   A missing listing is a result; do not change system policy to force it.
4. In the demo, press/release every face, D-pad, shoulder, trigger click,
   stick click, Menu/View/Steam/QAM and L4/L5/R4/R5 control individually.
   Sweep both sticks and triggers; move/touch/click both pads separately,
   then change raw gyro/accelerometer fields. Report each visible response,
   missing control, stuck state, axis polarity/range and unsupported display.
   A test screen without a motion display provides no motion evidence.
5. If Steam offers an identify/rumble action, invoke it and report the demo's
   retained/logged output request and any errors; no virtual hardware actuator
   effect is claimed. Do not send firmware/bootloader commands or use capture
   tools without separate authorization.
6. Remove only this controller in the demo. Confirm Steam removes its listing;
   create it again and report discovery/cleanup behavior. Then close the demo.
   Report leftovers and errors with sanitized summaries, never real serials,
   private paths or raw captures. Repeat reduced **linux.uinput** as a separate
   case, explicitly reporting its missing native HID controls/features.

The step 16 IBEX stub was not listed by Steam. That stub returned short synthetic
identity/attribute/settings replies. The current codec returns descriptor-sized
64-byte feature-1 frames: a per-instance synthetic or validated caller-set serial
for 0xae, the structural empty attribute response for 0x83, and a descriptor-sized
malformed command echo for unknown GETs after explicit unsupported SETs. Known
settings are retained, unknown setting IDs reject atomically, feature 2 remains
Unsupported, and no 0x47 input is emitted. These changes do not establish Steam
client acceptance or real-device reply fidelity.

## Puck feasibility (out of C6 scope)

C4's receiver/epoch infrastructure landed in #215; this generic dependency is
closed. Its synthetic 0-to-4-slot fixture is not a Puck implementation or evidence
for real receiver identities, slot protocol, pairing, reconnect or feedback routing.
Those device-specific gaps remain open. A direct wired 1302 controller uses one
unified HID interface; a single UHID node cannot be called faithful Puck USB.

USB receiver feasibility #168 is still open. It must establish whether USB/IP or
another provider can expose the required enumeration, interface topology, control
requests and exact success/error replies, interrupt endpoints and clean teardown
without falling back. UHID kernel binding for one fabricated Puck interface does
not answer that question. Step 16's single-node Puck had no matching SDL joystick,
despite exact-node access. Keep that negative result and the wired controller's
positive kernel/SDL tier decision separate.

The interface-count conflict is unresolved: OpenPuck describes four interfaces,
while the kernel consumer describes seven (0–1 non-HID, 2–5 slots, 6 pogo).
Consumer source agreement alone cannot settle physical topology. A separately
authorized physical descriptor/topology investigation and receiver protocol
research must settle transport/version differences and interface roles before a
faithful Puck plan. No receiver, Bluetooth, bootloader or streaming implementation
is introduced by this checkpoint.
