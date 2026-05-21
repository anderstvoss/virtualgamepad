# virtualgamepad demo

A companion program for the [`virtualgamepad`](../README.md) library. The demo is **not** intended to be embedded into real library users — it exists to drive hands-on testing and visualization as the library is built out.

The demo's growth tracks the library's growth.

## Growth phases

1. **CLI scaffold** (now) — a minimal [`clap`](https://docs.rs/clap)-based binary that prints diagnostic info. No real library use yet; the library exposes no public API.
2. **Diagnostic CLI** — once `gr-core`, `gr-profiles`, and `gr-planner` exist, the demo grows subcommands for listing profiles, inspecting capabilities, and printing planner output. These mirror the developer CLI commands described in [the Rust implementation plan](../docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md#gr-cli) but live separately from `gr-cli` because the demo is for human-facing exploration, not internal diagnostics.
3. **Simulator** — once `gr-session` and a fake backend exist, the demo can spin up a session against the fake backend and play canned input sequences. Useful for reverse-event replay and headless testing.
4. **TUI (optional)** — a terminal-based controller view as a stepping stone if the GUI work needs to wait.
5. **GUI with controller visualizer** — the end state. Real-time visualization of forward input being submitted, reverse commands coming back, planner output, and live diagnostics. Lands once the library is "functional" — meaning the architecture is ready for full device-emulation buildout, regardless of how many device profiles are actually implemented.

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
```

Add `--help` to any subcommand for usage details.

## License

[AGPL-3.0-only](../LICENSE), same as the library.
