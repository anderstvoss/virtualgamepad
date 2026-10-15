# Brief 05: retained host outputs and typed output/input kinds (re-assessment B and C)

Read `ground-rules.md` first.

**Lane:** Core, after brief 10 step 2 (facade), because every family root type is touched there.
**Blocked by:** D1, brief 02 merged, ADR-0020 (brief 04) merged, and **D2** (grow the API snapshot). Without D2, only 05a runs and stays SPI-only.
**Needed by:** every family under the "every hardware feature" target (LEDs, lightbar, trigger effects are features games set); the demo (brief 10 step 5) and acceptance runner (brief 09) read the kinds.

## Problem

Host-written outputs reach the application only through the bounded observation queue (32 entries, `dropped_output_events()` in `gr-curated-controllers/src/common/session.rs:47`). A burst of host writes can evict the latest lightbar value. Output metadata is evdev-shaped (`OutputSurface { name, event_type, event_code }`, `gr-controller-contract/src/lib.rs:710`), and the input vocabulary has no pressure buttons, capacitive touch, relative 2D, point sets or load cells, so new controllers would hide controls in `CustomInputModule`.

## 05a. Retained output state below the root (no snapshot change)

- Each family keeps a `FamilyOutputState` updated on every accepted host output: latest lightbar/LED/player indicator, trigger effect, rumble levels, plus a monotonically increasing `revision`.
- Expose it on the curated session (crate-level, not root). Events stay for edges and diagnostics.
- Tests per existing family: a burst larger than the queue still leaves the latest value retained; the revision increases exactly once per accepted change; rejected or malformed outputs do not change it; close keeps the last state readable.

## 05b. Root `host_outputs()` (needs D2)

- Add `host_outputs()` to each family root type through the step 2 facade macro, returning the retained state and revision.
- Regenerate the API snapshot with the reviewed addition, and list the added items in the PR body for Anders's review.

## 05c. Typed kinds in the contract (needs D2 for the public part)

- Add `OutputKind` (Rumble{motors}, ImpulseTrigger, Led{Mono|Rgb}, PlayerIndicator{count}, AdaptiveTrigger, Display{w,h,format}, Speaker{format}) alongside `OutputSurface`, filled for the four existing families and the registry's `output_kinds` stub from brief 02.
- Extend the input vocabulary with pressure button, capacitive touch, relative 2D, point set and load cell kinds. Existing families need none of them yet; add them with unit tests and leave them unused until Wii (IR point set), Steam (capacitive, pressure) or Switch 2 (relative) use them.
- Keep any type that would enter the snapshot `#[non_exhaustive]`.

## Done when

05a merged with tests; 05b and 05c merged if D2 is yes; `docs/APPLICATION_API.md` documents the retained-versus-event rule; #176 ticked.
