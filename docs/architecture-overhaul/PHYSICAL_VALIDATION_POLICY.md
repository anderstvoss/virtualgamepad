# Physical validation scope and development gates

The user reports these physical references as available: DualSense, Xbox Series
controller, and Steam Controller. Availability is not verified attachment or a
completed test. The Steam Controller model/generation and connection mode must
be recorded before selecting a protocol profile. Xbox Series hardware is not an
Xbox 360 reference and cannot validate the existing standard-HID Xbox 360 profile
as XInput or physical Xbox 360 emulation.

All other families, including DS4 and Switch Pro, are best-effort for now. Continue
source-backed implementation, independent fixtures, deterministic regression tests
and available virtual Linux/SDL tests. Missing physical hardware does not block
that work or require removing those families. Keep unsupported behavior explicit;
best-effort does not turn a skipped test or a known failure into a pass.

## Where physical DualSense evidence gates progress

| Work | Physical DualSense requirement | What can proceed independently |
| --- | --- | --- |
| Core state/personality/runtime, exact replies, lifecycle, queues and identity | None for implementation and deterministic acceptance | Continue all runtime fixes, provider-neutral contracts, concurrency and packaging |
| Existing USB/UHID Linux/SDL core | None for repeating virtual input/output and cleanup tests | Recorded virtual acceptance remains valid within its measured profile; isolate touch consumers before further injection |
| USB fidelity and B/P physical-reference axis | Needed to claim real-versus-virtual equivalence of descriptors/features, sensor calibration/timing, touch and output behavior | Controlled virtual bus/driver/consumer comparisons can continue; physical hardware alone does not resolve confounded driver experiments |
| Adaptive triggers, advanced haptics and other nonportable output claims | Physical observation or independently sourced physical evidence is needed for validated behavior/fidelity | Source-backed parsing, typed events, synthetic stress and explicitly experimental implementations |
| Gate F host audio | Not a prerequisite for virtual audio lifecycle/coherence experiments | User-session audio prototype and multi-controller cleanup; no physical audio equivalence claim |
| Gate H USB audio | The gate explicitly requires physical DualSense descriptors/topology, formats, endpoints and alternate settings in the corpus | Generic transport/composition research; no dependent DualSense USB-audio support promotion before those facts and host comparisons pass |
| Gate L Bluetooth personality | The gate requires physical BT fixtures; the available controller can supply them through a separately scoped experiment | Source-backed personality/CRC/framing tests; no validated BT profile without independent evidence |
| Gate M actual Bluetooth | Depends on L and isolated radio/host capability evidence; the controller alone is insufficient | Read-only feasibility review; pairing/reconnect acceptance waits for its dedicated setup |
| Gate G gadget broker | Physical DualSense does not fix missing kernel request metadata/completion authority | Interface review and deterministic broker tests; live gadget work still needs reserved resources and its own capability evidence |

Physical testing gates dependent acceptance claims and evidence-driven protocol
changes, not the whole architecture program. Physical comparison may expose a
bug that then requires a deterministic regression and a production fix.

## Next physical DualSense experiment

First record the exact model, firmware where observable, connection mode, driver,
consumer version and available reference interfaces. Start with read-only USB
identity/descriptors and narrowly selected consumer observations. Compare against
the already measured virtual profile with the same settings; vary bus and driver
independently when testing their necessity. Capture only if a declared question
cannot be answered by ordinary diagnostics, on an isolated device/bus. Keep raw
records private and review sanitized provenance before corpus adoption.

This preparation is separate from resolving the interrupted DS4 contact test.
Do not repeat virtual contact injection into the active desktop; use an isolated
consumer environment as required by EXP-0012. No hardware tests were run when
this policy was recorded.

## Attached reference update

[EXP-0015](experiments/EXP-0015-reference-layouts.md) confirms a USB DualSense
attachment and read-only report/topology observations. The user reports broken
trigger modules and stick drift: exclude trigger actuation/response and neutral
stick fidelity from this device's acceptance role. It can still provide protocol
and topology evidence. Xbox Series and Steam Controller were not enumerated in
this VM at that inspection; DS4 is explicitly unavailable. Use pinned OpenPuck
source evidence for DS4 and HHD for applicable DualSense layout checks, retaining
source lineage and differences. Compatibility implementations do not substitute
for physical observations when a gate specifically requires them.
