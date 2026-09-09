# Overhaul landing and initial alpha

This maintainer-directed sequence supersedes any interpretation that PR #106 must
implement every controller or pass every extension gate before landing. The gate
ledger retains its evidence and blockers; deferring a gate never counts as passing
it. No merge or release is performed by this planning update.

## 1. Refine PR #106 and stress the architecture with current controllers

Keep DualSense, DS4, Switch Pro and the existing standard-HID/evdev Xbox 360
behavior as the working set. Bring each to the best evidence-supported state
possible. Fix deterministic defects and consumer-visible feature loss; preserve
explicit limitations when hardware or isolated host evidence is unavailable.
Do not grow the working set to hide weaknesses in the current architecture.

Remaining review work, in order:

1. Complete the logical compound ownership design: one identity with derived
   controller-owned component roles, fresh transport state on recreation, bounded
   request ownership and controller-level removal/deadline failure integration.
   Retain fail-closed required-component policy, reverse rollback and independent
   session cleanup. Optional hot-removal requires an explicit controller model.
2. Stress current families with individual input/output observations, repeated
   polling, reused application IDs, concurrent removal, backpressure, uncertain
   delivery and consumer failure. Preserve source-backed best effort for DS4 and
   Switch; physical absence blocks fidelity claims, not deterministic fixes.
3. Review Eden/Steam disagreement using recorded versions, backends and mappings.
   EXP-0016 corrects Xbox HID button usages; Eden/Steam retest is still pending.
   Keep unavailable evidence visible instead of adding speculative axis swaps.
4. Review demo service ownership, bounded edits, readiness/deadlines and shutdown
   as an application integration example. Check provider purity, public API
   ergonomics, corpus publication/CI and ordinary offline package boundaries.
5. Run the complete required checks and hooks, reconcile documentation, and
   produce a final review of remaining defects versus explicitly scoped follow-up
   work. Commit and publish coherent batches on the existing feature branch.

Landing requires reviewer agreement that the supported core is coherent, has no
known uncontained lifecycle/protocol failure, and exposes its limitations honestly.
An unresolved reproducible core defect must be fixed or explicitly scoped out for
review; a green aggregate sweep is insufficient. DS4 split touch, gadget interface
replacement and audio/Bluetooth extensions must not be promoted without their
own gates. Their absence alone does not force speculative implementation in #106.
Keep #106 draft until that review is concrete; merge is a separate delivery step.

## 2. Prepare an initial alpha after landing

Review the public API and feature names, installation guidance, examples, package
contents, dependency boundaries and supported-platform statement. Publish a
changelog, migration notes, known limitations and the support matrices together.
Keep ordinary consumers free of corpus credentials and experiment tooling.

The workspace already declares `0.1.0`; this is a development manifest version,
not evidence of a published release. Proposed first release: `0.1.0-alpha.1`, then
`alpha.2`, etc. Apply a coordinated workspace version and internal dependency
requirement update in the alpha preparation batch, after checking existing tags
and published packages. Check prerelease dependency resolution, lockfile and
package contents before tagging. Do not relabel an existing release or imply
alpha API stability. This plan does not change manifests, publish flags, release
workflows, tags or repository settings.

## 3. Exercise controller addition after the core lands

Open focused controller issues with evidence, native controls, intended
realizations, per-function matrix, unknowns and measurable acceptance criteria.
Use Steam Controller (2026) + Puck, Xbox Series and Wii Remote to test the
controller-addition workflow. Do not assume a shared topology or borrow another
controller's protocol identity. Steam/Puck development remains deferred until
landing. Hardware availability must be verified when each issue begins.

Each addition should prove that source/corpus review, native state/personality,
realization selection, deterministic fixtures, demo lab controls, cleanup tests
and documentation can be added without controller-specific provider branches.
Record architectural friction as core follow-up work, not silent exceptions.

## Support reporting

[CONTROLLER_SUPPORT.md](../CONTROLLER_SUPPORT.md) is the public-facing controller
(vertical) by realization (horizontal) planning matrix. Every cell starts WIP.
Gate results describe measured experiments; they do not automatically promote a
support cell. Per-controller function matrices will record partial capabilities
and evidence separately, so one working gyro or input path cannot imply complete
controller support.
