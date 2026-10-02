# Alpha freeze candidate

This review block establishes the ordinary root snapshot after the merged post-GUI
review (#129, `7bb16df`). It is prepared for maintainer review, not a declaration
that final alpha acceptance or release has happened. Its exact source commit is
the head of the freeze-candidate PR; after merging, record the resulting main SHA
as the accepted freeze revision and repeat exact-revision release validation.

`ALPHA_API_SNAPSHOT.json` records all ordinary root declarations, methods, fields,
associated constants, concrete/public auto traits, feature names and empty default
features. Experimental exports and supporting SPI are excluded. Unknown root
modules fail closed rather than silently escaping the snapshot. Rustdoc compiler
implementation traits (Freeze, UnsafeUnpin, StructuralPartialEq) are excluded;
Send/Sync and their negative forms remain compatibility-relevant. The repository
pins Rust 1.95.0; changing that pin requires reviewing the snapshot/tooling too.

The Rust lint CI job enforces the reviewed snapshot on every existing CI trigger.
The maintainer explicitly authorized this single workflow step; release workflows
and repository settings are unchanged.

Run `python3 scripts/check-alpha-api.py`. It rebuilds locked strict all-feature
default and all-feature root documentation in an isolated target directory and compares semantic data,
not platform source URLs. A changed snapshot fails. Only an explicit `--write`
refresh updates the baseline; a refresh requires API review, demonstrated rationale
for breaking changes, and migration notes. This detects signature/trait/feature
drift; semantic changes still require review and cannot be proved by a snapshot.

The intended alpha is Git-based, with internal registry publication disabled.
Accepted claims retain the support/evidence boundaries in `CONTROLLER_SUPPORT.md`
and `CONTROLLER_AUDIO.md`. USB/IP is WIP pending installed security/recovery (#115),
matching is unavailable pending #116, and there is no audio continuity/latency
promise. Existing failed experiments remain evidence. The reviewed GUI scope has
maintainer feedback; #126's broader unperformed matrix is a final review checklist,
not an inferred pass. #117 remains open until the whole sequence is accepted.

After review: freeze the accepted contract, then review the separate quality block.
Public-semantic fixes reopen the affected API item. Final release preparation must
prove the exact Git candidate with locked consumers, offline rebuild, MSRV,
platform CI, security checks and reconciled support/migration/changelog statements.
Tagging, publication, merges and final acceptance remain maintainer actions.
