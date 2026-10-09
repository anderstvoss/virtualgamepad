# Controller output acceptance ledger

Gate 2 remains open. This ledger separates already inspected/accepted boundaries
from the process-level USB observations added during the sequential review.
A process socket fixture is not kernel-consumer or physical output acceptance.
See the canonical [gate criteria](ALPHA_GATE_CRITERIA.md).

| Family and implemented fields | HID / evdev boundary | USB validation boundary and remaining evidence |
| --- | --- | --- |
| DualSense: native report 2; three validity flags, compatible motors, two 11-byte trigger effects, mute LED, microphone mute, four audio path modes, speaker/microphone volume, speaker preamp, player indicators and RGB | Controlled UHID outputs and typed decoding have prior acceptance; evdev represents conventional rumble, not Sony-native report fields. Audio-control observations do not change PCM or establish physical actuator response. | Production worker process tests now verify both interrupt OUT and SET_REPORT acknowledgements, exact ordered raw observations across start/update/stop and four audio paths, no duplicate events and zero output drops. Typed field decoding remains covered by existing controller tests. Live USB consumer/callback observations and final lifecycle matrix remain required. |
| DS4: native report 5; motors and validity-gated RGB; unknown bytes retained | Controlled UHID outputs, evdev rumble and two-contact frames/releases have prior acceptance. Associated gamepad/contact nodes preserve ownership. | Same process-level validation verifies both output paths and start/update/stop observations. Live USB output/callback evidence remains required. Partial second-node creation has `second_open_failure_rolls_back_gamepad` deterministic coverage; complete live rollback/isolation disposition remains required. |
| Switch Pro: raw report 1/0x10 motor words, implemented subcommand/player acknowledgements and protocol state | Prior controlled UHID evidence includes exact player-command success acknowledgement and encoded rumble observations. Compressed amplitude/frequency physical interpretation remains unvalidated. Evdev conventional rumble is separate. | No compiled USB/audio worker profile exists for this family. Do not invent or infer native USB support from HID evidence. Available realization rejection/documentation must remain explicit. |
| Xbox 360: evdev conventional rumble; generic HID raw observations and host-request/lifecycle representation | Evdev rumble has prior acceptance. The HID realization is a local generic HID identity, not the physical xpad/XInput transport. | The compiled `Xbox360HidEmulated` descriptor implements generic inputs, and its controller output setter is unsupported. Both USB output paths must stall and enqueue no output observation. This is tested with start/update/stop-shaped packets; it is not Xbox USB rumble emulation. |

## Confirmed USB interrupt-output defect and correction

At preceding source `668fc11f0b76dbed12a2f5958c2f763521bee92f`, the production
worker accepted a malformed two-byte DS4 report `[5, 0]` on interrupt OUT with
status zero and actual length two. The identical payload sent as SET_REPORT
returned EPIPE (`0xffffffe0`) and length zero. The interrupt handler called the
permissive native HID observation hook instead of the controller setter policy.
This also acknowledged outputs the Xbox generic HID profile cannot represent.

`crates/gr-audio-worker/src/session.rs` now routes interrupt outputs through
`Protocol::request(RequestKind::Set(...))`, accepts only a successful setter
reply and enqueues its event once. It reuses the controller's policy without
copying report lengths, IDs or codecs into the worker. Native UHID unknown/raw
observation behavior is unchanged. The USB protocol fixture follows the same
policy. No ordinary-root signature or supporting-trait signature changes.
Downstream USB callers now receive EPIPE for malformed/unsupported outputs that
previously received misleading success; valid report observations remain intact.

The lowest-seam Rust regression tests all three compiled families, malformed
IDs/lengths, empty reports, supported Sony reports, unsupported Xbox reports,
setter/interrupt parity, exact events, no duplicates and zero loss. The expanded
production worker validator repeats all three families using descriptor slots
3/4/5 and 64/65/66, verifies both endpoint paths, empty queues after each event,
unsupported output GET rejection and normal PCM/diagnostics/shutdown behavior.
Five Python regressions cover fixture fields, rejection, generation/order and
incorrect acknowledgements. Both validators refuse optimized Python execution
rather than silently disable their existing assertions.

