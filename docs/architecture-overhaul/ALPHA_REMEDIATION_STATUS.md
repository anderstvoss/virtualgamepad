# Alpha remediation status

## Gate and revisions

**NOT READY for alpha under the all-implemented-paths boundary.** Code defects
F1–F8 are addressed; failed audio continuity and incomplete provider/consumer
acceptance remain release gates. PR #133 remains draft. Code review is useful now,
but green builds cannot authorize an alpha release.

- Branch: `codex/gui-long-run-lag`; PR: https://github.com/anderstvoss/virtualgamepad/pull/133.
- Review baseline: `0c08fb485292adb4c361a7210efc04bb223f4642`.
- Remote main/base: `11284f01c58cb80be0d187efa2fca95641513fbf`.
- Both baseline trees: `b733354460af8162c2863b1f38cb4b1157c21680`.
- Implementation checkpoint: `efb110b00812f42a1440817daef66c1c4be290f4`;
  checkpoint tree: `29bb56e5e177856456af5ddb31903f4a62cc7190`.
- Later documentation commits are resolved from the PR head. The handoff explains
  how the reviewer records the exact final head/tree without a self-referential
  document hash. Revalidate every subsequent code candidate separately.

The original review, coverage ledger and acceptance report remain external
historical evidence. This portable disposition replaces neither their failed logs
nor their revision boundaries. Attached plans and earlier issue closures are
context, not acceptance evidence. Unavailable Bluetooth, controller-matching audio
and proprietary XInput require truthful rejection, not implementation here.

## Finding disposition and regression map

