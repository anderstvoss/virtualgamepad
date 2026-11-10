# ADR-0024: Controller-owned typed, bounded, loss-aware host-output stream

Status: Proposed. Design only for [#344](https://github.com/anderstvoss/virtualgamepad/issues/344); no implementation, API removal, acceptance promotion or worker-wire approval is granted by this record.

## Context

[ADR-0004](ADR-0004-synchronous-hid-session.md) requires prompt controller-owned handling of HID and force-feedback requests, including bounded required replies independently of optional observers. [ADR-0006](ADR-0006-conventional-feedback.md) distinguishes force-feedback upload, play/stop and erase. [ADR-0020](ADR-0020-retained-outputs-and-kinds.md) retains the latest accepted native values and revisions; identical values do not advance those revisions. The existing bounded `service(callback)` observations can evict entries (`dropped_output_events()` counts those evictions). Neither retained values nor callbacks constitute an ordered history of accepted commands.

[ADR-0023](ADR-0023-controller-subsystems.md) requires typed, family-owned, bounded in-band subsystems and rejects treating PCM/speaker payloads as a generic state field or UAC endpoint. Steam Controller 2026 currently has an opt-in compatibility PCM FIFO and diagnostic/raw payload paths. Anders approved **option 3** in [#322 comment 6774052277](https://github.com/anderstvoss/virtualgamepad/issues/322#issuecomment-6774052277), recorded by [#347](https://github.com/anderstvoss/virtualgamepad/pull/347): retain those paths until a separately authorized typed stream API ships, and remove the compatibility payload paths in that **same implementation change**. [Brief 16](../plans/pre-controller-buildout/16-audio-haptics-buildout.md) places this contract in audio step S0, ahead of S1 Wii speaker and S2 Steam typed PCM.

This contract serves applications mirroring accepted host output commands and future family-native PCM/speaker consumers. Acceptance by a virtual controller does **not** establish physical execution, delivery to an external device or a source-controller fidelity claim.

## Decision

### 1. Acceptance, identity and the four separate observations

An **accepted host command** is a *distinct host-originated operation* whose complete family-native request has passed validation and whose controller-owned protocol transaction has reached its specified acceptance point. Only a new transaction obtains a command identity. For HID output/SET_FEATURE this is the controller's successful semantic commit after validation; GET_FEATURE is a read, not a command. If an HID reply remains pending, the record describes local acceptance, **not** successful host receipt of that reply. For uinput force-feedback upload/erase, the current [ADR-0006](ADR-0006-conventional-feedback.md) publication boundary applies: acceptance becomes observable only after the successful kernel reply completion. An FF play/stop event is accepted when the controller validates and resolves that host command, not when an application actuates anything. A failed or unsupported request is not an accepted command.

A second, independently received valid command with identical bytes or values receives a **new** sequence and identity; a transport retry or replay of the *same already-accepted request ordinal* does not. Valid native no-op commands can therefore be recorded even if they leave a retained value unchanged. Never infer command identity from a payload hash, effect ID, retained revision or host-supplied request ID alone.

Keep the following meanings separate:

| Observation | Meaning | Identity / cardinality |
| --- | --- | --- |
| Accepted command | A validated new protocol operation at the family-defined commit boundary | One distinct `(epoch, sequence)` per accepted record, including identical repeats |
| Retained-value change | Latest accepted native *level* changes | [ADR-0020](ADR-0020-retained-outputs-and-kinds.md) revision advances only on change; not a command count |
| Delivery/lifecycle observation | Reply attempted/completed, feedback scheduled, host START/STOP, failure or close | Explicitly classified; not physical execution and not automatically another accepted command |
| Physical execution | An external actuator, speaker or device actually performed work | Outside VirtualGamepad and never asserted by this API |

Rejections and family-native statuses may be reported as separately tagged **outcome/diagnostic** records when their meaning is known, without pretending they are accepted commands. They must not mutate retained outputs or be included in an accepted-command count. A stream record's ordering position orders both accepted commands and any expressly published diagnostics; the discriminant, not mere presence in the stream, determines acceptance.

### 2. Typed record and position contract

Each created controller owns one logical host-output journal, independent of the callback queue and retained latest-value store. The proposed shared **envelope**, not a generic mutable output payload, is:

```rust
// Illustrative API shape only; names, widths and signatures are not frozen.
HostOutputPosition { epoch: StreamEpoch, sequence: u64 }
HostOutputRecord<FamilyCommand> {
    position: HostOutputPosition,
    kind: Accepted(FamilyCommand)
        | Rejected(FamilyRejection)
        | Lifecycle(HostOutputLifecycle)
        | FeedbackScheduled(FamilyFeedback),
    accepted_monotonic_time: Option<MonotonicSample>,
    retained_revision_after: Option<u64>,
}
StreamRead<FamilyCommand> {
    gap: Option<SequenceGap>,             // [first_missing, first_available)
    records: BoundedBatch<HostOutputRecord<FamilyCommand>>,
    next_cursor: HostOutputCursor,
    oldest_available: u64,
    next_sequence: u64,
    terminal: Option<StreamTerminal>,
}
```

Actual Rust representations must use typed enums/structs and bounded, validated buffers, not untyped report maps. Each family owns decoded units, format, side/target, operation and any required **original native payload** (including reviewed padding where it matters); its type must not claim support for unreviewed variants. Record size and FIFO capacity have finite per-family maximums; capacity must accommodate at least one *largest valid* family record. No unbounded PCM history or per-poll growth.

Sequence starts at **1** in each epoch; every published record obtains the next monotonically increasing position and never reuses it. An accepted record's position is its command identity. The high-water/oldest-available metadata expose the complete range, including before the first reader attaches. Exhaustion of the sequence space is terminal, never wrapping. An opaque epoch token is unique to a controller presentation incarnation; cursors from another incarnation must be rejected, not silently reset. A new created controller always starts a new epoch. The token's implementation must avoid accidental equality across live/recreated instances without claiming globally durable identifiers.

A monotonic timestamp is `None` unless the owning controller has an explicitly defined sampled monotonic clock and origin. If present, document the clock domain, sampling point and whether it is synthetic; it is **not** a wall clock, audio sample clock, hardware execution time or cross-controller ordering guarantee. Sequence is authoritative for order. `retained_revision_after` is set only where a corresponding retained surface exists; it may equal the preceding revision for a repeated command.

### 3. Consumption: bounded cursor replay with explicit gaps

**Choose cursor-based, bounded replay**, not a required consumer acknowledgment or a backpressure handshake. The controller maintains a finite overwrite-oldest ring (bounded in **record count and payload bytes**), pre-sized to accept a largest valid command. Appending a validated command never waits for a reader. A reader supplies a typed `HostOutputCursor { epoch, next_sequence }` and a bounded maximum batch size; polling returns copies/borrowed records in order, with a new cursor. No persistent reader registration, delivery acknowledgment, callback or worker thread is required. A cursor can be saved in application memory and resumed while that exact controller epoch/ring survives; it is not a durable bookmark and must not survive recreation.

If the requested next position is below `oldest_available`, return an **explicit `SequenceGap { first_missing, first_available }` before any later records**. Advance the returned cursor past the missing interval but never label the replay as continuous. Missing records are irrecoverable; retained latest values may reconcile *levels* but not recover effects, edges or PCM. A cursor for a future sequence is invalid. An independent reader starting after acceptance began can see `oldest_available > 1` and must not claim full-epoch capture. No caller, however slow, gets unbounded lossless delivery. After a gap, a forwarding application must mark its lossless claim invalid and choose whether to stop, resynchronize or continue degraded; the controller itself remains serviceable.

This choice gives short stalls bounded replay without a second acknowledgment state machine and provides an exact loss boundary when finite history is exceeded. An ack-based blocking queue would either stall required replies or still lose data under sustained load. A destructive pop-only FIFO cannot support a detached reader resuming from a known position or independently diagnosing a missed interval. Bounded replay is **not** a guarantee against loss; explicit gaps are mandatory at the replay boundary.

Counters for journal overwrite/missing positions are separate from `dropped_output_events()`. A full ring evicts journal data, never required protocol work or retained latest values. Stream recording and draining must have bounded memory, time and work per service cycle; if an exceptional internal capacity failure prevents storage, the corresponding position must still be accounted for as missing, never silently disappear.

### 4. Lifecycle and terminal visibility

- **Bursts/lag:** accept all otherwise-valid host commands under their existing family protocol policy; overwrite journal history on journal pressure. Report the precise missing position interval at the next read. Journal pressure does not change whether the host operation is accepted.
- **Reader detach, panic or slow reader:** release any borrowed view without changing protocol servicing. Unread history remains only until normal ring eviction. No callback unwind may strand a required reply. Readers must not hold a lock/borrow across host service that can starve required work.
- **Host STOP then START in the same surviving presentation:** preserve epoch and sequence; explicitly record observed lifecycle transitions if the provider supplies them. Do not replay old commands on START or treat retained values as new commands. Commands actually accepted while stopped (if any under that personality's rules) still sequence normally; do not invent acceptance while suspended.
- **Protocol reset, uncertain delivery, worker death or reconnection with unprovable continuity:** close the old epoch with a typed reason and terminal/high-water metadata. Reinitialization begins a **new epoch**; never silently stitch disjoint sessions. A reconnect that retains an uninterrupted controller protocol session may keep the epoch only if continuity is proved by that realization.
- **Controller recreation:** always a new epoch, even when the same identity, association or effect ID is reused. A previous cursor is an epoch-mismatch error.
- **`close()` / failure close:** terminal, idempotent; no new accepted commands. Preserve the final available ring, gap bounds and terminal reason for draining through the still-owned closed handle, even if a terminal observation itself would otherwise be evicted. Close does not guarantee a caller subsequently polls, and destroying the handle loses that volatile history. Terminal loss or an incomplete tail must be distinguishable from normal end-of-stream; do not invent an orderly-close marker after unobserved worker death. No persistence or Drop-time export is implied.

No application service routine, stream reader or optional `service()` callback may block, delay past existing bounded service deadlines, or condition a required HID/FF reply. Preserve [ADR-0004](ADR-0004-synchronous-hid-session.md) and [ADR-0006](ADR-0006-conventional-feedback.md) ordering, exact retry bytes and terminal-uncertain semantics. `service(callback)` continues emitting *optional observations*, `dropped_output_events()` continues counting *those* observation losses, and `host_outputs()` continues reporting *retained accepted latest values/revisions* as defined by [ADR-0020](ADR-0020-retained-outputs-and-kinds.md). The stream is a third, independent read surface, **not** a reinterpretation of either.

### 5. Family mapping and realization restrictions

| Family / traffic | Accepted stream record | Rejected/status/lifecycle distinction | Realization bound |
| --- | --- | --- | --- |
| Conventional uinput FF | Typed `Upload { effect_id, validated_effect }` only after positive reply completion; `Play/Stop { effect_id, resolved parameters }` on accepted host operation; `Erase { effect_id }` only after positive erase reply | Negative upload/erase replies and invalid play references are rejected outcomes, never successful uploads; reply completion is not playback. Effect ID may be reused but position is distinct. Stop-before-erase remains ordered | `linux.uinput` conventional FF only; no HID-native PCM or speaker write path |
| HID native output / SET_FEATURE | Family-native validated report operation, decoded typed fields and bounded original payload when required to preserve device semantics; distinct valid repeats remain distinct | Invalid/unsupported request, GET_FEATURE read, required reply retry and provider delivery are not accepted output commands | Only actual declared report paths for the selected UHID/other realization; no universal raw write interface |
| Steam 2026 `0x86` | Typed configure/disable with operation, group, mode and native bytes if needed | Transport-level success does not imply internal group was configured: compat status `0x10` pad-group refusal is a *rejected semantic outcome*, while internal-group configuration/stop is accepted under current rules | Wired `linux.uhid.usb` with explicit `haptic_streaming`; uinput cannot deliver PCM |
| Steam 2026 `0x87` / `0x88` | Each valid accepted mono/stereo write is one record with original bounded packet, decoded 8-kHz S16LE bytes and target/left/right separation; preserve short final packets, identical repeats and order | Unconfigured/invalid packets are rejected; compat FIFO overrun/status is not journal loss. No raw padding interpreted as audio samples | Current compiled wired streaming decoder; do not invent other sample modes, targets or retail buffer limits |
| Steam input `0x44` feedback | **Not** a host command: optional typed feedback-*scheduled* diagnostic correlated when possible to cause; cannot be interpreted as successful host or physical delivery | Status `0x02/0x20/0x10`, low/high/overrun and response attempts retain existing protocol behavior. There is not one feedback ACK per PCM write | Existing wired firmware-compat feedback path; do not invent a MIDI/UAC endpoint |
| Future Wii Remote `0x18` | After separately approved **S1**: validated native speaker-data write with format/length, control context, rumble bit and bounded original bytes; separate typed control records for relevant `0x14`, `0x19`, `0x16` operations | **Today no `SpeakerDataAccepted` record is permitted**: `0x18` audio is discarded while its shared rumble bit is admitted. Do not count transport admission as speaker acceptance, fabricate playback, or treat register ACK as decoded audio | S1 first supports `linux.uhid.bluetooth` (kernel bus metadata only); `linux.uinput` remains unsupported. Wii S1/D7 evidence and codec validation are separate gates |

These are **family-typed APIs**, not a generic audio channel, physical output writer, MIDI synthesizer, UAC endpoint, routing engine or IUCM policy. Family validators and their reviewed research claims decide the meaningful native bytes. Future families add their own typed variants, format gates and bounded limits under ADR-0023; no runtime extension registry is implied. S1 remains **unsupported today**.

### 6. Steam option-3 implementation migration (later authorization required)

1. **Baseline and tests first:** pin existing current wired compat semantics for `0x86/0x87/0x88`, `0x44`, `0x83` MIDI/LFO, short final packets, identical writes, optional callbacks, retained revisions and opt-in behavior. Keep `steam_haptics_live` virtual-client E2E (Singer/Player/iczero) runnable with the same fixture and upstream executables; never downgrade it to a decoder-only test.
2. **Add family-native stream journal/reader behind the same controller ownership and opt-in gate**, with sequence/epoch, replay, explicit gaps, closure and deterministic fake-provider tests. Establish parity between accepted host PCM writes and stream records while the existing compatibility FIFO, `SteamController2026StreamOutputs::last_packet`, `SteamController2026Haptic::Stream` and raw PCM observations **all remain**. The compat FIFO remains the source of current virtual `0x44` feedback policy; the journal is not a replacement playback engine or a new wire path.
3. **In the same PR/change that exposes the authorized, tested typed API to applications**, migrate its consumers/examples and then remove the compatibility **payload access**: the retained `last_packet`, the PCM-bearing `SteamController2026Haptic::Stream` variant and raw reverse PCM payloads, and **remove the compatibility FIFO in that same change**, replacing its necessary bounded protocol-feedback/buffer model before deletion so host-visible `0x44` behavior remains correct. Do **not** remove feedback generation or the protocol's own finite buffering merely because the observation payload path moved. Preserve non-PCM diagnostics, other raw native events and existing non-stream haptic semantics. No interim release or intermediate commit in the implementation PR may publish removal without the typed path.
4. **Re-run parity and E2E:** every accepted repeated/mono/stereo/short packet arrives in order unless a provable reported journal gap; `0x44` statuses and exact host replies remain unchanged, as do MIDI tests, raw-observation non-PCM behavior, opt-in defaults, uinput refusal, stop/reset/recreate and cleanup. Compare original bytes and decoded samples without inventing retail buffer sizes. Document that stream receipt is not actuator output.

Option 3 has **already been approved**; this ADR does not reopen it or authorize step 2/3 yet. Later code PRs update ordinary-root/Wii/Steam API snapshots as affected and explain exact additions/removals in `docs/ALPHA_API_MIGRATION.md` under the current alpha API policy. This docs-only PR changes **none** of those APIs or snapshots.

### 7. In-process versus USB/IP worker

UHID and uinput in-process personalities can record at their controller-owned acceptance boundary; the journal belongs alongside native protocol state, not to a provider callback. Selected report/FF limitations remain explicit. A USB/IP worker runs in a separate process: exposing accepted commands and cursor replay to the application would require a **separate versioned worker-control operation** (or equivalent approved wire change) that transmits bounded typed records, epoch/cursor/gap/terminal metadata, validates family and generation, handles mixed versions, and reports worker death without claiming an unreceived tail is complete. Its transport operation must never hold up host replies.

The already approved [#203](https://github.com/anderstvoss/virtualgamepad/issues/203) **worker operation 8 v1** recovers *retained snapshots*, including terminal recovery. It does **not** carry an ordered stream and its narrowly scoped approval may not be reused for this purpose. Therefore USB/IP stream access is **explicitly unsupported** until separately specified, approved and implemented. **Yes: supporting USB/IP stream consumers would require an IPC wire change; Anders must decide whether/when to authorize it.** Do not alter broker framing, existing operation 8, public USB descriptors or packet bytes in this ADR.

### 8. Later implementation acceptance tests (not tests in this docs PR)

1. **Identity/order:** valid HID writes and FF commands sequence monotonically from one epoch, including two separate identical payloads; retry of the same acknowledged request does not duplicate; retained revisions increase only on changed levels and correlate without equaling command count.
2. **Atomicity:** malformed, unsupported, busy or failed FF upload/erase commands never appear as accepted; negative replies/status outcomes are correctly typed; upload versus playback versus stop versus erase remain distinct; kernel-assigned IDs are not stream positions.
3. **Finite replay:** generate bursts well past the existing 32-entry optional-observation queue and beyond journal record/byte budgets; in-range cursor replays exactly, lagging cursor receives exact first-missing/first-available bounds, no hidden loss, bounded memory/batch/time and no requirement to acknowledge.
4. **Priority:** stress saturated stream and optional queue, blocked/throwing observers and slow/panicking/detaching readers while required HID/FF requests complete within existing bounded retry/deadline rules; preserve exact replies, fail terminally on genuinely uncertain transport and avoid deadlocks/starvation.
5. **Steam:** `0x86` semantic results, `0x87` mono, `0x88` two-channel, `0x44` statuses, repeated identical chunks, short final chunks, configuration/stop/reset, compat overrun versus stream gap, missing configurations, opt-in disabled, `0x83` unaffected and no uinput PCM delivery. Preserve upstream virtual-client E2E and exact PCM byte correspondence through migration.
6. **Wii fixture only:** mark speaker `0x18` unsupported before S1 despite accepted rumble bit; after separate S1 authorization test `0x14/0x19/0x16` controls, exact `0x18` bounded representation, PCM/ADPCM validation, ACK/rumble/mute, buffering, status and no fake UAC or physical-playback assertion.
7. **Lifecycle:** host STOP/START preserves only a provable continuous epoch; reset, recreation, uncertain failure, worker death and close produce explicit terminal/new-epoch behavior; stale cursor rejected, last ring/gaps drainable while closed handle remains; post-close writes refused and cleanup idempotent.
8. **Realization parity:** fake-provider tests for in-process uinput and UHID plus explicit unsupported errors for unimplemented realization paths. A separately owner-approved worker step needs mixed-version/generation/shape tests, socket-pair loss/restart/terminal tests and later real USB/IP host evidence before acceptance.

## Consequences

Applications can forward ordered accepted host intent and *detect* loss, but cannot assert lossless delivery once a gap occurs, cannot recover missing command history from `host_outputs()`, and cannot infer that any physical device played the command. Journal capacity and native payload validation become part of each family's reviewed resource budget. Reader lag never reduces protocol correctness; large bursts can permanently invalidate a mirror's lossless claim. Separate journal-eviction diagnostics are needed without changing the meanings of existing observation counters or accepted-value revisions.

This is design only. Implementation changes the alpha family/root API and snapshots, may require new realization-specific refusal entries, and needs its own review, authorization and regressions. Brief 16's S1/S2 remain gated; no physical-speaker, faithful-actuator, USB/IP stream or hardware qualification is claimed.

## Alternatives

- **Required acknowledgments with consumer backpressure:** rejected. An absent/slow application must not stall host replies; finite memory still forces loss or refusal.
- **Destructive, pop-only event FIFO:** rejected as the authoritative interface. It cannot replay a short detach or establish the exact range lost before reconnection.
- **Only a terminal gap/fail-closed queue, without bounded replay:** viable but less useful for short transient reader pauses; choose bounded cursor replay *with* mandatory gap if the replay window is exceeded.
- **Use `service()` callbacks / `dropped_output_events()` / `host_outputs()`:** rejected. The first is an optional lossy observation surface; the latter is latest-level reconciliation, not command history.
- **Unbounded per-reader queues, generic PCM/UAC or provider-owned decoding:** rejected by finite-resource requirements, native ownership and ADR-0023.
- **Include USB/IP IPC in this PR or piggyback on #203:** rejected; owner approval for ordered worker-stream wire semantics is absent.

## Open questions for Anders

1. **Adopt this design and authorize a separate implementation PR?** This ADR is Proposed; neither #344 nor S0 automatically approves implementation.
2. **USB/IP scope:** approve a *new versioned worker-stream wire operation* in a later scoped PR, or retain an explicit USB/IP refusal until there is a concrete consumer? #203's operation-8 approval does not cover it.
3. **Resource/API specifics:** should the later implementation choose reviewed finite per-family default record/byte caps and a bounded maximum read batch (recommended), with any public tuning limited to validation at creation? Exact sizes and signatures need benchmarks rather than being falsely presented here as hardware limits.
4. **Diagnostic exposure:** retain the proposed optional typed rejection/feedback-scheduled records in the same ordered journal, or make accepted-command records the sole public entries and expose diagnostics through existing native status/observation surfaces? In either case, they must never be counted as accepted commands or gate required replies.
5. **Mirror disposition after gap:** this ADR invalidates the continuity claim and leaves stop/degraded continuation to the application. Does Anders want a separately specified fail-closed mirror policy outside VirtualGamepad?

No broker/IPC wire, #203 change, new hardware claim or public API is approved by accepting this *docs* PR.
