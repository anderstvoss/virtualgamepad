# Filter a `git format-patch` file: drop diff blocks whose `a/` or `b/` path
# matches the EXCLUDES regex. The EXCLUDES variable is expected to be an
# extended regex anchored at the start of the path (e.g. ^(AGENTS\.md|tasks/|...)).
#
# Everything outside diff blocks (mail headers, message body, diffstat, and
# the trailing git signature) is passed through verbatim. The diffstat may
# still mention excluded files; that is informational and not applied by
# `git am`.
#
# Usage:
#   awk -v EXCLUDES="$REGEX" -f filter-patch.awk <patch>

BEGIN { skip = 0 }

/^diff --git / {
    a = $0
    sub(/^diff --git a\//, "", a)
    sub(/ b\/.*$/, "", a)

    b = $0
    sub(/^diff --git a\/[^ ]* b\//, "", b)

    if (a ~ EXCLUDES || b ~ EXCLUDES) {
        skip = 1
        next
    }
    skip = 0
}

# Git's signature terminator ("-- ") marks end of diffs; resume passthrough.
/^-- $/ { skip = 0 }

{ if (!skip) print }
