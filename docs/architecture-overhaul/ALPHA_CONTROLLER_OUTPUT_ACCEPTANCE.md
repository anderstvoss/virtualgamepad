# Controller output acceptance ledger

Gate 2 passes for the implemented, representable transports after the current
closure audit below. Historical pending statements describe their earlier
checkpoint; the closure matrix supersedes them. Physical-controller comparisons
remain post-alpha issue #136, and sustained audio remains separately gated.
A process socket fixture alone is not kernel-consumer acceptance.
See the canonical [gate criteria](ALPHA_GATE_CRITERIA.md).

| Family and implemented fields | HID / evdev disposition | USB disposition |
| --- | --- | --- |
| DualSense: validity flags, motors, raw trigger fields, mute/microphone controls, four audio paths, volumes/preamp, player indicators and RGB | Exact live UHID typed outputs and ordered live evdev conventional feedback pass. Generic evdev cannot encode Sony-native fields; target restrictions and their regressions state the reason. | Twelve requests on both endpoint paths pass exact typed callbacks, including disabled validity fields and valid_flag2-only motor enabling. PCM remains separately failed. |
| DS4: motors, validity-gated RGB and raw retention | Exact live UHID values and live evdev feedback pass. Two owned nodes, contacts/releases, partial second-node rollback and sibling survival pass. Evdev RGB limitation is explicit and tested. | Six requests on both endpoint paths pass exact typed callbacks, including RGB-validity disabled during motor update. |
| Switch Pro: raw motor words, player/subcommand acknowledgements and protocol state | Exact live encoded reports, player acknowledgement and all six implemented USB-handshake replies pass through UHID; ordered evdev feedback and sibling survival pass. Physical amplitude/frequency interpretation is deferred. | No compiled USB/audio profile exists. Unsupported gadget realizations reject before resources are created; no fabricated attachment or native USB output claim. |
| Xbox 360: evdev conventional rumble; generic HID raw/host-request/lifecycle representation | Ordered live evdev feedback and sibling survival pass. Standard HID has no output capability; surface/unsupported-operation regressions retain that limitation. | All six generic-HID USB output requests reject with EPIPE and enqueue no callback. This does not implement XInput/xpad outputs. |

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

The next trial at `119cfe9d9a3de19933c230271f22eb3e0dc28855`
completed the first family's callback sequence but failed while waiting for the
PCM phase: the shared descriptor-receive helper had replaced the observer's
30-second phase deadline with a one-second handoff deadline. Its wrapper now
restores the caller's bounded deadline after successful descriptor receipt. A
real three-descriptor socket handoff regression checks the restored deadline.
Cleanup again reported no errors. This second failure is retained separately.

At `ebbaa6b9df405ef36da406256343fcbcddbdcac2`, all eight DualSense
kernel output requests reached the typed root callbacks exactly once. The
independent microphone trial then failed its unchanged marker/silence assertion
(8928 measured silence frames; producer wake intervals exceeded the 8 ms fill).
The observer completed the first family cleanly, but the client's fail-fast exit
prevented other families from running. The runner now continues independent
output cells only after complete typed observation, no initiating error and
successful cleanup; the original PCM result stays failed and the process still
exits nonzero after collecting all families. This is separate cell evidence,
not permission to call audio or the complete USB phase passed.

## Typed USB output cell acceptance and separate PCM failure

At source `9e5731e72818fe722b6c55d04b44c0f51a4b3865`, tree
`15f4c9e4e0bc8d23fa1929cad8842da8afe5d9d0`, the independently frozen
ordinary observer SHA-256 was
`06643664ea4f9ea93a35386b3c118828f45ebc5fbc19e051abeeb64e6b05e21f`.
Its selected live test passed all three families: eight exact ordered DualSense
callbacks, six exact ordered DS4 callbacks and six Xbox EPIPE rejections with
no callback. Both kernel endpoint paths, five idle service cycles after each
request, zero dropped outputs and repeated close were checked. The root client
receipt independently records typed callback completion for each family.

These Gate 2 output cells pass; the combined `usb-functional` phase and
coordinator deliberately remain **failed**. The separate unchanged PCM marker
assertions measured 7936/9376/10672 silence frames for DualSense/DS4/Xbox,
respectively. Producer wake intervals reached approximately 10 ms against the
8 ms operating fill. Playback delivered all 288000 markers per family with
zero gaps/corruption; microphone conservation reconciled without queue drops.
This narrows the microphone failure to producer refill starvation evidence,
not a demonstrated correction or permission to increase queues/relax limits.

Every family reports no initiating exception and no cleanup error. The root
supervisor and ordinary unit cleanup also report no errors; original broker
service inactive/socket active state was rechecked. Observer peak memory was
1.7 MiB, with roughly 18 GiB host memory available and zero measured memory
pressure. Broker and worker hashes remain identical to the accepted Gate 3
images. Raw failures and full receipts remain outside tracked source.

