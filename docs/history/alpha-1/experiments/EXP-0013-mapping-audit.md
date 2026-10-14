# EXP-0013 — Source-backed input mapping and demo range audit

This deterministic review follows the user's report of minor realization mapping
issues. No physical controller or live desktop test was performed. Families
without reference hardware remain best-effort; these fixes do not promote their
physical fidelity or close the interrupted DS4 split-node experiment.

## Corrections

| Surface | Previous behavior | Corrected behavior and regression |
| --- | --- | --- |
| DS4 and DualSense USB input | Analog trigger values were encoded but L2/R2 button bits stayed clear | Button bits 2/3 now follow nonzero trigger values, matching the existing evdev policy; independent left/right press, full range and release tests |
| DS4 USB touch | Releasing both contacts advertised zero touch reports, so a count-gated host could retain its old contact state | Always send one current touch sample, including inactive contacts; fake host checks two contacts, individual release, total release and repeated neutral on UHID and dummy_hcd framing |
| Sony demo stick conversion | Value zero overflowed the signed conversion; negative full scale was unreachable | Piecewise conversion preserves 0/128/255 and the signed endpoints; all 256 values round-trip and the entire i16 domain is monotonic |
| Demo signed stick pad | Pointer motion only reached -32767 | Negative edge reaches -32768 while positive edge remains 32767; bounded endpoint/neutral regressions |
| Demo face labels | Only spatial names were shown | Printed names plus positions, including Nintendo B/South, A/East, Y/West, X/North; wire assignments remain unchanged |

The trigger-bit positions and count-gated DS4 touch parsing are documented by the
[Linux v6.12 PlayStation driver](https://github.com/torvalds/linux/blob/v6.12/drivers/hid/hid-playstation.c).
The nonzero trigger threshold is the library's existing evdev policy, not a newly
measured physical actuation threshold. The old DS4 combined-control fixture is
updated to include both trigger bits while preserving its other byte assertions.

Existing Switch face, d-pad, stick-Y and evdev mapping regressions remain intact;
no speculative swap was applied to compensate for an unspecified consumer's
mapping database. Xbox retains the explicit standard-HID/legacy-evdev limitations.
See EXP-0011 for the prior native mapping evidence.

## Remaining evidence

Repeat consumer checks one control at a time with the exact family, realization,
SDL mapping and bus recorded. The older simultaneous button sweep alone cannot
prove labels or axis direction. Physical DualSense testing remains a fidelity
axis; unavailable DS4/Switch references do not block these deterministic repairs.
Touch consumer experiments still require isolation. These corrections do not
establish the cause of the earlier display-session crash.
