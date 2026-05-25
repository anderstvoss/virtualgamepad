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

## Check 3: reverse-output delivery for multiple events

Goal: confirm two distinct rumble reverse events both decode and reach
the output sink, and that the session remains `running` throughout.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- simulate-session samples/scenarios/slow-consumer.yaml
```

2. Confirm:
   - the command exits 0
   - `outputs:` reports at least 2 delivered output commands
   - the diagnostics dump shows `reverse_events.received: 2`,
     `reverse_events.emitted: 2`, no `last_error`
   - the session state remains `running`

### Known gap

This check does not yet exercise the spec's slow-consumer isolation
property — that a slow output callback on one session does not stall a
sibling session. The bounded reverse-event queue + delivery-worker
pattern is in place to provide that isolation, but no fixture exercises
it end-to-end. Adding such a fixture requires a multi-session scenario
plus a "subscribe slow" step in the harness; tracked as Phase 7
follow-up.

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

## Check 5: provider panic isolation

Goal: confirm that a provider failing to open is isolated by the session
manager (no process crash, no hang) and the error surfaces cleanly to the
caller.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- simulate-session samples/scenarios/provider-panic.yaml
```

2. Confirm:
   - the command exits non-zero
   - stderr contains an error message that includes `simulated provider panic`
   - the process exits within a couple of seconds without panicking

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 7 gate passed"
```
