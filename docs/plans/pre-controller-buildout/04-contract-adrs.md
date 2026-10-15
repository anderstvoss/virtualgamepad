# Brief 04: contract ADRs for live topology, retained outputs, presentation epochs and receivers

Read `ground-rules.md` first.

**Lane:** Design (docs only, `docs/adr/`). One ADR per PR.
**Blocked by:** D1 for opening PRs (docs-only; Anders exempted the docs PR #175 last time, so ask whether ADRs get the same exemption). ADR-0021 also needs D3. ADR-0023 only if D4 says "ADR now".
**Runs alongside:** brief 02 (no shared files beyond `docs/adr/README` or index, if one exists).

Source: `reviews/post-alpha-1-architecture-reassessment.md`, sections A to G and the future-controller stress test. Each ADR follows the existing ADR format in `docs/adr/` (read ADR-0013 and ADR-0014 as the closest precedents) and states: context, decision, rejected alternatives, consequences, which family first needs it, and what the API snapshot gains **when that family ships** (nothing grows before then).

Numbering (ADR-0018 is reserved for the broker allowlist in brief 02 step 1e):

| ADR | Topic | First consumer | Implementation brief |
| --- | --- | --- | --- |
| ADR-0019 | Versioned live topology (A) | Wii extensions (#158) | 06 |
| ADR-0020 | Retained host outputs and typed output/input kinds (B, C) | every family; DualSense LEDs today | 05 |
| ADR-0021 | Logical controller vs host presentation, presentation epochs (E) | Steam lizard/native (#159), 8BitDo modes (#172) | 08 |
| ADR-0022 | Receiver aggregates with typed slots (F) | Steam Puck (#159, #165), Joy-Con pairs | 08 |
| ADR-0023 | Controller-owned bulk subsystems: displays, in-band speaker, NFC, IR camera (D) | Wii speaker, later VMU-class displays | none yet |

## Content each ADR must settle

**ADR-0019 live topology.** Static `InputTopology` stays the family's maximum shape. Per session: `topology_generation()` and `current_topology()` (active subset). A retained, poll-able change counter outside the 32-entry lossy observation queue. Edits to controls absent in the current generation fail with `UnavailableInRealization`. Changes go through `Runtime::update_protocol` (ADR-0014, EXP-0020). Say how uinput realizations express a generation change (static superset plus neutralize, or recreate) and which the Wii uses first; the Wii proposal on #158 (L1) lists both.

**ADR-0020 retained outputs and kinds.** `host_outputs()` returns the latest accepted value per output (lightbar, LEDs, trigger effects, rumble levels) plus a monotonically increasing revision; events stay for edges. Typed `OutputKind` (Rumble{motors}, ImpulseTrigger, Led{Mono|Rgb}, PlayerIndicator{count}, AdaptiveTrigger, Display{w,h,format}, Speaker{format}) next to the evdev-shaped `OutputSurface`. Input vocabulary gains pressure buttons, capacitive touch, relative 2D, point sets and load cells so `CustomInputModule` stops being the escape hatch. Note: `OutputSurface` and `InputTopology` are in the API snapshot today, so the public part of this needs D2.

**ADR-0021 presentation epochs.** Split logical controller (semantic state, identity, persistent resources) from host presentation (provider sessions). Controller-owned transition: prepare new, swap, close old, reusing compound preflight and rollback. `association()` gains an epoch number observed through a retained counter. `close()` stays terminal for the logical controller. Lizard mode stays controller-owned policy, never a `RealizationId` (#165).

**ADR-0022 receiver aggregates.** A receiver handle owns one host presentation and N typed slots; each slot is a logical controller with independent presence, feedback routing and teardown; the receiver owns the shared failure boundary. Registry entry `kind: Receiver` lists which families it hosts. Puck slot IDs are not GamepadManager player slots. Joy-Con pairs are the fixed two-slot case. Cover non-repeatable UHID commands versus the older compound full-frame retry model (#165).

**ADR-0023 subsystems.** Only if D4 says so. Components (out-of-band host endpoints like UAC, companions) versus subsystems (in-band protocol data like Wii ADPCM): explicit opt-in at creation, bounded controller-owned buffers, typed borrow like `controller.audio()`, never in `service()` callbacks or the observation queue.

## Done when

Each ADR is merged with status "Accepted" (or "Proposed" if Anders wants to review it later), `docs/STATUS.md` links it, and #176 has a one-line comment per ADR.
