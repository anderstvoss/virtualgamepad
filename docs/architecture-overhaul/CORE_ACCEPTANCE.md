# Core acceptance matrix

The agreed review threshold is deterministic tests plus Linux and SDL evidence.
Steam and physical fidelity remain separate. No ignored test counts as a pass.
This matrix does not promote manifest support levels.

| Family / realization | Deterministic | Linux | SDL | Remaining core work |
| --- | --- | --- | --- | --- |
| DualSense USB/UHID | reports, ownership, retries, identity, lifecycle pass | startup, concurrent reused IDs and independent removal pass | baseline/rewrite controls, motion, touch, rumble/RGB and failure cleanup pass | repeat supported-host review; nonportable outputs remain explicitly scoped |
| DS4 USB/UHID | report/lifecycle regressions and injected pairing identity pass | three reused-ID sessions, independent removal and cleanup pass | not run in this batch | full control/sensor/output matrix and typed output parity review |
| Switch Pro USB/UHID | handshake, framing, cadence and lifecycle pass | not run in this batch | not run in this batch | consumer handshake/stream and feedback acceptance |
| Xbox standard HID/UHID | standard input and explicit limitations pass | not run in this batch | not run in this batch | SDL standard input matrix; no XInput/xpad/rumble claim |
| Existing uinput families | semantic/evdev parity regressions pass | historical basic provider test passed; current registration missing | family matrices pending | restore selected mechanism through existing administrator policy, then consumer parity |
| Existing compiled dummy_hcd | compiled-path regressions retained | reserved resources/configuration missing | pending | Gate G interface decision and live capability/startup/cleanup |

DS4 review found its feature address also used only the low 16 session bits.
It now opts into the same controller-owned OS entropy source as DualSense, before
provider open. Its immutable feature address is independently injected in tests;
other families do not acquire an entropy requirement automatically. The live DS4
test used IDs 7, 7 and 65543, observed three kernel identities/input devices,
removed the middle controller while the others serviced events, then removed all.
This does not establish DS4 SDL compatibility.

The current corpus pin remains remotely reachable and regeneration validation
passes. Corpus workflow tests retain absent/mismatched/unpublished checkout cases.
The repository has no corpus-read secret (name-only inspection); authenticated CI
is explicitly incomplete. Ordinary builds still use checked-in artifacts.

## Next implementation order

1. Finish DS4 consumer/output coverage using the exact-device SDL apparatus.
2. Apply the same checks to Switch Pro and standard-HID Xbox, then evdev parity.
3. Review demo readiness/deadline scheduling against measured requests and shutdown;
   change it only when its fixed scheduling violates the contract.
4. Resolve Gate G with explicitly reserved resources and supported metadata/completion
   authority. Missing permissions do not fix the inspected f_hid interface gap.
5. Begin extensions only after core review and each extension's declared gate.

See [EXP-0008](experiments/EXP-0008-sdl-core.md) for consumer/tool revisions and
measured limits. Remaining phases are incomplete; PR #106 stays draft.
