# Task 0002: Bootstrap the demo program

## Goal

Add a sibling demo program under `demo/` that grows alongside the
library — starting as a minimal `clap`-based CLI and eventually
landing as a full GUI with an internal controller visualizer.

## Scope

- Convert the repo to a Cargo workspace (root crate stays where it is;
  `demo/` becomes a workspace member).
- Add `demo/Cargo.toml`, `demo/src/main.rs`, `demo/README.md`.
- Update top-level `README.md` to declare the demo as a project goal
  and document the run command.
- Add a `CHANGELOG.md` `Unreleased` entry.
- Do **not** modify anything under `docs/spec/`.
- Do not modify `.github/`, `SECURITY.md`, release config, or license
  files.

## Non-goals

- No real library API usage (the library exposes none yet).
- No GUI implementation in this task — only the CLI scaffold and the
  growth plan in `demo/README.md`.
- No selection of a GUI framework yet.

## Acceptance criteria

- `cargo build --workspace --all-features` succeeds.
- `cargo test --workspace --all-features` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  is clean.
- `cargo run -p virtual_gamepad_demo -- info` prints scaffold info
  without panicking.
- `cargo run -p virtual_gamepad_demo -- --help` shows the CLI cleanly.
- `git diff` against `main` shows no changes under `docs/spec/`.
- Pre-commit and pre-push hooks pass.

## Validation

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo run -p virtual_gamepad_demo -- info
gitleaks detect
```

## Required Agent Output

At completion, report:

- Files changed
- New dependencies (`clap`) and rationale
- Commands run and pass/fail status
- Any unresolved issues
