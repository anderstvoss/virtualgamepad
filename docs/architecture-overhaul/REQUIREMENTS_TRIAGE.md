# Architecture requirements triage

## Branch and scope inspected

Inspected `architecture/protocol-session-rewrite` at `2af0a6c`, following the
architecture requirements handoff supplied by the maintainer. This is an initial
source-backed triage, not a completed audit of every proposed issue. Private
handoff material remains outside Git. The requirement references below retain
its identifiers, with their meaning restated here for reviewers.

## Findings and decisions

| Issue | Classification and evidence | Smallest durable next change |
| --- | --- | --- |
| A: service contract | API hazard. All four curated handles expose `poll_output`, readiness, write interest and relative deadlines. `common/session.rs` services required HID work through output draining. The name suggests an optional observer operation. | Implemented an explicit `service` entry point with the `poll_output` alias retained. Specify idle service, bounded callbacks, readiness plus timers, and immediate deadlines. Keep executor ownership with callers (AR-28). |
| B: edit contention | API hazard. The original shared controller mutex let drawing delay service. The demo now uses worker-exclusive ownership, detached native views and one acknowledged edit batch at a time. | Implemented and deterministically tested single service-owner integration using bounded native edit commands and detached UI snapshots; live GUI acceptance remains pending. Do not move demo workers into the library or collapse press/release transitions into an unchecked latest-state mailbox. |
| C/D: persistent identity | Missing API capability. `common/session.rs` generates per-creation identity before provider open; Sony personalities retain it, but callers initially could not restore a typed emulated identity. ADR-0008 now adds opt-in Sony USB/UHID restoration with fresh session state; other targets and compound identities remain pending. | Design controller-owned identity types and explicit opt-in restoration. Keep current ephemeral behavior until the default is decided. Fresh transport/request state must accompany every recreation; storage stays caller-owned (AR-8–11, AR-21). |
| E/F: compound ownership and construction | Partly preserved. `gr-controller-runtime/src/compound.rs` owns ordered components, preflights opens and closes partial opens in reverse order. | Audit failure at every open position and cleanup failure before adding any wrapper. Do not mechanically wrap all simple controllers. Component roles must remain controller-owned (AR-12–16). |
| G: required-component failure | Native helper policy implemented in ADR-0009: non-WouldBlock provider errors close every selected required component before returning component context. Controller-owned lifecycle/deadline interpretation and optional hot-removal remain pending. | Define a logical-owner policy distinguishing retryable transport errors from terminal required-component failure. Test convergence of all components after terminal read/write/removal/deadline failures. Do not close on every recoverable error (AR-17–18). |
| H/P: association and diagnostics | Missing validation/API review. Component IDs route requests, but do not alone establish host-visible association. The new one-node mapping harness intentionally covers only one-component profiles. | Define controller-owned stable roles plus realization metadata sufficient for exact correlation. Avoid display-name-only association and assumptions that one controller always means one node (AR-20). |
| I: DS4 split evdev | Blocked by evidence. Combined production node fails SDL classification; split prototype remains test-only. No isolated environment is available. | Retain production gate. Require both touch contacts/release, exact association, output playback, failure/rollback and all-node cleanup before adoption. Do not repeat on the active desktop. |
| K/L: HID/provider scope | Preserved core direction with a legacy exception. `gr-hid` contains report/session mechanics. `gr-provider-linux-dummy-hcd/src/lib.rs::report_length` still branches on compiled controller kinds. | Keep HID semantics controller-owned. Track compiled gadget leakage explicitly until Gate G establishes generic request/completion transport; additional permissions cannot supply missing metadata (AR-3, AR-22). |
| M/N/O: state and mapping | Native state direction preserved; neutralization and adapter ergonomics require further API review. Demo conversions are application-level. | Keep authoritative state controller-native. Assess native transactional neutralization before adding a separate adapter crate; no new dependency or normalized core state is justified yet. |
| Q: output timing | Further audit needed. Bounded observations and drop counts exist; cross-component temporal guarantees have not been established. | Document observed ordering first; add timestamps only for a concrete consumer requirement. Required replies must not depend on observers. |
| R/S/T: corpus and claims | Build boundary preserved by checked-in artifacts and separate conformance tooling. EXP-0014 adds exact Xbox/Switch evdev evidence only. | Verify authenticated CI using corpus-only read access and keep versioned acceptance evidence granular. No support-level promotion from deterministic or unrelated consumer results (AR-24–26). |
| U/V: gadget and privilege | Separately blocked. Existing interface lacks the required control metadata/completion authority; reserved resources also remain unavailable. | Retain compiled path. Keep host preparation explicit and realization-scoped; no kernel replacement or blanket access on this VM. |
| W/X: maintainability and scope | Follow-up review, not grounds for another rewrite. | Extract modules when responsibilities stabilize. Routing, discovery, persistence storage and worker policy stay above the library. |

## Proposed implementation batches

1. **Service contract and integration proof:** document and test a single explicit
   service operation across all families; exercise idle replies, immediate timers,
   backpressure and terminal close. Then remove demo edit-lock contention using a
   bounded queue with explicit rejection, ordering and shutdown semantics. Assess
   whether a public split handle is necessary only after this proof.
2. **Typed identity design:** separate application identity, emulated identity and
   transport identity. Test supplied-identity validation, stable protocol replies
   across recreation, fresh requests/retries and independent concurrent sessions.
   Actual SDL/Steam association remains a separate measured claim. Record an ADR
   extending per-creation identity decisions without weakening uniqueness.
3. **Logical compound failure and association:** preserve existing routing and
   rollback; add required-component terminal policy and identity-derived roles.
   Deterministic multi-personality tests may use overlapping request IDs. No OS
   atomic visibility claim follows from transactional logical state.
4. **DS4 production adoption:** blocked until isolated consumer evidence exists.
   This blocks that realization's acceptance, not batches 1–3.
5. **Boundary and build review:** audit provider dependencies, source-archive builds,
   support documentation and corpus CI. Keep Gate G independently blocked.
6. **After this overhaul lands:** Steam Controller development is deferred by maintainer direction. **Future complexity exercise:** use a hypothetical multi-component controller
   only as a paper stress test. Actual Steam Controller topology and protocol must
   come from independent evidence before production work. Audio/Bluetooth and
   other extensions keep their existing gates.

## No-change decisions and ADR work

Preserve controller-native state, synchronous protocol personalities, exact
realization selection, executor-neutral runtimes, exact retries and compiled
artifacts. Do not add mandatory threads, a runtime profile language, routing or
persistence storage. The existing component-scoped reply routing is useful and
must survive any composition changes.

Service ownership, identity restoration and required-component failure each need
an explicit decision record when their experiments settle the contract. None is
claimed settled by this triage. PR #106 remains draft; the acceptance matrix and
gate ledger remain the source of measured support claims.
