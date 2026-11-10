# Post-checkpoint follow-up disposition

Owner scope: [#176 comment 6311929600](https://github.com/anderstvoss/virtualgamepad/issues/176#issuecomment-6311929600).
C3 (#209), C4 (#215) and the research pin (#214) are merged to main.

The five small follow-ups are separate PRs to main:

1. [#216](https://github.com/anderstvoss/virtualgamepad/pull/216): checkpoint
   bookkeeping, issue evidence updates and draft #211 retargeting.
2. [#217](https://github.com/anderstvoss/virtualgamepad/pull/217): slot-local
   companion failure, sibling survival, queued replies and cleanup regressions.
3. [#218](https://github.com/anderstvoss/virtualgamepad/pull/218): gadget tag/length
   invariants, fallible audio tag lookup and production Bluetooth-bus selection.
4. [#219](https://github.com/anderstvoss/virtualgamepad/pull/219): retained-view
   documentation, stale comment removal and correct acceptance peer-error ownership.
5. This disposition report and the editable #176 status comment.

Each PR carries its own required validation and self-review. D10 permits agent
merge only with green current-head checks, no unresolved blocking feedback and
at least two hours ready. PR readiness is not a claim that it has merged.

[#165](https://github.com/anderstvoss/virtualgamepad/issues/165) stays open until
both #217 has merged and a receiver handle exists in a crate outside `tests/`.
The failure-isolation PR remains a synthetic proof; it does not supply that handle.

## Checkpoint disposition

| Checkpoint or decision | Current state |
| --- | --- |
| C5 · D6/D7 | Wii Remote shipped on uinput and local Bluetooth-bus UHID in [#233](https://github.com/anderstvoss/virtualgamepad/pull/233), `714958d`. Speaker and actual Bluetooth stay deferred. |
| C6 · D9 | Step 16 gate closed. Wired Steam Controller 2026 shipped in [#301](https://github.com/anderstvoss/virtualgamepad/pull/301), `ad80ae6`. [D9 results](../../C6_D9_ACCEPTANCE.md) retain their individual scopes. |
| C7 · Alpha API | [#329](https://github.com/anderstvoss/virtualgamepad/pull/329), `692f41b`, converged all six constructors, promoted Wii and Steam to the root, and updated the root snapshot to 155 items per feature view. [Migration](../../ALPHA_API_MIGRATION.md). |
| D2 | Step 10c shipped root retained outputs and typed surface kinds. The current alpha policy governs later changes. |
| D3 | Closed under E4. Step 13a dropped and ADR-0021 deferred. |
| D4 | ADR-0023 adopted. The Steam PCM API decision is tracked on #322. |
| D5 | Keep `dummy_hcd` experimental and unavailable through ordinary construction. |
| D8 | Historical step 17. Any new CI or release file change still requires Anders. |

Phase 2 is tracked on [#322](https://github.com/anderstvoss/virtualgamepad/issues/322).
C8 drafts the Alpha 2 host audit. C9 adds Xbox Series and its host setup before
Anders signs off #258. Historical virtual runs keep their original revisions,
hosts and missing observations.
