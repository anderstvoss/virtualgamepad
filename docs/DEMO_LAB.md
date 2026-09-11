# Manual lab work with the demo

Run `cargo run -p virtualgamepad-demo` as the ordinary user. First run
`python3 scripts/host-preflight.py all` for a read-only prerequisite inventory;
its unvalidated consumer checks are not failures that broader permissions fix.
The demo does not install prerequisites or alter permissions.

## Controls and observations

Choose the controller family and realization, optionally enter a name, then
create it. **Lab notes and gate prerequisites** contains a **Lab correlation ID**
for grouping observations. Disable **Advance ID after creation** to reuse that
label. Lab labels wrap after the maximum u64 value; library creation tokens do
not wrap. Reusing a label does not reuse controller identity or session ownership.

Select any controller to exercise its existing button, axis, motion, battery and
touch controls where supported. The lab panel shows its realization, lab correlation ID,
worker service-cycle count, maximum observed service gap and omitted worker log
count. These are process observations, not end-to-end latency or consumer passes.
Optional log loss includes worker backlog eviction and busy display snapshots;
the global on-screen log also retains only its newest entries.

Enter the reference model, firmware if known, USB/BT connection mode, consumer
version, experiment condition and result in **Lab notes and gate prerequisites**.
**Copy lab record** copies a versioned plain-text summary for the selected
controller. Paste it into a private experiment record; nothing is automatically
written, uploaded or added to the corpus. Typed reverse output and indicator
snapshots remain visible separately.

Remove arbitrary controllers to test independence, or use **Stop all controllers**
to stop/join workers and close all sessions. Verify consumer-side removal too;
the lab summary itself does not establish cleanup. Editing controls briefly wait
for the previous input batch's applied snapshot. Inputs during that wait are not
accepted; accepted press/release batches remain ordered. A full/disconnected queue
or rejected native edit closes the affected controller and reports the failure,
rather than dropping a release silently. Worker-owned controllers keep servicing
while GUI drawing or optional display consumption stalls. This is not a hard
realtime guarantee. Live GUI/consumer acceptance remains separate from fake-worker
tests; do not repeat touch injection on the active desktop.

Gadget selection is disabled in this application demo. The compiled gadget
profiles remain experimental research interfaces pending Gate G; a reachable
broker socket does not establish request handling or controller support.

## Safe experiment order and next gates

1. Begin with input-only neutral/buttons/sticks and typed output observation on
   the measured UHID profile. Record virtual observations separately from the
   physical DualSense, Xbox Series or Steam Controller reference. Xbox Series is
   not an Xbox 360 fidelity reference; record the Steam Controller generation.
2. For concurrent-session tests, reuse a lab correlation ID, remove a middle device,
   and observe continuing service on the others. Keep each consumer selection exact.
3. Touch injection needs an isolated consumer environment. Do not use this active
   desktop to repeat EXP-0012. A separate VM clone with no desktop consumer of the
   test nodes is the current resumption path; headless SDL alone is insufficient.
4. Gate G needs administrator-reserved gadget resources and a supported request
   interface before generic broker replacement. The current read-only inventory
   still lacks ConfigFS gadget availability, UDC authorization and broker socket.
   Keep compiled profiles; permissions cannot add missing GET/SET metadata.
5. Gate F can start with a normal-user audio lifecycle prototype after compound
   ownership is scoped; it does not require a physical controller. Gate H needs
   physical DualSense audio topology facts, L needs physical BT fixtures, and M
   follows L with isolated radio setup. Do not create dependent production IDs
   before those gates pass.

Other families remain best-effort. See the [physical validation policy](architecture-overhaul/PHYSICAL_VALIDATION_POLICY.md)
and [gate ledger](architecture-overhaul/GATE_STATUS.md). Deterministic worker tests
and selected live UHID lifecycle checks have passed; interactive controller and
consumer acceptance remains pending. See the
[current refinement evidence](CURRENT_CONTROLLER_REFINEMENT.md).

Face buttons show printed labels and spatial positions (Nintendo B is South).
Stick pads now reach both signed endpoints, and Sony unsigned conversions preserve
0/128/255. For mapping investigations, test one control at a time and record its
observed consumer label/direction; a sweep pressing every button cannot establish
that each individual mapping is correct. See
[EXP-0013](architecture-overhaul/experiments/EXP-0013-mapping-audit.md) for the latest
source-backed trigger/contact and range corrections.

## Identity restoration experiments

The demo currently uses the existing fresh-per-creation constructors. It does not
exercise the optional Sony identity-restoration API. An embedding test can generate
and save a `DualSenseIdentity` or `DualShock4Identity`, create with the corresponding
`create_*_with_identity` function, close completely, then recreate with the same
identity. Record actual removal, fresh physical-path identity, stable pairing/uniq
and consumer association separately. Use a different identity for each concurrently
connected logical controller. No physical DualSense is required for deterministic
session tests. The root UHID identity-restoration test has passed on the prepared
host; consumer reconnect/association behavior still requires hands-on evidence.


### Comparing Eden and Steam mappings

Record the exact consumer build and input backend separately. Eden nightly SDL
reports working DS4/Switch HID gyro, while user testing found Steam-only Sony
axis routing and Switch neutral offsets; the cause remains unestablished. Test
one axis at a time at neutral and both endpoints, with the exact realization,
selected identity, mapping string and any per-game override recorded. Keep
consumer settings unchanged during the comparison. Do not compensate in the demo
for one consumer before checking raw Linux and SDL observations.

The Xbox standard-HID descriptor now uses the same eleven Linux button codes as
its evdev profile. Recreate existing virtual devices after rebuilding: descriptors
are installed at creation. West is X and North is Y; legacy Linux BTN_X is
numerically also named BTN_NORTH, so use observed printed/spatial action rather
than that alias alone. See EXP-0016 for the old HID defect and retest scope.

## Release and correlation records

**Release all inputs** queues one native `neutralize()` edit through the same
acknowledged worker queue, then commits it. It releases contacts too, but preserves
battery metadata, identity, protocol state and host-owned rumble/LED state. The
button waits for the previous accepted edit; it cannot overtake a press. The UI
skips other control edits that frame so stale touch state is not reasserted.

Lab record v3 separates the lab correlation ID from library creation identity.
It includes consumer build, backend and mapping fields, plus component roles,
requested creation labels and any cached host observations.
A cached path is historical: verify identity and ancestry after re-enumeration.
After removal, **Copy cleanup diagnostics** preserves the last returned controller's
terminal state and any cleanup error. A worker that returns no controller is
reported as requiring host verification. This is not proof that every kernel
node was removed; verify that independently. Nothing is automatically saved.
