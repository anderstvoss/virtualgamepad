# ADR-0025: Native input levels and transactional edits

Status: Accepted ([owner decision on #322](https://github.com/anderstvoss/virtualgamepad/issues/322#issuecomment-6780450118), 2026-11-10).

## 1. Context

Seven realized families serve both snapshot-forwarding applications and
interactive editors. Device counters, battery, protocol modes and attachment
metadata must survive replacement of caller inputs. [ADR-0004](ADR-0004-synchronous-hid-session.md)
separates accepted levels from report delivery; [ADR-0011](ADR-0011-neutralization-and-observation.md)
defines release without resetting metadata; [ADR-0014](ADR-0014-protocol-edits.md)
establishes fallible clone/validate/install protocol edits. Retained host outputs
and typed native subsystems remain separate under [ADR-0020](ADR-0020-retained-outputs-and-kinds.md)
and [ADR-0023](ADR-0023-controller-subsystems.md).

This records the final merged VG-1b shape from the [input-model assessment](../spec/input-model-state-and-mutation.md#appendix-a--accepted-adr).
The [migration ledger](../ALPHA_API_MIGRATION.md#alpha1--alpha2-final-consumer-table)
contains final consumer names and the implementation PRs.

## 2. Native inputs and one transaction

Every family exposes input-only `<Family>Inputs`, borrowed `inputs()`, whole
`set_inputs(inputs)`, fallible `edit_inputs(|candidate| -> Result<(), ControlError>)`,
native field setters and handle `neutralize()`. Root and curated handles use the
same input types and semantic rules. There are no compatibility aliases.

Each path edits an isolated candidate, validates its complete final state against
native domains, active realization and live topology, derives controller-owned
transitions from the accepted difference, then installs the accepted edit.
Intermediate values in the closure need not be valid. A rejected edit preserves
accepted inputs, status, dirty work, pending report bytes and host outputs. An
unchanged edit adds no dirty work. Closed handles reject input edits. Closures
must not perform external side effects; input acceptance performs no provider I/O.

Identity, battery, protocol modes and counters, host outputs, audio/subsystem
streams, resource/calibration images, Wii revision and attachment presence are
outside caller input ownership. Wii `attach()` is a separate live topology and
protocol transaction. `WiiRemoteInputs` carries native extension levels, not
permission to change the extension kind through input replacement.

## 3. Time, neutralization and delivery

Motion clocks advance on accepted motion changes: DualSense timestamp +12,000
with its sequence; DS4 timestamp +750 with its sequence; Switch Pro accepted timer
+1. DS4 touch sequence advances once for a changed contact set, including last
release; a simultaneous motion change does not increment it twice. Returning to
neutral is an accepted change, not a clock reset. Steam motion timestamp remains
caller-supplied input. Transport packet cadence and stream modes are not input
acceptance clocks.

`refresh_motion()` is the explicit sample tick, retained separately from
`edit_inputs`: Sony advances clocks for equal motion and commits, including the
existing DS4 active-contact policy. Switch Pro retains its realization/stream
policy and independent HID cadence. Motion refresh availability remains
realization-specific. After delivery backpressure, retry `commit()` rather than
advancing the sample again. See the [accepted tick decision](https://github.com/anderstvoss/virtualgamepad/pull/353#issuecomment-6785199597).

Neutralization releases levels while preserving identity, battery, live
attachment and host output intent. For Wii use handle `neutralize()`, or clone
`inputs()`, neutralize that snapshot and `set_inputs` it. Default Wii inputs have
extension None and are rejected once an extension is attached. Live topology is
read with `current_topology()`; `projected_input_topology(bool)` is for detached
editor projections of pending attachments.

Explicit `commit()` and independent readiness/deadline `service()` retain
ADR-0004 semantics. Levels coalesce: a press and release before commit may be
unobserved. Acceptance is not atomic submission, delivery or host observation.
No general input event queue, universal state, partial-mask/remapping layer or
broker/IPC change is introduced. A future occurrence/edge API requires a separate
bounded, typed, family-native design and evidence.

## 4. Uniform status read view

Every realized family exposes `status() -> <Family>Status`, a small `Copy` read
view, even when its meaningful metadata is limited to the selected realization.
This keeps the input/status access pattern uniform without inventing telemetry.
`XboxSeriesStatus` therefore exposes only `target()`; it has no fabricated battery
or input clock. A future family with no metadata can keep an empty copyable view
until real metadata exists; the presence of the method does not promise fields.
No existing status method is removed by this decision.

Status reads remain available after close. Values come from their authoritative
runtime/protocol owner, not from caller-editable input snapshots. Wii protocol
ownership is required at creation and retains its last accepted battery after
close. Steam status uses optional fields when the selected realization has no
protocol channel: configured `lizard_mode()` and accepted host `lizard_setting()`
are separate observations. Host outputs remain in `host_outputs()`.

## 5. Consequences and evidence

Whole replacements and small edits cannot disagree on final-state validity,
realization refusal, clock transitions or neutralization. Native domains are kept
without normalization or silent provider-specific loss. Deterministic
fake-provider tests cover clean/dirty rejection, no-op edits, clone/readback,
clock/metadata/output preservation, attach rollback, retry and idempotent close
on each supported realization. Existing report/ACK and EEPROM regressions remain.
These tests establish implementation behavior, not new hardware qualification.

Alpha.1 consumers must migrate to input-only types and explicit metadata reads.
This decision introduces no new API delta beyond the merged VG-1b PRs and does
not authorize VG-2 implementation or change its separate ADR-0024 contract.
