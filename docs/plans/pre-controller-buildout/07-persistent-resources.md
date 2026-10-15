# Brief 07: controller-owned persistent resources, production seam (#162)

Read `ground-rules.md` first, then the audit comment on #162 (comment 6163240564), which is the detailed spec.

**Lane:** Runtime, alongside brief 06 if the two avoid the same files (agree on `common/session.rs` ownership: 07 owns the close/disposition path, 06 owns topology).
**Blocked by:** D1; brief 02 step 1a merged.
**Needed by:** Wii Remote EEPROM lifecycle only (#158). Base Wii input, report modes and buttons are not blocked. Also the future home for Switch SPI and profile storage, without a generic memory trait.

## What exists (do not rebuild)

ADR-0013 and EXP-0019: 7 deterministic tests in `crates/gr-hid/tests/persistent_resource.rs` with a synthetic 256-byte image (import, host writes once before replies, retries keep bytes, clone isolation, terminal snapshot, KeepSnapshot/Discard). `Runtime::close` is terminal and exposes `cleanup_error()`; `Drop` never exports. The production `ControllerSession<D>` has **no** resource disposition or snapshot API yet.

## 07a. Minimal production seam

- Use the `cfg(test)` family from brief 02 as the concrete owner: give it a protocol-owned value resource (synthetic bytes, not Wii data).
- Add only what the session needs: validated import **before** any provider opens (rejects leave no host nodes); explicit disposition after deliberate or error close (`KeepSnapshot` returning an exact immutable snapshot, or `Discard`); first disposition wins, repeats are defined; `Drop`, host STOP/START and consumer OPEN/CLOSE never persist.
- Resource dirty is tracked separately from `Runtime::is_dirty()` with EXP-0019's sticky meaning.
- Write down which core files changed and why; if `gr-hid` needed no change, say so.

## 07b. Optional file helper (only if 07a shows a caller needs it)

Caller-chosen paths only (never from a device request), atomic replace via temp file plus same-filesystem rename, rollback on failure, a failed export leaves the snapshot intact. State whether fsync is part of any durability claim.

## Tests

The regression list in the #162 audit comment ("Minimum regressions"), minus Wii-specific address maps, which belong to the Wii package. Include a stateless control family proving no overhead for families with no resources.

## Done when

07a merged with a #162 comment naming the integration recipe Wii will follow; 07b merged or explicitly skipped with a reason; #176 ticked.
