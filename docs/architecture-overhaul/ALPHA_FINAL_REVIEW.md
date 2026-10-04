# Historical final alpha review handoff

This document preserves the earlier, narrower candidate review. It does not
establish current alpha readiness under the expanded all-implemented-paths gate.
Use [the remediation reviewer handoff](ALPHA_REMEDIATION_REVIEW_HANDOFF.md) and
[status](ALPHA_REMEDIATION_STATUS.md) for the current PR.

## Review blocks and exact candidate

Review and merge in dependency order:

1. PR #130: root freeze candidate and the explicitly authorized CI snapshot guard.
2. PR #131: separate implementation quality review and PipeWire construction
   cleanup retention fix. Public API remains identical to the freeze candidate.
3. PR #132: exact Git consumer tooling/evidence, release notes and this acceptance
   checklist. Its separate audio fix restores the declared Rust 1.85 compatibility
   discovered by expanding release validation to all root features.

These blocks are prepared for review, not automatically merged or accepted.
The exact source candidate tested through Git is
`76b7b3fcb1e10dc073ec637b1c8469cc5c7a4b02` (release-validation audio fix). The consumer evidence
is `ALPHA_GIT_CONSUMER_EVIDENCE.json`: fresh Git resolution, eight locked root-only
compile combinations and cached offline rebuild; consumer and lock hashes identify
what was tested; its adjacent `.lock` file preserves the resolved dependency set
for review. The consumer imports no supporting crate or private corpus.
Source-path and deterministic fake lifecycle tests supplement this compile evidence;
Git compilation does not exercise hardware, PCM delivery or latency.

After squash merges, the final main commit has a different identity. Before final
acceptance/tagging, rerun the exact Git consumer command against that main SHA and
require its CI to pass. Later evidence/docs changes do not change the tested runtime
or API; a different merge commit still requires validation. Record accepted freeze
and release SHAs in the review.

## Prepared validation

- Locked workspace fmt, all-target/all-feature check and strict clippy passed.
- Workspace tests: 533 passed, 56 ignored; ignored probes are not passes.
- Strict workspace docs and fresh default/all-feature 110-item snapshot passed.
- Root-only source-path feature matrix and cached offline rebuild passed.
- Exact Git matrix/offline rebuild passed at the revision above.
- Python tooling: 98 tests passed; snapshot drift/module/trait tests and exact-SHA
  input tests are deterministic and sanitized.
- MSRV Rust 1.85 default and all-feature root compilation passed. The expanded
  check caught an audio worker let-chain requiring newer Rust; an equivalent tuple
  pattern restores compatibility. The socket regression also exercises an empty
  microphone write followed by nonempty delivery and terminal close for each profile.
- Cargo audit/deny passed with existing ttf-parser unmaintained and duplicate/
  license-policy warnings; no new suppression or dependency.
- Full-history/public-source secret scans and whitespace checks passed.
- PR CI supplies native Linux/macOS/Windows compile/test, corpus pin and policy
  evidence. CodeQL runs automatically for the main-targeted freeze PR; dispatch
  the existing workflow for the stacked final head. Require green final-head checks; network
  installation failures are retained/retried rather than called passing tests.

## Scope for acceptance

The root library defaults remain empty. Local Linux UHID + optional emulated
PipeWire audio is the implemented local-session path. USB/IP remains WIP/excluded
from accepted support pending #115's installed security/recovery acceptance.
Controller matching remains unavailable pending #116. No audio continuity, p99
latency, physical microphone fidelity, or new-family support guarantee is proposed.
Historical failed VM tests and confirmed physical output/reconnect results remain.
Non-Linux API compilation is not non-Linux provider implementation.

The GUI refinement has actual maintainer observations, approval and merged fixes.
#126 still has a broader unchecked matrix than those observations. Final review
must either record any additional exercises performed or explicitly accept the
bounded reviewed GUI scope and identify deferred cases. Do not mark unperformed
cases passing or require a fabricated full hardware session to close a scoped gate.
#112 is closed, but its unchecked native matrix is not latency acceptance.
#117 remains the sequence umbrella until the maintainer accepts the package.

## Maintainer final acceptance checklist

- [ ] Review the three PRs, their scoped evidence, fixes and migration notes; resolve
  findings and require checks at each final head before merging in order.
- [ ] Accept the ordinary root contract, snapshot policy, Git-based release model
  and support exclusions; record the exact accepted freeze commit.
- [ ] Reconcile #126 with actual reviewed observations and explicitly accepted or
  deferred cases. #115/#116 remain separate work when their claims are excluded.
- [ ] Accept the separate quality review and known dependency/native scheduling
  limits; reopen API review for any fix that changes public semantics.
- [ ] Run final locked workspace checks, docs/negative boundaries, Python tooling,
  snapshot, MSRV, security/corpus checks and platform CI on the final main SHA.
- [ ] Run `python3 scripts/check-alpha-git-consumers.py --revision <full-final-sha>
  --report <review-output.json>`; review exact resolution and offline results.
- [ ] Review release notes/version/tag naming. Proposed Git tag: `0.1.0-alpha.1`;
  workspace package version remains `0.1.0`, unpublished. This is a Git alpha stage,
  not a registry release or 1.0 compatibility promise.
- [ ] Record final accepted release SHA, limitations and disposition of #117.
- [ ] Separately authorize tagging/publication. This package does not create a tag,
  GitHub release, registry publication or repository-setting change.

## Reproduction commands

```bash
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features
python3 -m unittest discover -s scripts/tests
python3 scripts/check-alpha-api.py
python3 scripts/check-root-consumers.py
cargo +1.85 check --locked -p virtualgamepad
cargo +1.85 check --locked -p virtualgamepad --all-features
cargo audit
cargo deny check
gitleaks detect --redact
python3 scripts/check-protocol-corpus.py
```

The Git consumer requires the public commit to be reachable, ordinary Cargo/Rust
build prerequisites and feature-specific native build prerequisites. It creates a
disposable standalone package, never opens controllers or changes host routing,
and preserves its newly generated consumer lock across locked/offline checks.
