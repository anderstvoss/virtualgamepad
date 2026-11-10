# Input model: native snapshots, mutation and transition semantics

**Status:** Design assessment for owner decision; **not** an implemented contract. **Baseline:** 2026-11-10, main at 69c2283fe6fd6ab143f650018c0d24522eaa4f9a, plus the unmerged VG-1 candidate branch codex/vg1-whole-state-inputs ([PR #348](https://github.com/anderstvoss/virtualgamepad/pull/348)). All statements describing the four new set_inputs methods refer to **that branch, not main**. No code, API snapshot, test, CI or wire-format change is proposed in this PR.

Related: [#343](https://github.com/anderstvoss/virtualgamepad/issues/343), [#322](https://github.com/anderstvoss/virtualgamepad/issues/322), output-stream design [#344](https://github.com/anderstvoss/virtualgamepad/issues/344), timing [#166](https://github.com/anderstvoss/virtualgamepad/issues/166). Families: [#154](https://github.com/anderstvoss/virtualgamepad/issues/154), [#155](https://github.com/anderstvoss/virtualgamepad/issues/155), [#156](https://github.com/anderstvoss/virtualgamepad/issues/156), [#157](https://github.com/anderstvoss/virtualgamepad/issues/157), [#158](https://github.com/anderstvoss/virtualgamepad/issues/158), [#159](https://github.com/anderstvoss/virtualgamepad/issues/159), [#160](https://github.com/anderstvoss/virtualgamepad/issues/160); planned [#296](https://github.com/anderstvoss/virtualgamepad/issues/296) and [#297](https://github.com/anderstvoss/virtualgamepad/issues/297).

## One-page decision summary

**Recommendation: both caller styles, one transactional semantic core.** Make a family-typed fallible edit of an isolated candidate the *single implementation primitive*. Expose whole-state set_inputs and individual native setters as thin wrappers around it on **both** root and curated handles. Every path validates the complete *final* candidate against the active realization and attachment/topology, applies controller-owned clock/sequence transitions by comparing old and new semantic inputs, and either installs the entire accepted edit or changes nothing. Keep explicit commit and independent service; neither API style promises atomic report submission or consumer visibility (docs/adr/ADR-0004-synchronous-hid-session.md:5-13).

**Prefer an input-only native snapshot type per family** (proposed naming: DualSenseInputs, DualShock4Inputs, SwitchProInputs, Xbox360Inputs, WiiRemoteInputs, SteamController2026Inputs and XboxSeriesInputs). Do not copy battery/identity, live attachment configuration, protocol counters, retained host output or calibration into such snapshots. A read-only status view and separate family-specific metadata/topology APIs may still expose those things. This is a recommended alpha refactor, *not* a claim that PR #348 has already separated them: its Sony/Xbox/Wii state shapes currently mix some of these concerns (source inventory below). Keep native domains, not a universal gamepad type.

**Delivery and edge rule:** input snapshots represent the latest *level*, not a history of events. A press then release before commit may disappear, under ADR-0004. Commit between edges when edge observation matters; even then delivery is not a consumer-observation guarantee. Do **not** add a generic input-event queue now. Require a separate bounded, typed, family-native submission design if real tests establish a need for events or repeated identical sensor samples.

**Known blocker:** [#348 review](https://github.com/anderstvoss/virtualgamepad/pull/348#issuecomment-6777788230) identifies frozen DualSense/DS4 motion timestamps and Switch Pro timer on whole-state updates. Never import controller-owned counters from a candidate, but **do** advance them for eligible accepted transitions, whichever API caused the transition. Current per-field setters advance on every motion call, while proposed VG-1 set_inputs preserves the old counter; reconcile this before treating the interfaces as equivalent. DS4 touch sample sequencing needs the same rule.

### Owner questions (recommended answers)

1. **Provide both set_inputs and mutable edits?** **Yes.** Keep native per-field convenience setters; add a family-typed edit_inputs fallible closure; keep set_inputs as an ownership-taking convenience over that core.
2. **Should snapshots contain metadata/protocol state?** **No.** Split public caller-owned Inputs from controller-owned status, battery, identity, topology, clocks and outputs; accept an alpha-breaking replacement of old state-as-input shapes.
3. **How should edits compose?** **Ordered last accepted edit wins**, except an edit rejected in full cannot alter state or dirty status. set_inputs replaces all caller-owned input levels; a later setter changes that field only.
4. **What does an unchanged candidate do?** **No new semantic dirty work**, preserving any pre-existing dirty/queued work. Protocol-mandated periodic generation remains in service; explicit sample/event submissions would be separate operations.
5. **Who advances clocks?** **The controller**, on accepted qualifying transitions using a shared transition function. Initially use motion-value change and DS4 touch-contact change; do not assume an identical value is a new sample. Decide a typed sample-tick facility separately if needed.
6. **Edge preservation?** **No generic edge FIFO in this API change.** Callers commit between desired edges; document that this is not a guaranteed consumer event. Escalate evidence-backed one-shot/event needs to a distinct bounded API.
7. **Multiple input writers and partial snapshots?** **No core masks, merge policy or broker format.** Serialize producers above one handle; use edit_inputs for field-group updates. A later explicit input-ownership/coordinator design must resolve conflicts and epochs.
8. **Unsupported controls?** **Reject meaningful unrepresentable input**, expose restrictions in typed surfaces, and never silently drop/translate. Do not make a whole-state path more permissive than field edits for an equivalent resulting state.
9. **Wii attachments and Steam lizard mode?** **Keep them outside ordinary input snapshots.** Wii attach remains a distinct transactional protocol/topology operation. Steam host-selected lizard/desktop mode belongs to protocol/host state, not caller input.

## 1. Evidence and terminology

This is a source review, not a hardware or timing measurement. Checked: both facade and curated paths, the VG-1 candidate branch, core/native/HID transaction paths, relevant controller definitions and encoders, ADR-0004/0011/0014/0020/0023 and the application/migration/API inventory documentation. The published snapshots are inventory evidence, not a substitute for the checked source implementation: docs/api/ALPHA_API_SNAPSHOT.json, docs/api/WII_REMOTE_API_SNAPSHOT.json, docs/api/STEAM_CONTROLLER_2026_API_SNAPSHOT.json and docs/api/API_INVENTORY.md. Current docs describing VG-1 only exist on its PR branch (docs/ALPHA_API_MIGRATION.md:155-177 there).

- **Whole-state replacement:** submit a complete family-native *input-level* candidate. An accepted update replaces every caller-owned level, including controls omitted by the caller because they were left at neutral. It does not mean replacing device metadata or clocks.
- **In-place field setters:** calls on the handle like set_native, set_touch or set_motion. Their *observable intended meaning* is a small patch to the accepted levels; the current implementation generally clones the state internally, so “in place” refers to the caller-facing API, not unvalidated mutation of a live object.
- **Detached mutable state construction:** mutate a Default or cloned state value with state-level helpers, then set_inputs. No device work occurs while building the detached value. In VG-1 the state-level helpers themselves are not the final realization validator.
- **Transactional mutable edit:** edit_inputs(|candidate| -> Result<(), ControlError> { ... }) runs on an isolated clone; final post-state validation and transition policy precede installation. The closure must not perform external side effects. ADR-0014 uses this pattern for protocol edits (docs/adr/ADR-0014-protocol-edits.md:5-18).
- **Edge/event input:** an ordered press, release, one-shot action, touch lifecycle event or sample occurrence whose *occurrence* matters even when the final levels match. This is a different semantic contract; neither a state replacement nor a mutation syntax makes intermediate events durable.

The first three are largely alternative *ergonomics* for editing retained input levels. The fourth adds a **multi-field all-or-nothing transaction boundary**, not an independent representation. The fifth is genuinely different because it needs sequencing, buffering, deadlines and loss/backpressure semantics. Compare output vocabulary: the accepted level/revision versus lossy event distinction is established at docs/adr/ADR-0020-retained-outputs-and-kinds.md:14-38. Input sequence/epoch, if ever exposed, must name the semantic input acceptance order rather than reuse a device packet counter or #344 host-output revision.

There is no universal normalized state, raw report injection, implicit mapping or fallback in this proposal. The types and permissible transitions are each family's responsibility.

## 2. Inventory: all seven families, both layers

**Read table as of VG-1 branch, with its PR still open.** R = ordinary root (src/controllers.rs); C = curated handle. On these surfaces R forwards to C, so validation generally occurs in C/common, not in an independently maintained R implementation (src/controllers.rs:137-225, 266-368). “Fields” describes handle setters; detached value edits are mentioned separately.

| Family / realization(s) | Root and curated input methods | Snapshot and validation distinctions / non-input operations | Evidence |
| --- | --- | --- | --- |
| DualSense: uinput, UHID USB, dummy_hcd USB HID, USB/IP USB audio | **Both**: state, set_inputs **(VG-1)**, set_digital, set_native, left/right sticks, triggers, touch slot, motion, neutralize, commit. Battery setters separate. | Snapshot checks hat opposition, HID touch IDs 0–127, non-neutral motion and microphone mute on uinput; preserves battery, input sequence and sensor timestamp. Per-field set_motion advances timestamp/sequence and rejects uinput, while set_native mute and touch restrictions do not completely match snapshot validation. | src/controllers.rs:403-430; crates/gr-curated-controllers/src/dualsense.rs:148-184, 1285-1416; crates/gr-curated-controllers/src/common.rs:96-132 |
| DualShock 4: uinput, UHID USB, dummy_hcd USB HID, USB/IP USB audio | **Both**: state, set_inputs **(VG-1)**, set_digital, set_native, sticks, triggers, motion, touch slot, neutralize, commit. | Snapshot rejects opposing hat directions, touch IDs above 127 on **all** targets, non-neutral uinput motion; preserves battery, sequence, sensor timestamp, touch sequence. Field setters currently increment motion timing and touch sample sequence on each call. | src/controllers.rs:496-518; crates/gr-curated-controllers/src/dualshock4.rs:123-153, 1114-1205, 1438-1447 |
| Switch Pro: uinput, UHID USB, dummy_hcd USB HID | **Both**: state, set_inputs **(VG-1)**, set_digital, set_native, left/right sticks, motion, neutralize, commit; stream/timer observation is separate. | Hat-opposition restriction only on uinput; USB packet has distinct direction bits. Non-neutral motion unavailable on uinput. Snapshot preserves timer and host-selected stream_enabled; field motion setter increments timer. | src/controllers.rs:582-611; crates/gr-curated-controllers/src/switch_pro.rs:62-88, 968-1050 |
| Xbox 360 **standard HID**, not Xbox Series/xpad: uinput, UHID USB, dummy_hcd USB HID, USB/IP USB audio | **Both**: state, set_inputs **(VG-1)**, set_digital, set_native, sticks, triggers, neutralize, commit; battery setters separate. | Snapshot rejects opposing D-pad directions for hat representation; preserves battery. Trigger and stick domains are native newtypes. | src/controllers.rs:654-674; crates/gr-curated-controllers/src/xbox360.rs:64-85, 796-863 |
| Wii Remote: uinput, UHID Bluetooth **metadata** (not a radio-pairing service) | **Both**: state, set_inputs, set_digital (returns unsupported), set_button, accelerometer, IR, battery_raw, extension values, MotionPlus values, neutralize, commit; separate attach, current_topology, EEPROM resource operations. | Snapshot refuses revision, extension **kind** or MotionPlus attachment changes. Live attach performs a topology/protocol transaction. Extension values are validated, including unsupported Classic Pro; ten-bit accelerometer and fourteen-bit MotionPlus constraints. Battery raw is presently mixed into the snapshot. | src/controllers.rs:711-769; crates/gr-curated-controllers/src/wii_remote/controller.rs:129-184, 231-319; crates/gr-curated-controllers/src/wii_remote.rs:270-319, 339-415; crates/gr-curated-controllers/src/wii_remote/extensions.rs:5-22 |
| Steam Controller 2026 wired: reduced uinput, UHID USB | **Both**: state, set_inputs, set_digital, set_button, neutralize, commit. **Most analog/touch/motion changes require constructing/editing the public state** rather than handle-specific setters. | Trigger words must be nonnegative; wired state carries sticks, pads (touch, click, pressure), capacitive grips/sticks and a **caller-provided** motion timestamp. Reduced uinput emits selected pads/sticks/buttons but omits the native IMU, capacitance and some pressure semantics. Lizard/desktop mode is protocol/host policy. | src/controllers.rs:818-838; crates/gr-curated-controllers/src/steam_controller_2026.rs:105-174; crates/gr-curated-controllers/src/steam_controller_2026/controller.rs:284-309; crates/gr-curated-controllers/src/steam_controller_2026/definition.rs:140-157, 198-224; crates/gr-curated-controllers/src/steam_controller_2026/evdev.rs:65-112; crates/gr-curated-controllers/src/steam_controller_2026/codec.rs:251-257, 346-402 |
| Xbox Series: uinput xpad-shaped only (C9 #342 merged on main) | **Both**: state, set_inputs, set_digital, set_button, neutralize, commit. Public state exposes sticks and validated trigger mutator, not handle-specific analog setters. | Trigger newtype enforces range, no motion/complex touch claim; four native D-pad buttons. No HID or impulse-motor input claim. | src/controllers.rs:863-883; crates/gr-curated-controllers/src/xbox_series.rs:65-128; crates/gr-curated-controllers/src/xbox_series/controller.rs:76-114; crates/gr-curated-controllers/src/xbox_series/definition.rs:325-352 |

The four VG-1 state-value helper sets are source-backed at crates/gr-curated-controllers/src/dualsense.rs:238-287, dualshock4.rs:212-267, switch_pro.rs:133-174 and xbox360.rs:129-165; the corresponding migration summary is docs/ALPHA_API_MIGRATION.md:157-176 **on the PR branch**. Root forwarding uses the same native state types (src/controllers.rs:266-283, 359-368).

**Nonuniformity requiring action, not just documentation:**

1. **Realization checks in snapshot wrappers, not shared validation:** DualSense, DS4 and Switch Pro's driver validate_state is primarily a supported-target check (dualsense.rs:852-865; dualshock4.rs:770-783; switch_pro.rs:581-594). Critical hat/touch/unsupported-motion checks are in set_inputs (ranges above), so a field setter or digital edit can currently reach a different accepted state. Move final-state checks to one family transition validator used by every path. Per-operation capability refusals can remain for attempts to manipulate an absent feature.
2. **Temporal disagreement:** per-field motion/touch edits advance counters; VG-1 snapshots copy old clocks. See the linked #348 review, dualsense.rs:290-295, 1370-1381; dualshock4.rs:1183-1199; switch_pro.rs:1019-1024.
3. **Mixed snapshot ownership:** battery in DualSense/DS4/Xbox360/Wii, counters in Sony/Switch, Wii attachment kind and revision, versus Steam caller timestamp and Xbox Series input-only shape. See the state struct declarations and sections 5–6.
4. **Different convenience coverage:** Steam and Xbox Series rely on editing their exposed native state for analog changes. A single edit_inputs API removes this usability asymmetry without pretending every family has identical controls.
5. **No-op accepted edits:** the native runtime marks dirty for every successful update, whether levels changed (crates/gr-controller-runtime/src/lib.rs:52-77). HID similarly marks dirty (crates/gr-hid/src/lib.rs:241-253). That is existing behavior, not the recommended idempotent behavior.

## 3. Which callers benefit, and failure modes

| Caller | Recommended style | Failure if forced into the other style |
| --- | --- | --- |
| Native-frame mirror/forwarder | set_inputs(native snapshot), with explicit commit cadence | Dozens of setter calls each frame create intermediate accepted states and partial acceptance on late failure; failure to reset omitted fields leaves stuck inputs. |
| Script or game logic changing one button | set_native or set_button; commit when useful | Rebuilding a full Default snapshot for each press neutralizes unrelated held controls; cloning manually at every call is noise. |
| GUI with independent widgets/sliders | edit_inputs for grouped UI changes; per-field for individual interactions | Full replacement from stale UI widgets overwrites other live fields; individually validated setters can apply half of a logical two-control gesture. |
| Deterministic test and acceptance fixture | set_inputs to define complete scenario, edit_inputs for controlled transitions; explicit commits | Setter-only fixtures are verbose and can inherit unexpected prior state; snapshot-only fixtures obscure narrow cause/effect. |
| Several producers (buttons, motion, touch) | Application-level serialized coordinator applying family-specific edits to the current candidate | Competing full snapshots erase each other's fields; naive read-modify-write can race. A single &mut handle serializes methods but does not resolve which producer owns each field or event ordering. |
| Touch/gyro source with meaningful sample history | Explicit sample operations only if supported; otherwise commit each desired transition | Latest-level snapshots erase intermediate contacts and identical samples; a closure does not create a queue. |

The multi-writer solution is **not** a library-wide field mask. A coordinator can keep one family-native snapshot and apply disjoint family-specific edits in an explicit order; it must define conflicting-write policy, release on producer disappearance and topology-epoch invalidation. If this later crosses a process boundary, it needs a separately reviewed broker/IPC contract; this document proposes none.

## 4. Single semantic core and composition

**Proposed canonical operation:** for the selected family/realization, make one isolated editable Inputs candidate, run a fallible closure, run full post-state and topology/realization validation, derive controller-owned transitions from old to candidate, and install state **once**. If anything fails, preserve old Inputs, status/counters, dirty flag, pending bytes and outputs. No provider I/O in the closure. Existing internals already implement clone/apply/check/install in native (crates/gr-controller-runtime/src/lib.rs:64-77), HID/worker (crates/gr-curated-controllers/src/common/session.rs:286-324), and protocol configuration (docs/adr/ADR-0014-protocol-edits.md:5-18). The HID layer currently clones again in Runtime::update (crates/gr-hid/src/lib.rs:241-253); this is an optimization opportunity, not a semantic exemption.

Both public conveniences can be defined through this core:

~~~rust
// Proposed API; not present in PR #348.
pub fn edit_inputs(
    &mut self,
    edit: impl FnOnce(&mut FamilyInputs) -> Result<(), ControlError>,
) -> Result<(), ControlError>;

// Equivalent semantics:
pub fn set_inputs(&mut self, replacement: FamilyInputs) -> Result<(), ControlError> {
    self.edit_inputs(|candidate| { *candidate = replacement; Ok(()) })
}

pub fn set_native(&mut self, button: FamilyButton, pressed: bool)
    -> Result<(), ControlError> {
    self.edit_inputs(|candidate| { candidate.set_native(button, pressed); Ok(()) })
}
~~~

The example uses placeholder *type names*, not a proposed universal Rust trait or public generic gamepad. In real code every family keeps its own specific argument and button types. Implementation may avoid the redundant old-state clone for set_inputs after validating ownership; a fast path must not bypass shared validation or clock derivation.

**Normative composition (recommendation; not current implementation):**

| Sequence | Intended accepted semantic result |
| --- | --- |
| set_inputs(A) → set_native(X,true) → commit | All levels from A, except X=true. One or more reports, not necessarily one. |
| set_native(X,true) → set_inputs(B) → commit | All levels from B; previous pending setter edits are replaced. |
| set_inputs(A) → edit_inputs(change X and Y) → commit | Complete A with X and Y edited together; failure of either edit aborts the closure. |
| setters → neutralize() → commit | All caller-owned controls at family-defined neutral; preserve identity, telemetry, clock position, host outputs, topology and resource state. DS4 releasing active contacts may generate its next touch sequence. |
| set_inputs(invalid) after successful edits | Reject entirely; previously accepted edits and dirty flag remain. |
| edit_inputs closure returns Err or candidate is invalid | Reject entirely; existing accepted state, dirty, protocol counters and queued bytes remain. Side effects *outside* candidate are caller error, not rollbackable. |
| accepted edit with no level change | Do not newly dirty a clean session; do not clear already-dirty work or discard queued reports. |
| commit encounters definitely-unsent/backpressure | Preserve dirty/retryable accepted work and exact already accepted queue bytes. A new later edit cannot rewrite an already queued report. |
| uncertain transport submission or closed session | Follow terminal close rules; all future edit forms return Closed with no state mutation. Do not retry uncertain bytes. |
| service while inputs unchanged | Continue host work, required responses, controller clocks/cadence and optional output observation as the realization requires. |

The first four rows specify **last accepted edit wins at the desired level**, not last report wins on the host. ADR-0004:7-13 separates desired state, protocol timing and delivery. For HID, pending transport bytes and interest are distinct from semantic dirty (crates/gr-hid/src/lib.rs:281-309); worker commit keeps dirty until bridge.update succeeds and closes on terminal failures (crates/gr-curated-controllers/src/common/session.rs:385-424). Rejection must be tested when clean **and** when dirty.

Neutralize is not equivalent to blindly installing Default when Default would reset metadata or the active Wii attachment. ADR-0011 explicitly preserves battery, clocks and host outputs, and advances the DS4 touch sequence only for a release (docs/adr/ADR-0011-neutralization-and-observation.md:5-16); the DS4 implementation is dualshock4.rs:1438-1447.

**One final-state validation path; operation intent still matters.** An unsupported feature should be rejected when a setter explicitly requests it, even if it requests a neutral value; a full neutral snapshot need not be rejected simply because the device lacks that feature. Conversely, unsupported *non-neutral* state must never be silently accepted and discarded by an encoder. Controller-newtype constructors should validate ranges before the edit and family validation should still recheck all final invariants.

## 5. Temporal semantics: edges, sensors and per-family cases

**Levels may coalesce.** A=true then A=false before one commit is indistinguishable from no change in a pure level model. This is acceptable for a joystick snapshot mirror whose producer deliberately samples latest state, but **not** for scripted button pulses, countable actions, DS4 touch sample lifetimes or relative-pointer deltas. Applications must commit between meaningful transitions and continue service; consumers may still miss transitions due to downstream sampling/report behavior. Do not claim edge preservation or minimal duration. A future typed event API would need bounded capacity, ordered acceptance, retry/loss status and reconciliation with the current level; #344 concerns host **outputs**, not that input queue.

**Clock ownership policy:** separate (a) caller sample data, (b) controller-owned packet/stream sequence and (c) time/occurrence of a new sensor sample. A new semantic motion value causes a qualifying change; copying snapshots never copies controller counters. Apply identical temporal rules to per-field setters, whole-state replacement, transactional edits and neutralize. An unchanged value is not evidence of a new observation: repeated identical samples are a separate **sample occurrence** question. For now default to change-triggered advancement as #348 review recommends, with tests. Do not assume that this alone faithfully reproduces periodic stationary IMU samples. A later explicit family-native submit_motion_sample/sample_tick or an input-sample timestamp/sequence can distinguish repeated samples, only after an evidence-backed design. No invented wall-clock timestamp from arbitrary commit calls.

Specific cases:

- **DualSense:** motion sensor timestamp advances by 12,000 native units (4 ms at the code's nominal 250 Hz), with input sequence advancing, in advance_sensor_timestamp (dualsense.rs:290-295). Current set_motion advances unconditionally, while set_inputs copies both old fields (dualsense.rs:1291-1315, 1370-1381). Proposed: increment exactly once on accepted motion *value change*, independent of entry point; keep input report generation/sequence obligations controller-owned. Check static-sample behavior separately against #166.
- **DS4:** motion sample timestamp/sequence and touch_sample_sequence are separate; touch IDs persist across moves and are released via None; contact re-ID is an observable lifecycle transition. The current set_motion/set_touch advance on every call (dualshock4.rs:1183-1199), while snapshot preserves counters (1114-1142). Proposed: exactly one contact sequence step per accepted touch-set transition, motion clock step per motion change, and no step for rejected/identical levels; neutralize retains the established active-contact release behavior (1438-1447). Confirm sample count semantics at codec/fake-provider level.
- **Switch Pro:** its timer and host stream_enabled are not caller snapshot fields (switch_pro.rs:62-88, 968-989). Preserve the stream handshake and advance appropriate timer work on motion changes; USB can express concurrent D-pad direction bits where uinput hat cannot. Timing under a host-selected stream requires tests with service while the input snapshot is unchanged, rather than treating every call as a sample (1019-1050).
- **Wii Remote:** buttons, IR and accelerometer are levels; MotionPlus readings and extension bytes are samples for the current attachment. Revision, current extension kind, MotionPlus presence, topology generation, EEPROM/resource state and host-driven status/ACK behavior are **not** replaceable by set_inputs. attach is a separate transaction that may create protocol/status edges; do not flatten it into a snapshot assignment (wii_remote/controller.rs:129-184, 231-245; wii_remote.rs:306-319, 359-379). No invented factory calibration.
- **Steam Controller 2026:** pad touch, pad click, position and pressure are independent, as are capacitive stick/grip contacts (steam_controller_2026.rs:105-154). Motion includes caller timestamp_us, acceleration, gyro and quaternion; preserve that caller timestamp rather than stamping it from handle commit. A pad movement can feed relative desktop/lizard behavior whose path is lost if intermediate pad positions are overwritten; its protocol compares current and prior positions (steam_controller_2026/lizard.rs:62-86). Lizard and host feature changes belong to protocol, not input mutation (steam_controller_2026/codec.rs:346-402).
- **Xbox 360 and Xbox Series:** button levels, D-pad representation and analog triggers are snapshot levels; no fabricated sensor clocks. Xbox Series has four D-pad buttons in its type and xpad-shaped uinput, not a synonym for the Xbox 360 standard HID implementation (xbox_series.rs:65-128; xbox360.rs:64-85).
- **Future GameCube adapter/Joy-Con:** require a fresh family-native typed contract and any attachment/component topology policy before reusing these semantics; do not invent hypothetical fields now.

## 6. What an input snapshot owns

**Caller-owned:** native buttons, sticks, triggers, current contact coordinates/IDs/active flags, IMU measurements, Wii IR points and *currently attached* extension values, Steam capacitive/pad levels, and an explicit sensor **source timestamp only if the native type exposes it**, as Steam does. Each family defines neutral values; Steam's raw zero pose is not a claim of factory calibration (steam_controller_2026.rs:119-154).

**Controller-owned or separately managed:** persistent identity and session association; battery/status telemetry (set via separate explicit APIs where supported); packet sequence, DS4 touch sample sequence, HID sensor timestamps/stream mode; host outputs and loss/revisions; Wii revision, attachment topology, MotionPlus presence, EEPROM and calibration resources; Steam lizard/desktop protocol settings and host effects; provider delivery and audio/subsystem state. See ADR-0011:11-16, ADR-0020:14-38, ADR-0023:14-40, dualsense.rs:148-184, dualshock4.rs:123-153, switch_pro.rs:62-88, wii_remote.rs:270-319 and steam_controller_2026.rs:138-154.

**Current boundary is inconsistent:** the Sony/Xbox360 set_inputs implementations copy current battery over incoming battery; Sony/Switch copy current clocks; Wii forbids attachment changes but still carries the attachment kind and raw battery in its public state; Steam carries its own *caller-controlled* timestamp. These are code facts, not a universal justification for copying every similarly named field.

**Recommended alpha cleanup:** split input-only family value types from read-only status/protocol metadata. Expose inputs() for accepted native levels and independent typed telemetry/topology access. Rename/remove the old whole-state candidate signature if necessary; do not keep a compatibility alias solely for alpha. If retaining the existing FamilyState name, make its replacement semantics explicitly input-only and move noninput fields out. The implementation should choose **one** name consistently across root/curated, not maintain two parallel candidate concepts.

**Partial snapshots:** do not introduce a universal bit mask/field-group taxonomy. Family-specific helpers (e.g. edit_inputs closure changing only a pad) are sufficient to express partial updates safely. A separate per-producer domain coordinator may later need typed ownership and topology epochs; that is beyond this API assessment.

## 7. Realization restrictions: reject, document or type away

Use native newtypes/constructors for intrinsic domains (e.g. Xbox Series triggers, Sony touch coordinates). Use final-state validation for constraints depending on target, current topology or multiple fields. Surface metadata must describe limitations *and* mutators must enforce them; documentation alone is insufficient for non-neutral inputs that silently vanish.

| Case | Required behavior | Current baseline gap / evidence |
| --- | --- | --- |
| Hat versus independent D-pad bits | Reject simultaneous opposing directions on hat encoders; retain valid independent bits on Switch USB. | common.rs:95-105; snapshot validation in dualsense.rs:1291-1295, dualshock4.rs:1114-1118, xbox360.rs:802-809, switch_pro.rs:968-974; field setters rely on weaker driver checks. |
| DualSense mute on uinput | Reject active mute, not just list a registry refusal; enforce on set_native as well as set_inputs. | dualsense.rs:1291-1308, 1320-1329; [#348 reviewer note](https://github.com/anderstvoss/virtualgamepad/pull/348#issuecomment-6777788230). |
| Sony uinput motion | Reject non-neutral full-state motion and explicit unsupported motion setter. An all-neutral snapshot remains valid. | dualsense.rs:787-796, 1304-1308, 1370-1381; dualshock4.rs:1125-1134. |
| Sony touch ID | Honor actual target ranges: DualSense HID report max 127 but uinput advertises 255; DS4 uinput advertises 127. No truncation of IDs or inconsistent setter path. | common.rs:107-118; dualsense.rs:466-474, 1295-1303; dualshock4.rs:411-419, 1117-1124. |
| Wii extension values/kind | Validate bounded source-backed values; unsupported Classic Pro refused; kind change requires attach, not snapshot. | wii_remote/extensions.rs:5-22; wii_remote/controller.rs:129-184, 231-299. |
| Steam reduced uinput | Preserve supported analog/pad click/position; **reject** meaningful IMU/capacitance/pressure values not delivered, or explicitly redesign a separately documented native input-only reduced type. Do not advertise unsupported semantics as accepted. | steam_controller_2026/definition.rs:198-224 currently validates trigger range only; steam_controller_2026/evdev.rs:65-112 emits a reduced subset (design inference from comparing encoder to State). |
| Xbox Series uinput | Use its exact xpad-like buttons/triggers/sticks and target; no unsupported HID or impulse-motor claim. | xbox_series/controller.rs:76-114; xbox_series/definition.rs:325-352. |

This distinguishes *representable neutral* from *explicit capability access*: a neutral full-state candidate may be valid on a reduced target, while a direct request for an absent sensor can return UnavailableInRealization. In every case, an accepted **non-neutral level** must be expressible; no hidden normalization or fallback.

## 8. Cost and timing risk (code-based estimates only)

The seven input structures shown in the source contain fixed-size primitive arrays, native numeric structs and small enum variants, not an input-frame heap buffer; copying a state is **O(size of native state)** and running the visible validators is bounded by each family's fixed number of controls, slots and attachments. Native ControllerRuntime::update_state clones once, edits, validates and installs (crates/gr-controller-runtime/src/lib.rs:64-77). Worker does the same (crates/gr-curated-controllers/src/common/session.rs:306-324). The curated HID wrapper first clones, and gr-hid::Runtime::update clones again (common/session.rs:292-305; crates/gr-hid/src/lib.rs:241-253), so the HID path can perform **at least two** state clones; separate HID protocol generation may clone protocol state as required by ADR-0004.

**Order-of-magnitude inference, not a measurement:** current native level values are of the order of tens to a few hundred bytes each, with no unbounded input vector. A conservative 512-byte-per-clone planning allowance corresponds to 128 KiB/s of state copy at 250 edits/s, or 500 KiB/s at 1,000 edits/s, **per copy, per controller**; two copies double that allowance. Actual sizeof, optimizer behavior, validation cost, report generation, provider writes, allocations and latency were **not measured here**. The control path may be dominated by encoding/delivery rather than copying; do not infer timing compliance from small structs.

Implementation follow-up for [#166](https://github.com/anderstvoss/virtualgamepad/issues/166): record std::mem::size_of for each real input type and any accessory variants, benchmark p50/p95/p99 of validate+edit, set_inputs, per-field edit and commit separately at idle and queued/backpressured conditions for 60/250/1,000 Hz synthetic workloads, per target and host platform as meaningful. Record environment, optimizer/compiler and payload sizes, compare with service/deadline budgets and first-valid-neutral acceptance. Never claim benchmark results from this docs-only review. Avoid adding per-frame heap allocation or an extra full state clone solely to share an API wrapper.

## 9. Proposed public API shape, delta and rejected alternative

**Same pattern for each family, both layers; no universal family trait requirement:**

~~~rust
// Proposed names/signatures; not compiled or implemented.
let mut snapshot = DualSenseInputs::default();
snapshot.set_native(DualSenseControl::Cross, true);
snapshot.set_sticks((lx, ly), (rx, ry));
controller.set_inputs(snapshot)?;              // full level replacement
controller.commit()?;

// Incremental convenience; preserves unrelated accepted levels:
controller.set_native(DualSenseControl::Cross, false)?;
controller.commit()?;

// Multi-control transaction; only final candidate is validated/installed:
controller.edit_inputs(|inputs| {
    inputs.set_native(DualSenseControl::Cross, true);
    inputs.set_sticks((lx, ly), (rx, ry));
    Ok(())
})?;
controller.commit()?;

// Caller must separate meaningful edges with commits; service still runs:
controller.set_native(DualSenseControl::Cross, false)?;
controller.commit()?;
controller.service(&mut |output| { /* consume */ })?;
~~~

Specific family Input types expose native getters and mutators appropriate to that family. For Steam, callers edit native pad fields and timestamp without inventing Sony-like generic button axes; for Wii, extension value editing is constrained to the *active* attachment and topology change still uses attach. Keep neutralize() as a distinguished semantically validated operation, not a reset of entire runtime metadata.

**Delta from VG-1 branch, to be decided before implementation:** add edit_inputs to all seven root/curated handles; introduce or refactor family-specific input-only candidate types and readback; route all setters, set_inputs and neutralize through one family transition validator; derive counters on accepted input changes, not entry-point identity; enforce per-realization constraints consistently; provide no-op idempotence with pending-work preservation. Keep commit, service, diagnostics, host_outputs and controller-specific attach/resource/feature APIs as distinct operations. **Remove** duplicate per-entry-point validation, direct generic live mutable references if any are proposed, state snapshot fields that inadvertently own battery/clocks/topology, and any alpha-only aliases created solely for compatibility. There are no such removals in this docs-only PR.

**Rejected main alternative: only expose set_inputs and delete handle field setters.** It would provide one atomic *edit* surface, but a GUI or script must clone/read/modify/rewrite a complete snapshot for every small change, and a stale snapshot could erase inputs from another producer. A “setters only” design is worse for mirror/replay and late-validation rollback. A universal patch mask, raw HID ingress or silent canonical normalization would also violate native-family modeling and is not proposed. Keep both ergonomic surfaces and one semantic implementation instead.

## 10. Later implementation test plan — **not run**

Use deterministic injected fake-provider sessions and exact encoded-report or evdev-event oracles, not physical hardware, to prove the contract before any hardware gate. No new tests are included in this PR. Fixtures must contain no real serials, addresses, captured identifiers, or invented address-shaped placeholders. Use an independent test-local dummy identity only if the existing framework requires one, without exposing it in public documentation.

**Shared matrix, for every supported family and target:** successful complete replacement; default/neutral; setters before/after replacement; two-control edit transaction with a late invalid field; rejection before and after already-dirty work; unchanged accepted candidate; no I/O until commit; first commit and service scheduling; definitely-unsent fail-once and retry bytes/order; pending accepted reports not rewritten by later edits; uncertain/terminal failure, closed rejection, close idempotence; unchanged host outputs/identity and required replies; revision/counter invariants; parity between root forwarding and curated; callback order and no consumer-atomicity assertion. Exercise no-op after dirty, retry after changed desired state, and service with no new input. Source for existing fake-provider scaffolding: crates/gr-curated-controllers/src/common/session_tests.rs:1490-1618 and src/controllers/tests.rs; do not mistake existing VG-1 tests for proof of this proposed design.

| Family / target-specific cases | Deterministic oracle |
| --- | --- |
| DualSense — uinput, UHID USB, dummy_hcd USB HID, USB/IP worker | Two contacts and ID bounds per encoder; active mute rejection on uinput from both paths; motion rejection on uinput; hat opposition, accepted motion transition timestamp/sequence and repeated identical candidate behavior; neutralize preserves battery/output; worker backpressure and USB packet bytes. |
| DS4 — same four targets | Touch slot lifetimes, 127 ID limit for evdev and HID, touch sequence exactly once for changed/released contact set, no change for identical, motion timestamp/sequence equivalence across APIs, hat and unavailable uinput motion. |
| Switch Pro — uinput, UHID USB, dummy_hcd USB HID | Uinput hat reject vs USB independent direction bits; host stream handshake retained; timer transition rules, repeated equal snapshot, idle service and refresh_motion behavior. |
| Xbox 360 standard HID — uinput, UHID USB, dummy_hcd USB HID, USB/IP worker | Hat direction and trigger bounds, true standard HID encoding rather than Xbox Series xpad, preserved battery, complete frame parity and worker retry. |
| Wii Remote — uinput, UHID Bluetooth metadata | Active revision/extension/MotionPlus guard for snapshot and edits; valid Nunchuk/Classic values and rejected Classic Pro/out-of-range values; attach status edges and topology generation not replaceable by input; neutralize across attached accessories; EEPROM/calibration and ACK/host requests unaffected. |
| Steam Controller 2026 — reduced uinput, wired UHID USB | Negative trigger all-or-nothing rejection; pad touch/click/position/pressure independence; non-representable uinput IMU/capacitance restrictions, native source timestamp, repeated sample semantics; lizard pointer/keyboard transitions and held-key release; unchanged feature/streaming/haptic host outputs. |
| Xbox Series — xpad-shaped uinput | Four D-pad bits, stick polarity/ranges, both trigger validation, share/guide buttons, force-feedback output isolation; no accidental standard Xbox 360 HID target. |

For the planned GameCube adapter and Joy-Con (#296/#297), write their own acceptance tables when actual types/realizations are designed; **do not** report them as tested or extrapolate Sony/Wii/Steam timing semantics. A later runtime benchmark and live consumer test under #166 is separately gated.

**Validation performed for this document:** remote source/ADR review only. No cargo build, cargo doc, benchmarks, fake-provider tests, hardware experiments or local workspace checks were run; there are no code modifications. This is a docs-only owner-decision request.

---

## Appendix A — Proposed ADR text (not accepted; not a new file under docs/adr)

**Title:** Native input levels and transactional edits

**Status:** Proposed for owner decision.

**Context:** Seven realized families expose family-native setter APIs; a full-state replacement API is present in three families on main and proposed for four more in VG-1. Stateless forwarding needs whole-snapshot replacement; interactive applications need small edits; protocol counters and attachment state must not be accidentally overwritten. ADR-0004 separates semantic edits from report delivery, ADR-0011 defines neutralization, ADR-0014 provides a fallible cloned protocol edit precedent, ADR-0020 distinguishes host-output levels and edges, and ADR-0023 reserves typed native subsystems.

**Decision proposed:** Every family exposes an input-only native snapshot, a borrowed accepted-input read view, set_inputs(input), edit_inputs(fallible native closure), targeted field setters and neutralize(). Root and curated use identical types and semantic validation. A single family/realization-aware input transition validates the candidate once, makes controller-owned counter/sequence updates from the accepted difference and installs the whole change without I/O or partial side effects. All methods return typed failure with no accepted-state/dirty/pending mutation; closed handles reject edits. commit() and service() retain ADR-0004 delivery semantics. Device identity, battery/status, protocol modes/clocks, host outputs, attachments, calibration/resources and subsystem streams are not owned by caller input snapshots. Wii attach remains a separate protocol/topology transaction. No general raw-input events, universal state, partial mask, hidden remap, broker/IPC change or backward-compatibility shim is required.

**Consequences:** Mirroring and interactive control both remain first-class. Snapshot and mutation paths cannot disagree on validity, clock behavior or realization refusal. Pure level editing coalesces intermediate edges; edge/sample occurrence requires separate explicit design. Alpha migration may rename public state shapes. Tests prove each family/target's transition, clock, loss/retry, topology and output-isolation behavior under deterministic fake providers before live qualification.

**Open decisions:** Resolve numbered owner questions at the top of this document; ratification and Rust implementation require a separate PR.
