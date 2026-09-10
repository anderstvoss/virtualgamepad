# Contributing to VirtualGamepad

Thanks for your interest. This project is in early WIP — expect APIs to
shift. Please read this file end-to-end before opening a PR.

## Architecture overhaul

For protocol/corpus rewrite work, begin with the [architecture execution kit](docs/architecture-overhaul/README.md). Its gate register owns implementation sequencing; its status ledger distinguishes planned work from validated results. Existing repository and PR rules still apply.

## Development setup

Run once after cloning:

```bash
cargo install cargo-deny cargo-audit
git config core.hooksPath .githooks
```

`pre-commit` itself must be installed system-wide (via pipx or pip — see
<https://pre-commit.com/#install>). The committed wrappers in `.githooks/`
delegate to whichever `pre-commit` is on `PATH`.

If you previously ran `pre-commit install` on this clone, remove the
now-stale wrappers so `core.hooksPath` is the only source:

```bash
rm -f .git/hooks/pre-commit .git/hooks/pre-push
```

## Required gates

Every commit runs pre-commit hooks: gitleaks, custom blockers for env
files / keys / local paths / private IPs / cloud URIs, plus
`cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`.

Every push runs additional pre-push hooks: gitleaks full-tree scan,
tracked-file blocker, local-paths guard, `cargo deny check`, and
`cargo audit`.

CI re-runs the cargo gate on the matrix (Ubuntu + macOS + Windows) plus
`cargo-deny`, `cargo-audit`, `dependency-review` on PRs, and OpenSSF
Scorecard on pushes to `main`.

## PR process

1. Branch from `main`; PRs target `main`.
2. Keep changes minimal and task-scoped.
3. Justify any new dependency in the PR description.
4. Don't commit secrets, credentials, `.env` files, logs, private keys,
   local configs, hardcoded local paths, usernames, or private IPs.
5. Add or update tests for behavior changes.
6. Sign your commits if possible (`git commit -S`) — helps with
   provenance, not required.
7. Make sure CI is green before requesting review.

## Reporting bugs / requesting features

Use the issue templates:

- **Bug report** — reproduction steps, expected vs actual, environment.
- **Feature request** — motivation, proposed API, alternatives considered.

Security issues: **do not** open a public issue. Use GitHub's private
vulnerability reporting (see [SECURITY.md](SECURITY.md)).

## License

By contributing, you agree your contributions are licensed under
[AGPL-3.0-only](LICENSE).

## Protocol corpus checks

The private protocol corpus is pinned at `protocol-corpus/`. Contributors with access initialize it using `git submodule update --init --recursive`. Install its `requirements-dev.txt` in an isolated Python environment, then run its validator and unit tests. Run `python3 scripts/check-protocol-corpus.py --verify-remote` and `python3 -m unittest discover -s scripts/tests` before adopting a corpus commit. `--write` regenerates checked-in test artifacts after a reviewed gitlink update.

Normal Cargo builds do not require corpus access. CI retains the repository's existing private-job gating; authenticated corpus jobs require a read credential, and fork jobs do not establish corpus conformance.
