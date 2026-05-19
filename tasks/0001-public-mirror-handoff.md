# Task 0001 — Public Mirror Bootstrap Handoff (archived)

> Archived from `~/Projects/virtualgamepad/_AGENT_HANDOFF.md` after the
> public mirror's initial local seed (2026-05-19). Kept here for reference
> on the remaining server-side enablement steps and the public/private
> split rationale. The authoritative procedure lives in
> [`docs/REPO-SETUP.md`](../docs/REPO-SETUP.md); this file is the
> point-in-time record of the seed.

---

## Context

The public mirror of the VirtualGamepad project lives at
`anderstvoss/virtualgamepad`. The private repo
(`anderstvoss/virtualgamepad-private`, this one) is the source of truth;
sync flows one-way via `scripts/sync-to-public.sh`. The two repos live as
sibling local clones:

```
~/Projects/
├── virtualgamepad-private/   ← source of truth
└── virtualgamepad/           ← public mirror
```

Sync from private → public is one-way and **manual**, driven by
`scripts/sync-to-public.sh` in the private repo. Public-only files
(CONTRIBUTING.md, CODE_OF_CONDUCT.md, issue templates, full CI
workflows) live under `scripts/public-overrides/` in the private repo
and are overlaid by the sync script.

## Project shape (what this library is)

VirtualGamepad is an early-WIP Rust library to create virtual gamepad
devices emulating physical hardware at varying accuracy levels. Crate
name `virtual_gamepad`. License MIT. Currently a stub
(`fn main() { println!("Hello, world!"); }`) — no real implementation
yet.

## Where to find more

After the initial seed lands in this repo, read in this order:

