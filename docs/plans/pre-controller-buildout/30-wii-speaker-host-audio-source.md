# Brief 30: Wii Remote speaker as a host audio source (design only)

Read `ground-rules.md` first. **Status:** proposal only; not permission to implement or qualify audio. This brief compares caller-owned playback against an explicitly opted-in caller-session PipeWire source. No broker or IPC changes are proposed.

## Baseline and constraints

The pure `WiiSpeakerDecoder` is decode-only; emitting an authoritative ordered speaker stream still requires Wii family journal wiring under ADR-0024. This brief does not enable playback, promote Wii speaker support, or assert host or physical results.

## Alternatives and recommendation

Design evaluation follows in this branch.