The five mandatory checks, forced Rust 1.85 workspace/all-target/all-feature
check and 349 Python tests pass. No dependencies or ordinary-root signatures
changed. Gate 2's remaining realization/lifecycle dispositions, final-image GUI
acceptance, routing, Steam and sustained continuity remain open. Review the
shared socket flag ownership/pause protocol, bounded receive deadlines and
retention of the original terminal error separately from cleanup failures.

## Gate 2 closure audit

The audit was completed at `adf82fd58c77df2c8fa929e3d95ecd58c3b9bd79`.
It combines independent cells rather than converting failed PCM into success.

| Requirement | Authoritative evidence and disposition |
| --- | --- |
| All representable HID outputs and required service replies | Fresh selected UHID tests at `59e5b14ff791522858c417cb79330dd0016c6547`: DualSense exact native fields, DS4 motors/RGB validity, Switch motor reports/player reply and commands 1–6. All return one passed test with exact replies, idle no-replay and repeated cleanup. HID test image `fac45f5f839b2c925e12328e8df19141e653dd1788f602592774e7eadc3ae328` is byte-identical to the earlier accepted image. Declared report success/error and unsupported-operation regressions remain passing. |
| Ordinary root lifecycle, removal/recreation and sibling isolation | Fresh four-family root UHID lifecycle/sibling test plus identity restoration pass at `59e5b14`. Image `7a1a055ccf2d46e1e3d9e8f0a33acb57fb9b0f72c44ded2f6bc47a8c96993103`. Start callbacks arrive through public service; removal affects only the selected object; a sibling continues service; recreated identities differ; repeated close leaves no owned node or retained error. |
| Evdev start/update/stop, retained effects, exact ordering and consumer death | At `3be7cb28b65be451f57cbf0cd81fc7bddc74af65`, image `b720e4bb15739bed5cf1b9a0a89c7c9261d989b6155a29dde0be09d5047d8b0f`, both selected live suites pass all four families. Each surviving sibling completes six uploads, three starts, six stop observations and three erases in order after its peer is removed. DS4 removes/preserves both corresponding nodes. Consumer death, repeated cleanup and descriptor physical-identity checks also pass. |
| Sony contacts, frame splitting and release | The immutable administrator-installed `1e07adf04d1fdd3bcb7451256f7f86fc7f0b8991` archive was independently retrieved during the current rejection phase. Its manifest and report both pin that source; the actual image matches manifest SHA `9c191fe74974f4533f9c838d2fa1bc79db43e9bb88979b78c9b6e168c64a6dbc`; report SHA is `b46a4ea48eea12c7cdcc8acb8bbadc1c54bdb866eccaffef78550c35dbe13c7f`. All six exact input tests pass, including both HID and evdev two-contact paths and exact evdev frames/releases. Each isolation receipt records no initiating/cleanup error and rule removal. Relevant production controller, HID, runtime and provider code is unchanged; the only later DS4 evdev diff is the additional test below. These accepted cells are reused, not represented as new live runs. |
| DS4 partial second-node failure | The real three-repeat rollback test at `1be7d948e5df6353dda99c832f2d19c25223fa08` removes every observed gamepad after injected contact-open failure. Its deterministic rollback regression remains passing; no contact input is injected. |
| USB output parity, flags and public callbacks | At `59e5b14`, frozen observer `1e109d87cc773b3037e86a834969f105d93d6cd63971947a514d6c811356bafc` passes twelve DualSense, six DS4 and six rejected Xbox requests, both endpoint paths, exact typed observations, idle no-replay, zero output drops and repeated close. Six production-worker process cells also pass. All family, observer and supervisor cleanup records are empty. The combined phase remains failed solely for its separate PCM assertions. |
| Technically unavailable realizations/features | Current rejection phase passes with the unchanged candidate images. Gadget operations reject before allocation; Switch has no USB/audio worker profile; Xbox generic HID output rejects explicitly; evdev native Sony output limitations are present in surfaces and deterministic regressions. No unsupported feature is silently promoted. |

The first neutral-run command used a nonexistent Switch handshake test name.
Its zero-test output was explicitly rejected as acceptance; the correct exact
name subsequently ran and passed. Both receipts are retained. No child entrypoint
or full ignored-test set was blanket enabled.

The fixed historical reader has no caller-selected path or command. It checks
root-owned ancestry, exact report filename shape, bounded reads and held file
identities; unavailable history stays unknown. It does not execute archived code,
change privileges, start old services or claim a historical run occurred today.
Two deterministic regressions cover capture, path traversal, quotas and changed
identities. Five mandatory checks, forced Rust 1.85 and 351 Python tests pass.
No dependencies or public API signatures changed. The current broker/worker
images remain byte-identical to the accepted provider candidate.

This closes Gate 2 only. USB/PipeWire routing, sustained audio, full GUI keyboard
acceptance/final-image soak and Steam remain required. Microphone PCM failures
and the old failed probes remain visible; alpha is still not ready.
