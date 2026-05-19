#!/usr/bin/env bash
# scripts/sync-to-public.sh
#
# Replay new private commits onto a sibling public clone as filtered
# patches, then run a local security gauntlet. Stops before push so the
# user can review.
#
# Range detection: the most recent `Synced-From: <sha>` trailer on the
# public clone's `main` defines the base; pass `--base <sha>` to override.
#
# For each non-merge commit in <base>..HEAD on private, `git format-patch`
# produces a patch; commits marked private-only (either via a
# `Private-Only: true` trailer in the message body, or by SHA in
# scripts/private-only-commits) are dropped entirely; filter-patch.awk
# drops diff blocks touching private-only or override-managed paths; the
# patch series is applied via `git am` onto a fresh sync/<date>-<short-sha>
# branch in the public clone, preserving author identity, dates, and
# messages. Merge commits in the range are omitted by format-patch's
# default — their content reaches public via the individual side-branch
# commits, producing a linear public history. A trailing "Apply
# public-overrides @ <sha>" commit is added if public-overrides content
# drifts from the public tree.
#
# Before any patch reaches the public clone, leak-scan.sh checks each
# retained patch's commit message body and added diff lines against the
# same private-path regex used by the gauntlet — fail-fast with a clear
# remediation message.
#
# Then run-gauntlet.sh runs six checks (tracked-path scan, tree
# path-string grep, commit-message path-string scan over the new range,
# full-tree gitleaks, gitleaks history over the new range, and a full
# pre-commit + pre-push replay) against the produced branch.
#
# Usage:
#   scripts/sync-to-public.sh [--base <private-sha>] [--dry-run] /path/to/public-clone

set -euo pipefail

print_usage() {
  cat >&2 <<'USAGE'
Usage: scripts/sync-to-public.sh [--base <private-sha>] [--dry-run] /path/to/public-clone

  --base <sha>   Range base on private side. Default: parse the most
                 recent `Synced-From:` trailer from public main.
  --dry-run      Produce filtered patches under /tmp/sync-to-public-<sha>/
                 and exit; do not touch the public clone.

The destination must be an existing git clone of the public mirror.
USAGE
}

DRY_RUN=0
BASE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run)   DRY_RUN=1; shift ;;
    --base)      BASE="${2:-}"; shift 2 ;;
    --base=*)    BASE="${1#--base=}"; shift ;;
    -h|--help)   print_usage; exit 0 ;;
    --)          shift; break ;;
    -*)          echo "Unknown flag: $1" >&2; print_usage; exit 1 ;;
    *)           break ;;
  esac
done

PUBLIC_DIR="${1:-}"
if [ -z "$PUBLIC_DIR" ] || [ ! -d "$PUBLIC_DIR/.git" ]; then
  print_usage
  exit 1
fi
PUBLIC_DIR="$(cd "$PUBLIC_DIR" && pwd)"

# Derive REPO_ROOT from the script's own location so the script can be
# invoked from any cwd (e.g. /tmp, the public clone, etc.).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB_DIR="$SCRIPT_DIR/sync-lib"
OVERRIDES_DIR="$SCRIPT_DIR/public-overrides"
SKIPLIST_FILE="$SCRIPT_DIR/private-only-commits"

# All git operations on the private repo go through REPO_ROOT so cwd
# does not matter.
cd "$REPO_ROOT"
if [ "$(git rev-parse --show-toplevel)" != "$REPO_ROOT" ]; then
  echo "ERROR: $REPO_ROOT is not the private git repo root." >&2
  exit 1
fi

for f in "$LIB_DIR/filter-patch.awk" "$LIB_DIR/run-gauntlet.sh" "$LIB_DIR/leak-scan.sh"; do
  if [ ! -e "$f" ]; then
    echo "ERROR: missing helper $f" >&2
    exit 1
  fi
done
if [ ! -d "$OVERRIDES_DIR" ]; then
  echo "ERROR: $OVERRIDES_DIR does not exist" >&2
  exit 1
fi

# Paths excluded from replayed patches. Private-only paths are dropped
# entirely; override-managed paths are dropped from patches and instead
# re-materialized from $OVERRIDES_DIR in a single trailing commit.
EXCLUDES='^(AGENTS\.md|_AGENT_HANDOFF\.md|tasks/|\.agents/|\.codex/|\.claude/|scripts/|target/|CONTRIBUTING\.md|\.github/workflows/ci\.yml|\.github/workflows/scorecard\.yml)'

PRIV_HEAD="$(git rev-parse HEAD)"
PRIV_SHORT="$(git rev-parse --short HEAD)"

# Resolve base from public main's most recent Synced-From trailer.
if [ -z "$BASE" ]; then
  if git -C "$PUBLIC_DIR" rev-parse --verify --quiet main^{commit} >/dev/null; then
    BASE="$(git -C "$PUBLIC_DIR" log main --format=%B \
      | awk '/^Synced-From:[[:space:]]+[0-9a-f]+/ { print $2; exit }' || true)"
  fi
  if [ -z "$BASE" ]; then
    echo "ERROR: no Synced-From trailer on public main (or public has no main yet)." >&2
    echo "       Pass --base <private-sha> to set the range base explicitly." >&2
    exit 1
  fi