1. `README.md` — project intro + local dev setup
2. `SECURITY.md` — defensive posture + reporting
3. `docs/REPO-SETUP.md` — full two-repo split procedure (also explains
   what's enabled where and why)
4. `CONTRIBUTING.md` — PR process, required gates, dev setup
5. `.pre-commit-config.yaml` + `.githooks/pre-push` — what runs on every
   commit and push

## State at archival time (2026-05-19)

**Private repo (`virtualgamepad-private`):** Fully hardened. PRs #2–#10
merged. Trimmed CI (macOS + Windows only). Pre-commit + pre-push hooks
committed under `.githooks/`. Dependabot enabled. Branch protection
blocked (Free plan limitation). Sync script and public overrides exist
on the `harden/sync-to-public-script` branch (PR #11 not yet opened at
seed time).

**Public repo (`virtualgamepad`):** Local working tree seeded via
`scripts/sync-to-public.sh`. Content verified identical to private +
overrides. `core.hooksPath` set to `.githooks/`. Not yet committed or
pushed.

## Seed procedure (followed on 2026-05-19)

The 2026-05-19 seed used an earlier rsync-based version of
`scripts/sync-to-public.sh` to populate the public clone for the very
first time. That rsync flow has since been **replaced** by a filtered
per-commit replay model — see
[`docs/REPO-SETUP.md`](../docs/REPO-SETUP.md#sync-mechanism) for the
current procedure. The point-in-time content of the public clone after
the seed is what the new script's initial `--base` flag refers to.

## Remaining work on the public repo

### 1. Initial commit + push (superseded)

The original handoff envisioned a single `git add -A && git commit -m
"Initial release"` for the seed. Under the new replay model that step
is replaced by an explicit-base sync:

```bash
cd ~/Projects/virtualgamepad-private
git pull --ff-only
scripts/sync-to-public.sh --base <private-sha-matching-seed> ~/Projects/virtualgamepad

# Review the resulting sync/<date>-<sha> branch in the public clone,
# then push:
cd ~/Projects/virtualgamepad
git log main..HEAD
git push -u origin sync/<date>-<sha>
```

The committed pre-push hook on the public clone will still run
gitleaks + cargo-deny + cargo-audit on `git push`, on top of the
five-step gauntlet that the sync script already ran locally.

### 2. Server-side controls (free on public — gh api)

```bash
PUB=anderstvoss/virtualgamepad

# Secret scanning + push protection
gh api --method PATCH /repos/$PUB --input - <<'EOF'
{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"enabled"}}}
EOF

# Private vulnerability reporting
gh api -X PUT /repos/$PUB/private-vulnerability-reporting

# Dependabot
gh api -X PUT /repos/$PUB/vulnerability-alerts
gh api -X PUT /repos/$PUB/automated-security-fixes

# Branch protection — 8 required contexts (matches CI job names)
gh api -X PUT /repos/$PUB/branches/main/protection --input - <<'EOF'
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "Rust lint",
      "Rust build and test (ubuntu-latest)",
      "Rust build and test (macos-latest)",
      "Rust build and test (windows-latest)",
      "Policy checks (ubuntu-latest)",
      "Policy checks (macos-latest)",
      "Policy checks (windows-latest)",
      "Supply chain audit"
    ]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "required_approving_review_count": 0,
    "require_code_owner_reviews": false,
    "dismiss_stale_reviews": false
  },
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_conversation_resolution": true
}
EOF
```

### 3. Verification

```bash
PUB=anderstvoss/virtualgamepad
gh api /repos/$PUB | jq '.security_and_analysis'
gh api /repos/$PUB/branches/main/protection | jq '.required_status_checks.contexts'
gh run list --limit 5 --repo $PUB
```

Expected after Phase 2 server-side enablement:
- `security_and_analysis` shows secret scanning + push protection enabled
- Branch protection has 8 contexts
- A test push or PR triggers the full 8-job CI matrix + dep-review on PRs
- Scorecard SARIF appears in Security tab within ~5 min of next push to main

## What's in this repo vs the private one

| File | Public | Private | Notes |
|---|---|---|---|
| `src/`, `tests/`, `Cargo.toml`, `Cargo.lock` | same | same | the project code |
| `.githooks/`, `.pre-commit-config.yaml` | same | same | dev gates |
| `.github/workflows/ci.yml` | **full** (5 jobs) | trimmed (2 jobs × 2 OS) | public restores rust-lint, supply-chain, dependency-review, ubuntu legs |
| `.github/workflows/scorecard.yml` | **enabled triggers** | workflow_dispatch only | public has cron + push triggers + `publish_results: true` |
| `.github/workflows/gitleaks-history.yml` | same | same | weekly cron |
| `deny.toml`, `SECURITY.md`, `README.md`, `LICENSE` | same | same | doc/config |
| `docs/REPO-SETUP.md` | same | same | reusable two-repo procedure doc |
| `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `.github/ISSUE_TEMPLATE/` | **public only** | absent | community-facing |
| `AGENTS.md`, `tasks/`, `scripts/` | absent | present | internal-only |

## What's blocked on public (don't try these — they're already chosen)

- **`dependency-review-action`**: GHAS-gated on private; works free here.
- **Secret scanning + push protection**: GHAS-gated on private; free here.
- **Scorecard**: token scope insufficient on private; works on public.
- **Branch protection / rulesets**: Pro-gated on private; free here.

## If something is broken

- **CI failing on first push**: the harden-runner egress allowlist may
  miss an endpoint a new dependency pulls. Check the harden-runner step
  output for `blocked egress`; add the host to the allowlist in
  `.github/workflows/ci.yml` (override lives in
  `~/Projects/virtualgamepad-private/scripts/public-overrides/.github/workflows/ci.yml`
  — edit there, not here, since the next sync overwrites this file).
- **Pre-push hook failing**: `cargo install cargo-deny cargo-audit` and
  `pre-commit` must be on PATH. Setup steps: `git config core.hooksPath
  .githooks; rm -f .git/hooks/pre-{commit,push}`.
- **Sync overwrites something I changed locally**: the sync script now
  applies private commits as a patch series via `git am` onto a fresh
  `sync/<date>-<sha>` branch, then refreshes overrides from
  `scripts/public-overrides/` if drifted. It never touches the public
  clone's `main` directly. If you modified files on `main` and want
  them to persist across syncs, either move the change into
  `scripts/public-overrides/` in the private repo (it'll re-apply each
  sync) or land it on private and let the next sync replay it.

## Pointers for future agents

- The portable hardening procedure is in `docs/REPO-SETUP.md` — it's
  designed to be reusable for any project that wants the same
  private/public split.
- The private repo's `AGENTS.md` has general agent rules; carry them over
  in spirit (minimal changes, no destructive ops, no committing secrets
  or local paths).
- Use absolute paths in bash commands; the user's workspace has had
  folder renames mid-session that can stale-out `cwd`.
