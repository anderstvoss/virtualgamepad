# Alpha GUI control inventory

Gate 1 covers the production demo and its implemented supporting input views.
This inventory separates deterministic input/focus evidence from the complete
interactive acceptance that remains required. Source names below are portable;
raw host receipts and temporary profiles must stay outside the repository.

| Control boundary | Expected keyboard behavior | Current deterministic evidence | Remaining acceptance |
| --- | --- | --- | --- |
| Creation family, realization, optional name, clear-name button, count, Create | Tab traversal; select/edit/activate; unsupported target reason stays visible | Target-surface/layout tests; real keyboard realization-help rendering and adjacent traversal; count-arrow focus/name regression; clear-name Tab/Space regression; bounded creation count | Owned-display traversal, descriptive names for all fields and symbol buttons, unavailable target help |
| Controller rows and per-controller removal | Select any position, remove one, preserve nearest selection/sibling service | `selection_and_removal_preserve_positions_beyond_the_viewport`; both removal-selection tests; `symbol_buttons_describe_and_activate_keyboard_actions_beyond_the_viewport`; `removing_one_worker_preserves_another_workers_service` | Production remove-button Tab/Space regression removes exactly controller 23 from a 28-row scroll and preserves neighbors; full owned-display row selection/removal remains required |
| Controller rename, Apply, service log-period field | Edit/activate with visible focus; no service interruption | Service-gap/log bounds and worker lifecycle tests | Complete keyboard traversal and field identification |
| Auxiliary, face and digital trigger buttons; per-group Hold | Space/Enter activation; momentary release; Hold toggle/release | Quick button-frame/hold/release tests; `trigger_keyboard_traversal_identifies_each_control` exercises production stack | Actual press/hold/release at all family controls, descriptive group context |
| Stick and D-pad pads, click buttons and Reset | Focused WASD; neutral on release/focus loss unless held; buttons keyboard-operable | `focused_pad_handles_keyboard_and_neutralizes_on_focus_loss`; keyboard D-pad exact transitions and pad bounds tests | Owned-display focus outline, traversal and click/reset behavior |
| Analog triggers | Arrow adjustment; neutral on release/focus loss unless held; identify each trigger | Real slider key/release test; production stack Tab/name/neutral regression | Numeric entry, both trigger stacks and disabled states in display |
| Touch canvas, contact selector, contact Hold, relative/multitouch toggles, click/deflection, Reset, lockout toggle/duration | Space down/up; bounded WASD; selected contact only; Hold retains; safe focus/reset/lockout release | Real canvas key/focus/Tab tests; selected-contact/relative/lockout/clamping/release regressions | Whole-second lockout keyboard regression covers exact increment, both bounds, disabled no-op, names and adjacent traversal; complete control traversal, expiry display and isolated live input application remain required |
| Motion six-axis sliders and Hold | Arrow changes simulated values; momentary reset on release/focus loss | Real motion slider release/focus regression and scaling/neutral tests | All six names/focus, numeric entries and complete traversal |
| Supporting extra one/two-dimensional axes | Named slider/pad; adjustment/reset in declared range | Synthetic surface and pad-range tests; `extra_axes_keyboard_traversal_names_controls_and_emits_exact_values` | Real Tab/Space/WASD regression now identifies the declared slider, reset and pad names, exact reset/movement and focus-loss neutralization; owned-display verification remains required even when no curated controller declares them |
| Battery exposure, level slider and percentage entry | Toggle/edit without mouse; unavailable state remains explicit | Battery visibility/layout/native rail tests | Real Tab/arrow traversal now verifies named slider/entry, exact percentage changes, disabled no-op and adjacent focus; owned-display exposed states remain required |
| Audio enable/backend/ownership creation options and information control | Select/activate via keyboard; validation/loading/error reason accessible | Creation capability/ownership/backend validation tests; real Tab-only audio-information rendering with unchanged options | Complete option selection and field identification; root-installed transport acceptance separate |
| Audio routing devices/channels, jack types, linking/microphone toggles, device refresh/retry and copy selectors | Keyboard routing/refresh; no discovery blocking service; disconnected selections remain visible | Route/backend/native-ownership/channel/disconnected-surface tests; stalled-discovery removal/shutdown regression | Complete keyboard route selection, loading/error/retry display and isolated live route transitions |
| Outputs, diagnostics and error/log displays | Readable status, no controls invented for read-only observations | Display-backlog bounds, fixed-height audio diagnostics, focus-anchored information and focus-loss dismissal tests | Owned-display readability and error/loading/output presentation |

The keyboard touch/motion actions operate the demo's virtual input controls;
they do not describe physical controller gestures or sensor rest values. Keep
those explanations visible in the production GUI. Deterministic rendering does
not establish full assistive-technology compatibility: the demo's current eframe
feature selection and native accessibility backend require separate scrutiny.

## Completion evidence still required

Run the complete interactive suite in an owned display. Active touch must use
the reviewed isolated input seat; never inject contacts into the shared desktop.
Capture exact generated events, adjacent focus traversal, disabled/error/loading
states and controller removal. Then run the final byte-pinned two-hour neutral
creation/removal soak with no overlapping audio, Steam or compilation. Explain
RSS/descriptors/threads/children/node trends and verify complete owned cleanup.
Passing focused tests does not close this gate.
