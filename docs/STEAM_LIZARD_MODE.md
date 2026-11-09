# Wired Steam desktop mode

`SteamController2026Options::lizard_mode` enables the captured desktop
mouse/keyboard mapping on `linux.uhid.usb`. Its default is `false` by the owner's decision;
[#311](https://github.com/anderstvoss/virtualgamepad/issues/311) tracks possible
refinement later. The demo's Advanced
options use the same creation-options default. Reduced uinput is unchanged: it
declares gamepad keys/absolute axes, with no desktop keys, relative mouse axes
or HID host-mode channel. This is a limit of this realization, not a general
uinput inability to represent pointer or keyboard events.

The mapping comes from the owner-guided D9 physical sessions recorded by
`steam-d9-lizard-control-mapping`, `steam-d9-lizard-sdl-transition` and
`steam-d9-lizard-relative-scaling`. These records were merged in
[research #110](https://github.com/anderstvoss/controller-protocol-research/pull/110)
and are adopted at research revision `4a887d4`.

| Control | Desktop input |
| --- | --- |
| Right pad movement | Relative mouse X and inverted Y |
| Right pad click / right trigger button | Mouse Button 1 |
| Left trigger button | Mouse Button 2 |
| Left pad movement | Horizontal pan and inverted vertical wheel |
| A / B / Options / View | Enter / Escape / Escape / Tab |
| D-pad | Arrow keys |

Trigger clicks use the existing semantic trigger button flags. Analogue trigger
values are not converted using a guessed threshold. Controls that produced no
desktop report in the individual capture windows have no invented mapping.

## Host state

With the creation option off, neither report 0x40 nor 0x41 is emitted. With it on,
unknown initial mapping/mouse state is treated as enabled: the startup capture
measured kernel restoration, not firmware power-on defaults. Command 0x81 clears
mappings, 0x85 restores mapping intent, and 0x8e restores unknown/default setting
intent. Setting 9 independently gates the mouse. In the normal SDL capture,
setting 9 = 0 stopped mouse output but A still emitted Enter. This distinction
supersedes treating that setting as a blanket mouse-and-keyboard disable.
Accepted mode changes clear the corresponding cached input history even when
disable/enable requests arrive before another input cycle. Rejected setting
batches preserve the history. If keys or mouse buttons were held, accepted
mapping resets queue one all-zero keyboard and/or button-release mouse report;
mouse-only clears release only held mouse buttons. Repeated resets before
polling coalesce these releases. A pending release forces input service even
without an application state change and bypasses the newly disabled desktop
gate once. The cleanup cycle does not immediately reassert the affected held
state. Nothing is emitted when nothing was held. This neutralization is
safe virtual policy: physical release behavior on these transitions is unmeasured.
No setting-71 watchdog is invented. Raw-client open/close alone does not change
protocol mode; the host's feature writes do.

On UHID Stop/device removal, Linux already releases held EV_KEY states (including
mouse buttons): `hid_hw_stop()` disconnects HID inputs before the low-level Stop,
and `input_disconnect_device()` sends key-up events before unregister completes.
See [HID shutdown](https://github.com/torvalds/linux/blob/c03c1a14507dfaf8b856c2608f9ac216021fc222/drivers/hid/hid-core.c#L2459)
and [input disconnect](https://github.com/torvalds/linux/blob/c03c1a14507dfaf8b856c2608f9ac216021fc222/drivers/input/input.c#L757).
The codec clears cached and pending desktop history at Stop/Start, so a release
from a removed device is never replayed into a restarted session.

## Compatibility limits

Right-pad relative counts use a 128-native-unit scale, and left-pad scroll uses
1024 units, with fractional accumulation and Y inversion. These are approximate
fits to captured interior strokes. They do not reproduce an established retail
filter, rounding phase, acceleration or edge clipping algorithm. Initial touch
establishes a baseline; lifting resets it. Large deltas are split into bounded
signed 8-bit reports without wrapping or dropping whole counts.

Keyboard state changes emit complete 9-byte packets; duplicate Escape mappings
are combined. More than six distinct simultaneous keys use standard keyboard
rollover reporting, a virtual policy not measured in the single-control capture.
Captured held mouse buttons repeated about every 4 ms. The virtual device uses
4-ms refreshes while a button is held; exact retail scheduling and filter jitter
are not claimed. Neutral idle has no refresh deadline. Native gamepad input and
haptic/feature handling remain independent.

Physical caveats include uneven slow movements, early motion in the first
right-pad center window, and the omitted reconnect in the first pad session.
The separate startup capture fills the USB/kernel attachment gap; it does not
establish autonomous timeout or firmware reset defaults.
