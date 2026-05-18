# Example Task 0000: Example Task

## Goal

Add a TOML config loader for the CLI.

## Scope

- Add `src/config.rs`.
- Update `src/main.rs` only as needed.
- Add tests under `tests/`.
- Do not modify `.github/`, `SECURITY.md`, release config, or license files.
- Do not read `.env` directly.
- Do not hardcode absolute local paths.
- Use only existing dependencies unless a new crate is clearly justified.

## Acceptance Criteria

- Valid TOML config loads successfully.
- Missing config file returns a clear error.
- Invalid TOML returns a clear error.
- Existing behavior without `--config` remains unchanged.
- Unit tests cover valid, missing, and malformed config files.

## Validation

Run:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
gitleaks detect
```

## Required Agent Output

At completion, report:

- Files changed
- Behavior changed
- Tests added or modified
- Commands run and pass/fail status
- New dependencies, if any
- Any unresolved issues
