# Phase 6 Manual Gate

Phase 6 covers translator behavior, descriptor-backed HID report shaping,
reverse-event decoding, and the reviewer-facing replay surface.

Run the automated portion first:

```bash
cargo test --workspace --all-features
cargo insta test --check
cargo run -p gr-cli -- capability-coverage
cargo run -p virtual_gamepad_demo -- phase-gate 6
```

Manual checklist:

1. Run `cargo run -p virtual_gamepad_demo -- replay-trace crates/gr-translators/fixtures/dualsense-buttons-roundtrip.yaml`.
   Confirm the replay output shows the raw report and a decoded DualSense summary with the expected dpad/button state.
2. Run `cargo run -p virtual_gamepad_demo -- replay-trace crates/gr-translators/fixtures/dualsense-rumble-from-host.yaml`.
   Confirm the replay output decodes the host report into rumble plus any populated lighting/player-indicator or trigger-effect commands.
3. Run `cargo run -p virtual_gamepad_demo -- replay-trace crates/gr-translators/fixtures/steam-controller-lighting.yaml`.
   Confirm the replay output shows both the Steam Controller HID summary and the decoded lighting command.
4. Run `cargo run -p gr-cli -- capability-coverage`.
   Confirm the output has `gaps: []`.
5. Review the updated snapshots in `crates/gr-profiles/src/snapshots/` and `crates/gr-cli/src/snapshots/`.
   Confirm the new descriptor bytes and replay output are stable and readable.

Sign off with:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 6 gate passed"
```
