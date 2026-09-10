# Core rewrite assessment — 2026-09-09

Scope: current feature branch and PR #106, four existing controller families.
This assessment supersedes the earlier incremental status on this page. The
[gate ledger](GATE_STATUS.md) remains authoritative for experiment scope. The
full E0–E6 extension roadmap is not complete; no merge, release, new controller
or support-cell promotion is included.

## Remaining-item disposition

| Review item | Implemented evidence | Limitation or resumption prerequisite |
| --- | --- | --- |
| Compound identity and roles | ADR-0010: distinct logical/creation identities, controller-owned roles, associated UHID/uinput labels; duplicate roles and failure at every open position covered. Test-only DS4 prototype checks shared creation prefix, distinct role suffixes and independent reused-ID creations. | Requested labels plus verified host ancestry are required for actual association. DS4 split production adoption still needs an isolated host. |
| Required-component lifecycle and service | Native terminal containment plus `RequiredGroup`: fair bounded service, earliest deadlines, readiness, component-scoped requests, expiry, uncertain-delivery cancellation and terminal reopen rejection. Final audit retains observations from the component whose later action failed. | Optional hot removal is outside the required-component contract. No production compound topology is inferred from synthetic tests. |
| Native state and neutralization | ADR-0011: all four handles provide one transactional `neutralize()` edit, explicit commit, preserved metadata/protocol/output state, retryable dirty state and terminal rejection. | Native controller units and conversion helpers remain; no normalized core or persistence service. |
| Output and cleanup API | Exact required replies precede optional observations. Bounded loss counts and component-local service order are tested, including recoverable-error sibling service. All handles retain cleanup diagnostics and creation metadata. | Cached host paths are historical observations, not authority to operate on reused nodes. Kernel cleanup failure is not reported as success. Compressed Switch motor words are exposed without physical amplitude/frequency claims. |
| Current-controller acceptance | EXP-0017: 624 isolated HID control/neutral observations and twelve cleanup passes. Existing report-class, framing, cadence, concurrency, motion/output and fault-injection tests retained. Xbox descriptor correction has exact SDL retesting. | DS4 combined evdev fails SDL classification and remains documented. No isolated touch rerun. Physical DualSense absence/damaged controls and absent DS4/Switch references limit fidelity. |
| Consumer disagreement | Pinned SDL exact paths/backends/mappings and neutral/endpoints recorded. Initial Sony Up probe failures preserved; polling neutral before activation fixes the transition apparatus. User Eden gyro observations retained. | Exact Steam/Eden builds, backend, mapping and same-device observations still needed. No Steam bug inferred. Held-input-at-open is outside the settled transition test. |
| Demo integration | Acknowledged “Release all inputs”, consumer build/backend/mapping fields, association/cleanup records and retained last-removal diagnostics. Fake-worker/UI helpers cover stalls, bounded rejection, press/release ordering, arbitrary removal, failure and shutdown. | Live interactive GUI acceptance is not established by these tests. Records are copied only at user request and stay outside Git. Touch is not injected into the active desktop. |
| Boundary, corpus and delivery | Neutral UHID/uinput providers, synchronous personalities and caller-owned execution retained. Corpus generation/publication verification and ordinary offline source-archive builds checked separately. No new runtime dependencies or host permission changes. | Compiled gadget controller profiles/report lengths/startup remain the explicit Gate G exception. Missing metadata/completion authority cannot be fixed with permissions. Audio/Bluetooth/compatibility extension gates remain separate. |

## Validation and review boundary

Required checks: workspace formatting, all-target/all-feature check and strict
Clippy, all-feature tests, Python tooling tests, corpus regeneration and remote
publication verification, Gitleaks and configured commit/push hooks. Source
archives are checked without Git metadata, corpus checkout, credentials or Cargo
network access. Ordinary builds use checked-in artifacts. Authenticated corpus
CI uses existing corpus-only read access; no repository permissions or workflow
policy changed. Each published head's CI result remains separately inspectable.

The scoped current-core changes are suitable for review with the limitations
above explicit. The final code review is complete, with readiness conditional on green final-head
CI; the
known DS4 combined-node classification failure is contained by its documented
restriction and test-only split prototype. It is not a passing realization.
Steam/Eden interactive comparisons, isolated compound/touch evidence and physical
fidelity do not silently become passes. The maintainer decides landing and performs the merge; alpha
versioning and individual new-controller work follow landing.

## Provider purity assessment

UHID owns Linux event/framing and transport identity, while curated personalities
own controller protocol semantics. uinput transports prepared axes, keys,
physical paths and neutral force-feedback events; controller presentation and
feedback policy remain unprivileged. Provider manifests do not depend on curated
controller personalities. The dummy_hcd provider's compiled `report_length`
selection and broker's controller profiles/static feature windows remain visible
legacy exceptions. Retain them until EXP-0004/G establishes a supported generic
control metadata/completion interface and owned-resource live cleanup. Do not
introduce speculative staged IPC or broader standing privileges to hide that gap.

## Final code review

Reviewed the runtime request/delivery/deadline paths, compound failure and output
ownership, curated lifecycle/identity/feedback boundaries, UHID and uinput
transports, broker ownership policy, development-helper trust boundaries, demo
worker/edit integration, and the documented acceptance/build limits.

One defect was reproduced: LED-only HID updates cleared the demo rumble indicator.
The display now retains each motor independently and changes only fields present
in a report. A failing-before/fixed-after regression covers LED-only updates,
one-motor updates, explicit stop and retained indicator timing. No controller
protocol or host-owned output is rewritten to compensate for a display defect.

An additional deterministic Sony regression verifies held Up in the very first
input report and across consumer OPEN/CLOSE/OPEN, followed by terminal library
close and idempotent cleanup. It confirms that the library does not inject neutral
on consumer reopen; actual consumer interpretation remains the separately
recorded SDL/Steam/Eden evidence boundary.

No remaining actionable blocker was identified for the scoped core review.
The known DS4 evdev classification failure and unvalidated compound/gadget,
physical and interactive-consumer capabilities remain explicit limitations, not
passes. No broad refactor, dependency addition or privilege change is justified
by this review. Mark #106 ready only after the published final head passes checks;
merging remains the maintainer's action.