| ID / priority | Trigger and previous consequence | Correction and source | Regression / disposition |
| --- | --- | --- | --- |
| F1 / P1 | Full subscription queue loses close message; close/Drop waits forever | `gr-controller-runtime/src/reverse_delivery.rs`: optional sender behind mutex, disconnect then join, self-close avoids join | Full/zero capacity, Drop, concurrent closers, publish race, callback panic and self-close pass. Finite callbacks must return; arbitrary callback preemption is not promised. |
| F2 / P1 | Stalled or oversized discovery command blocks required controller service and shutdown | `demo/src/gui/host_audio.rs`, `gui.rs`, `audio_lab.rs`: GUI-owned coordinator, zero-capacity worker handoff plus one coalesced pending request, generations, cached display, two-second command bound and 256 KiB limit, owned process-group cancellation/reap before reader join | Stalled descendant, oversized output, non-UTF8/nonzero exit, cancellation, stale result, retry/coalescing, worker crash/no respawn, continued service/removal/shutdown pass. Loading/error visible; no per-frame respawn. |
| F3 / P2 | UHID Start/Open/Close/Stop counted internally but never delivered to root callbacks | `gr-hid`, curated protocol implementations and common session: lifecycle hook returns optional observation after required state handling | Fake ordered events through all four root families, no replay and existing bounded-observation/required-reply tests pass. Live neutral UHID Start delivery passes all four families. No events synthesized by application close. |
| F4 / P2 | Five let chains reject Rust 1.85 | `demo/src/gui.rs`, `gui/audio_lab.rs`: equivalent supported conditionals | Forced Rust 1.85 workspace/all-target/all-feature check passes; existing behavior regressions retained. |
| F5 / P1 | Installed minimum compiler is bypassed by repository override | `.github/workflows/ci.yml`: explicit toolchain environment, actual rustc/cargo print and minimum-version assertion | Forced compiler check and negative newer-syntax fixture pass. Remote job/compiler log must belong to the exact candidate. |
| F6 / P2 | Removed packages/phase-gate CLI make scheduled/manual validation invalid | `.github/workflows/provider-tier-b.yml`: current deterministic tests and production/USB worker validators; required Ubuntu audio dependency endpoints | Cargo-metadata command inventory rejects removed packages/examples and missing scripts; Python tests pass. Privileged job remains explicitly disabled, so its skip is missing live evidence. |
| F7 / P2 | Uploaded SBOM contains 16 member reports, omits root | `.github/workflows/sbom.yml`, `scripts/collect-sbom.py`: metadata-driven collection and extracted-artifact verification | All 17 packages locally generated/validated, including root. Missing/duplicate/foreign identity, missing dependency inventory and stale output reject. Uploaded checkpoint artifact verifies all 17 identities; final-head artifact must also be checked. |
| F8 / P2 | Entry documents contradict demo defaults, topology and issue status | README, application/audio/support/demo docs and historical ledger annotations | Root defaults empty; demo ALSA/WIP USB/IP defaults distinguished; #112 closure is not native acceptance; new DS4 topology and lifecycle semantics documented. Historical failures preserved. |
| A1 / P1 | Samples/native audio loses markers even with zero reported queue drops | Marker producer accounting, graph-clock diagnostics, queue/drop/underrun counters, per-marker client accounting and explicit drain; simultaneous duplex for all four ownership combinations | Deterministic duplicate/corrupt/partial-frame and producer/drain tests pass. Protocol fixture now primes exact microphone frames before READY. **Continuity unresolved; assertions unchanged; PR stays draft.** |
| A2 / P2 | Combined DS4 node is classified as touchscreen and misses SDL gamepad discovery | Production `dualshock4/evdev.rs`: transactionally associated gamepad/contact nodes; primary retains controls/feedback, companion retains both contacts and release | Capability split, frame routing, partial creation rollback, reverse feedback, sibling identity/cleanup and root companion metadata pass. Current host lacks registered uinput; production SDL/touch acceptance blocked. |
| A3 / gate | Installed candidate provenance, privileged authorization/recovery and dummy_hcd report semantics unverified | Exhaustive compiled profile/ID reply-or-terminal-rejection test; unknown GET becomes terminal so registry unbinds its owned gadget; initiating + cleanup errors retained for terminal requests and failed construction | Broker admission/peer/FD/framing/recovery and worker process validators pass. Kernel f_hid exposes ID only and no STALL operation: full report-type/length parity **not certified**. Installed/root prerequisites remain external. |
| A4 / gate | Consumer breadth, long-run GUI, accessibility and physical/native timing evidence incomplete | Selection/removal tests through 1,024 positions; explicit neutral-owned-device GUI soak; selected root lifecycle/audio/reconnect tests | Deterministic selection/lockout/routing/error/lifecycle coverage passes. Two-hour live soak and final cleanup report pending. Steam/game/manual keyboard/physical/native-host evidence remains unverified. |

Source paths in this table are relative to the repository (`gr-*` sources are
under `crates/`). Tests run at the lowest practical seams and retain prior tests.
No broad architecture rewrite or acceptance-assertion relaxation was introduced.

## Public contract and architectural implications

Ordinary root signatures still match the 110-item frozen snapshot. Lifecycle
variants now deliver their intended semantics. `service` performs required host
handling before optional bounded observation delivery; application callbacks
cannot become the required protocol engine.

The supporting `gr_hid::Protocol::lifecycle` implementation signature changes to
`Option<Self::Output>`. External protocol implementations must return `None` or an
observation after state handling. Supporting curated `ControllerAssociation`
struct literals must initialize `companions`. These are SPI changes outside the
ordinary-root compatibility promise; all in-tree implementations/fixtures were
updated. Root consumers must still rerun their feature/borrowing/API checks.

DS4 uinput topology now has two owned input components. The primary surface
describes the complete logical controls; the contact companion advertises its
role and distinct creation-scoped association, without inventing observed paths.
Consumers must iterate components rather than assume one node. Both nodes share
logical ownership and are removed on rollback/close; SDL gamepad touch APIs cannot
represent the separate evdev contact presentation, an explicit surface restriction.

Discovery owns only its spawned process groups. Generation changes cancel stale
commands; normal controller workers never discover devices. External subscription
close serializes joiners and releases the sender lock before joining. Self-close
must also avoid waiting on a join lock held by another closer.

