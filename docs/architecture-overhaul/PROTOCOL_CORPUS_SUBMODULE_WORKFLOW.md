# Protocol Corpus Git Submodule Workflow

## Current authority and rewrite freedom — 2026-09-05

Development memories and imported conversation/context files are non-authoritative research leads. They cannot establish current implementation, physical truth, a gate pass, or fresh permission. Verify implementation against exact code revisions and merge/content history; verify host/protocol claims with scoped primary sources or experiment artifacts. Current user direction and reviewed decisions govern product scope.

This is early development: breaking APIs, removing obsolete modules, and replacing the internal architecture are acceptable. Preserve evidence and important correctness properties, not historical type names, binary names, crate topology, or dual-runtime compatibility. Git history is sufficient for recovering the old implementation; it need not remain in the shipping build. See [context reassessment](CONTEXT_REASSESSMENT.md), [branch review](BRANCH_REVIEW.md), and [ADR-0002](decisions/ADR-0002-early-development-rewrite.md).


## Revision and authority — 2026-09-04

This revision incorporates the final decisions in “Review And Draft Plan” and a code review of local `main` at `9b466e0`. This is an inspected baseline, not a claim about the latest remote revision or completed host validation. Reconcile the actual checkout before implementation.

`ARCHITECTURE_DECISION_EXPERIMENTS.md` section 17 is the single authority for execution dependencies and exit gates. This file defines its subject's requirements. Examples are illustrative unless explicitly identified as settled contracts. No experiment in this revision is marked passed merely because it is specified.


## Intent

`controller-protocol-corpus` remains an independent Git repository.

`virtualgamepad` includes it as:

```text
virtualgamepad/protocol-corpus/
```

using a standard Git submodule.

The `virtualgamepad` commit therefore pins one exact corpus commit.

## Initial setup

From the `virtualgamepad` repository:

```bash
git submodule add <controller-protocol-corpus-url> protocol-corpus
git commit -m "Add controller protocol corpus submodule"
```

## Clone

Preferred:

```bash
git clone --recurse-submodules <virtualgamepad-url>
```

Existing clone:

```bash
git submodule update --init --recursive
```

## Research + implementation change

1. Enter the submodule:

```bash
cd protocol-corpus
git switch -c <research-branch>
```

2. Perform research/tooling/claim changes.
3. Commit and push them to the corpus repository.
4. Merge or otherwise establish the desired corpus commit.
5. Return to the superproject:

```bash
cd ..
git add protocol-corpus
git commit -m "Update protocol corpus revision"
```

6. Make `virtualgamepad` implementation changes against that pinned revision.

A `virtualgamepad` PR must never point at a corpus commit that exists only in a developer's local clone.

## CI

Checkout must enable submodules.

Corpus-aware CI should fail with an actionable message if `protocol-corpus/` is absent/uninitialized.

## Runtime boundary

The submodule is a development/research/verification dependency.

Do not turn it into a runtime YAML/profile loader.

Preferred flow:

```text
protocol-corpus/
    ↓
tests / codegen / fixture extraction
    ↓
reviewed compiled Rust
```

## Release boundary

Downstream users should not need a Git-aware source tree for ordinary runtime usage if avoidable.

Keep runtime-required generated Rust artifacts in the normal crate source or package source distributions with the necessary generated outputs.

## Why this arrangement

Benefits:
- independent reusable research repository;
- exact corpus revision per implementation commit;
- no manual copy/snapshot drift;
- convenient local access for agents;
- simple provenance.

Costs:
- clones must initialize submodules;
- research + implementation changes often require two repository commits/PRs.

Those costs are accepted.


## Reproducibility and failure checks

- Adopt only corpus commits reachable from the configured remote's retained integration branch or a retained release tag. Fetch and verify reachability in the adoption check; do not pin a local-only commit or rely on a disposable PR ref. Do not rewrite away adopted history.
- Normal checkout uses the recorded gitlink, never `submodule update --remote`. A developer may work on a research branch, but adoption records one reviewed commit. After a squash/rebase merge, explicitly check out the final adopted commit before staging the gitlink; the old local topic tip is not necessarily the merged revision.
- In maintainer CI require initialization, exact HEAD/gitlink equality, a clean corpus worktree, and schema/claim/hash checks. Report absent checkout separately from protocol conformance failure. A dirty corpus is allowed during research, but cannot produce reproducibility signoff.
- Generated outputs record corpus commit, schema version, generator version, and input hashes. Derive this provenance from the gitlink and checked-out content. Regeneration must leave no diff. A generated manifest is diagnostic provenance, not a second editable revision lock.
- Run ordinary package-build verification with no corpus checkout and no network access, with normal Cargo dependencies pre-cached. Runtime-required Rust outputs ship with the crate. Corpus validation/codegen is a separate maintainer command that fails clearly when the submodule is unavailable.
- Exercise fresh recursive clone, ordinary clone plus init, missing submodule, stale generated output, mismatched HEAD, adoption of an unpublished commit, and package build without Git metadata. Keep these as workflow checks rather than kernel/hardware gates.

## Contribution and visibility boundary

The corpus remains independently usable and independently validated. Document how fork CI retrieves it; do not make public conformance depend on a private developer checkout. Do not change visibility, credentials, repository settings, or release policy as an incidental submodule setup action. Resolve actual repository ownership/URL during E1; placeholders in these documents are not an existing remote.

The handoffs are planning instructions, not evidence that repositories, submodules, CI, tags, or releases have been created. Follow the gate register's status records for execution state.

## Authenticated CI with a read-only deploy key

The corpus job uses `PROTOCOL_CORPUS_SSH_KEY`, registered as a read-only deploy key
on the corpus repository only. It is not an account token and grants no write
access or access to other private repositories. The ordinary project checkout
uses its standard ephemeral Actions credential with persistence disabled; only
the separately checked-out corpus uses the deploy key. The corpus revision is
read from the main checkout's gitlink, with history available for the remote
publication check. `actions/checkout` removes its persisted SSH configuration and
key at job cleanup. No key is stored in source, fixtures or artifacts.

The existing public-repository and same-repository-PR conditions remain unchanged.
Fork PRs and ordinary package builds need no corpus credential. Missing access is
an explicit job failure. Same-repository CI code is trusted with corpus read
access, so review changes to that job before running them. Do not replace this
key with a broad personal development token.

For rotation, create a fresh corpus-only read-only deploy key, update the Actions
secret, verify the pinned-corpus job, then revoke the old deploy key. For immediate
revocation, remove the deploy key from the corpus repository and remove the
Actions secret; ordinary builds remain available. The old
PROTOCOL_CORPUS_READ_TOKEN mechanism is no longer used.
