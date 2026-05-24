# Phase 7 Manual Gate

This guide is the reviewer checklist for Phase 7 (`gr-session`,
`gr-host-bridge`). It covers the fake-backend-backed session runtime,
reverse-output delivery, diagnostics, and the new multi-session demo
surface.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 7
```

## Check 1: coalesced input diagnostics

Goal: confirm the runtime coalesces stale queued input and surfaces the
counter in diagnostics output.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- simulate-session samples/scenarios/dualsense-coalesce.yaml
```

2. Confirm the output includes:
   - `mode: runtime-session`
   - `frames_written:`
   - a diagnostics dump with `frames.coalesced` at least `1`

## Check 2: concurrent fake sessions

Goal: confirm many fake-backed sessions can run concurrently and a
closed session does not take down unrelated ones.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- many-sessions 32
```

2. Confirm the output lists multiple running sessions and does not fail
when the first session is closed.

## Check 3: reverse-output delivery

Goal: confirm reverse commands are delivered through the session runtime
and surfaced in scenario output.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- simulate-session samples/scenarios/slow-consumer.yaml
```

2. Confirm the output includes at least one delivered reverse command
and the session remains in `running` state during the scenario.

## Check 4: discrete audio command path

Goal: confirm the discrete audio command path is wired even though the
fake backend does not expose PCM streams.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- simulate-session samples/scenarios/dualsense-audio-mode.yaml
```

2. Confirm the output includes:
   - at least one delivered output command
   - `audio_sink: none`

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 7 gate passed"
```
