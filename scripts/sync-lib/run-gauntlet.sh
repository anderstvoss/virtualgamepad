#!/usr/bin/env bash
# scripts/sync-lib/run-gauntlet.sh
#
# Six-step local security gauntlet (seven with --deep), run against a
# freshly-built public sync branch before the user reviews and pushes.
# Designed to fail fast with a clear marker so the user knows which
# check tripped.
#
# Usage:
#   run-gauntlet.sh <public-clone-dir> <base-ref> [--deep]
#
# --deep adds a step 7 that runs gitleaks across the full history of
# every ref (`--log-opts=--all`) instead of only the new range. Slower
# but recommended before publishing.
#
# Exits non-zero on the first failed check. The sync branch is left
# checked out in the public clone so the user can inspect.

set -euo pipefail

PUBLIC_DIR="${1:?public clone dir required}"
BASE_REF="${2:?base ref required (e.g. main)}"
DEEP=0
if [ "${3:-}" = "--deep" ]; then
  DEEP=1
fi

if [ "$DEEP" -eq 1 ]; then
  TOTAL=7
else
  TOTAL=6
fi

cd "$PUBLIC_DIR"

# Paths that must NEVER appear in the public branch.
PRIVATE_PATHS=(
  AGENTS.md
  _AGENT_HANDOFF.md
  tasks
  .agents
  .codex
  .claude
  scripts
  target
)

# Allowlist of files permitted to mention private-path strings in their
# content (e.g., the cross-published setup doc explains the split).
PATH_GREP_ALLOWLIST=(
  ':!docs/REPO-SETUP.md'
)

echo "→ [1/$TOTAL] tracked-private-path scan"
tracked_leaks=""
for p in "${PRIVATE_PATHS[@]}"; do
  hits="$(git ls-files -- "$p" "$p/*" 2>/dev/null || true)"
  if [ -n "$hits" ]; then
    tracked_leaks+="$hits"$'\n'
  fi
done
if [ -n "$tracked_leaks" ]; then
  echo "FAIL [1/$TOTAL]: private-only paths tracked in public sync branch:" >&2
  printf '%s' "$tracked_leaks" >&2
  exit 1
fi

echo "→ [2/$TOTAL] private-path string leak grep"
# Word-bounded match for path strings; allowlist intentional mentions.
grep_pattern='(\bAGENTS\.md\b|\b_AGENT_HANDOFF\.md\b|(^|[^A-Za-z0-9._-])tasks/|(^|[^A-Za-z0-9._-])\.agents/|(^|[^A-Za-z0-9._-])\.codex/|(^|[^A-Za-z0-9._-])\.claude/)'
if matches="$(git grep -n -E -I "$grep_pattern" -- "${PATH_GREP_ALLOWLIST[@]}" 2>/dev/null)"; then
  echo "FAIL [2/$TOTAL]: private-path references found in retained content:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "→ [3/$TOTAL] commit-message scan over ${BASE_REF}..HEAD"
# Same regex as step 2 but scanned across log messages (subject + body)
# of the new commits — closes the "path strings sanitised in the tree
# but still present in `git log`" gap.
if msg_matches="$(git log --format=%H%n%B%n--END--%n "${BASE_REF}..HEAD" \
  | awk -v re="$grep_pattern" '
      BEGIN { sha = "" }
      /^--END--$/ { sha = ""; next }
      sha == "" { sha = $0; next }
      $0 ~ re { printf "%s: %s\n", substr(sha, 1, 12), $0 }
    ')"; then
  if [ -n "$msg_matches" ]; then
    echo "FAIL [3/$TOTAL]: private-path references in commit messages:" >&2
    echo "$msg_matches" >&2
    exit 1
  fi
fi

echo "→ [4/$TOTAL] full-tree gitleaks scan"
if ! command -v gitleaks >/dev/null 2>&1; then
  echo "FAIL [4/$TOTAL]: gitleaks not on PATH (install: https://github.com/gitleaks/gitleaks)" >&2
  exit 1
fi
gitleaks detect --source . --no-banner

echo "→ [5/$TOTAL] gitleaks history scan over ${BASE_REF}..HEAD"
gitleaks detect --source . --no-banner --log-opts="${BASE_REF}..HEAD"

echo "→ [6/$TOTAL] full pre-commit replay (all-files + pre-push stage)"
if ! command -v pre-commit >/dev/null 2>&1; then
  echo "FAIL [6/$TOTAL]: pre-commit not on PATH (install via pipx/pip)" >&2
  exit 1
fi
pre-commit run --all-files
pre-commit run --all-files --hook-stage pre-push

if [ "$DEEP" -eq 1 ]; then
  echo "→ [7/$TOTAL] deep gitleaks (--log-opts=--all over every ref)"
  # Bring in branches and tags so --all reaches them, then scan. The
  # `|| true` on fetch lets the gauntlet still run when offline / no
  # remote.
  git fetch --all --tags --prune --quiet 2>/dev/null || true
  gitleaks detect --source . --no-banner --log-opts='--all'
fi

echo "✓ gauntlet PASS"
