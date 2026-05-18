# VirtualGamepad

WIP: a rust library to create virtual gamepad devices emulating physical hardware at varying accuracy levels.

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

See `SECURITY.md`.