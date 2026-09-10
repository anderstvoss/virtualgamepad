# EXP-0021 — Native controls and exposure (Gate T)

Question: Can typed state, target surfaces and corpus reporting separate physical,
raw, semantic, realization, Linux, SDL and remapping facts?
Prerequisites: current corpus schema; pinned SDL sources and vendor documentation.
Experiment: retain existing claim/source schemas, add scoped claims in companion
corpus revision `e677a26` ([companion PR](https://github.com/anderstvoss/controller-protocol-corpus/pull/2)), and exercise test-only typed state/surface restrictions
in `gr-controller-contract/tests/control_exposure.rs`.

| Case | Raw/control evidence | Semantic admission | Realization/Linux | SDL | Profile / physical evidence |
| --- | --- | --- | --- | --- | --- |
| DualSense Edge function/paddles | SDL PS5 decoder byte 2 masks 0x10/0x20/0x40/0x80 | Test-only native names preserve all four controls; no production package | Not measured, no target support claim | Separate joystick slots in pinned 3.2.0 decoder; exact gamepad mapping not measured | Physical corroboration unknown |
| Switch 2 Pro paddles | Newer SDL decoder data[8] 0x01/0x02 | Source supports native-control research; no production package | Not measured | Separate Pro paddle slots in newer source; not baseline SDL 3.2.0 support | Firmware/physical corroboration unknown |
| 8BitDo Pro 2 P1/P2 | Raw independence unknown | Do not admit independent paddle state from macro documentation | Not measured | Not measured | Vendor describes macro controls; no mode-specific proof of raw absence |

Evidence: corpus source/claim IDs `edge-independent-raw`, `edge-sdl-decoder`,
`switch2-independent-raw`, `switch2-sdl-decoder`, `pro2-physical-macro-controls`,
`pro2-independent-raw-unknown` and separate Linux/physical unknown records.
The corpus adds three tests guarding source lineage, excerpt hashes and unknowns.
The core adds two deterministic typed-state/admission-policy examples. Existing
`TargetRestriction` carries the missing mapping and technical reason; no generic
optional control map or mutable universal state is necessary.

Result: representation tests pass. Full Gate T evidence acceptance remains
**blocked**: the requested real host-visible/backend-dependent and mode-specific
remap-only cases have not been demonstrated. Synthetic cases exercise those
representations but cannot replace that evidence. Core corpus adoption also waits
for companion publication on main; the current pinned corpus is unchanged.

Decision: preserve layered reporting and explicit unknowns; no schema extension.
Consequences: sources cannot promote support cells. Do not interpret a marketing
button or SDL joystick slot as an independently observed event on every backend.
Revisit condition: exact firmware/mode/kernel/SDL observations resolve these gaps.
No physical tests, privileges, dependencies or production controls were added.
