# Brief 06: live topology for hot-plug extensions (#163, re-assessment A)

Read `ground-rules.md` first.

**Lane:** Runtime. **Blocked by:** D1; brief 02 step 1a merged (for `max_topology`); ADR-0019 (brief 04) merged before 06b.
**Needed by:** Wii Remote extensions (Nunchuk, Classic/Pro, MotionPlus pass-through, #158). Also headset presence and accessory slots later. **Not** needed for the base Wii Remote, Xbox Series or wired Steam.

## 06a. Audit the existing seam (#163's own work)

`Runtime::update_protocol` (`crates/gr-hid/src/lib.rs:262`, ADR-0014) and the 6 synthetic tests in `crates/gr-hid/tests/internal_topology.rs` (EXP-0020) already provide transactional typed attach, detach and reinitialize. Do not reimplement them. Add the missing tests #163 lists, at the `gr-hid` seam:

- accepted queued bytes and replies are never rewritten by a later topology change;
- rapid plug/unplug, queued edge saturation, invalid transitions, malicious register writes;
- one protocol personality with a changing logical extension;
- host probe overlapping a change; terminal close mid-change;
- non-repeatable sends never retried ambiguously.

If every test passes with no core change, write that up on #163 as "no core change needed" with the test list. If one fails, fix only that deficiency, with the failing test as the regression.

## 06b. Public live topology (needs ADR-0019; the snapshot grows only when Wii ships)

- Per session: `topology_generation()` and `current_topology()` as a subset of the family's `max_topology`.
- A retained generation counter outside the lossy observation queue.
- Edits to controls absent in the current generation fail with `UnavailableInRealization`.
- Prove it with the `cfg(test)` family from brief 02: give it one optional extension module and test attach, detach, rejected edit while absent, counter monotonicity, uinput behaviour (the ADR's chosen policy) and close during a change.
- Keep the API crate-internal until the Wii family exposes it; record the planned root additions in the PR.

## Done when

#163 closed (or left open only for the Wii production integration recipe), 06b merged, #176 ticked.