All six production-process cells and all three protocol-fixture cells passed.
The five mandatory repository checks, forced Rust 1.85 check and all 329 Python
tests passed for this increment. Raw before/after receipts remain outside Git.
These results close only the tested process cells, not the whole gate.

## Invalidation and remaining gates

The production worker image changed. All seven provider phases subsequently
passed again at `e5402d24c7ca8e0c209260c879feb205a95f8dae`, with matching
root receipt hashes and restoration. Gate 3 remains passed at that candidate.

The normally built GUI-soak image also changed from its previously accepted
byte hash because the supporting worker crate changed. Do not reuse the old
soak for this new executable. Repeat the separate two-hour neutral soak on the
final byte-pinned GUI image after further relevant changes settle, without
compilation, Steam or competing audio trials. Interactive keyboard/accessibility,
live USB outputs, complete isolation/rollback, routing, Steam and sustained audio
acceptance remain required. No dependencies were added and no loss/latency
threshold was relaxed.

## Kernel attachment output probe

The maintained `usb-functional` phase now requests one HID descriptor per
established session from its root supervisor. It does not grant an input group,
change a node's permissions or expose foreign input devices. The supervisor
checks peer UID and the owned client unit, the root-held lease journal, live
worker image/identity, VHCI port/device, exact compiled generation serial and
HID ancestry. It opens without following symlinks, verifies character-device
numbers and repeats identity checks before transferring the held descriptor.
Descriptors close on failures and teardown; the client runs without privileges.

The ordinary client sends the same synthetic rumble start/update/stop and Sony
indicator/trigger/audio-control reports used by the process validator. It checks
exact raw worker observations, ordering and no duplicates, separately recording
bounded kernel initialization events. Xbox's generic HID USB profile must reject
unsupported outputs rather than fabricate rumble support. This is a kernel-to-
worker observation check; typed application callbacks and full gate closure still
require their own evidence. The short ALSA duplex and cleanup checks remain in
the phase. Tests cover serial/port/generation collisions, ambiguous/foreign nodes,
symlink and regular-file refusal, identity changes during open, descriptor cleanup,
partial writes, duplicate/reordered reports and bounded startup draining.

The bounded interrupt and control-path results are recorded below from
committed packages and independently retrieved root receipts.

## Live interrupt output acceptance

At `15b0f01f45602a1f15d58a0b74a9e8b43312ad5e`, tree
`acf455f3431d8db66383c8435aee7b3375284757`, the bounded `usb-functional`
phase passed for all three compiled families. Its independently retrieved root
receipt pins worker SHA-256
`bf3e795fb1d9759c39898884fb6dcc6b7fc92b5d68955c9029c90815076d0c37`.
DualSense delivered four distinct report-2 payloads and DS4 three report-5
payloads exactly once, in order, through live hidraw writes and worker RPC.
Xbox start/update/stop-shaped writes each rejected with EPIPE (errno 32) and
produced no event. Driver startup outputs were separately recorded before the
synthetic sequence. All three ALSA duplex trials retained 288000 exact playback
markers, zero gaps/corruption and empty initiating/cleanup errors. Original
service/socket state was restored; memory pressure averages remained zero.

The first probe attempt at `c64e1a101dba437a552ff04976d1d80e67c6c085`
failed before descriptor handoff because its port lookup did not parse the
kernel's zero-padded `0000` port. Its cleanup was complete. The corrected parser
and exact-format regression are retained; that failed attempt is not acceptance.

The next probe increment adds Linux HIDIOCSOUTPUT to test the control SET_REPORT
path through the same held descriptor. It uses only the same fixed synthetic
reports, checks exact returned length and one ordered worker observation, and
requires explicit rejection for unsupported Xbox outputs. Its results remain
pending until the new immutable package completes a bounded live run.

## Live USB control-path acceptance

At `e47da258977e58e014d3121eac3d9e3205c6454d`, tree
`56a6fd72ecbb50bac7a529315e46d6ed578587cb`, `usb-functional` passed again.
The worker hash is unchanged from the preceding live interrupt receipt. Each
DualSense report was written on interrupt OUT and HIDIOCSOUTPUT with exact
length 48; each DS4 report returned length 32 on both paths. Every request
produced exactly one matching ordered observation. All six Xbox requests
rejected with EPIPE and no event. Short duplex markers and final ownership
cleanup passed for all three families. This fulfills the previously pending
live USB raw-output paths; it does not claim typed application callback or
sustained audio acceptance.

