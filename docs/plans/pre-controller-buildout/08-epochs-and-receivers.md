# Brief 08: presentation epochs and receiver aggregates (#165, re-assessment E and F)

Read `ground-rules.md` first.

**Lane:** Runtime, after brief 06 and 07 settle the session files.
**Blocked by:** D1; brief 02 merged; ADR-0021 (needs **D3**) and ADR-0022 merged; brief 03's tier decision (it decides whether the Puck work also needs #168).
**Needed by:** Steam lizard/native mode and companions, and the Puck's four slots (#159). Later 8BitDo mode switches (#172) and Joy-Con pairs. **Not** needed for a wired Steam Controller with lizard mode off, so that family can start before this brief finishes.

## 08a. Presentation epochs (ADR-0021)

- Separate the logical controller (semantic state, identity, persistent resources) from its host presentation (provider sessions).
- A controller-owned transition prepares the new presentation, swaps, closes the old, reusing compound preflight and rollback (`docs/spec/implementation/COMPOUND_CONTROLLER_SESSIONS.md`). A failed prepare leaves the old presentation untouched.
- `association()` gains an epoch number; a retained counter (outside the observation queue) signals the change.
- `close()` stays terminal for the logical controller.
- Tests with the `cfg(test)` family: swap success, prepare failure, failure mid-swap, close during swap, retained state and identity carried across, transport counters and queues reset.

## 08b. Receiver aggregate with a synthetic 0-to-4-slot fixture (#165)

- A receiver handle owns one host presentation and N typed slots; each slot has independent presence, identity, feedback routing and teardown; the receiver owns the shared failure boundary.
- Registry `kind: Receiver` (brief 02 stub) lists the families a receiver hosts.
- Optional, capability-limited keyboard and pointer companions, created and removed transactionally. A `LizardOnly` mode suppresses native presentation without losing retained native state. Lizard mode stays controller-owned policy, never a `RealizationId`.
- Assess non-repeatable UHID commands against the older compound full-frame retry model; if they conflict, design only the minimal migration.
- Synthetic fixture first. Real Puck bytes, identities and slot protocol stay in the Steam package and the research repo.

Completion evidence (from #165): deterministic 0-to-4-slot fixture with optional companions, simultaneous changes, rollback, required requests, close/failure and sibling survival.

## Done when

08a and 08b merged; #165 closed with a comment; #176 ticked. Public API stays crate-internal until the Steam family ships.
