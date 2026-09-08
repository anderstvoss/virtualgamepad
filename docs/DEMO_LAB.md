# Manual lab work with the demo

Run `cargo run -p virtualgamepad-demo` as the ordinary user. First run
`python3 scripts/host-preflight.py all` for a read-only prerequisite inventory;
its unvalidated consumer checks are not failures that broader permissions fix.
The demo does not install prerequisites or alter permissions.

## Controls and observations

Choose the controller family and realization, enter a name and application
session ID, then create it. Disable **Advance ID after creation** to create
multiple devices with the same application ID; their kernel identities remain
creation-owned. IDs wrap after the maximum u64 value. This is an application-ID
stress tool, not a physical-identity override.

Select any controller to exercise its existing button, axis, motion, battery and
touch controls where supported. The lab panel shows its realization, session ID,
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
the lab summary itself does not establish cleanup. Busy controllers briefly hide
their editing controls instead of blocking the UI or being treated as failures.
The workers still share controller mutexes with short UI edits; this is not a
hard realtime or lock-free design.

**Check broker socket** runs only when clicked. A reachable socket is merely
connectivity evidence; it does not validate Gate G or authorize gadget resources.

## Safe experiment order and next gates

1. Begin with input-only neutral/buttons/sticks and typed output observation on
   the measured UHID profile. Record virtual observations separately from the
   physical DualSense, Xbox Series or Steam Controller reference. Xbox Series is
   not an Xbox 360 fidelity reference; record the Steam Controller generation.
2. For concurrent-session tests, reuse an application ID, remove a middle device,
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
and [gate ledger](architecture-overhaul/GATE_STATUS.md). This GUI increment was
validated with fake controllers and compilation, not a live desktop run.
