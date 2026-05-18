# VirtualGamepad

WIP: a rust library to create virtual gamepad devices emulating physical hardware at varying accuracy levels.

## Setup

After cloning, run once:

```bash
cargo install cargo-deny cargo-audit
git config core.hooksPath .githooks

# If you previously ran `pre-commit install` on this clone, remove the
# now-stale wrappers in .git/hooks/ so git only consults .githooks/:
rm -f .git/hooks/pre-commit .git/hooks/pre-push
```

`core.hooksPath` redirects git to the committed `.githooks/` directory. Both
hook wrappers (`pre-commit` and `pre-push`) are committed there, so no
separate `pre-commit install` step is needed. The `pre-commit` wrapper
delegates to the `pre-commit` Python package (install via pipx or pip — see
<https://pre-commit.com/#install>); the `pre-push` wrapper runs the custom
safety checks (gitleaks, tracked-file blocker, local-paths scan) and then
hands off to pre-commit's pre-push-stage hooks (`cargo deny` + `cargo
audit`).

## Development

Build:

```bash
cargo build
```

Run tests:

```bash
cargo test
```

Run checks:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
gitleaks detect
```

## Configuration

Copy `.env.example` to `.env` for local development.

Do not commit `.env` or other local configuration files.

## Security

See `SECURITY.md`. For the full private/public split and the
end-to-end setup procedure (reusable across projects), see
[`docs/REPO-SETUP.md`](docs/REPO-SETUP.md).