## Real DS4 partial-construction regression

`live_second_open_failure_removes_observed_gamepad` creates a real Linux uinput
gamepad using the production provider, records its kernel-observed object and
verifies its exact compound label, then injects failure of the second/contact
open. Three repeated attempts each remove the observed first node. The test
runs non-root, requires explicit opt-in, sends no input frames and never creates
a contact node. It removes no device by remembered name; production compound
rollback owns the descriptor. The original deterministic
`second_open_failure_rolls_back_gamepad` regression remains. The new selected
live test passed within its 25-second bound; no other ignored tests were enabled.

The separately pinned live rollback receipt is source `1be7d948e5df6353dda99c832f2d19c25223fa08`, tree `6bbb2381af63c171b67c3c777c37d1da0aee2976`, with unit-test executable SHA-256 `5e97b256bca48cde97fcf3ca6863116c40f6408e8ec6e3415d15eedefcfb0759`. The exact selected test returned zero after all three rollback attempts. Rebuilt production worker and broker images remain byte-identical to the accepted provider candidate, so this test-only increment does not invalidate that functional evidence.

## Root callback pipeline and native observer

The root USB bridge delegates its unchanged one-observation operation to a
private `worker_output` helper. Three deterministic regressions exercise actual
worker IPC framing, this production bookkeeping and the ordinary root handles'
`service()` callbacks: exact DualSense fields across all four audio paths;
DS4 start/update/stop with validity-gated RGB; and rejected/malformed Xbox replies.
They verify order, five idle service cycles without replay, retained observation
counts, terminal errors and repeated close. Ordinary-root signatures are unchanged.

The selected opt-in root unit test can also receive the three unprivileged data
channels from the existing ordinary lab client. The owner remains responsible
for attachment, PCM checks and broker cleanup; the observer drives the actual
root callback pipeline on those live control replies. No privileged device FD
or root execution capability crosses that observer seam. A bounded abstract
socket admits only the matching non-root UID, metadata pins generation/family,
and the existing safe three-descriptor receiver validates handoff. Both Sony
output paths must produce exact typed events; rejected Xbox writes must produce
none. Worker loss diagnostics and post-event idle service are checked. This is
output-pipeline evidence, not new application creation or audio-owner acceptance.

The portable ordinary-user coordinator defaults to a read-only plan:

```sh
python3 scripts/run-alpha-usb-output-acceptance.py
python3 scripts/run-alpha-usb-output-acceptance.py --apply --report /exclusive/external/report
```

Before applying, install the matching committed lab through the scoped updater.
The coordinator requires clean, matching source, prebuilds and freezes the exact
root test executable, then starts only `usb-functional` and the exact selected
observer test. Its separate user unit has a 512 MiB cap and 120-second deadline.
It records executable/source receipts, pins the unit invocation, refuses changed
cleanup identities, preserves failures and requires all three families plus both
root-phase and observer success. No new sudo action, phase name or administrator
provisioning is required. Live callback results remain pending until that
coordinated run terminates and its root receipt is retrieved.

## Typed-observer handoff failure and correction

The coordinated trial at `bd1da3ba6d1bdb4e6717cf78126c6b89400039f7`
failed before callback acceptance; restoration reported no cleanup errors.
Python's timeout-mode socket shared its nonblocking file-description flag with
Rust after descriptor transfer. The Rust control client expects blocking I/O.
The harness now grants exclusive blocking control access to the observer, waits
for its explicit pause before restoring Python's timeout mode for PCM accounting,
and grants blocking access again only after production and drain have finished.
A socket-pair regression checks the actual shared descriptor flags at each step.
This failed trial remains historical evidence, not a successful acceptance cell.

The failure also exposed a root error-retention defect: output service after a
terminal diagnostic failure could overwrite the initiating cause with a later
broken-pipe error. The root bridge now returns the retained terminal cause without
another request or event-count increment. Its regression covers repeated output
service and a separate broker cleanup failure, preserving both causes. Public
signatures and dependencies are unchanged; consumers receive a more accurate
existing `ProviderError::Read` reason. Live acceptance must be rerun after this
correction.
