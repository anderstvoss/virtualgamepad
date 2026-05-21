# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog 1.1.0][keep-a-changelog], and
this project adheres to [Semantic Versioning 2.0.0][semver].

## [Unreleased]

### Added

- Companion demo binary under `demo/` (`virtual_gamepad_demo`) that
  grows from a CLI scaffold into a future GUI controller visualizer
  alongside the library.

- Phase 0 workspace split scaffold with placeholder crates under
  `crates/`, a `gr-testkit` fixture-envelope loader, a `gr-cli`
  validation/phase-gate skeleton, and `vgpd-demo phase-gate <N>`
  reading the manual checklist from the implementation plan.

- Phase 1 `gr-core` domain model with serializable identifier
  newtypes, fidelity/backend enums, semantic function and capability
  enums, stub built-in profile payloads, and canonical input frame
  types.

- Phase 1 fixtures and snapshots, including checked-in neutral
  DualSense and Xbox 360 input-frame samples, `gr-testkit`
  profile-input builders, and snapshot/property coverage for the new
  `gr-core` types.

### Changed

- Top-level README adds an explicit project-goals section covering
  both the library and the demo program.

- `vgpd-demo` now exposes `show-types`, and `gr-cli phase-gate 1 --auto`
  runs the automated Phase 1 checks including snapshot verification.

### Deprecated

### Removed

- The single-crate `virtual_gamepad` package at the repository root.
  Its slot is now occupied by `crates/gr-core/` (initially a placeholder
  with a smoke test), and the workspace root `Cargo.toml` is a pure
  workspace manifest. A facade crate may be reintroduced later to
  re-export from the `gr-*` crates if a single import surface proves
  useful.

### Fixed

### Security

[keep-a-changelog]: https://keepachangelog.com/en/1.1.0/
[semver]: https://semver.org/spec/v2.0.0.html
[Unreleased]: https://github.com/anderstvoss/virtualgamepad/compare/HEAD...HEAD
