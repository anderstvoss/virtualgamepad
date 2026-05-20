#!/usr/bin/env bash
# scripts/sync-lib/leak-scan.sh
#
# Scan a filtered patch file for private-path string leaks in both the
# commit message body and the diff additions (`+` lines, excluding the
# `+++` file marker). Exits non-zero on any hit, with one
# "LEAK in <patch>:<line>: <text>" line per finding on stderr.
#
# Usage:
#   leak-scan.sh <patch-file>
#
# The regex must stay in sync with run-gauntlet.sh step 2 (tree grep)
# and with the equivalent log-message scan in step 6 — same private
# path set, same word-boundary handling.

set -euo pipefail

LEAK_REGEX='(\bAGENTS\.md\b|\b_AGENT_HANDOFF\.md\b|(^|[^A-Za-z0-9._-])tasks/|(^|[^A-Za-z0-9._-])\.agents/|(^|[^A-Za-z0-9._-])\.codex/|(^|[^A-Za-z0-9._-])\.claude/)'

# Files allowed to mention private-path strings in their content
# (matches the gauntlet's PATH_GREP_ALLOWLIST). Diff `+` lines under
# these paths are skipped. Commit-message lines are always scanned —
# the message body has no per-file scope.
ALLOWLIST_RE='^(docs/REPO-SETUP\.md)$'

PATCH="${1:?patch file required}"
if [ ! -f "$PATCH" ]; then
  echo "ERROR: $PATCH is not a file" >&2
  exit 2
fi

# awk does the splitting in one pass:
#   - header block: until first blank line after a "From "/"Subject:"
#     mail header. (Format-patch puts the message body after the blank
#     line that ends the mail headers.)
#   - message body: from end-of-headers up to the first "---" separator.
#   - diff section: from "---" onward; scan only "+" lines that are not
#     "+++" file markers.
awk -v re="$LEAK_REGEX" -v allow="$ALLOWLIST_RE" -v path="$PATCH" '
  BEGIN { section = "headers"; status = 0; curfile = "" }

  section == "headers" {
    if ($0 == "") { section = "message"; next }
    next
  }

  section == "message" {
    if ($0 == "---") { section = "diff"; next }
    if ($0 ~ re) {
      printf "LEAK in %s:%d (message): %s\n", path, NR, $0 > "/dev/stderr"
      status = 1
    }
    next
  }

  # section == "diff"
  # Track the current target file via "+++ b/<path>" markers so we can
  # honour the per-file allowlist.
  /^\+\+\+ / {
    f = $2
    sub(/^b\//, "", f)
    sub(/^"b\//, "\"", f)
    curfile = f
    next
  }
  /^\+/ {
    if (curfile != "" && curfile ~ allow) next
    if ($0 ~ re) {
      printf "LEAK in %s:%d (diff %s): %s\n", path, NR, curfile, $0 > "/dev/stderr"
      status = 1
    }
  }

  END { exit status }
' "$PATCH"
