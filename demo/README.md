# virtualgamepad demo

A companion program for the [`virtualgamepad`](../README.md) library. The demo is **not** intended to be embedded into real library users — it exists to drive hands-on testing and visualization as the library is built out.

The demo's growth tracks the library's growth, one phase at a time, per the [Rust implementation plan](../docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md). Each phase ends with a manual gate the user runs via `vgpd-demo phase-gate <N>`.

## Growth phases

The demo grows in lockstep with library phases. Highlights:

- **Phase 0 (CLI scaffold + gate runner)** — shipped: a minimal [`clap`](https://docs.rs/clap)-based CLI, plus the `phase-gate <N>` driver that reads the gate checklist out of the implementation plan.
- **Phase 1 (core domain model gate)** — shipped: `show-types` prints the canonical fidelity/backend/capability names that reviewers confirm during the gate, and those names are snapshot-tested for stability.
- **Phases 1–3 (foundation gates)** — adds `show-types`, `list-profiles`, `show-capabilities`, `validate-config`. Each gate exercises authoring custom YAML fixtures.
- **Phases 4–7 (runtime gates)** — adds `simulate-session`, `replay-trace`, `plan-session`, `many-sessions`. The demo can drive end-to-end fake-backend sessions and surface diagnostics to a human.
- **Phases 8–11 (Linux provider gates)** — adds `run-uinput-smoke`, `run-uhid-smoke`, `run-transport-smoke`. The demo brings up real virtual devices on Linux and prints what host software sees.
- **Phase 12 (cross-platform planner gates)** — `plan-session --host-platform windows|macos` exercises the planner-only stubs.
- **After Phase 12 (GUI graduation)** — the controller visualizer GUI lands: real-time visualization of forward input, reverse commands, planner output, and live diagnostics across active sessions.

The split between `vgpd-demo` (this binary, human-facing) and `gr-cli` (internal/CI, scriptable) is specified in [TESTING_TOOLING_SPEC.md](../docs/spec/implementation/TESTING_TOOLING_SPEC.md#cli-surfaces). The two share backing libraries; `vgpd-demo` is the one humans run at phase gates.

## GUI framework

Not chosen yet. Candidates being tracked:

- [`egui`](https://github.com/emilk/egui) — immediate-mode, integrates with native windowing easily, lowest ceremony
- [`iced`](https://github.com/iced-rs/iced) — Elm-style retained-mode, more structured
- [`slint`](https://github.com/slint-ui/slint) — declarative UI language, more separation of view and logic

Selection happens when the demo actually needs a GUI.

## Non-goals

- The demo is **not** a reusable component. Hosts embedding the library should not depend on `virtual_gamepad_demo`.
- The demo does **not** mirror or test every library code path. The library has its own test suite under `tests/` and (eventually) per-crate test modules. The demo is for human-facing exploration.
- The demo does **not** participate in the library's public API stability story.

## Running

```bash
cargo run -p virtual_gamepad_demo -- info
cargo run -p virtual_gamepad_demo -- show-types
cargo run -p virtual_gamepad_demo -- phase-gate 0
cargo run -p virtual_gamepad_demo -- phase-gate 1
```

Add `--help` to any subcommand for usage details.

## License

[AGPL-3.0-only](../LICENSE), same as the library.
