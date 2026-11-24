# Wii speaker decode (Brief 16 S1, Alpha 2)

The S1 speaker framing/decode implementation is compiled, but the controller
still refuses audible speaker playback. Decoding commands and delivering
samples are distinct capabilities. Do not report a working speaker before
the downstream journal, Dolphin and hardware checks.

TODO(alpha2-final): G6 Wii speaker journal integration and Dolphin acceptance.

Source-evidence audit at vg `324d250bb01a5ddbfc1dd189513d4c9160429ee1`; research `6fc8a2d4d8a48a8f025d98ac8448e46c0f97ea7d`. No host or physical run. This is decode-only; journal wiring and playback are outside this change.

## Baseline audit

- [codec.rs](https://github.com/anderstvoss/virtualgamepad/blob/324d250bb01a5ddbfc1dd189513d4c9160429ee1/crates/gr-curated-controllers/src/wii_remote/codec.rs#L295-L406): `0x14` and `0x19` accept one-byte controls, retain internal enable/mute flags and conditionally ACK bit 1. Status report `0x20` reflects enable intent, not playback. Conventional retained outputs change only when LEDs or rumble change.
- [memory.rs](https://github.com/anderstvoss/virtualgamepad/blob/324d250bb01a5ddbfc1dd189513d4c9160429ee1/crates/gr-curated-controllers/src/wii_remote/memory.rs#L149-L158): `0x16` speaker-register writes update a bounded ten-byte image; invalid ranges return error 7. No decoder activation or audio event follows.
- `0x18` requires a 21-byte report payload, but discards the audio area (including oversized encoded lengths) and applies only its shared rumble bit. No speaker-data ACK or typed audio record is produced.

The source-only consumer contract is [research #145 brief](https://github.com/anderstvoss/controller-protocol-research/blob/6fc8a2d4d8a48a8f025d98ac8448e46c0f97ea7d/docs/wii-speaker-consumers.md). Libogc and CWiid differ in control order, divisor and pacing. Physical decoder state, volume limits, FIFO and data ACK policy remain unknown.

## Source-model decode contract

- Existing `Protocol::request`/`output` return optional raw HID output observations, including the accepted speaker reports and rejected memory transactions with a protocol error ACK. These are not authoritative speaker records. The new pure `WiiSpeakerDecoder::decode` returns a candidate decoder state and typed record; it has no journal or transport dependency. Live realization behavior and the existing raw observation path remain unchanged in this decode-only step.
- The enable and mute control records retain the exact control byte and both host flags. A register record retains the full write payload and resulting ten-byte register image. `0x00` is Yamaha ADPCM, `0x40` signed 8-bit PCM, any other selector is unknown. Format at offset 2, LE divisor at 3–4, raw volume at 5. Rates are integer truncations of the nominal Dolphin formulas: 6 MHz/divisor (ADPCM) or 12 MHz/divisor (PCM); zero divisor yields no rate. Retain raw volume, not a guessed hardware maximum.
- Both libogc and CWiid configure through the same ordered register writes but differ in mute/enable order, divisor and cadence. Each accepted bounded write is represented; no hardware activation latch is assumed. Dolphin updates its registers immediately and keeps decoder state across register changes, enable/mute and PCM packets; reset starts at predictor 0 and step 127. The `0x18` record retains the entire 21-byte payload, encoded length and control context. Oversized/empty/muted/unconfigured/unknown-format writes yield explicit dispositions and no samples. The host enable bit is not the Dolphin application playback setting; valid disabled-status writes are decoded and explicitly flagged, not treated as proof of physical behavior. Zero volume still decodes. No per-packet ACK change.
- Mathematical test vector: from predictor 0 and step 127, high nibble 0 adds 15 (step clamps to 127), nibble 7 adds 238 (step 304), nibble 8 subtracts 38 (step 273), nibble 15 subtracts 511 (step 654). Expected samples: 15, 253, 215, -296. Additional tests cover PCM signed widening, saturation, state continuity and configuration sequencing. No upstream table/code copied.

Claims: `audio-wii-dolphin-control-and-stream-reports`, `audio-wii-dolphin-pcm-adpcm-rate`, `audio-wii-dolphin-speaker-register-model`, `audio-wii-dolphin-yamaha-adpcm-nibbles`, `audio-wii-dolphin-volume-range-discrepancy`, `wii-gap-speaker-libogc-startup-status`, `wii-gap-speaker-cwiid-beep-sequence`, `wii-gap-speaker-physical-decoder-behavior-unknown`. Exact arithmetic and gating are additionally pinned by [audio-wii-dolphin-decoder-arithmetic-and-state](https://github.com/anderstvoss/controller-protocol-research/blob/20d030d58cac270233fe47acffd6f7f649342e53/records/claim/audio-wii-dolphin-decoder-arithmetic-and-state.json) (research PR #154; pending adoption into the submodule). Manifest references only claims available at the existing global research pin; the new claim is a PR dependency, not a silent global pin change.

Physical decoder behavior, reset timing, activation, ACK policy, pacing and playback remain unknown. Future journal/consumer/hardware checks: not run.
