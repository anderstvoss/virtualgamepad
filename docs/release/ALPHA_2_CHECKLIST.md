# Alpha 2 release checklist — DRAFT

> This is a **maintainer gate**, not evidence of a passed release.
> Draft source: `main` at
> [`77ec6e39457ddf123ef732ac0e28237307e05f8c`](https://github.com/anderstvoss/virtualgamepad/commit/77ec6e39457ddf123ef732ac0e28237307e05f8c).
> Modelled on [Alpha 1's release review](../history/alpha-1/ALPHA_FINAL_REVIEW.md)
> and [remediation disposition](../history/alpha-1/ALPHA_REMEDIATION_STATUS.md).
> The eventual **release commit**, not this docs branch/PR head, owns final evidence.

## Close source and scope gates

- [ ] Confirm [#322 phase 2](https://github.com/anderstvoss/virtualgamepad/issues/322)
  close condition: VG-1b **a–e** are merged to `main`, and the #304 Steam
  kernel observations are recorded through
  [research PR #122](https://github.com/anderstvoss/controller-protocol-research/pull/122).
  Do not mark #122 completed from a draft or open PR.
- [ ] **TODO(VG-1b e):** after step e merges, review the final
  `docs/ALPHA_API_MIGRATION.md` ledger, `docs/APPLICATION_API.md`,
  per-controller documentation, all three API snapshots and changes since
  the draft baseline. Consolidate exact PR/SHA links, removed APIs,
  migration examples, and intentional behavioral differences in
  [these release notes](ALPHA_2_RELEASE_NOTES_DRAFT.md). Do **not** edit
  the local agent's files as part of this draft PR.
- [ ] Verify [C5 #233](https://github.com/anderstvoss/virtualgamepad/pull/233),
  [C6 #301](https://github.com/anderstvoss/virtualgamepad/pull/301),
  [C7 #329](https://github.com/anderstvoss/virtualgamepad/pull/329),
  [C8 #337](https://github.com/anderstvoss/virtualgamepad/pull/337),
  [C9 #342](https://github.com/anderstvoss/virtualgamepad/pull/342),
  [VG-1 #352](https://github.com/anderstvoss/virtualgamepad/pull/352)
  and each merged VG-1b step on **the selected release SHA**.
- [x] [#258 audit](https://github.com/anderstvoss/virtualgamepad/issues/258)
  was signed off at `69c2283` in
  [owner comment 6777536122](https://github.com/anderstvoss/virtualgamepad/issues/258#issuecomment-6777536122).
  Re-review if subsequent changes introduce a **new host setup or permission
  requirement**; the #350 private worker tag is not, by itself, a verified
  operator-level hardware test.
- [ ] Review the [support matrix](../CONTROLLER_SUPPORT.md), compiled registry,
  per-function manifests, limitations and future-family issue exclusions.
  Do not promote WIP cells without qualifying evidence.
- [ ] Leave **VG-2/#344 and ADR-0024** on the audio track; do not gate Alpha 2
  on the separately planned loss-aware stream.

## Validate the exact release commit

- [ ] Freeze the candidate as a full 40-character `main` SHA after all
  intended merges. Review that branch-protected **required checks are green**
  on that *exact* SHA (not just on a prior PR head). A skipped, cancelled or
  pending required check is not a pass; repeat qualification if the SHA changes.
- [ ] Required CI/platform checks: inspect the live required-check names in
  GitHub branch protection; include Rust fmt, locked build/Clippy/tests on
  Linux/macOS/Windows, MSRV, root API snapshot/doc guard, Python tooling,
  protocol-research pin, cargo audit/deny, full-history secret scan and
  any currently required CodeQL/policy checks. Record each check's exact name,
  conclusion and run URL at release.
- [ ] Run the repository's normal exact-SHA regression/policy checks:
  `cargo fmt --all -- --check`,
  `cargo check --locked --workspace --all-targets --all-features`,
  `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --locked --workspace --all-features`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features`,
  `python3 -m unittest discover -s scripts/tests`,
  `python3 scripts/check-alpha-api.py`,
  `cargo audit`, `cargo deny check`, and full-history Gitleaks per
  `scripts/deep-scan.sh`. Distinguish CI evidence from local diagnostics.
- [ ] Verify the reviewed API snapshots match the release source, including
  `docs/api/ALPHA_API_SNAPSHOT.json`,
  `docs/api/WII_REMOTE_API_SNAPSHOT.json`, and
  `docs/api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json`.
  Final step-e migration and snapshot consistency must be reviewed together.
- [ ] Run the **exact Git consumer** with the final full commit SHA:
  ```bash
  python3 scripts/check-alpha-git-consumers.py \
    --revision <40-character-release-main-sha> \
    --report <outside-repository>/alpha2-git-consumers.json
  ```
  It checks a fresh Git dependency for all **eight** subsets of the
  three nondefault root features (`experimental`, `audio-pipewire`,
  `audio-usbip`), with `--locked --all-targets`, then a
  `--locked --offline` all-features rebuild. Require unchanged lockfile,
  the exact revision in `Cargo.lock`, and recorded compiler version,
  feature list, consumer SHA-256 and lock SHA-256. Retain JSON and lockfile
  **outside the source tree** for release evidence. This is compile-only,
  never hardware acceptance.
- [ ] Perform the **real USB/IP test on kalux** for
  [#350](https://github.com/anderstvoss/virtualgamepad/pull/350):
  confirm (1) live `host_outputs()` snapshots update during service,
  (2) a FINAL retained snapshot after orderly close, and (3) a stale
  installed worker is rejected with correct reinstall guidance. Record
  tested client/worker SHAs, actual results and failure details; socket-pair
  tests alone do not meet this item.
- [ ] Review #126 GUI/consumer disposition and any open alpha-labelled
  issues against the selected commit. Preserve deferred/failing evidence,
  particularly audio continuity, USB/IP stream, Bluetooth, Puck,
  dummy_hcd, XInput/GIP and unsupported Xbox Series features.
- [ ] Refresh this checklist and
  [Alpha 2 draft release notes](ALPHA_2_RELEASE_NOTES_DRAFT.md) against
  **the final release commit**; remove all `TODO(VG-1b e)` placeholders
  only after the actual final ledger is available.

## Release-owner decisions and publication

- [ ] **Anders chooses** the Alpha 2 Git tag, workspace version and any
  version bump. The current workspace declares `version = "0.1.0"`
  and `publish = false`; do not assume an increment or modify
  `Cargo.toml` in release-prep. If a prerelease crate version is chosen,
  explicitly review internal path-dependency version requirements.
- [ ] Anders reviews the **final, exact release SHA**, sign-off evidence,
  known limitations, and release-note text.
- [ ] **Anders tags and publishes** the Git-only release. No agent tags,
  creates a GitHub Release, publishes crates, or asserts completion on
  Anders's behalf.
- [ ] Verify published tag resolves to the accepted exact SHA, archive the
  outside-tree consumer JSON and lockfile with the release evidence, and
  cross-check release notes, CHANGELOG and support claims. Do not mistake
  PR-head CI or an earlier candidate for the tagged revision.

