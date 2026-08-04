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

- Phase 8 prep scaffolding: a restored root `virtualgamepad` package
  for provider feature flags, the `gr-provider-linux-uinput` contract
  surface, `run-uinput-smoke` / `support-report` command surfaces, and
  a Tier B provider workflow scaffold.

- Phase 2 through Phase 7 runtime foundation: built-in profile and
  capability registry, configuration validation, compiled session
  options, runtime model types, backend API and fakes, planner,
  translators, session manager, host bridge, trace replay, and
  reviewer-facing demo/CLI surfaces.

- Phase 8 through Phase 11 Linux provider buildout: `uinput`
  compatibility-tier support, DualSense UHID identity-aware support,
  Linux transport state-machine foundation, and the Phase 11
  DualSense USB hardware-faithful transport provider surface.

- Linux UHID broker provider: an opt-in, constrained local socket boundary for
  the identity-aware DualSense provider, keeping `/dev/uhid` out of client
  applications while `uinput` remains the standard production path.

- Phase 12 Windows and macOS HID provider foundations with planning-only
  inventories, deployment-requirement reporting, planner integration,
  cross-platform checks, and realization-roadmap READMEs.

### Changed

- Top-level README adds an explicit project-goals section covering
  both the library and the demo program.

- User-facing status text now reflects Phase 12 provider-complete
  closure while keeping deferred profile/platform validation explicit.

- `vgpd-demo` now exposes `show-types`, and `gr-cli phase-gate 1 --auto`
  runs the automated Phase 1 checks including snapshot verification.

- DualSense `ProfileInputPayload` / `ProfileInputDeltaPayload` variants
  now serialize with the on-wire tag `dualsense` (matching the
  `ProfileId` convention) instead of serde's auto-kebab `dual-sense`.

- The d-pad fields shared by `GenericGamepadInput`, `Xbox360Input`, and
  `DualSenseInput` are now factored into a `Dpad` substruct, surfaced
  in fixtures and snapshots as a single nested `dpad:` map.

- `ProfileInputDeltaPayload` variants are now sparse per-profile delta
  structs (`*Delta`) with `Option<T>` fields and a shared `DpadDelta`,
  so a delta carries only the changed fields rather than mirroring a
  full snapshot.

- Phase 1 `canonical_yaml_snapshots_are_human_readable` now emits one
  snapshot per variant for every Phase 1 enum (`FidelityTier`,
  `BackendLevel`, `BackendFamily`, `CapabilityCategory`).

- Phase 1 manual gate expanded with sparse-delta authoring, d-pad
  nesting visibility, and per-variant snapshot coverage items.

- The repository root is once again a tiny `virtualgamepad` package so
  provider feature flags and target-filtered optional provider
  dependencies have a stable host surface.

### Deprecated

### Removed

### Fixed

- Full-repo validation commands in repo docs and CI now use explicit
  `--workspace` flags so formatting, linting, build, and test checks
  cover every crate instead of only the root package's default member
  set.

- `gr-cli`'s interactive Linux `uinput` smoke helpers are now clean
  under the current pinned Clippy toolchain without changing runtime
  behavior.

### Security

[keep-a-changelog]: https://keepachangelog.com/en/1.1.0/
[semver]: https://semver.org/spec/v2.0.0.html
[Unreleased]: https://github.com/anderstvoss/virtualgamepad/compare/HEAD...HEAD
