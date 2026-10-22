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

## Waiting on Anders

| Decision | Remaining work |
| --- | --- |
| D6/D7 | Wii production gate and first realizations; C5 not started |
| D9 | Host running kernel 7.3-rc1 or newer for Steam step 16, then C6 |
| D2 | Ordinary-root output/kind API growth, step 10c |
| D3 | Presentation epochs implementation, step 13a |
| D4 | ADR-0023 adoption; #211 stays draft |
| D5 | Provider disposition, step 12b; retain `dummy_hcd` |
| D8 | Authorized CI-file work, step 17 |

The current 6.12 host cannot run Steam P1. Historical virtual-host measurements
retain their original revisions and environments; missing consumer and physical
cells remain explicit. The ordinary-root snapshot remains 110 items. No new
third-party dependency, broker/IPC format change or prohibited-file change is
part of these follow-ups. C5 and C6 remain unstarted; stop here under Part 3.
