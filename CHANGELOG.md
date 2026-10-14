# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Documentation reorganized after Alpha 1: decision records moved to `docs/adr/`,
  the API snapshot and inventory to `docs/api/`, and Alpha 1 planning, gate and
  evidence records to `docs/history/alpha-1/`. Added `docs/STATUS.md` and a
  user-facing README.

## [0.1.0-alpha.1] - 2026-10-14

- Git-only pre-release on `ae9e87b`. The Git
  tag `0.1.0-alpha.1` carries crate version `0.1.0` (`publish = false`). Consumers
  should pin by tag or full SHA; no crates.io publication is planned.
- Reviewed 110-item ordinary-root API for DualSense, DualShock 4, Switch Pro and
  standard-HID Xbox 360 on Linux uinput/UHID; Xbox is not proprietary XInput.
  Experimental exports and supporting crates remain outside the alpha contract.
- PR #149 lands lifecycle, callback, Sony touch-companion, Xbox identity and demo
  keyboard/routing corrections. Unsupported dummy_hcd creation rejects before
  resource allocation. Library default features remain empty.
- Seven scoped broker security/recovery phases passed (#115 closed). Opt-in
  USB/IP audio remains WIP: functional playback/microphone flow was observed,
  while strict zero-loss assertions failed and full routing remains deferred
  (#143/#145).
- Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.
- Broader GUI review (#126), physical fidelity (#136), controller matching (#116)
  and native timing (#135) remain deferred. No screen-reader support is provided.
  Steam is deferred to x86-64 #144, confirmed by the release owner.
- The private protocol corpus is pinned by `protocol-corpus.rev` instead of a Git
  submodule, so Git consumers of the tag need no access to the corpus.
- See [release notes](docs/ALPHA_RELEASE_NOTES.md) and
  [current dispositions](docs/history/alpha-1/ALPHA_REMEDIATION_STATUS.md).

### Security

- Enforce `// SAFETY:` comments on every `unsafe` block
  (`clippy::undocumented_unsafe_blocks = "deny"`).
- Lint Python helpers with ruff (pyflakes + bandit rules) and run their
  unit tests in CI.
- Extend CodeQL to scan GitHub Actions workflows.
- Add harden-runner egress blocking to the full-history gitleaks workflow.
- Require a full-history secret scan before changing repository
  visibility; document Actions token, fork-approval and tag-protection
  settings.
