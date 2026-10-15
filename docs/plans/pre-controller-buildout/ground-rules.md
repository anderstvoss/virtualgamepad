# Ground rules for every pre-controller buildout brief

Every brief in this folder starts with "Read `ground-rules.md` first". These rules apply on top of the repository's own `AGENTS.md`, which wins where they differ.

## Repositories

- Code: `github.com/anderstvoss/virtualgamepad`, branched from the latest `main` (5a0a7dd or later). Use the branch your session designates; otherwise name it `post-alpha/<brief-id>`.
- Evidence: `github.com/anderstvoss/controller-protocol-research` (public). Follow its `README.md` and `CONTRIBUTING.md`. Records go under `records/{source,claim,experiment,fixture,report}/` as JSON validated by `schemas/`. Run `tools/publication_guard.py`, `tools/validate.py` and its tests before pushing. No raw captures, serials or GPL code or comment excerpts. Kernel driver behaviour is *consumer evidence* (kernel commit, driver, file:line) and never promotes a device-side claim on its own.
- virtualgamepad pins the research repo as the `protocol-research/` submodule.

## Hard constraints

- **Starting execution is Anders's call.** Do not open a code PR for a brief whose "Blocked by" list has an open item. Research and docs-only briefs say so explicitly when they are free to run.
- Keep the compiled, curated, controller-native architecture: no runtime profiles, plugins, descriptor injection, generic mutable gamepad state, provider-owned protocol semantics, implicit realization fallback or host policy in virtualgamepad. The registry is a compiled table.
- **The ordinary-root API snapshot must not change** unless the brief says Anders approved a specific addition. `python3 scripts/check-alpha-api.py` must print "matches the reviewed snapshot (110 items)". If a change would grow it, stop and ask on the tracking issue.
- Do not touch `.github/`, `SECURITY.md`, the license or release files without an explicit comment from @anderstvoss authorizing that exact change.
- Do not change the broker wire format or protocol version without asking.
- One brief step = one PR. Every fixed bug gets a regression test at the lowest seam. Never weaken, skip or delete existing tests.
- Do not commit `__pycache__/` (brief 01 adds the ignore rule).
- Bluetooth: `linux.uhid.bluetooth` (a local UHID node with Bluetooth bus metadata; no radio, pairing or BlueZ) is available now. Real Bluetooth is deferred to #164. `wip/btvirt` is unvalidated prior work: reference only, never evidence.
- No hosted runner for uinput/UHID: #177 is deferred indefinitely and the privileged CI job stays off. Live host runs happen on a recorded local host.

## Checks before every push (virtualgamepad)

```bash
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
python3 scripts/check-alpha-api.py
gitleaks detect
# when scripts/ changed:
ruff check .
(cd scripts/tests && python3 -m unittest $(ls test_*.py | grep -vx test_protocol_research.py | sed 's/\.py$//'))
```

Host setup: `--all-features` and the API check (it runs `cargo doc --all-features`) need `libpipewire-0.3-dev`, `libasound2-dev` and `clang`. On a bare host about five Python tests (`test_broker_audio_live`, one in `test_host_helper`) fail for environment reasons, as they do on `main`; say so in the PR rather than skipping them.

## Merging

- All 13 required checks green, branch up to date with `main` (strict).
- The Windows job sometimes fails crate downloads with `CRYPT_E_REVOCATION_OFFLINE` (#150). Re-run it once. Never disable revocation checks.
- Anders merges unless he has said a Claude thread may merge that PR.

## Reporting

- Tracking issue: #176. After each merged PR, tick its checklist line and add a one-line comment linking the PR.
- GitHub comments post under Anders's account, so start each with a bold "Claude …" label and end with the Claude Code footer.
- GitHub GraphQL is blocked from Claude sessions: use REST (`gh api`) or the GitHub MCP tools.
- Stop and ask on #176 when a step would change the API snapshot or broker wire format, needs `.github/`, or hits an open decision.
- Finish each PR with the `AGENTS.md` completion summary.

## Background documents (project files)

- `docs/plans/pre-controller-buildout/README.md` (in this repo): the index, lanes, order and decisions.
- Project files `plans/post-alpha-1-cleanup-agent-plan.md`: the original step 0 to 5 plan (#176).
- `reviews/post-alpha-1-architecture-review.md`, `reviews/post-alpha-1-architecture-reassessment.md` (contract gaps A to G, realization tiers).
- `plans/post-alpha-orchestration-context.md`: the orchestration plan Anders uploaded.

If you cannot read project files, the same content is on #176 and in the issues each brief cites.
