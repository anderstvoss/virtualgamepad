# VirtualGamepad

Rust library for creating virtual gamepad devices that emulate
physical hardware at varying accuracy levels.

> **Status:** early WIP. The crate is a scaffold; the public API will
> land in subsequent releases. See [CHANGELOG.md](CHANGELOG.md) for
> tracked changes, and [docs/spec/](docs/spec/) for the architecture,
> implementation, and validation spec the buildout follows.

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
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
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