fi

if ! git rev-parse --verify --quiet "$BASE^{commit}" >/dev/null; then
  echo "ERROR: --base $BASE is not a valid commit in the private repo." >&2
  exit 1
fi
if ! git merge-base --is-ancestor "$BASE" HEAD; then
  echo "ERROR: --base $BASE is not an ancestor of private HEAD ($PRIV_SHORT)." >&2
  exit 1
fi

BASE_SHORT="$(git rev-parse --short "$BASE")"
RANGE_COUNT="$(git rev-list --no-merges --count "$BASE..HEAD")"
echo "→ private range: $BASE_SHORT..$PRIV_SHORT ($RANGE_COUNT non-merge commits)"

PATCH_DIR="/tmp/sync-to-public-$PRIV_SHORT"
rm -rf "$PATCH_DIR"
mkdir -p "$PATCH_DIR"

if [ "$RANGE_COUNT" -gt 0 ]; then
  echo "→ git format-patch $BASE_SHORT..$PRIV_SHORT → $PATCH_DIR"
  # format-patch on a range walks all commits, skipping merges by
  # default — exactly what we want: every individual file-changing
  # commit on side branches becomes its own patch, applied linearly
  # to the public sync branch. Merge bubbles flatten out.
  #
  # --no-stat is intentionally NOT used: it also removes the "---"
  # separator we anchor the Synced-From trailer to. The diffstat is
  # informational only (git am does not apply it) and may mention
  # excluded files harmlessly — only the diff hunks decide what lands.
  git format-patch \
    --no-signature \
    --no-numbered \
    -o "$PATCH_DIR" \
    "$BASE..HEAD" >/dev/null

  # Build the skiplist of full-SHA private-only commits. Sources:
  #   1. $SKIPLIST_FILE — operator-maintained file of SHAs, useful for
  #      already-merged commits where amending the message is not an
  #      option without rewriting history.
  #   2. `Private-Only: true` trailer on the source commit message —
  #      preferred for new commits.
  SKIP_SHAS=""
  if [ -f "$SKIPLIST_FILE" ]; then
    while IFS= read -r raw; do
      # strip inline comments and surrounding whitespace
      entry="${raw%%#*}"
      entry="$(echo "$entry" | tr -d '[:space:]')"
      [ -z "$entry" ] && continue
      if full="$(git rev-parse --verify --quiet "$entry^{commit}" 2>/dev/null)"; then
        SKIP_SHAS+="$full"$'\n'
      else
        echo "WARN: skiplist entry '$entry' did not resolve to a commit; ignoring" >&2
      fi
    done < "$SKIPLIST_FILE"
  fi

  echo "→ filter patches + inject Synced-From trailer"
  for p in "$PATCH_DIR"/*.patch; do
    orig_sha="$(awk '/^From [0-9a-f]+ Mon Sep 17/ { print $2; exit }' "$p")"
    if [ -z "$orig_sha" ]; then
      echo "WARN: could not extract source SHA from $(basename "$p"); skipping trailer" >&2
    fi

    # Drop private-only commits before any filtering/trailer work.
    drop_reason=""
    if [ -n "$orig_sha" ] && printf '%s' "$SKIP_SHAS" | grep -qxF "$orig_sha"; then
      drop_reason="skiplist"
    elif awk '
        # Walk only the in-reply-to/message section: stop at the first "---".
        /^---$/ { exit }
        # Match "Private-Only: true" (case-insensitive on key, value=true).
        tolower($0) ~ /^private-only:[[:space:]]*true[[:space:]]*$/ { found=1; exit }
        END { exit (found ? 0 : 1) }
      ' "$p"; then
      drop_reason="trailer"
    fi
    if [ -n "$drop_reason" ]; then
      subj="$(awk '/^Subject: / { sub(/^Subject: (\[PATCH\] )?/, ""); print; exit }' "$p")"
      short="${orig_sha:0:7}"
      echo "  dropping ($drop_reason): ${short:-???????}  $subj"
      rm -f "$p"
      continue
    fi

    awk -v EXCLUDES="$EXCLUDES" -f "$LIB_DIR/filter-patch.awk" "$p" > "$p.tmp"

    # Insert "" + Synced-From trailer right before the first "---" separator,
    # so it becomes part of the commit message body that `git am` keeps.
    if [ -n "$orig_sha" ]; then
      awk -v trailer="Synced-From: $orig_sha" '
        BEGIN { done = 0 }
        /^---$/ && !done { print ""; print trailer; done = 1 }
        { print }
      ' "$p.tmp" > "$p"
    else
      mv "$p.tmp" "$p"
    fi
    rm -f "$p.tmp"
  done
fi

echo "→ leak pre-scan (filtered patches)"
leak_any=0
for p in "$PATCH_DIR"/*.patch; do
  [ -e "$p" ] || continue
  if ! "$LIB_DIR/leak-scan.sh" "$p"; then
    leak_any=1
  fi
done
if [ "$leak_any" -ne 0 ]; then
  cat >&2 <<EOF

✗ LEAK PRE-SCAN FAILED.

One or more retained patches contain private-path references in their
commit message body or in added diff lines. The script will NOT touch
the public clone.

To proceed, choose one per leaky commit:

  1. Add a \`Private-Only: true\` trailer to the source commit
     (preferred for not-yet-pushed commits — amend or rebase locally
     and re-run).
  2. Add the source SHA to scripts/private-only-commits
     (escape hatch for already-merged commits).
  3. Re-run with --snapshot to collapse the entire range into one
     sanitized snapshot commit on the public side.

Filtered patch artefacts retained at: $PATCH_DIR
EOF
  exit 3
fi

if [ "$DRY_RUN" -eq 1 ]; then
  retained="$(find "$PATCH_DIR" -maxdepth 1 -name '*.patch' -type f | wc -l | tr -d '[:space:]')"
  dropped=$((RANGE_COUNT - retained))
  cat <<EOF

DRY RUN — no changes applied to $PUBLIC_DIR
  patches:        $PATCH_DIR
  would-be base:  $BASE_SHORT (private)
  would-be branch: sync/$(date +%Y%m%d)-$PRIV_SHORT
  range size:     $RANGE_COUNT non-merge commit(s)
  retained:       $retained patch(es)
  dropped:        $dropped patch(es) (private-only via trailer or skiplist)
EOF
  exit 0
fi

# ---- Apply on public ----
cd "$PUBLIC_DIR"

if ! git rev-parse --verify --quiet main^{commit} >/dev/null; then
  cat >&2 <<EOF
ERROR: $PUBLIC_DIR has no 'main' branch / no commits yet.

  This script replays commits onto a sync branch off public 'main'.
  For the first-ever sync, create an initial commit on main first:

    cd $PUBLIC_DIR
    git add -A
    git commit -m "Initial release (seed @ $(git -C "$REPO_ROOT" rev-parse --short "$BASE"))"

  Then re-run this script with --base $(git -C "$REPO_ROOT" rev-parse --short "$BASE").
EOF
  exit 1
fi

echo "→ refresh public/main"
git fetch origin || echo "  (no origin remote; continuing)"
git checkout main
git pull --ff-only 2>/dev/null || echo "  (no upstream; continuing)"

BRANCH="sync/$(date +%Y%m%d)-$PRIV_SHORT"
if git rev-parse --verify --quiet "$BRANCH" >/dev/null; then
  echo "ERROR: branch $BRANCH already exists in $PUBLIC_DIR. Delete or rename first." >&2
  exit 1
fi
echo "→ create sync branch $BRANCH"
git checkout -b "$BRANCH"

if [ "$RANGE_COUNT" -gt 0 ]; then
  nonempty=()
  for p in "$PATCH_DIR"/*.patch; do
    if grep -q "^diff --git " "$p"; then
      nonempty+=("$p")
    else
      echo "  skipping fully-excluded patch: $(basename "$p")"
    fi
  done

  if [ "${#nonempty[@]}" -gt 0 ]; then
    echo "→ git am ${#nonempty[@]} patch(es)"
    git am "${nonempty[@]}"
  else
    echo "→ no patches with retained content to apply"
  fi
fi

echo "→ refresh public-overrides into $PUBLIC_DIR"
while IFS= read -r -d '' rel; do
  rel="${rel#./}"
  src="$OVERRIDES_DIR/$rel"
  dst="$PUBLIC_DIR/$rel"
  mkdir -p "$(dirname "$dst")"
  cp "$src" "$dst"
done < <(cd "$OVERRIDES_DIR" && find . -type f -print0)

if [ -n "$(git status --porcelain)" ]; then
  echo "→ commit overrides drift"
  git add -A
  git commit -m "$(cat <<EOF
Apply public-overrides @ $PRIV_SHORT

Refresh CONTRIBUTING.md and public-only workflow overrides from the
private repo's scripts/public-overrides/ tree.

Synced-From: $PRIV_HEAD
EOF
)"
else
  echo "→ overrides already up to date"
fi

# ---- Gauntlet ----
echo ""
echo "→ running local security gauntlet"
echo ""
if ! "$LIB_DIR/run-gauntlet.sh" "$PUBLIC_DIR" main; then
  cat >&2 <<EOF

✗ Gauntlet FAILED. Sync branch $BRANCH left in place at $PUBLIC_DIR for inspection.
  Fix the issue (likely in the private repo, then re-run sync) or delete the branch:
    cd $PUBLIC_DIR && git checkout main && git branch -D $BRANCH
EOF
  exit 2
fi

cat <<EOF

✓ Sync complete. Review and push from the public clone:
  cd $PUBLIC_DIR
  git log main..HEAD
  git diff main..HEAD
  # When happy:
  git push -u origin $BRANCH
  # Or merge to main and push (PR or fast-forward — your call).

Patch artefacts retained at: $PATCH_DIR
EOF
