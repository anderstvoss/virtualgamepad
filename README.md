# VirtualGamepad

WIP: a rust library to create virtual gamepad devices emulating physical hardware at varying accuracy levels.

## Setup

After cloning, install hooks once:

```bash
git config core.hooksPath .githooks
pre-commit install
```

`core.hooksPath` redirects git to the committed `.githooks/` directory so the
pre-push checks (gitleaks + tracked-file and local-path scans) travel with the
repo. `pre-commit install` then writes its pre-commit hook into the same
location.

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