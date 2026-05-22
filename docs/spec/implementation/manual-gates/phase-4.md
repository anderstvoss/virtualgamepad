# Phase 4 Manual Gate

This guide expands the manual portion of Phase 4 into concrete,
repeatable reviewer steps.

Use it only after the automated Phase 4 checks are green.

Related docs:

- [Rust Implementation Plan](../RUST_IMPLEMENTATION_PLAN.md)
- [Testing Tooling Specification](../TESTING_TOOLING_SPEC.md)

## Before you start

1. Make sure you are on the Phase 4 work branch.
2. Run the automated gate first:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 4
```

3. If the automated checks fail, stop here and fix them before
   continuing.

## Check 1: end-to-end fake session

Goal: confirm the fake backend can accept input, emit reverse output,
and produce reviewer-friendly trace output.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- simulate-session crates/gr-testkit/fixtures/community/fake-session-rumble.yaml
```

2. Confirm the output shows:
   - the scenario id
   - one outbound HID input report
   - one inbound reverse rumble event
   - a clean close

### What to record

- Any output line that feels too low-level or too vague for a reviewer

## Check 2: would-block recovery

Goal: confirm the fake backend surfaces `WouldBlock`, the runner re-arms
via readiness, and the retry succeeds.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- simulate-session crates/gr-testkit/fixtures/community/fake-session-send-would-block.yaml
```

2. Confirm the output shows:
   - `send: would-block`
   - at least one readiness line
   - a recovered send
   - the reverse rumble event afterward

### What to record

- Whether the readiness/recovery story is easy to follow from the text

## Check 3: record then replay

Goal: confirm the recorder writes a stable `backend-trace` fixture and
the replay surface renders the same observable interaction.

### Steps

1. Record a trace:

```bash
cargo run -p gr-cli -- simulate-session crates/gr-testkit/fixtures/community/fake-session-rumble.yaml --record /tmp/fake-session-rumble-trace.yaml
```

2. Replay the recorded trace:

```bash
cargo run -p gr-cli -- replay-trace /tmp/fake-session-rumble-trace.yaml
```

3. Compare the replay output to the original scenario run and confirm
   the same outbound input report and inbound rumble event are shown.

### What to record

- Any surprising differences between recorded and replayed output

## Check 4: malformed reverse event handling

Goal: confirm malformed trace content is logged as a failure step rather
than crashing the replay surface.

### Steps

1. Run:

```bash
cargo run -p gr-cli -- replay-trace crates/gr-testkit/fixtures/community/fake-trace-malformed.yaml
```

2. Confirm the output shows the outbound feature report followed by an
   error line for `drain-reverse-events`.
3. Confirm the command exits successfully instead of panicking.

### What to record

- Whether the malformed-event message is actionable enough

## Check 5: plan/doc/guide consistency

Goal: confirm the new runtime review surface is described consistently
across the plan, this guide, the demo help text, and the assertion
helper failure messages.

### Steps

1. Run `cargo run -p virtual_gamepad_demo -- --help`.
2. Confirm `simulate-session` and `replay-trace` are listed.
3. Review `crates/gr-testkit/src/assertions/snapshots/` — assertion-helper
   failure messages are human-readable and stable (one snapshot per
   helper: `assert_captured_frames`, `assert_trace_directions`,
   `assert_diagnostics_counters`).
4. Review:
   - `docs/spec/implementation/RUST_IMPLEMENTATION_PLAN.md`
   - `docs/spec/implementation/manual-gates/phase-4.md`
   - `demo/README.md`
5. Confirm the Phase 4 commands and fixture paths align across those
   surfaces.

### What to record

- Any wording or command drift between the docs and the implementation
- Any assertion-helper message that would confuse a reviewer reading it cold
