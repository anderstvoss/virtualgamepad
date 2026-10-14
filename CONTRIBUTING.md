# Contributing to VirtualGamepad

Thanks for your interest. This project is in early WIP — expect APIs to
shift. Please read this file end-to-end before opening a PR.

## Where things are documented

- [Status](docs/STATUS.md): current dispositions and known limitations.
- [Architecture](docs/CORE_ARCHITECTURE.md) and [decision records](docs/adr/):
  the current model. Record a significant technical decision as a new ADR rather
  than rewriting an accepted one; mark superseded parts explicitly.
- [Support matrix](docs/CONTROLLER_SUPPORT.md): status definitions and promotion rules.
- [Alpha 1 history](docs/history/alpha-1/README.md): the architecture rewrite plan,
  gate ledgers, experiments and evidence. Historical; do not extend it for new work.

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

The private protocol corpus is pinned by the revision in `protocol-corpus.rev`; it is not a submodule, so Git consumers never need access to it. Contributors with access clone it into the ignored `protocol-corpus/` directory and check out that revision: `git clone https://github.com/anderstvoss/controller-protocol-corpus.git protocol-corpus && git -C protocol-corpus checkout "$(cat protocol-corpus.rev)"`. Install its `requirements-dev.txt` in an isolated Python environment, then run its validator and unit tests. Run `python3 scripts/check-protocol-corpus.py --verify-remote` and `python3 -m unittest discover -s scripts/tests` before adopting a corpus commit. `--write` regenerates checked-in test artifacts after a reviewed `protocol-corpus.rev` update.

Normal Cargo builds do not require corpus access. CI retains the repository's existing private-job gating; authenticated corpus jobs require a read credential, and fork jobs do not establish corpus conformance.
