# scripts/

Local operator helpers.

| Script | Purpose |
|---|---|
| [deep-scan.sh](deep-scan.sh) | Full-history gitleaks across every branch and tag (`--log-opts=--all`). Run ad-hoc before publishing or merging anything that matters. The pre-commit and pre-push hooks only scan staged content and the current ref's history; this closes the un-merged-branch gap. |

Requires `gitleaks` on PATH.