The demo adds a direct `rustix` process feature for safe process-group signalling;
version 1.1.5 was already locked transitively. No new ecosystem dependency or
advisory suppression is introduced. The GUI soak is an explicitly invoked example,
with neutral UHID state and no routing/touch injection.

## Validation scope

Local locked fmt/check/Clippy/workspace tests, Rust 1.85 all-target/all-feature
check, strict workspace rustdoc, ordinary API snapshot, root feature consumers and
cached offline rebuild pass. Python tooling tests pass (103 tests). Corpus remote
pin verification passes at `a1789d6ed92b2325016dd78be765342f3ca19aa4`.
Cargo audit/policy checks pass with the previously acknowledged unmaintained
`ttf-parser` warning. Gitleaks passes. Inventory source links require rebuilding
root rustdoc after supporting source pages; semantic signatures remain unchanged.

Production-worker validation passes all three profiles with low and high FD slot
layouts. The USB/IP protocol probe passes all three families, including exact
invalid SET_REPORT stall and PCM patterns, in explicit protocol-fixture mode.
This mode is only for deterministic protocol validation; ordinary timed live
source behavior and silence/loss diagnostics remain unchanged.

Neutral root UHID creation/service/Start delivery/removal/recreation and identity
restoration pass. Selected root audio creation/terminal cleanup, bounded PCM/HID worker
counts, and processing-close/recreate checks pass in isolated graphs. These checks do not substitute for sustained marker continuity.

Live audio matrix and GUI soak are in progress at this documentation checkpoint.
Results and exact final-CI/SBOM revisions will be recorded before completion.
Every sustained direction and duplex/mixed cell uses three trials, a private
PipeWire graph, 62 seconds of markers (two-second warm-up boundary), bounded drain
and unchanged zero-loss assertions. Concurrent GUI/build work makes these VM
stress observations; no native-host latency or physical-fidelity claim follows.

## Remaining release backlog and acceptance prerequisites

1. **P1 audio continuity:** retain failures and localize loss across producer,
   graph, queue, client and drain. Producer-all/queue-zero/client-missing already
   rules out incomplete planned generation or reported application queue overflow
   for some failed cells. Graph discontinuities correlate but do not prove cause.
   Fix only a demonstrated seam with deterministic regression, then repeat all
   family/direction/duplex/ownership cells without competing review workloads on
   a prepared host. Native-host timing remains independently required.
2. **P2 DS4 consumer parity:** prepare an isolated consumer environment with an
   active uinput kernel device. Verify exact gamepad and companion selection,
   controls, two contacts/releases, feedback, partial rollback, sibling removal
   and shutdown. Do not repeat desktop-affecting contact injection here.
3. **Installed provider/security:** administrator-tied candidate binaries, peer
   authorization and reserved VHCI/UDC resources are prerequisites. ConfigFS is
   missing, UDC authorization is unverified; a root socket peer is not candidate
   provenance. Run attached worker-death, malformed-client, broker restart/stale
   lease and recovery acceptance only on test-owned authorized resources. A
   failed unbind must remain visible and must not release a still-owned lease.
4. **dummy_hcd contract:** establish kernel support for type/length and exact
   negative completion or retain explicit experimental rejection/limitations.
   Current unknown-ID handling terminates the gadget; it does not pretend to
   send a STALL. Never promote its report-type parity from a known-feature test.
5. **Consumer/physical breadth:** hands-on keyboard/accessibility/error review,
   Steam/game compatibility, native Linux timing and physical comparisons remain
   unverified. macOS/Windows CI is compile/test evidence, not live provider support.

No persistent host provisioning, installed-service replacement, issue mutation,
merge, tag or publication is performed by this remediation. Historical failed
artifacts remain outside tracked source. The reviewer handoff gives portable
commands and rejection criteria for every finding.
