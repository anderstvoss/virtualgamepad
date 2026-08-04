# VirtualGamepad

Rust library for creating virtual gamepad devices that emulate
physical hardware at varying accuracy levels.

> **Status:** early WIP, through Phase 12 provider-complete closure.
> Linux provider surfaces exist for `uinput`, UHID, and the DualSense
> USB transport target. Windows and macOS provider foundations are
> planning-only and report deployment requirements, not real device
> realization. By the project plan this means the architecture is
> functional for full device-emulation buildout, while profile- and
> platform-specific validation remains ongoing. See [CHANGELOG.md](CHANGELOG.md)
> and [docs/spec/](docs/spec/) for tracked changes and the buildout
> spec. A companion [demo program](demo/) grows alongside the library.

Linux `uinput` is the normal production path and does not require UHID. Native
HID identity is an explicit higher-fidelity scope; see the
[UHID broker decision](docs/decisions/uhid-broker.md) for its constrained,
separately managed privilege boundary.

## Project goals

- ship a Rust library for virtual controller emulation per the
  [spec](docs/spec/)
- ship a separate [demo program](demo/) that grows from a CLI today
  into a full GUI with an internal controller visualizer once the
  library is functional (architecture ready for full device-emulation
  buildout, not necessarily every device supported)

## License

[AGPL-3.0-only](LICENSE).

## Setup

After cloning, run once:

```bash
cargo install cargo-deny cargo-audit
git config core.hooksPath .githooks

# If you previously ran `pre-commit install` on this clone, remove the
# now-stale wrappers in .git/hooks/ so git only consults .githooks/:
rm -f .git/hooks/pre-commit .git/hooks/pre-push
```

`core.hooksPath` redirects git to the committed `.githooks/`
directory. Both hook wrappers (`pre-commit` and `pre-push`) are
committed there. The `pre-commit` wrapper delegates to the
`pre-commit` Python package (install via pipx or pip — see
<https://pre-commit.com/#install>); the `pre-push` wrapper runs the
custom safety checks (gitleaks, tracked-file blocker, local-paths
scan) and then hands off to pre-commit's pre-push-stage hooks
(`cargo deny` + `cargo audit`).

## Development

Build:

```bash
cargo build
```

Test:

```bash
cargo test
```

Local gates (also run in CI on every PR):

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```

Before publishing or merging anything that matters, run a deep
gitleaks scan across all branches and tags:

```bash
scripts/deep-scan.sh
```

Record user-visible changes in [`CHANGELOG.md`](CHANGELOG.md) under
the `Unreleased` section as part of any feature, fix, or breaking
change.

## Demo program

The repo ships a separate demo binary under [`demo/`](demo/) that grows
with the library. To run the current scaffold:

```bash
cargo run -p virtual_gamepad_demo -- info
cargo run -p virtual_gamepad_demo -- phase-gate 12
cargo run -p virtual_gamepad_demo -- run-uinput-smoke generic-gamepad
cargo run -p virtual_gamepad_demo -- support-report --profile dualsense --tier hardware-faithful
cargo run -p virtual_gamepad_demo -- plan-session dualsense --goal identity-aware --host-platform windows --inventory samples/inventories/windows-hid-stub.yaml
cargo run -p virtual_gamepad_demo -- plan-session dualsense --goal identity-aware --host-platform macos --inventory samples/inventories/macos-hid-stub.yaml
```

See [demo/README.md](demo/README.md) for the planned growth phases and
non-goals (the demo is **not** intended to be embedded by real users
of the library).

## Configuration

Copy `.env.example` to `.env` for local development. Do not commit
`.env` or other local configuration files (the pre-commit and
pre-push hooks block this).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

See [SECURITY.md](SECURITY.md). For the end-to-end hardening
procedure (reusable across projects), see
[`docs/REPO-SETUP.md`](docs/REPO-SETUP.md); for the tickable one-page
bootstrap list, see [`docs/HARDENING-CHECKLIST.md`](docs/HARDENING-CHECKLIST.md).
