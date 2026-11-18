# Wii speaker decode (Brief 16 S1, Alpha 2)

Source-evidence audit at vg `324d250bb01a5ddbfc1dd189513d4c9160429ee1`; research `6fc8a2d4d8a48a8f025d98ac8448e46c0f97ea7d`. No host or physical run. This is decode-only; journal wiring and playback are outside this change.

## Baseline audit

- [codec.rs](https://github.com/anderstvoss/virtualgamepad/blob/324d250bb01a5ddbfc1dd189513d4c9160429ee1/crates/gr-curated-controllers/src/wii_remote/codec.rs#L295-L406): `0x14` and `0x19` accept one-byte controls, retain internal enable/mute flags and conditionally ACK bit 1. Status report `0x20` reflects enable intent, not playback. Conventional retained outputs change only when LEDs or rumble change.
- [memory.rs](https://github.com/anderstvoss/virtualgamepad/blob/324d250bb01a5ddbfc1dd189513d4c9160429ee1/crates/gr-curated-controllers/src/wii_remote/memory.rs#L149-L158): `0x16` speaker-register writes update a bounded ten-byte image; invalid ranges return error 7. No decoder activation or audio event follows.
- `0x18` requires a 21-byte report payload, but discards the audio area (including oversized encoded lengths) and applies only its shared rumble bit. No speaker-data ACK or typed audio record is produced.

The source-only consumer contract is [research #145 brief](https://github.com/anderstvoss/controller-protocol-research/blob/6fc8a2d4d8a48a8f025d98ac8448e46c0f97ea7d/docs/wii-speaker-consumers.md). Libogc and CWiid differ in control order, divisor and pacing. Physical decoder state, volume limits, FIFO and data ACK policy remain unknown.
