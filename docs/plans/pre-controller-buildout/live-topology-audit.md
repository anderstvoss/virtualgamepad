# Live internal topology seam — source audit and step 6 test recipe

**Scope:** brief 06a, issue #163; preparation for the local agent's step 6, **not** completion of step 6 or brief 06b.
**Source revision:** `fac473d9ae7263706c01aae23b38b2cf15fdb966` (`main` read 2026-10-15).
**Verification:** source-reviewed; Rust test sketches below **written, not run** (no Rust toolchain in isolated environment); no workspace, Linux host, transport or hardware qualification; no CI result at authoring.
**Provisional verdict:** no demonstrated reason to change `gr-hid` core; retain ADR-0014's transactional seam unless the local agent obtains a reproducible failing test. This is **not** a claim that the new cases pass.

## Sources and present implementation

- [Runtime clone/edit/validate/install](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-hid/src/lib.rs#L241-L275): `update_protocol` rejects closed sessions, edits a clone, validates semantic state, commits on success and marks dirty. No provider I/O occurs in the method; the closure and `Clone` must isolate resources and external effects.
- [Bounded service, request ownership and pending queues](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-hid/src/lib.rs#L339-L451): at most one host event per call; required replies have priority; pending input retains accepted `Command` bytes; new input is generated only after pending input empties; generation uses a cloned protocol and is committed only if bounded and well formed. `QueueFull`/`InvalidState` are recoverable; other service errors close the session.
- [Reply deadline and close](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-hid/src/lib.rs#L461-L501): a pending reply keeps its command/deadline; `DefinitelyUnsent` retries the same command, `Uncertain` terminates, `close` is idempotent and clears queued work.
- [Synthetic personality/fixtures and six tests](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-hid/tests/internal_topology.rs#L1-L509): `Remote` (`Port::Empty`, `Nunchuk`, `MotionPlus`), four-edge queue, synthetic feature reports, `enqueue`/`payloads`/`runtime`. The [Fake transport](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-hid/tests/support/mod.rs#L1-L38) already records attempts, submitted commands, programmable `Delivery` outcomes and close counts.
- [ControllerSession HID integration](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-curated-controllers/src/common/session.rs#L164-L202) permits semantic state edits using `Runtime::update`, not protocol edits. [Read-only protocol access](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-curated-controllers/src/common/session.rs#L400-L405) is present. Source search finds no production `update_protocol` caller, only its definition and the synthetic tests. A Wii-specific typed wrapper is future integration, not currently exercised production behavior.
- Decision basis: [ADR-0014](../../adr/ADR-0014-protocol-edits.md), [ADR-0011](../../adr/ADR-0011-neutralization-and-observation.md), [EXP-0020](../../history/alpha-1/experiments/EXP-0020-internal-topology.md), [brief 06](06-live-topology.md).

## #163 gap matrix

| #163 case | Existing evidence at the cited revision | Step 6 addition or remaining limit |
| --- | --- | --- |
| Transactional typed attach/detach/reinitialize | T1 `one_live_session…` (229), T2 `invalid_edits…` (303); clone-and-validate (`lib.rs` 262–275) | Use a controller-owned typed action wrapper in Wii later; no generic core attachment enum. |
| Accepted input and reply bytes kept across edits | T4 `edits_preserve…` (360), ordered `pending`/`reply` | S1 holds a reply through multiple successful edits and service calls, then checks its **original** deadline; S6 checks exactly-once replay of `DefinitelyUnsent` reply bytes. |
| Rapid plug/unplug | T3 `rapid_edges…` (339), T1 repeated transitions | Existing synthetic check sufficient for basic sequence; continue with queued-send pressure in S2. |
| Queued edge saturation | T3 rejects fifth edge with `QueueFull` and rolls back | S2 saturates **while a previously accepted input remains DefinitelyUnsent**; validates FIFO, rollback and cadence. Lossy observation count is separate; see fixture extension below. |
| Invalid transitions and semantic state | T2 validates current state and rolls back entire `Remote`/deadline | Optional seeded mixed valid/invalid sequences; not essential to repeat T2 in the first pass. |
| Malicious register writes | T5 `every_report_class…` (393), T6 host activation | S3 tests oversized framing, wrong report ID and feature writes while Empty/initializing; check precise error replies and no invalid mutation. |
| One host personality, changing extension | T1 and T6 verify Nunchuk / bridge / pass-through while session 42 remains | No additional core API; future typed Wii session must preserve one host identity. |
| Host probe overlapping an edit | T4 and T6 check valid memory replies | S4 distinguishes an edit **before** request dispatch from an edit **after** reply acceptance but before transport submission. |
| Terminal close mid-change | T1 verifies repeated close and after-close protocol edit | S5 begins with unsent accepted input and queued edges; asserts terminal service/edit behavior and no post-close submit. Cleanup/resource disposition belongs to #162. |
| Nonrepeatable send ambiguity | T5 demonstrates `Uncertain` on input; source has same logic for reply | S7 proves `Uncertain` on a **reply** closes and never retries; S6 proves `DefinitelyUnsent` retries the same reply command. |
| Request completion and cadence bounded | `service_inner` submits replies ahead of input; `Limits` reserve >=2 submissions and reply timeout | S1 expiration and S2 input backlog; final local run should also check actual attempt counts per service. |
| Output observations/feedback | 32-item curated feedback queue belongs to ADR-0011; `gr-hid` observation overflow is separately bounded | S8 fixture extension: synthetic output events exceed observation limit; track drop count, verify no loss of queued protocol commands. Do not equate observation loss with dropping required replies. |
| Delivery feedback after a topology edit | `delivered(command, outcome)` is invoked on **current** `self.protocol` even for older queued commands (`lib.rs` service/flush paths) | S9 fixture extension: trace command bytes and current personality at callback; confirm behavior. A controller must derive submission bookkeeping from `command`, not assume the current personality generated it. No production counterexample is known. |

T1–T6 line anchors are in `crates/gr-hid/tests/internal_topology.rs` at the pinned revision above. S1–S7 below are **unexecuted sketches**, intended to be placed under those six tests; they reuse existing `Remote`, `Fake`, `enqueue`, `runtime` and `payloads`. All IDs and bytes remain synthetic.

## Step 6: Rust test sketches (code written, not run)

```rust
// Paste into crates/gr-hid/tests/internal_topology.rs after the existing tests.
// The module's existing imports, Remote/Port/node, runtime, Fake and enqueue apply.

#[test]
fn held_reply_keeps_bytes_and_original_deadline_across_repeated_edits() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node()))).unwrap();
    enqueue(&io, 1, RequestKind::Get {
        kind: ReportType::Feature, id: Some(3),
    });
    io.0.borrow_mut().outcomes.push_back(Delivery::DefinitelyUnsent);
    r.service(1).unwrap(); // Reply was accepted, but is held.
    let original = io.0.borrow().attempts.iter().find(
        |c| matches!(c, Command::Reply { .. })
    ).unwrap().clone();

    for (at, port) in [(2, Port::Empty), (3, Port::Nunchuk(node()))] {
        r.update_protocol(|p| p.attach(port)).unwrap();
        io.0.borrow_mut().outcomes.push_back(Delivery::DefinitelyUnsent);
        r.service(at).unwrap();
    }
    let retries: Vec<_> = io.0.borrow().attempts.iter().filter(
        |c| matches!(c, Command::Reply { .. })
    ).cloned().collect();
    assert_eq!(retries, vec![original.clone(), original.clone(), original]);
    // The request arrived at t=1; edits must not reset its t=100001 expiry.
    assert_eq!(r.service(100_001), Err(Error::Deadline));
    assert!(r.is_closed());
    assert_eq!(io.0.borrow().closes, 1);
}

#[test]
fn accepted_unsent_input_does_not_erase_bounded_topology_edges() {
    let io = Fake::default();
    let mut r = runtime(&io);
    io.0.borrow_mut().outcomes.push_back(Delivery::DefinitelyUnsent);
    r.service(0).unwrap();
    let original = io.0.borrow().attempts[0].clone();
    for port in [
        Port::Nunchuk(node()), Port::Empty,
        Port::Nunchuk(node()), Port::Empty,
    ] {
        r.update_protocol(|p| p.attach(port)).unwrap();
    }
    let before = r.protocol().clone();
    assert_eq!(
        r.update_protocol(|p| p.attach(Port::Nunchuk(node()))),
        Err(Error::QueueFull)
    );
    assert_eq!(*r.protocol(), before);
    io.0.borrow_mut().outcomes.push_back(Delivery::DefinitelyUnsent);
    r.service(1).unwrap();
    assert_eq!(io.0.borrow().attempts[1], original);
    r.service(2).unwrap();
    assert_eq!(io.0.borrow().submitted[0], original);
    assert_eq!(payloads(&io, 1), vec![vec![1], vec![0], vec![1], vec![0]]);
    assert_eq!(payloads(&io, 2).last(), Some(&vec![0, 0]));
}

#[test]
fn malformed_feature_writes_are_acknowledged_without_topology_mutation() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    assert_eq!(
        Report::new(ReportType::Feature, Some(4), vec![0; 4096]),
        Err(Error::Framing)
    );
    for (ordinal, id, data, expected) in [
        (1, 4, vec![0], ReplyError::Invalid),     // Empty: cannot activate.
        (2, 5, vec![0], ReplyError::Unsupported), // Wrong ID.
        (3, 4, vec![0; 100], ReplyError::Invalid),
    ] {
        enqueue(&io, ordinal, RequestKind::Set(
            Report::new(ReportType::Feature, Some(id), data).unwrap()
        ));
        r.service(ordinal).unwrap();
        assert_eq!(r.protocol().identity(), 0);
        assert!(io.0.borrow().submitted.contains(&Command::Reply {
            token: RequestId { session: 42, ordinal },
            reply: Reply::Set(Err(expected)),
        }));
    }
    r.update_protocol(|p| p.attach(Port::MotionPlus {
        active: false, downstream: None, passthrough: false,
    })).unwrap();
    assert!(r.protocol().initializing);
    enqueue(&io, 4, RequestKind::Set(
        Report::new(ReportType::Feature, Some(4), vec![1]).unwrap()
    ));
    r.service(4).unwrap();
    assert_eq!(r.protocol().identity(), 2);
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId { session: 42, ordinal: 4 },
        reply: Reply::Set(Err(ReplyError::Invalid)),
    }));
}

#[test]
fn host_probe_uses_dispatch_topology_and_held_reply_is_not_recomputed() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    enqueue(&io, 1, RequestKind::Get {
        kind: ReportType::Feature, id: Some(3),
    });
    r.update_protocol(|p| p.attach(Port::Nunchuk(node()))).unwrap();
    r.service(1).unwrap(); // Request dispatched AFTER the edit.
    let wanted = Reply::Get(Ok(Report::new(
        ReportType::Feature, Some(3), vec![1, 0x11, 0x22]
    ).unwrap()));
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId { session: 42, ordinal: 1 },
        reply: wanted.clone(),
    }));
    enqueue(&io, 2, RequestKind::Get {
        kind: ReportType::Feature, id: Some(3),
    });
    io.0.borrow_mut().outcomes.push_back(Delivery::DefinitelyUnsent);
    r.service(2).unwrap(); // Reply accepted, but not submitted.
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    r.service(3).unwrap(); // Retry original reply, not new Unsupported.
    assert!(io.0.borrow().submitted.contains(&Command::Reply {
        token: RequestId { session: 42, ordinal: 2 },
        reply: wanted,
    }));
    assert_eq!(r.protocol().identity(), 0);
}

#[test]
fn close_with_queued_edges_and_unsent_input_is_terminal() {
    let io = Fake::default();
    let mut r = runtime(&io);
    io.0.borrow_mut().outcomes.push_back(Delivery::DefinitelyUnsent);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node()))).unwrap();
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    let attempts = io.0.borrow().attempts.len();
    let submitted = io.0.borrow().submitted.len();
    r.close().unwrap();
    r.close().unwrap();
    assert_eq!(r.service(1), Err(Error::Closed));
    assert_eq!(r.update(|_| panic!("edit after close")), Err(Error::Closed));
    assert_eq!(
        r.update_protocol(|_| panic!("protocol edit after close")),
        Err(Error::Closed)
    );
    assert!(!r.wants_write());
    assert_eq!(r.deadline(), None);
    assert_eq!(io.0.borrow().attempts.len(), attempts);
    assert_eq!(io.0.borrow().submitted.len(), submitted);
    assert_eq!(io.0.borrow().closes, 1); // No assertion on external resources.
}

#[test]
fn definitely_unsent_reply_retries_identical_bytes_once() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node()))).unwrap();
    enqueue(&io, 1, RequestKind::Get {
        kind: ReportType::Feature, id: Some(3),
    });
    io.0.borrow_mut().outcomes.push_back(Delivery::DefinitelyUnsent);
    r.service(1).unwrap();
    r.update_protocol(|p| p.attach(Port::Empty)).unwrap();
    r.service(2).unwrap(); // Retry succeeds by default.
    r.service(3).unwrap();
    let attempts: Vec<_> = io.0.borrow().attempts.iter().filter(
        |c| matches!(c, Command::Reply { .. })
    ).cloned().collect();
    let submitted: Vec<_> = io.0.borrow().submitted.iter().filter(
        |c| matches!(c, Command::Reply { .. })
    ).cloned().collect();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0], attempts[1]);
    assert_eq!(submitted, vec![attempts[0].clone()]);
}

#[test]
fn uncertain_reply_while_topology_changes_terminates_without_replay() {
    let io = Fake::default();
    let mut r = runtime(&io);
    r.service(0).unwrap();
    r.update_protocol(|p| p.attach(Port::Nunchuk(node()))).unwrap();
    enqueue(&io, 1, RequestKind::Get {
        kind: ReportType::Feature, id: Some(3),
    });
    io.0.borrow_mut().outcomes.push_back(Delivery::Uncertain);
    assert_eq!(r.service(1), Err(Error::UncertainDelivery));
    assert!(r.is_closed());
    assert_eq!(
        r.update_protocol(|_| panic!("edit on terminal session")),
        Err(Error::Closed)
    );
    assert_eq!(r.service(2), Err(Error::Closed));
    r.close().unwrap();
    assert_eq!(io.0.borrow().attempts.iter().filter(
        |c| matches!(c, Command::Reply { .. })
    ).count(), 1);
    assert_eq!(io.0.borrow().closes, 1);
}
```

### S8 and S9: small test-fixture extensions (local agent)

S8 — **observation overflow is not protocol-edge overflow.** Note that a successful `service()` immediately drains observations and at most one host event is read per call: three ordinary successful services **cannot** overflow this queue. To exercise the recoverable-error retention path, have the test-only `Remote::output` return `Some(())` for one reserved synthetic output ID. Create a runtime with `Limits { input_queue: 2, ..Limits::default() }`, and accept two queued topology edges before the first service (two edges plus cadence produce three inputs, exceeding the generation batch limit). Feed three such output events; each `service()` should return recoverable `Error::QueueFull` *after* observing its output and *before* draining observations. Check `dropped_observations() == 1`, `take_observations().len() == 2`, and the `Remote` edge queue is unchanged. This covers observation eviction under a recoverable generation failure, **not** successful-service overflow. S2 separately verifies accepted input FIFO under `DefinitelyUnsent`. Do not silently clear queued edges to manufacture a pass or change behavior of real protocols.

S9 — **old accepted command delivered to new personality.** Add a clone-owned `delivery_trace: Vec<(Command, Delivery, u8)>` field to the *test-only* `Remote` (default empty); append `(command.clone(), outcome, self.identity())` inside its `delivered` method. Accept an input command with `DefinitelyUnsent`, edit topology, resubmit it, then assert the command is byte-identical while the callback records the **post-edit** identity. This captures the present runtime contract; it does not establish that an actual controller sequence counter is correct. If a future Wii codec needs generation-local delivery acknowledgments, first write a failing fixture showing why command-derived bookkeeping is insufficient; that is a new #176 discussion, not a speculative `gr-hid` patch. No change to shared `tests/support/mod.rs` is needed for S1–S7; S8/S9 are additive within the topology test fixture.

## Contract questions

**(a) Stop versus “never discard accepted input.”** `HostEvent::Lifecycle(Lifecycle::Stop)` explicitly clears `pending` and marks `dirty` in [`service_inner`](https://github.com/anderstvoss/virtualgamepad/blob/fac473d9ae7263706c01aae23b38b2cf15fdb966/crates/gr-hid/src/lib.rs#L373-L399); it does **not** clear `reply`. The ADR-0014 promise is best read as *no silent purge by `update_protocol`*; a host Stop is a distinct explicit lifecycle cancellation and allows regeneration under the new state. ADR-0011 discusses neutralization and required reply preservation, not this specific Stop exception. For precision, add a low-seam Stop test before changing documentation: leave an accepted input `DefinitelyUnsent`, queue Stop, assert it is not replayed, observe any newly generated cadence, and assert accepted required replies remain protected. Do **not** claim literal preservation across Stop, and do not change core lifecycle semantics without a reproducible counterexample and owner review.

**(b) Delivery feedback after an edit.** Both input and reply submission call `self.protocol.delivered(command, outcome)`, where `self.protocol` is the **current** personality even when `command` was generated before an edit. `Remote::delivered` currently does nothing, so the six accepted tests cannot validate generation-local counters. `Command` is preserved; any controller-owned counter logic should use that immutable command or clone-safe explicitly retained bookkeeping. S9 exposes the distinction; no production deficiency or API addition is demonstrated.

## Production integration recipe (future Wii #158; not implemented here)

1. The Wii package owns typed attachment state and legal combinations (`Empty`, Nunchuk, Classic/Pro, MotionPlus with legal bridge/pass-through). This information is *not* a generic runtime extension trait or a separate host component for controller-internal accessories.
2. At a controller action boundary, a Wii-specific typed method obtains the underlying HID session and calls `Runtime::update_protocol(|candidate| { /* modify typed Wii attachment; validate entire combination */ Ok(()) })`. The callback must have no external side effects; protocol clones must not share mutable state.
3. Protocol-owned queues encode bounded attach/detach/status edges; return `QueueFull` without changing state when full. Keep required host-request replies complete within service budgets. Drain accepted input/reply bytes in FIFO/order; do not regenerate them after a later attachment edit.
4. Maintain the existing request/deadline/`DefinitelyUnsent`/`Uncertain` contracts. On host Stop and terminal close, follow lifecycle rules separately from protocol-edit preservation.
5. Expose a controller-specific editing method in `ControllerSession` only once a production Wii consumer is ready. Tests must cover the production wrapper, not just the synthetic `Remote`.
6. Brief **06b** and ADR-0019 separately define `max_topology`, retained `topology_generation()` and `current_topology()`, unavailable absent controls and any uinput presentation policy. These are explicitly not part of 06a.

## Local-agent acceptance, limitations and handoff

- Run `cargo test -p gr-hid --test internal_topology` on the real workspace; integrate S1–S7, extend test fixture for S8/S9, and run the complete existing suite plus the required AGENTS.md checks. If a new sketch fails, first distinguish a mistaken test assumption from a runtime counterexample. Never weaken T1–T6 to obtain a pass.
- Report tests individually, including any genuine failure and exact SHA. A failing test requiring public API snapshot growth (110 items), broker wire change or changed ADR-0014/ADR-0011 semantics requires a **stop-and-ask on #176** before a fix. All other deficiencies still need the smallest targeted regression-backed patch.
- Verification of this document: **source-reviewed only; sketches written, not run**; no claim of deterministic execution, workspace CI, consumer/physical fidelity, Bluetooth radio or host proof. The future local step 6 result must supersede this provisional verdict.
- Repository changes in this lane: this document only. No changes to `crates/`, `src/`, `scripts/` or `.github/`; no code, dependency, public interface or broker behavior modified.
