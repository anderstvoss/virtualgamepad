# Brief 10: root facade, crate tidy and topology-driven demo (cleanup steps 2, 3, 5)

Read `ground-rules.md` first.

**Lane:** Core (serial, after brief 02). **Blocked by:** D1 and brief 02 merged. Step 3's dummy_hcd part needs **D5**. Step 5 benefits from brief 05c's kinds but does not wait for them.
**Needed by:** cheaper root wiring for each new family (step 2) and a demo that needs no per-family GUI code (step 5). Not a hard prerequisite for controller packages.

## Step 2. Thin per-family root facade

- A crate-private generic `Controller<F>` in the root crate owns the curated session, optional `ControllerAudio`, association and identity, with one `reap_audio_failure()` used by all mutators.
- Each public type (`DualSenseController`, etc.) stays a newtype; shared forwarding methods come from a `macro_rules!`; only family-specific methods are hand-written.
- Doc comments stay identical (the snapshot includes docs). `src/controllers.rs` (1,254 lines today) should land near 400. `src/controllers/tests.rs` passes unchanged.

## Step 3. Crate tidy

- Move `src/audio.rs`, `src/audio/`, `src/usb_audio.rs`, `src/usb_audio_bridge.rs` into a new `gr-root-audio` crate; the root re-exports the same names behind the same features.
- Fold `gr-dualsense-wire` into `gr-controller-wire`, keeping `gr_controller_wire::dualsense` as the path.
- (`.gitignore` for `__pycache__` already landed in brief 01b.)
- **dummy_hcd, only if D5 says park:** create `archive/dummy-hcd` from current `main`, then remove `gr-provider-linux-dummy-hcd`, `gr-privileged-broker/src/dummy_hcd.rs`, the per-family `DUMMY_HCD_*` surfaces and restrictions, and the docs rows. `RealizationId::LINUX_DUMMY_HCD_USB_HID` stays (it is in the snapshot) and creation keeps rejecting it. If D5 is "keep", skip this part.

## Step 5. Demo draws from topology

- `demo/src/gui/editor.rs` and `input_clusters.rs` render controls from `InputTopology` and the registry (and from live topology once brief 06b lands).
- Remove per-family branches in `demo/src/gui.rs` except genuinely family-specific panels (DualSense adaptive triggers), driven by declared outputs.
- Keep the GUI lifecycle tests; add a test that every registry family produces an editor.

## Done when

Each step merged as its own PR with #176 ticked.
