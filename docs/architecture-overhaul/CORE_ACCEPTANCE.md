# Core acceptance matrix

The agreed review threshold is deterministic tests plus Linux and SDL evidence.
Steam consumer and physical fidelity remain separate. The available physical
references are DualSense, Xbox Series and Steam Controller; all other families
are best-effort for now. Missing reference hardware does not block independent
implementation or virtual testing. See [physical validation policy](PHYSICAL_VALIDATION_POLICY.md)
for the precise DualSense dependencies and model distinctions. No ignored test counts as a pass.
This matrix does not promote manifest support levels. Earlier UHID/SDL results
were recorded on kernel 6.12.105; EXP-0010 uses the currently running 6.12.107.
EXP-0011 repeats all four rewrite UHID/SDL profiles on 6.12.107 successfully.
The baseline comparison and earlier fault-injection evidence remain tied to their
recorded kernel; no new baseline or physical-comparison pass is inferred.

| Family / realization | Deterministic | Linux | SDL | Remaining core work |
| --- | --- | --- | --- | --- |
| DualSense USB/UHID | reports, ownership, retries, identity, lifecycle pass | startup, concurrent reused IDs and independent removal pass | baseline/rewrite controls, motion, touch, rumble/RGB and failure cleanup pass | repeat supported-host review; nonportable outputs remain explicitly scoped |
| DS4 USB/UHID | report/lifecycle regressions and injected pairing identity pass | three reused-ID sessions, independent removal and cleanup pass | three ten-second controls, touch, sensors, typed rumble/RGB runs pass | remaining evdev and family failure/concurrency review |
| Switch Pro USB/UHID | handshake, framing, cadence and lifecycle pass | startup/removal during three SDL runs pass | three ten-second control/motion runs pass | compressed-rumble feedback fidelity and concurrency |
| Xbox standard HID/UHID | standard input and explicit limitations pass | startup/removal during three SDL runs pass | three ten-second standard control runs pass after axis fix | concurrency; no XInput/xpad/rumble claim |
| DualSense/Switch/Xbox uinput | individual button mapping and bounded FF regressions pass | four-family FF/consumer-exit cleanup pass | three control/neutral/range/typed-rumble runs each pass | touch/auxiliary association and remaining failure/concurrency coverage |
| DS4 uinput | touch retained, mapping/FF regressions pass | FF and cleanup pass | fails gamepad discovery: combined node classifies as touchscreen | prove separate gamepad/touch presentation and compound ownership |
| Existing compiled dummy_hcd | compiled-path regressions retained | reserved resources/configuration missing | pending | Gate G interface decision and live capability/startup/cleanup |

DS4 review found its feature address also used only the low 16 session bits.
It now opts into the same controller-owned OS entropy source as DualSense, before
provider open. Its immutable feature address is independently injected in tests;
other families do not acquire an entropy requirement automatically. The live DS4
test used IDs 7, 7 and 65543, observed three kernel identities/input devices,
removed the middle controller while the others serviced events, then removed all.
Subsequent SDL results are recorded in [EXP-0009](experiments/EXP-0009-family-sdl.md).

The current corpus pin remains remotely reachable and regeneration validation
passes. Corpus workflow tests retain absent/mismatched/unpublished checkout cases.
The repository has no corpus-read secret (name-only inspection); authenticated CI
is explicitly incomplete. Ordinary builds still use checked-in artifacts.

## Next implementation order

1. Resolve DS4 combined-node discovery and validate touch/auxiliary association. [EXP-0011](experiments/EXP-0011-evdev-sdl.md) records three passing evdev families and the precise DS4 failure; [EXP-0010](experiments/EXP-0010-evdev-feedback.md) covers required FF completion.
   The compound runtime now routes exact component replies and preserves records
   delivered before read failure, with partial snapshot retry and independent
   cleanup regressions. The DS4 split-node prototype is test-only after a reported
   display-session crash interrupted live acceptance; [EXP-0012](experiments/EXP-0012-ds4-compound-interruption.md)
   requires isolated consumer validation before production adoption.
2. All four HID families now have interleaved three-session regressions covering
   reused IDs, repeated exact SET rejection, arbitrary removal, partial-read
   failure and terminal reopen prevention. Continue remaining concurrency axes;
   Switch physical compressed-rumble fidelity remains best-effort/unvalidated.
3. Review demo readiness/deadline scheduling against measured requests and shutdown;
   change it only when its fixed scheduling violates the contract.
4. Resolve Gate G with explicitly reserved resources and supported metadata/completion
   authority. Missing permissions do not fix the inspected f_hid interface gap.
5. Begin extensions only after core review and each extension's declared gate.

See [EXP-0008](experiments/EXP-0008-sdl-core.md) for consumer/tool revisions and
measured limits. Remaining phases are incomplete; PR #106 stays draft.
