# EXP-0021 — Native controls and exposure (Gate T)

Question: Can typed state, target surfaces and corpus reporting separate physical,
raw, semantic, realization, Linux, SDL and remapping facts?
Prerequisites: current corpus schema; pinned SDL sources and vendor documentation.
Experiment: retain existing claim/source schemas, add scoped claims in companion
corpus revision `0468aeb` ([companion PR](https://github.com/anderstvoss/controller-protocol-corpus/pull/2)), and exercise test-only typed state/surface restrictions
in `gr-controller-contract/tests/control_exposure.rs`.

| Case | Raw/control evidence | Semantic admission | Realization/Linux | SDL | Profile / physical evidence |
| --- | --- | --- | --- | --- | --- |
| DualSense Edge function/paddles | SDL PS5 decoder byte 2 masks 0x10/0x20/0x40/0x80 | Test-only native names preserve all four controls; no production package | Actual binding/events not measured; pinned kernel parser does not forward these bits | Pinned 3.2.0 default HIDAPI mapping includes all four controls; active mapping not measured | Physical corroboration unknown |
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

## Follow-up investigation — 2026-09-10

The earlier report conflated incomplete source research with unavailable physical
observations. [SDL's pinned mapping generator](https://github.com/libsdl-org/SDL/blob/535d80badefc83c5c527ec5748f2a20d6a9310fe/src/joystick/SDL_gamepad.c)
does specify the Edge default: `paddle1:b16,paddle2:b15,paddle3:b14,paddle4:b13`.
Combined with the PS5 decoder this maps right paddle, left paddle, right function
and left function respectively. This resolves the default-source mapping gap;
user mappings can still override it. The [existing pinned kernel parser](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/hid/hid-playstation.c)
handles home, microphone and touchpad in buttons[2] but does not forward the four
auxiliary bits. That establishes a source-path difference, not the selected
driver or actual Linux events of a physical Edge.

The [Pro 2 FAQ](https://support.8bitdo.com/faq/pro2.html) establishes macro
functionality but gives no mode/firmware-scoped raw report proving remap-only
exposure. Documentation for the separately named Pro 2 Wired model cannot fill
that gap for this case. Retain `pro2-independent-raw-unknown`; changing it to
absent or admitting independent semantic paddles would invent evidence.

Read-only host inventory found virtual mouse/keyboard devices and no physical
gamepads. No reference input, output actuation or host-policy changes were made.
The companion corpus adds scoped default-mapping, parser and backend-signature
claims; its existing Linux/physical unknowns remain. Gate T stays blocked on the
unambiguous contrasting exposure/remap-only evidence and corpus adoption, rather
than requiring every physical device before accepting the representation tests.
