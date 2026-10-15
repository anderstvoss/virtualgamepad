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

## Protocol research checks

Controller protocol evidence lives in the public
[controller-protocol-research](https://github.com/anderstvoss/controller-protocol-research)
repository, pinned here as the `protocol-research/` submodule. New controller
research (sources, claims, experiments, synthetic fixtures and consumer
evidence) goes there as a pull request against that repository, following its
README and CONTRIBUTING; issue comments here link to the research records rather
than carrying the evidence themselves.

The submodule is marked `update = none` in `.gitmodules`, so Cargo and
`git clone --recurse-submodules` skip it and Git consumers never fetch it.
Check it out explicitly with `git submodule update --init --checkout protocol-research`.
Install its `requirements-dev.txt` in an isolated Python environment, then run
its publication guard, validator and unit tests. To adopt a newer research
commit, check it out in `protocol-research/` (it must already be on the
research repository's `main`), run `python3 scripts/check-protocol-research.py --write --verify-remote`,
then `git add protocol-research tests/fixtures/protocol-research` and run
`python3 -m unittest discover -s scripts/tests`.

Normal Cargo builds do not require the research checkout. CI's
`protocol-research` job needs no credentials and also runs for fork PRs.