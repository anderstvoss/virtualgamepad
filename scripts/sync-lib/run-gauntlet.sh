#!/usr/bin/env bash
# scripts/sync-lib/run-gauntlet.sh
#
# Five-step local security gauntlet, run against a freshly-built public
# sync branch before the user reviews and pushes. Designed to fail fast
# with a clear marker so the user knows which check tripped.
#
# Usage:
#   run-gauntlet.sh <public-clone-dir> <base-ref>
#
# Exits non-zero on the first failed check. The sync branch is left
# checked out in the public clone so the user can inspect.

set -euo pipefail

PUBLIC_DIR="${1:?public clone dir required}"
BASE_REF="${2:?base ref required (e.g. main)}"

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

echo "→ [1/5] tracked-private-path scan"
tracked_leaks=""
for p in "${PRIVATE_PATHS[@]}"; do
  hits="$(git ls-files -- "$p" "$p/*" 2>/dev/null || true)"
  if [ -n "$hits" ]; then
    tracked_leaks+="$hits"$'\n'
  fi
done
if [ -n "$tracked_leaks" ]; then
  echo "FAIL [1/5]: private-only paths tracked in public sync branch:" >&2
  printf '%s' "$tracked_leaks" >&2
  exit 1
fi

echo "→ [2/5] private-path string leak grep"
# Word-bounded match for path strings; allowlist intentional mentions.
grep_pattern='(\bAGENTS\.md\b|\b_AGENT_HANDOFF\.md\b|(^|[^A-Za-z0-9._-])tasks/|(^|[^A-Za-z0-9._-])\.agents/|(^|[^A-Za-z0-9._-])\.codex/|(^|[^A-Za-z0-9._-])\.claude/)'
if matches="$(git grep -n -E -I "$grep_pattern" -- "${PATH_GREP_ALLOWLIST[@]}" 2>/dev/null)"; then
  echo "FAIL [2/5]: private-path references found in retained content:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "→ [3/5] full-tree gitleaks scan"
if ! command -v gitleaks >/dev/null 2>&1; then
  echo "FAIL [3/5]: gitleaks not on PATH (install: https://github.com/gitleaks/gitleaks)" >&2
  exit 1
fi
gitleaks detect --source . --no-banner

echo "→ [4/5] gitleaks history scan over ${BASE_REF}..HEAD"
gitleaks detect --source . --no-banner --log-opts="${BASE_REF}..HEAD"

echo "→ [5/5] full pre-commit replay (all-files + pre-push stage)"
if ! command -v pre-commit >/dev/null 2>&1; then
  echo "FAIL [5/5]: pre-commit not on PATH (install via pipx/pip)" >&2
  exit 1
fi
pre-commit run --all-files
pre-commit run --all-files --hook-stage pre-push

echo "✓ gauntlet PASS"
