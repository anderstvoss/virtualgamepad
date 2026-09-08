# EXP-0014: individual evdev control observations

## Scope and apparatus

Linux 6.12.107 arm64, SDL 3.2.0 (source revision
`535d80badefc83c5c527ec5748f2a20d6a9310fe`), private user-owned tooling.
The Xbox 360 and Switch Pro evdev profiles were selected by their exact newly
created event node, with HIDAPI disabled for this experiment. No physical device,
touch injection, capture, module change or new permission was needed.

Each family used three sequential creations with application IDs 7, 7 and 65543.
Each creation tested neutral, all 15 standard SDL buttons, both endpoints of
four stick axes, and both trigger endpoints. Every case was followed by neutral:
52 observations per creation, each observed for 500 ms. The selected state must
remain correct for the final 50 ms, with every unrelated button and axis neutral.
The parent services the controller while the consumer runs and enforces a timeout.
Versioned JSON includes exact identity, SDL version, final button mask and axes,
timing, errors and cleanup. Raw records remain outside Git.

## Results

| Profile | Exact-state observations | Creations / clean removals | Result |
| --- | --- | --- | --- |
| Xbox 360 evdev | 156 | 3 / 3 | pass |
| Switch Pro evdev | 156 | 3 / 3 | pass |

Every child was reaped, each session event node disappeared, and repeated close
succeeded. No additional production mapping correction is justified by these
results. Existing output/FF acceptance remains separate; this profile does not
submit rumble or lightbar requests or require motion observations.

## Apparatus corrections and rejected evidence

The first Xbox attempt held a trigger before SDL opened the device; SDL observed
neutral after startup calibration. The probe now emits an explicit ready record
and the parent applies the case only after opening a neutral controller. This is
an apparatus startup correction, not evidence of a production trigger defect.
A preliminary Switch attempt was invalidated by rebuilding the probe executable
while the run was active. Only the subsequent complete runs using an unchanged
binary support the table above. Deterministic C contract tests reject cross-wired
axes, extra buttons, absent activation and invalid case arguments without SDL.

## Remaining boundaries

This closes the individual-control evidence gap for these two evdev profiles
only. It does not validate Xbox Series/XInput, physical Switch fidelity, touch,
auxiliary nodes, or individual-control mappings on other realizations. DS4 split
touch adoption remains blocked: no isolated VM or console environment is available,
and the interrupted desktop experiment must not be repeated on the active desktop.
The demo shared-controller mutex and Gate G metadata/completion gap remain open.
