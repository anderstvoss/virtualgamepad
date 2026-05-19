# Two-Repo Hardening Setup

Reusable procedure for bootstrapping a private "work" repo plus a
public mirror, with hardening split across the two so that GitHub
Free-tier limitations on private repos don't prevent strong server-side
controls. Apply this file to any new project that wants the same
split — copy it verbatim and follow the checklists.

---

## Rationale

GitHub Free **private** repos cannot access secret scanning, push
protection, private vulnerability reporting, branch protection (or the
newer Rulesets), `dependency-review-action`, or OpenSSF Scorecard with
default token scope. These features are gated behind GitHub Advanced
Security (paid) or public visibility.

The two-repo split preserves a private working repo while unlocking
every gated control at zero cost on a public mirror. The trade-off is
the cost of keeping two repos in sync — solvable several ways (see
[Sync Mechanism](#sync-mechanism)).

---

## Repo Roles

| Role | Repo | Responsibilities |
|---|---|---|
| **Private** | Source of truth for work in progress | Minimal GHA CI (macOS + Windows only); developer-side pre-commit + pre-push covers everything else. No community-facing files. |
| **Public** | Community-facing mirror | Full GHA CI surface; all server-side controls enabled; CONTRIBUTING / CoC / issue templates. Receives content from private via the chosen sync mechanism. |

---

## Hardening Split Reference

| Control | Private (Free) | Public (Free) |
|---|---|---|
| Pre-commit gitleaks + custom blockers | ✅ | ✅ |
| Pre-push gitleaks + tracked-file + local-paths + `cargo deny` + `cargo audit` | ✅ | ✅ |
| Dependabot alerts + automated security updates | ✅ | ✅ |
| Codeowner review (informational without branch protection) | ✅ | ✅ |
| `step-security/harden-runner` egress block on CI | ✅ | ✅ |
| CI: `cargo check` + `cargo test` matrix | macOS + Windows only | full matrix |
| CI: `cargo fmt` + `clippy` + `cargo-deny` + `cargo-audit` | — (local only) | ✅ |
| CI: `dependency-review-action` on PRs | ❌ GHAS-gated | ✅ |
| Secret scanning | ❌ GHAS-gated | ✅ |
| Push protection | ❌ GHAS-gated | ✅ |
| Private vulnerability reporting | ❌ | ✅ |
| Branch protection / Repository Rulesets | ❌ Pro+ | ✅ |
| OpenSSF Scorecard workflow | ❌ token scope | ✅ |
| Scorecard public dashboard | ❌ (visibility) | ✅ |
| Weekly full-history `gitleaks` workflow | ✅ | ✅ |

---

## File Inventory

| File / Path | Private | Public | Notes |
|---|---|---|---|
| `.pre-commit-config.yaml` | ✅ | ✅ | Identical. `cargo-deny` + `cargo-audit` as pre-push hooks on both. |
| `.githooks/pre-commit` | ✅ | ✅ | Generic wrapper, no machine paths. |
| `.githooks/pre-push` | ✅ | ✅ | Custom safety checks then hands off to `pre-commit hook-impl --hook-type=pre-push`. |
| `.github/CODEOWNERS` | ✅ | ✅ | May differ in scope (e.g., public adds community reviewers). |
| `.github/dependabot.yml` | ✅ | ✅ | Identical. |
| `.github/PULL_REQUEST_TEMPLATE.md` | ✅ | ✅ | Identical. |
| `.github/workflows/ci.yml` | trimmed | full | Private: macOS + Windows only. Public: adds `rust-lint`, `supply-chain`, `dependency-review`, ubuntu legs. |
| `.github/workflows/scorecard.yml` | `workflow_dispatch:` only | full triggers | File present in both; private just has triggers disabled with a header note. |
| `.github/workflows/gitleaks-history.yml` | ✅ | ✅ | Identical. |
| `SECURITY.md` | ✅ | ✅ | Wording differs slightly (public mentions PVR). |
| `deny.toml` | ✅ | ✅ | Identical. |
| `Cargo.toml` / `Cargo.lock` / `src/` / `tests/` | ✅ | ✅ | Identical (the project itself). |
| `.gitignore` | ✅ | ✅ | Identical. |
| `.env.example` | ✅ | ✅ | Identical. |
| `AGENTS.md` | ✅ | optional | Internal AI agent rules; safe to publish but not required. |
| `LICENSE` | ✅ | ✅ | Same license (e.g., AGPL-3.0). |
| `docs/REPO-SETUP.md` | ✅ | ✅ | This file. |
| `CONTRIBUTING.md` | — | ✅ | Public-only — how to contribute, dev setup. |
| `CODE_OF_CONDUCT.md` | — | ✅ | Public-only — Contributor Covenant 2.1. |
| `.github/ISSUE_TEMPLATE/` | — | ✅ | Public-only — channel reports. |

---

## Private Repo Bootstrap

Run from inside a freshly-created private GitHub repo (cloned to your
machine).

### Step 1 — Drop in the committed files

Copy the files marked `✅ Private` in the inventory above from this
repo. The minimum set:

```
.pre-commit-config.yaml
.githooks/pre-commit
.githooks/pre-push
.github/CODEOWNERS
.github/dependabot.yml
.github/PULL_REQUEST_TEMPLATE.md
.github/workflows/ci.yml            # trimmed version
.github/workflows/scorecard.yml     # workflow_dispatch only
.github/workflows/gitleaks-history.yml
SECURITY.md
deny.toml
.gitignore
.env.example
LICENSE
docs/REPO-SETUP.md                  # this file
```

Make the hook scripts executable:

```bash
chmod +x .githooks/pre-commit .githooks/pre-push
```

### Step 2 — Enable free server-side controls

```bash
REPO=<owner>/<repo>

gh api -X PUT /repos/$REPO/vulnerability-alerts
gh api -X PUT /repos/$REPO/automated-security-fixes
```

Verify:

```bash
gh api /repos/$REPO/automated-security-fixes | jq
# → {"enabled": true, "paused": false}
```

Note: PVR, secret scanning, push protection, and branch protection all
return 403/422/404 on Free private. Don't bother trying — they unlock
on the public mirror.

### Step 3 — Local clone setup

On every clone the project gets:

```bash
cargo install cargo-deny cargo-audit
git config core.hooksPath .githooks

# Optional: remove any stale wrappers from a prior `pre-commit install`.
rm -f .git/hooks/pre-commit .git/hooks/pre-push
```

`pre-commit` itself must be installed system-wide (via pipx / pip).
The committed wrappers in `.githooks/` invoke whichever `pre-commit`
is on `PATH`.

Verify:

```bash
git config core.hooksPath        # → .githooks
ls .githooks/                    # → pre-commit  pre-push
pre-commit run --all-files       # all commit-stage hooks Pass
pre-commit run --all-files --hook-stage pre-push   # cargo-deny + cargo-audit Pass
```

### Step 4 — First push

The pre-push hook will run gitleaks + tracked-file + local-paths
checks, then `cargo deny check` + `cargo audit`. All must pass before
the push is accepted.

CI runs 4 jobs (`Rust build and test (macos-latest|windows-latest)`,
`Policy checks (macos-latest|windows-latest)`). That's the entire
private CI surface.

---

## Public Mirror Bootstrap

### Step 1 — Create the public repo

```bash
PUB=<owner>/<public-repo-name>
gh repo create $PUB --public --license agpl-3.0     # or your license
```

### Step 2 — Decide on a sync mechanism

See [Sync Mechanism](#sync-mechanism) below. Pick before pushing
content — it affects branch naming and what should/shouldn't live on
private.

### Step 3 — Seed initial content

Push the private repo's current main (or a filtered subset) to the
new public repo. The bulk of files in the inventory carry over unchanged.

### Step 4 — Add public-only files

Create:

```
CONTRIBUTING.md
CODE_OF_CONDUCT.md
.github/ISSUE_TEMPLATE/bug_report.md
.github/ISSUE_TEMPLATE/feature_request.md
.github/ISSUE_TEMPLATE/config.yml         # disable blank issues, link to security policy
```

Optional: `.github/FUNDING.yml`, README badges (CI status, license,
Scorecard).

### Step 5 — Restore the full CI surface

Edit `.github/workflows/ci.yml` on the public repo:

- Re-add the `rust-lint` job (ubuntu, fmt + clippy, with harden-runner
  block-mode + persist-credentials: false)
- Re-add the `supply-chain` job (ubuntu, cargo-deny + cargo-audit, with
  harden-runner)
- Add `ubuntu-latest` back to the `rust-test` and `policy` matrices;
  re-add the Linux harden-runner step on both
- Add a `dependency-review` job (PR-only) using
  `actions/dependency-review-action` with `fail-on-severity: moderate`

Use SHAs already pinned in the private repo's git history as the
canonical reference.

Edit `.github/workflows/scorecard.yml` on the public repo:

- Replace `on: workflow_dispatch:` with the full original triggers:

  ```yaml
  on:
    branch_protection_rule:
    schedule:
      - cron: "23 4 * * 1"
    push:
      branches: [main]
  ```

- Set `publish_results: true` in the `ossf/scorecard-action` step
- Add `id-token: write` to the job permissions (needed for
  `publish_results: true`)

`gitleaks-history.yml` stays unchanged.

### Step 6 — Enable server-side controls

```bash
PUB=<owner>/<public-repo-name>

# Secret scanning + push protection (free on public)
gh api --method PATCH /repos/$PUB --input - <<'EOF'
{"security_and_analysis":{"secret_scanning":{"status":"enabled"},"secret_scanning_push_protection":{"status":"enabled"}}}
EOF

# Private vulnerability reporting (free on public)
gh api -X PUT /repos/$PUB/private-vulnerability-reporting

# Always free: Dependabot alerts + security updates
gh api -X PUT /repos/$PUB/vulnerability-alerts
gh api -X PUT /repos/$PUB/automated-security-fixes

# Branch protection (free on public)
cat > /tmp/pub-branch-protection.json <<'EOF'
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
gh api -X PUT /repos/$PUB/branches/main/protection --input /tmp/pub-branch-protection.json
```

For repos accepting community PRs, bump `required_approving_review_count`
to `1` and flip `require_code_owner_reviews` to `true`.

### Step 7 — Verify the public mirror

```bash
PUB=<owner>/<public-repo-name>

gh api /repos/$PUB | jq '.security_and_analysis'
# → secret_scanning + secret_scanning_push_protection both "enabled"

gh api /repos/$PUB/branches/main/protection | jq '.required_status_checks.contexts'
# → returns all 8 contexts

gh pr create --draft --title "verify ci" --body "noop"
# → opens a PR running full 8-job CI + dep-review

gh workflow run scorecard.yml --repo $PUB
# → SARIF appears in Security tab within ~5 min

gh workflow run gitleaks-history.yml --repo $PUB
# → completes green
```

Sanity test: try pushing a fake AWS access key on a scratch branch.
GitHub push protection should reject it server-side.

---

## Sync Mechanism

How the public repo receives content from private. This project uses
**filtered per-commit replay** via `scripts/sync-to-public.sh`:

- For each new non-merge commit in `<base>..HEAD` on private,
  `git format-patch` produces a patch. Merge commits in the range are
  skipped — their content reaches public via the individual side-branch
  commits they merged, producing a linear public history.
- An awk filter (`scripts/sync-lib/filter-patch.awk`) drops `diff --git`
  blocks that touch private-only paths (`AGENTS.md`, `tasks/`,
  `.agents/`, `.codex/`, `.claude/`, `scripts/`, `target/`,
  `_AGENT_HANDOFF.md`) or override-managed paths (`CONTRIBUTING.md`,
  `.github/workflows/ci.yml`, `.github/workflows/scorecard.yml`).
- A `Synced-From: <private-sha>` trailer is appended to every patch's
  message body, so the next sync can detect the range automatically.
- The filtered patch series is applied to a fresh
  `sync/<date>-<short-sha>` branch in the public clone via `git am`,
  which preserves author identity, dates, and commit message.
- A single trailing commit re-materializes the public-only files from
  `scripts/public-overrides/` if their content drifts from what's on
  the public branch (back-to-back syncs produce zero override commits
  when nothing changed).
- Before handing back to the user, `scripts/sync-lib/run-gauntlet.sh`
  runs a five-step local security gauntlet against the sync branch:
  tracked-path scan, private-path string grep (allowlist for this
  doc), full-tree `gitleaks detect`, `gitleaks detect
  --log-opts="main..HEAD"` over the new range, and a full
  `pre-commit run --all-files` plus the same with
  `--hook-stage pre-push`. Any failure aborts the script with the
  sync branch left in place for inspection.

The sync script never pushes — review locally and push manually so
the public clone's committed pre-push hook still runs at push time
(`gitleaks` + `cargo deny check` + `cargo audit`).

### `Synced-From:` trailer and bootstrapping

`scripts/sync-to-public.sh` reads the most recent
`Synced-From: <sha>` trailer on the public clone's `main` to compute
its range base. On the very first run (no trailer yet on public),
pass `--base <private-sha>` matching whichever private commit the
public main is currently consistent with. Each subsequent sync picks
the range up automatically.

### Trade-offs vs. other mechanisms

| Mechanism | Setup effort | Trade-off |
|---|---|---|
| **Filtered per-commit replay** (this repo) | Medium one-shot setup | Per-commit public history, same authors/dates/messages, no force-push, no history rewrite. Cost: merge bubbles flatten to linear; `git am` conflicts require manual resolution. |
| **Force-mirror** (`git push --mirror`) | Lowest | All commits + branches leak to public, including WIP / private notes. Pick only if private really has nothing sensitive. |
| **[`git filter-repo`](https://github.com/newren/git-filter-repo)** | Medium per sync | Rewrites and force-pushes; loses any public-side state between runs. |
| **Manual squash-merge of releases** | Highest per release | Cleanest public history; public sees one commit per release. Private and public diverge in commit shape but track each other in content. |

---

## Local Developer Setup (one-liner reference)

For both private and public repos:

```bash
cargo install cargo-deny cargo-audit
git config core.hooksPath .githooks
rm -f .git/hooks/pre-commit .git/hooks/pre-push    # remove stale wrappers if any
```

Plus `pre-commit` itself installed system-wide:

```bash
pipx install pre-commit
# or:
pip install --user pre-commit
```

---

## Maintenance Notes

- **Dependabot updates land separately on each repo.** Apply manually if syncing release-by-release.
- **The Scorecard workflow file lives in both repos but is functionally inert on private.** When migrating, the only change is the `on:` block.
- **Branch protection contexts must exactly match CI job names** (case-sensitive, including matrix expansion). If you add or rename a job, update the required-status-checks list via `gh api -X PUT .../branches/main/protection`.
- **The CI allowlist for `harden-runner`** is a starter list. First real dependency that pulls from a new endpoint (e.g., a build-script fetching a vendored library) will fail with `blocked egress` — add the endpoint to the allowlist and re-run.
- **`required_linear_history`** on the public ruleset means PRs must be squashed or rebased — merge commits are blocked. Adjust if your workflow expects merge commits.
