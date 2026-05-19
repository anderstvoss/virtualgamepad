# scripts/

Tooling for the **private → public mirror sync**. This whole directory
is excluded from the public mirror; everything here is internal.

The authoritative design / rationale lives in
[`docs/REPO-SETUP.md`](../docs/REPO-SETUP.md#sync-mechanism). This file
is the day-to-day operator's guide.

## Layout

```
scripts/
├── sync-to-public.sh        # orchestrator — the only entry point
├── sync-lib/
│   ├── filter-patch.awk     # strips excluded paths from each patch
│   └── run-gauntlet.sh      # five-step local security gauntlet
├── public-overrides/        # files copied into public verbatim
│   ├── CONTRIBUTING.md
│   └── .github/workflows/   # full-surface ci.yml + scorecard.yml
└── README.md                # this file
```

## What the sync does

For each new commit on private since the last sync:

1. `git format-patch` produces a mailbox patch.
2. `sync-lib/filter-patch.awk` drops `diff --git` blocks for any path
   that is private-only or override-managed (see [Filtering](#filtering)
   below).
3. A `Synced-From: <private-sha>` trailer is appended so the next sync
   can detect the range automatically.
4. `git am` applies the filtered patch series onto a fresh
   `sync/<date>-<short-sha>` branch in the public clone, preserving
   author identity, dates, and messages.
5. If `scripts/public-overrides/` content differs from what's on the
   public branch, a single trailing commit re-materializes it.
6. `sync-lib/run-gauntlet.sh` runs five local checks (see
   [Gauntlet](#gauntlet)). Any failure aborts the script with the sync
   branch left in place for inspection.

The sync **never pushes**. Review locally and push manually so the
public clone's own committed pre-push hook still runs at push time.

## Usage

Normal sync — base auto-detected from the most recent `Synced-From:`
trailer on public `main`:

```bash
scripts/sync-to-public.sh ~/Projects/virtualgamepad
```

First-ever sync (no trailer on public yet) — pass `--base` explicitly:

```bash
scripts/sync-to-public.sh --base <private-sha-matching-public-main> ~/Projects/virtualgamepad
```

Inspect what would happen without touching the public clone:

```bash
scripts/sync-to-public.sh --dry-run --base <sha> ~/Projects/virtualgamepad
# Filtered patches land under /tmp/sync-to-public-<short-sha>/
```

After a successful sync, the script prints the next steps:

```bash
cd ~/Projects/virtualgamepad
git log main..HEAD
git diff main..HEAD
git push -u origin sync/<date>-<sha>
```

Exit codes: `0` success, `1` usage / setup error, `2` gauntlet failure.

## Bootstrap (one-time)

The public clone needs `main` to exist before the first sync. If you're
starting from scratch:

```bash
cd ~/Projects/virtualgamepad
git add -A
git commit -m "Initial release (seed @ <private-short-sha>)"
# Then run the first sync with --base <same-sha>
```

`<private-short-sha>` is whichever private commit the public clone's
working tree currently matches (rsynced/seeded from).

## Filtering

Two categories of paths never appear on public:

- **Private-only** (dropped entirely): `AGENTS.md`, `_AGENT_HANDOFF.md`,
  `tasks/`, `.agents/`, `.codex/`, `.claude/`, `scripts/`, `target/`.
- **Override-managed** (dropped from patches; re-materialized from
  `scripts/public-overrides/`): `CONTRIBUTING.md`,
  `.github/workflows/ci.yml`, `.github/workflows/scorecard.yml`.

To add a new private-only path or a new override file, edit both:

- the `EXCLUDES` regex in [`sync-to-public.sh`](sync-to-public.sh), and
- the `PRIVATE_PATHS` array in [`sync-lib/run-gauntlet.sh`](sync-lib/run-gauntlet.sh)
  for private-only paths (so the gauntlet catches regressions).

## Gauntlet

Runs against the produced sync branch in the public clone, fail-fast:

| # | Check | Purpose |
|---|---|---|
| 1 | Tracked private paths | Catches filter bugs — private-only paths must not be tracked on public. |
| 2 | Private-path string grep | Catches stale doc links to `tasks/`, `AGENTS.md`, etc. in retained content. `docs/REPO-SETUP.md` is allowlisted. |
| 3 | Full-tree `gitleaks detect` | Catches anything that slipped past per-commit incremental scans. |
| 4 | `gitleaks detect --log-opts=main..HEAD` | History-mode scan over the just-produced commit range. |
| 5 | `pre-commit run --all-files` + `--hook-stage pre-push` | Full replay of every committed hook (`cargo fmt`, `clippy -D warnings`, `cargo test`, `cargo deny`, `cargo audit`, local pygrep blockers). |

Requires `gitleaks`, `pre-commit`, `cargo-deny`, `cargo-audit` on PATH.

## Troubleshooting

- **`git am` conflict** — a private commit's hunk doesn't apply against
  the public branch (likely because public diverged or a prior sync was
  incomplete). The script leaves you in `git am` mid-stream; resolve
  with `git am --skip` / `--abort` / fix-and-`--continue`, then re-run
  the gauntlet manually: `scripts/sync-lib/run-gauntlet.sh ~/Projects/virtualgamepad main`.
- **Gauntlet step 1 fails** — the filter regex missed a path. Update
  `EXCLUDES` and re-run.
- **Gauntlet step 2 fails** — a retained file mentions a private path
  in its content. Either edit the source on private (preferred) or
  add the file to the allowlist in `run-gauntlet.sh` if the mention is
  intentional.
- **Gauntlet step 5 fails on `cargo test`** — the public sync branch
  has the same code as private; if `cargo test` fails on public it
  should already be failing on private. Fix on private, re-sync.
- **`--base <sha>` is not an ancestor** — you passed a SHA that isn't on
  the private branch you're syncing from. Verify with
  `git merge-base --is-ancestor <sha> HEAD`.
- **Sync branch already exists** — delete it on the public clone:
  `git -C ~/Projects/virtualgamepad branch -D sync/<date>-<sha>`.

## Adding a public-only file

Drop it under `scripts/public-overrides/` at its target path
(e.g. `scripts/public-overrides/.github/FUNDING.yml` →
`/.github/FUNDING.yml` on public). The next sync's overrides-refresh
commit will land it.

If the file would shadow a private file, also add the private path to
the `EXCLUDES` regex so the patch filter drops the private version.
