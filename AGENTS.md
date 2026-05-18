# AGENTS.md

## General Rules

- Keep changes minimal and task-scoped.
- Work only on the current branch.
- Do not commit secrets, credentials, `.env` files, logs, private keys, or local configs.
- Do not hardcode local paths, usernames, private URLs, private IPs, tokens, or machine-specific assumptions.
- Do not modify `.github/`, release workflows, `SECURITY.md`, license files, or repository settings unless explicitly instructed.
- Do not rewrite Git history.
- Do not run destructive commands.
- Do not add dependencies without explaining why.
- Do not use real user data or private logs as fixtures.
- Use fake or sanitized test fixtures.

## Rust Checks

Before completing a task, run:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
gitleaks detect
```

## Required Completion Summary

At the end of every task, report:

- Files changed
- Behavior changed
- Tests added or modified
- Commands run and pass/fail status
- New dependencies, if any
- Any unresolved issues

Use this format:

```text
Completion Summary

Files changed:
- path/to/file.rs: brief description

Behavior changed:
- Brief description of behavior change

Tests added or modified:
- Brief description of tests

Validation:
- cargo fmt --all -- --check: passed/failed/not run
- cargo check --all-targets --all-features: passed/failed/not run
- cargo clippy --all-targets --all-features -- -D warnings: passed/failed/not run
- cargo test --all-features: passed/failed/not run
- gitleaks detect: passed/failed/not run

New dependencies:
- None
# or:
- crate_name: reason added

Unresolved issues:
- None known
# or:
- Description of unresolved issue
```
