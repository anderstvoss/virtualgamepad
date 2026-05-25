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

## Agent Memory

Per-project agent auto-memory lives at `.agents/memory/` in the repo.
The directory's contents are per-user scratch state and never reach
the public mirror (`.agents/` is in the sync EXCLUDE_PATHS). Other
agent runtimes (Codex, etc.) should write to the same location.

### One-time setup on each clone

The runtime expects its memory at a per-user global path. Symlink
that path at the in-repo directory so memories live next to the code:

```bash
mkdir -p .agents/memory

# Adjust the project-slug segment if your local checkout path
# differs from $HOME/Projects/virtualgamepad.
RUNTIME_MEM="$HOME/.claude/projects/-home-$USER-Projects-virtualgamepad/memory"
mkdir -p "$(dirname "$RUNTIME_MEM")"
[ -e "$RUNTIME_MEM" ] && [ ! -L "$RUNTIME_MEM" ] && rmdir "$RUNTIME_MEM" 2>/dev/null
[ -L "$RUNTIME_MEM" ] || ln -s "$(pwd)/.agents/memory" "$RUNTIME_MEM"
```

The repo's tracked `.gitignore` does not list `.agents/memory/` — that
rule would itself leak path names into the public mirror. Instead,
add the rule to your per-clone exclude file once:

```bash
grep -qxF '/.agents/memory/' .git/info/exclude \
  || echo '/.agents/memory/' >> .git/info/exclude
```

## Rust Checks

Before completing a task, run:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
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
- cargo check --workspace --all-targets --all-features: passed/failed/not run
- cargo clippy --workspace --all-targets --all-features -- -D warnings: passed/failed/not run
- cargo test --workspace --all-features: passed/failed/not run
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
