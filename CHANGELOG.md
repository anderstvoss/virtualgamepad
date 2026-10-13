# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Proposed Git alpha `0.1.0-alpha.1`

- Proposed source tag only; no tag/release is published by this closeout. The Git
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
  Steam is deferred to x86-64 #144 with release-owner confirmation outstanding.
- See [release notes](docs/ALPHA_RELEASE_NOTES.md) and
  [current dispositions](docs/architecture-overhaul/ALPHA_REMEDIATION_STATUS.md).
  Final merge-SHA qualification and separate tag authorization remain pending.

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
