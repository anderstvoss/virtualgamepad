# Branch and integration review — 2026-09-05

## Coverage and method

GitHub branch listing was paginated to exhaustion; no open PRs were returned. Main was verified remotely as `9b466e0bd0dafb141ae8e504648c744c0d81374f`. Local refs, merge-base deltas, patch-equivalence counts, endpoint tree comparisons, and PR merge metadata were inspected. Both discovered local clones were checked: the current main checkout and an older checkout on `workflow/controller-support-toolchain`; neither exposed a `wip/btvirt` branch. Unpublished work in other locations remains unknown.

Ahead/behind counts below are local ancestry counts against inspected main. They are not missing-work counts. Squash merges can retain many commits outside main ancestry and defeat per-commit patch equivalence. Remote tips were separately verified for the two unmerged candidates below; other rows record locally available tips with GitHub branch/PR evidence.

## Actionable dispositions

| Ref / evidence | Finding | Disposition for rewrite |
| --- | --- | --- |
| `workflow/controller-support-toolchain` at `26067ee` | One branch-introduced commit, 15 changed files, dossier schema/fixture, CLI support reports and interactive workflow. No matching merged PR in returned history. Remote tip verified. | Extract/reimplement dossier and reporting concepts during E1; discard profile-runtime coupling and linear compatibility→identity→hardware claim rules. Do not merge branch wholesale. |
| `feat/linux-uhid-broker` at `77372a6` | One branch-introduced commit, 9 changed files; versioned bounded socket messages, allowed profile/input size, session disconnect cleanup. No matching merged PR in returned history. Remote tip verified. | Reuse security/fault-test ideas. Its Create/Send/Drain/Diagnostics/Close protocol rejects non-input client frames, cannot carry dynamic client GET/SET replies, and depends on old backend types. Brokered UHID is optional deployment work, not a new mandatory service. |
| `archive/dualsense-dummy-hcd-poc` at `a5310c6` | Recoverable POC, host procedure and recorded gyro finding. Branch is an ancestor of the port; #101 explicitly excluded the POC and port history contains its revert. | Extract experiment method and independently verify fixtures; preserve archive. Do not treat it as an unmerged replacement for current #105 broker. |
| `codex/provider-realization-api-port` at `273bfc7` | [PR #101](https://github.com/anderstvoss/virtualgamepad/pull/101) merged. Its entire tree equals squash commit `3141471` (empty endpoint diff). | Already integrated. Apparent 73 ahead commits do not require merging. Main subsequently changed. |
| `codex/curated-controller-native-api`, `codex/realization-mode-core` | [#96](https://github.com/anderstvoss/virtualgamepad/pull/96), [#97](https://github.com/anderstvoss/virtualgamepad/pull/97) merged; later architecture superseded parts. | Historical design/tests, not outstanding work. |
| `feat/demo-debug-gui`, `feat/linux-default-backends`, `feat/session-plan-observability` | [#95](https://github.com/anderstvoss/virtualgamepad/pull/95), [#94](https://github.com/anderstvoss/virtualgamepad/pull/94), [#93](https://github.com/anderstvoss/virtualgamepad/pull/93) merged. | Do not restore obsolete provider tiers or planner APIs merely to retain branch code. Preserve useful diagnostics intent. |
| `feat/dummy-hcd` recorded head `b5799af` | [#105](https://github.com/anderstvoss/virtualgamepad/pull/105) merged as current compiled-profile work; branch absent from current listing. | Current main implementation is the starting code evidence. PR test claims remain reported until reproduced where needed. |
| `wip/btvirt` | Mentioned in #105 body, absent from GitHub branch list and both discovered clones; user unsure of location. | Unresolved evidence lead. Do not invent contents, mark BT complete, or delay USB work waiting for it. |

Archived profile-era/SC1 branches and old phase branches provide salvage candidates only. Windows/macOS phase-12 PRs describe planning/provider foundations, not working platform device realization. No obligation exists to revive them in the Linux-focused initial rewrite.

## Full local remote-ref inventory

Counts are ahead/behind relative to main, not chronological age. A merged PR is evidence of integration at the recorded head, not proof that later main retains all its behavior.

| Ref | Local tip | Ahead/behind | Classification |
| --- | --- | --- | --- |
| `agent/fix-cargo-audit-ci` | `8bb063d` | 1/26 | Merged PR #89 (exact recorded head) |
| `archive/dualsense-dummy-hcd-poc` | `a5310c6` | 56/7 | Archived POC; ancestor of port, later reverted/excluded |
| `archive/profile-era-stack` | `54f4012` | 0/7 | Contained in main ancestry |
| `archive/steam-controller-1` | `2462d7e` | 0/8 | Contained in main ancestry |
| `chore/cargo-metadata` | `41b58bb` | 0/108 | Merged PR #15 (exact recorded head) |
| `chore/collapse-single-repo` | `1de8188` | 0/84 | Merged PR #23 (exact recorded head) |
| `chore/gitignore-tidy` | `db03b5e` | 1/62 | Merged PR #39 (exact recorded head) |
| `chore/phase-0-signoff` | `0cdaab8` | 4/65 | Merged PR #36 (exact recorded head) |
| `chore/phase-10-gate-signoff` | `9291596` | 1/34 | Merged PR #68 (exact recorded head) |
| `chore/phase-12-gate-signoff` | `5b03652` | 2/30 | Merged PR #72 (exact recorded head) |
| `chore/project-local-memory` | `e89cda0` | 0/108 | Merged PR #17 (exact recorded head) |
| `chore/remove-archived-task-0001` | `993af0d` | 1/70 | Merged PR #32 (exact recorded head) |
| `chore/rename-prep` | `dbf803c` | 1/72 | Merged PR #29 (branch tip differs; inspect) |
| `ci/disable-private-matrix` | `1fb2dee` | 0/110 | Merged PR #13 (exact recorded head) |
| `codex/curated-controller-native-api` | `23b0281` | 60/9 | Merged PR #96 (exact recorded head) |
| `codex/provider-realization-api-port` | `273bfc7` | 73/4 | Merged PR #101 (exact recorded head) |
| `codex/realization-mode-core` | `da789c9` | 7/8 | Merged PR #97 (exact recorded head) |
| `demo-development` | `1d59015` | 2/67 | Merged PR #34 (exact recorded head) |
| `dependabot/github_actions/Swatinem/rust-cache-2.9.2` | `7557877` | 2/6 | Divergent; inspect content |
| `docs/changelog` | `d17cae2` | 0/108 | Merged PR #16 (exact recorded head) |
| `docs/hardening-checklist` | `b626cc2` | 0/74 | Merged PR #28 (exact recorded head) |
| `docs/phase-10-prep` | `b158180` | 1/37 | Merged PR #65 (exact recorded head) |
| `docs/phase-11-prep` | `b57e6d8` | 1/35 | Merged PR #67 (exact recorded head) |
| `docs/phase-12-prep` | `8c5e655` | 1/32 | Merged PR #70 (exact recorded head) |
| `docs/phase-2-prep` | `fa5908b` | 2/61 | Merged PR #40 (exact recorded head) |
| `docs/phase-4-prep` | `2b1dcf0` | 1/57 | Merged PR #44 (exact recorded head) |
| `docs/phase-5-prep` | `127ee7e` | 1/55 | Merged PR #46 (exact recorded head) |
| `docs/phase-6-prep` | `22ebe40` | 3/53 | Merged PR #48 (exact recorded head) |
| `docs/phase-7-prep` | `7f8c573` | 2/51 | Merged PR #50 (exact recorded head) |
| `docs/phase-9-prep` | `d4ee8bd` | 1/39 | Merged PR #63 (exact recorded head) |
| `feat/demo-debug-gui` | `092467b` | 21/10 | Merged PR #95 (exact recorded head) |
| `feat/linux-default-backends` | `32adc55` | 4/11 | Merged PR #94 (exact recorded head) |
| `feat/linux-uhid-broker` | `77372a6` | 1/10 | Divergent; inspect content |
| `feat/phase-2-scaffolding` | `570fe41` | 2/60 | Merged PR #41 (exact recorded head) |
| `feat/session-plan-observability` | `addb013` | 2/12 | Merged PR #93 (exact recorded head) |
| `fix/rust-toolchain-action-input` | `979f8eb` | 2/71 | Merged PR #30 (exact recorded head) |
| `fix/sbom-workspace-aware` | `c3776de` | 1/64 | Merged PR #37 (exact recorded head) |
| `harden/codeowners-pr-template` | `618d042` | 0/140 | Merged PR #2 (exact recorded head) |
| `harden/codeql` | `4a6ae80` | 0/78 | Merged PR #26 (exact recorded head) |
| `harden/githooks-and-checkout-bump` | `185d1a6` | 0/125 | Merged PR #7 (exact recorded head) |
| `harden/gitleaks-deep-scan` | `0464222` | 0/91 | Merged PR #22 (exact recorded head) |
| `harden/msrv-ci` | `8d28081` | 0/80 | Merged PR #25 (exact recorded head) |
| `harden/post-pr7-cleanup` | `97b66e0` | 0/123 | Merged PR #8 (exact recorded head) |
| `harden/pre-commit-gap-fixes` | `d48114b` | 0/112 | Merged PR #12 (exact recorded head) |
| `harden/pre-commit-pin-sha` | `d950cb3` | 0/140 | Merged PR #4 (exact recorded head) |
| `harden/private-ci-trim` | `dee9862` | 0/120 | Merged PR #9 (exact recorded head) |
| `harden/repo-setup-doc` | `26f7dc3` | 0/117 | Merged PR #10 (exact recorded head) |
| `harden/runner-egress` | `1ba8933` | 0/131 | Merged PR #6 (exact recorded head) |
| `harden/rust-toolchain-pin` | `f86c2c0` | 0/82 | Merged PR #24 (exact recorded head) |
| `harden/sanitize-pr17-leaks` | `f17b188` | 0/93 | Merged PR #21 (exact recorded head) |
| `harden/sbom` | `e8b6343` | 0/76 | Merged PR #27 (exact recorded head) |
| `harden/scorecard-followups` | `4b7791c` | 3/69 | Merged PR #31 (exact recorded head) |
| `harden/scorecard-secret-scan` | `37fb9fe` | 0/140 | Merged PR #5 (exact recorded head) |
| `harden/supply-chain-audit` | `9ac589a` | 0/127 | Merged PR #3 (exact recorded head) |
| `harden/sync-commit-skip` | `3041611` | 0/99 | Merged PR #18 (exact recorded head) |
| `harden/sync-leak-prescan` | `55867d4` | 0/97 | Merged PR #19 (exact recorded head) |
| `harden/sync-snapshot-mode` | `032d8df` | 0/95 | Merged PR #20 (exact recorded head) |
| `harden/sync-to-public-script` | `fe975d5` | 0/114 | Merged PR #11 (exact recorded head) |
| `main` | `9b466e0` | 0/0 | Contained in main ancestry |
| `phase-1-gr-core-domain-model` | `5278fd9` | 11/63 | Merged PR #38 (exact recorded head) |
| `phase-10-linux-transport` | `f477dd8` | 4/36 | Merged PR #66 (exact recorded head) |
| `phase-11-linux-transport` | `d2fcb26` | 7/33 | Merged PR #69 (exact recorded head) |
| `phase-12-windows-macos-provider-foundations` | `cdadfd2` | 3/31 | Merged PR #71 (exact recorded head) |
| `phase-2-gr-profiles-registry` | `1a1db46` | 7/59 | Merged PR #42 (exact recorded head) |
| `phase-3-config-runtime-model` | `0d0a4b3` | 4/58 | Merged PR #43 (exact recorded head) |
| `phase-4-backend-api` | `7e7ebe6` | 6/56 | Merged PR #45 (exact recorded head) |
| `phase-5-planner` | `87269ec` | 7/54 | Merged PR #47 (exact recorded head) |
| `phase-6-translators` | `e89fd27` | 4/52 | Merged PR #49 (exact recorded head) |
| `phase-7-session-runtime` | `a66cf7f` | 10/50 | Merged PR #51 (exact recorded head) |
| `phase-8-linux-uinput` | `0c306b9` | 0/48 | Merged PR #60 (branch tip differs; inspect) |
| `phase-9-linux-uhid` | `f8e02a2` | 10/38 | Merged PR #64 (exact recorded head) |
| `plan-overhaul` | `618539c` | 1/66 | Merged PR #35 (exact recorded head) |
| `post-phase-8-midscope-cleanup` | `31ed4a2` | 4/41 | Merged PR #61 (exact recorded head) |
| `scaffold/library` | `f888e4f` | 0/108 | Merged PR #14 (exact recorded head) |
| `spec-development` | `1182bfc` | 3/68 | Merged PR #33 (exact recorded head) |
| `workflow/controller-support-toolchain` | `26067ee` | 1/28 | Divergent; inspect content |

## Limits

This is a source/content/history assessment. Branch code was not built or run, no hardware acceptance was performed, and no refs were merged, checked out, deleted, or rewritten. CI/dependency branches were inventoried but not individually audited as an architecture deliverable. Refresh relevant tips before any later extraction. Memory files and PR prose do not override the source/content distinctions above.
