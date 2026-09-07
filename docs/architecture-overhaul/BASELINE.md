# Inspected baseline — 2026-09-04

## Source record

- Repository: `anderstvoss/virtualgamepad`.
- Inspected local branch: `main`.
- Commit: `9b466e0bd0dafb141ae8e504648c744c0d81374f`.
- GitHub comparison on 2026-09-05 also reports this exact SHA for main. See the branch review for the inspected remote inventory and merge distinctions.
- Before this execution-kit preparation: five handoffs were untracked under `.agents/`; no tracked implementation edits appeared in `git status`.
- No tracked `.gitmodules` or `protocol-corpus` gitlink was present. Corpus remote/ownership and first adopted commit are not yet established.
- Declared toolchain: `rust-toolchain.toml` pins 1.95.0; workspace `rust-version` is 1.85. This records declarations, not installed-toolchain or MSRV validation.

## Source observations, not host acceptance

| Observation | Source |
| --- | --- |
| DualSense UHID already selects USB bus metadata (`0x03`) and submits initial neutral input during creation | `crates/gr-curated-controllers/src/dualsense.rs`, `hid_realization` and `create_dualsense` |
| UHID stores static GET responses and acknowledges SET before downstream handling | `crates/gr-provider-linux-uhid/src/lib.rs`, `Session::drain_reverse_events` |
| Broker services Sony startup feature requests before exposing its session | `crates/gr-privileged-broker/src/dummy_hcd.rs`, creation and `service_initial_feature_requests` |
| Runtime preserves dirty state after failed send | `crates/gr-controller-runtime/src/lib.rs`, `ControllerRuntime::commit` |
| Compound lifecycle and bounded reverse-delivery infrastructure exist | `crates/gr-controller-runtime/src/compound.rs` and `reverse_delivery.rs` |

## Regression inventory to preserve

Existing test symbols verified by source inspection:

- `failed_commit_preserves_dirty_state_for_retry`
- `rejected_update_preserves_state_and_dirty_status`
- `prepared_realization_cannot_open_a_different_controller`
- `failed_later_open_rolls_back_earlier_component_in_reverse_order`
- `complete_ordered_frames_and_reverse_close_are_enforced`
- `synthetic_accessory_display_benchmark_preserves_attachment_boundaries`
- `fake_io_covers_numbering_static_replies_and_terminal_close`
- Ignored UHID host test: `creates_and_destroys_a_process_owned_hid_device`.

Static-response tests should evolve to verify protocol-owned replies when that boundary changes; preserve their numbering/error/cleanup regressions rather than mechanically keeping obsolete ownership.

## Current operator tools

- `scripts/run-sdl3-gamepad-probe.sh` and `scripts/sdl3-gamepad-probe.c`.
- `scripts/steam-controller-ab-report.sh`.
- Existing procedure: `docs/SDL_ACCEPTANCE.md`.
- Demo binary declared in `demo/Cargo.toml`: `virtualgamepad-demo`.
- Historical `vgpd-demo`/`gr-cli` names describe a workflow intent; do not assume those binaries exist in this checkout.

## Baseline checks — executed 2026-09-05

Commands ran from the repository root against the SHA above with documentation changes present. Execution output is retained in the review task; no raw logs were added to the repository.

```bash
git status --short --branch
git rev-parse HEAD
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect --redact
```

The format, workspace check, strict Clippy, and workspace test commands all passed (exit 0). Ignored device/broker tests remained ignored; this is not live acceptance. Gitleaks passed (exit 0), scanning 349 commits with no leaks found; history scanning does not certify untracked files. Documentation local links and fenced blocks were checked across all 17 kit Markdown files, and `git diff --check` passed. Physical, kernel, SDL, Steam, audio, and Bluetooth acceptance has not been executed. Historical successful dummy_hcd gyro and failed UHID gyro reports require original artifacts/revisions before use as comparative evidence.

Tool versions: rustc 1.95.0 (`59807616e`), Cargo 1.95.0 (`f2d3ce0bd`). The installed Gitleaks executable reports `version is set by build process`, so its release number is unavailable. The declared 1.85 MSRV was not tested.

## E0 completion record

Implementation owner: unassigned. Plan commit: not yet recorded. Baseline checks: passed on 2026-09-05; live/ignored checks not run. Host survey: not_run. Evidence inventory reconciliation: pending. E0 is therefore partially prepared, not complete.

## Additional context and branch reassessment

Read [CONTEXT_REASSESSMENT.md](CONTEXT_REASSESSMENT.md) and [BRANCH_REVIEW.md](BRANCH_REVIEW.md) before choosing reuse. Memories are not implementation authority. Archived gyro-finding commit `3a62825` already uses UHID USB bus metadata, so the historical BUS_VIRTUAL explanation remains unresolved. Unmerged workflow and UHID-broker code are candidates for selective extraction, not merge prerequisites. Early-development source/API replacement is allowed.

## Implementation execution baseline — 2026-09-05

Owner: Codex. Starting revision remains `9b466e0`. All five required baseline checks passed again: formatting, workspace check, strict workspace Clippy, workspace tests, and `gitleaks detect --redact`. Ignored live tests were not run. Existing changes comprise the architecture kit, contributor link, local research, and a user-owned `.gitignore` edit; only reviewed kit files and the contributor link enter the plan commit. Resolve that commit by `git log -- docs/architecture-overhaul/BASELINE.md`.

The approved execution scope is E0–E6 with feature-specific gates, a private independent `anderstvoss/controller-protocol-corpus` repository, targeted corpus CI changes, and host provisioning prepared for review before system changes. No virtualgamepad push is requested.
