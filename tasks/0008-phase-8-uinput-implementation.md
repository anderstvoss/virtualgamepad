# Task 0008: Phase 8 — Linux `uinput` provider (Compatibility tier)

## Goal

Land the real Linux `gr-provider-linux-uinput` implementation against
the contract surface that prep PR `3602b9a` ("docs(phase-8-prep): land
uinput prep contracts and smoke/report surfaces") already shipped.
Result: a fake-free path from the runtime session engine through to a
host-visible Linux gamepad on `/dev/uinput`, plus an `EV_FF` reverse
path that surfaces as `OutputCommand::Rumble`.

This task closes out Phase 8 as described in
[docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md:648-708](docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md#L648).

## Entry state (verify before starting)

- On `main` past commit `26904fb` (Phase 7 merged).
- Phase 7 manual gate signed off (`e840375`).
- `cargo test --workspace --all-features` clean (228 tests as of this writing).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- `cargo fmt --all -- --check` clean.
- Phase 8 prep surface in place (see Background).
- Developer host on Linux with `/dev/uinput` reachable (root, or `input` group via udev rule).

## Background — what already exists

The Phase 8 *prep* contracts shipped in commit `3602b9a`. Do **not**
redo any of this; build on it.

- [crates/gr-provider-linux-uinput/src/lib.rs](crates/gr-provider-linux-uinput/src/lib.rs):
  - `LinuxKernelIoctl` trait + `DeferredLinuxKernelIoctl` placeholder
  - `LinuxUinputBackendFactory` with `can_realize` (forward Full, reverse Partial/None as appropriate)
  - `LinuxUinputBackendSession` stub (`open`/`send`/`close` advance state, `drain_reverse_events` returns `WouldBlock`, `readiness` returns `NoReverseEvents`)
  - `LinuxUinputSmokeReport` data type
  - Crate `#![forbid(unsafe_code)]`
- [crates/gr-cli/src/lib.rs:127](crates/gr-cli/src/lib.rs#L127) `run_uinput_smoke`, [`support_report`](crates/gr-cli/src/lib.rs#L141) with insta snapshots
- [demo/src/main.rs:40](demo/src/main.rs#L40) `RunUinputSmoke` + `SupportReport` subcommands
- [samples/inventories/linux-uinput-only.yaml](samples/inventories/linux-uinput-only.yaml)
- [.github/workflows/provider-tier-b.yml](.github/workflows/provider-tier-b.yml) workflow scaffold (currently only a non-privileged contract-surface job)

## Scope

Recommended two-PR sequence; agent may bundle if the diff stays reviewable.

### PR 1 — `docs/phase-8-prep-followup` (small, no `unsafe`, no kernel I/O)

1. **Phase-gate runner wiring**
   - In [crates/gr-cli/src/lib.rs](crates/gr-cli/src/lib.rs): add `PHASE_8_COMMANDS` (modeled on `PHASE_7_COMMANDS` at line 116). Include at minimum `cargo test --workspace --all-features` and `cargo insta test --check`.
   - In `phase_gate_commands` at [crates/gr-cli/src/lib.rs:1389](crates/gr-cli/src/lib.rs#L1389): replace the `8..=12 => Err(UnimplementedPhase)` arm with an explicit `8 => Ok(PHASE_8_COMMANDS…)` branch; keep `9..=12 => Err(UnimplementedPhase)`.
   - In [demo/src/phase_gate.rs:195](demo/src/phase_gate.rs#L195) `automated_item_status`: add matcher arms for any new Phase 8 plan rows that aren't a literal backticked command (mirror the "concurrent test passes" arm from Phase 7).

2. **Manual gate doc**
   - Create [docs/spec/implementation/manual-gates/phase-8.md](docs/spec/implementation/manual-gates/phase-8.md) following the shape of the Phase 7 gate doc. Source the six checks verbatim from [RUST_IMPLEMENTATION_PLAN.md:701-706](docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md#L701):
     1. `vgpd-demo run-uinput-smoke generic-gamepad` creates a device; `evtest`/`jstest` finds it under `/dev/input/`
     2. `evtest` shows expected buttons/axes; emitted presses match
     3. `vgpd-demo run-uinput-smoke xbox360` produces a controller SDL recognizes
     4. Native Linux SDL game or `jstest-gtk` receives scripted inputs
     5. `fftest` / game-triggered EV_FF rumble surfaces as `OutputCommand::Rumble` in verbose demo output
     6. Killing the demo removes the device cleanly (no zombie `event*` entries)

3. **Host setup sample**
   - Add `samples/setup/99-virtualgamepad-uinput.rules` (udev rule granting `input` group `0660` on `/dev/uinput`) plus a `samples/setup/README.md` explaining installation. Out of repo scope: actually installing the rule.

4. **Workflow expansion**
   - Update [.github/workflows/provider-tier-b.yml](.github/workflows/provider-tier-b.yml) to run the existing contract-surface tests against the Phase 8 implementation surface on every dispatch. The privileged `/dev/uinput` job is still gated to a self-hosted runner — leave it stubbed with a clear `if: false` + comment until that runner exists.

### PR 2 — `phase-8-implementation` (the real work)

5. **`unsafe` wrapper module**
   - Create one module (e.g. `crates/gr-provider-linux-uinput/src/kernel.rs`) that *deletes* the crate-level `#![forbid(unsafe_code)]` and instead applies it at module scope to every *other* module. The unsafe module exposes a safe API and contains all `libc::ioctl` and `write`/`read` calls. Document each unsafe block's invariants in `// SAFETY:` comments.
   - Prefer `nix` for the ioctl macros if it stays inside that one module. Don't propagate `nix` types past the module boundary.

6. **RAII fd type**
   - Owned wrapper around `OwnedFd` (or a newtype over `RawFd` with explicit `Drop`) that issues `UI_DEV_DESTROY` followed by `close` on drop. Lives inside the unsafe module.

7. **`LinuxKernelIoctl` real impl**
   - Implement `LiveLinuxKernelIoctl` (name TBD) that drives the unsafe wrapper. `LinuxUinputBackendFactory::default()` should install this when `target_os = "linux"`; the `DeferredLinuxKernelIoctl` stays for tests + non-Linux builds.

8. **Forward path**
   - `LinuxUinputBackendSession::open`: actually open `/dev/uinput`, declare capabilities via the `UI_SET_EVBIT` / `UI_SET_KEYBIT` / `UI_SET_ABSBIT` / `UI_SET_FFBIT` ladder, then `UI_DEV_CREATE`.
   - Capability declaration must come from the planned `BackendOpenContext` / profile capability set, not hardcoded. The forward translator for the evdev level already produces `BackendFrame::EvdevEvents { events }`; consume those events and write them with `EV_SYN/SYN_REPORT` terminators.
   - **Verify** before implementation: trace whether the existing dualsense/xbox360 forward translators in [crates/gr-translators/src/lib.rs](crates/gr-translators/src/lib.rs) cover `BackendLevel::Evdev`. If they don't, add or extend a translator — the planner is configured to route uinput through `BackendLevel::Evdev`.

9. **Reverse path — EV_FF**
   - `readiness()` polls (or uses non-blocking read) on the uinput fd; returns `Readable` when an `input_event` is pending.
   - `drain_reverse_events()` decodes the `UI_BEGIN_FF_UPLOAD` / `UI_END_FF_UPLOAD` ioctl handshake (the kernel issues an `EV_UINPUT` `input_event` carrying a request id; userspace must read the effect with `UI_BEGIN_FF_UPLOAD`, complete with `UI_END_FF_UPLOAD` carrying `retval`). Surface the effect as a new `BackendReversePayload` variant (or reuse `Hid`-shaped bytes if the dualsense reverse translator can decode it; review needed). Handle `UI_BEGIN_FF_ERASE` / `UI_END_FF_ERASE` symmetrically.
   - Translator wiring: either extend an existing reverse translator to recognize the new payload or add a uinput-specific one. The output must map to `OutputCommand::Rumble` with the strong/weak magnitudes the runtime expects.

10. **Tests**
    - Unit tests against a fake `LinuxKernelIoctl` substitute (no kernel access) covering: ioctl sequencing for `generic-gamepad` and `xbox360`, descriptor construction, capability bits emitted, event-write batching, EV_FF protocol state machine.
    - Linux-gated integration tests (`#[cfg(target_os = "linux")]` + `#[ignore]` unless an env var like `VGPD_UINPUT_TESTS=1` is set, to keep them out of the default CI matrix): real `/dev/uinput` open, device-visible-under-`/dev/input/`, capability query matches, event flow, EV_FF round-trip, clean teardown.

11. **Snapshot updates**
    - The existing `run_uinput_smoke_generic_gamepad` and `support_report_generic_gamepad_compatibility` snapshots pin the prep-surface output. After hooking the live `LinuxKernelIoctl`, those `planned_ioctl_sequence` strings will likely change — regenerate the snapshots only after reviewing the new wording; do not blindly accept.

## Non-goals

- No identity-aware tier work — that's Phase 9 (`gr-provider-linux-uhid`).
- No `gr-provider-linux-transport` changes — separate later phase.
- No new `unsafe` outside the single wrapper module.
- No changes to `docs/spec/specs/*` (the design spec). All wording changes belong in `docs/spec/implementation/*`.
- No `chore(phase-gate): Phase 8 gate passed` commit — the user authors that manually after walking the gate (see `feedback_phase_gate_signoff_last.md`).

## Acceptance criteria

- Workspace tests pass on Linux, macOS, and Windows CI runners (the `unsafe` module and uinput-specific tests must be `cfg(target_os = "linux")` so non-Linux builds stay clean).
- `cargo run -p virtual_gamepad_demo -- run-uinput-smoke generic-gamepad` creates a real device on a Linux host with `/dev/uinput` access; `evtest` enumerates it.
- `cargo run -p virtual_gamepad_demo -- phase-gate 8` exits 0 (all automated rows ✓).
- Forward inputs flow into `jstest-gtk` / `sdl2-test`.
- EV_FF rumble triggered from `fftest` surfaces in the demo's verbose output as `OutputCommand::Rumble`.
- Killing the demo removes the `event*` node within a couple of seconds (no zombies on subsequent runs).
- All Phase 8 manual-gate items walk cleanly per `docs/spec/implementation/manual-gates/phase-8.md`.
- `cargo deny`, `cargo audit`, `gitleaks` clean (pre-commit / pre-push hooks already enforce this).

## Validation

```bash
# baseline
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Linux-only privileged tests (skipped by default)
VGPD_UINPUT_TESTS=1 cargo test -p gr-provider-linux-uinput --all-features -- --ignored

# manual gate
cargo run -p virtual_gamepad_demo -- phase-gate 8
cargo run -p virtual_gamepad_demo -- run-uinput-smoke generic-gamepad
cargo run -p virtual_gamepad_demo -- run-uinput-smoke xbox360

# external tools to verify on a Linux dev box:
sudo evtest        # confirm device visibility + button/axis press events
sudo fftest        # confirm EV_FF rumble round-trip
jstest-gtk         # confirm SDL recognition
```

## Risk callouts

- **EV_FF protocol is semi-asynchronous in a way the current `BackendSession` reverse-event model hasn't exercised.** Kernel hands userspace an `EV_UINPUT` event carrying a request id; userspace must call back into the kernel via `UI_BEGIN_FF_UPLOAD` / `UI_END_FF_UPLOAD` ioctls. Design this on paper before writing the unsafe wrapper — sketch the state machine, the buffering between the actor's poll-loop and the kernel ioctl callback, and where the new payload variant slots into `BackendReversePayload`.
- **`/dev/uinput` permissions.** Without the udev rule from PR 1 step 3, the manual gate requires `sudo` for every demo invocation. Install the rule before walking the gate.
- **Capability declaration ordering.** `UI_SET_EVBIT` must precede the per-bit `UI_SET_KEYBIT` / `UI_SET_ABSBIT` ioctls; `UI_DEV_CREATE` must come last. Failure modes are silent (device created but missing buttons), so make this a contract test against the fake `LinuxKernelIoctl`.
- **Worker pool**. The session runtime defaults to 64 tokio workers ([crates/gr-session/src/lib.rs:1149](crates/gr-session/src/lib.rs#L1149) `build_runtime`). If `LinuxUinputBackendSession::send` blocks on `write(2)` to a slow consumer, the worker is held — that's expected behavior, but verify it doesn't cause the Phase 7 backpressure semantics to misfire under load.

## Required agent output

When done, report:

- Files changed (with line-count summary)
- New dependencies added (likely `nix` or `libc`; justify)
- Commands run + pass/fail
- The exact `evtest` / `fftest` output captured during the manual gate walk (paste into the PR description so reviewers can reproduce)
- Any deviations from this task spec, with rationale
- Updated `feedback_*.md` memories if the work surfaced any new project-wide patterns
- Open follow-ups for Phase 9 (UHID) if any contract changes here affect the identity-aware tier
