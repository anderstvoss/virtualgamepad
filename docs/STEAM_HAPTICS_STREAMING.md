# Steam Controller 2026 haptic MIDI and PCM

The MIDI test uses **SteamHapticsSinger itself**. It reads a generated MIDI file
and sends haptic tone requests; virtualgamepad does not add a MIDI synthesizer,
MIDI server, or USB MIDI interface. PCM tests use the pinned SteamHapticsPlayer
and iczero clients. The wired descriptor remains unchanged and has no invented
USB Audio Class endpoint.

## Creation and output consumption

`SteamController2026CreationOptions::haptic_streaming` defaults to `false`.
Enable it before creating a wired `linux.uhid.usb` instance to handle the captured
8-kHz s16le internal-haptic streams. It is independent of `steam_client_compat`.
The demo exposes both options in Advanced options. Neither option adds a HID
stream channel to uinput, whose FF_RUMBLE interface cannot carry PCM packets.

`SteamController2026Haptic::Stream` wraps a decoded
`SteamController2026StreamPacket`. `host_outputs().streaming` retains bounded
configuration/counter diagnostics and the **last** packet, not a recording.
Consume every chunk from the existing raw reverse callback with
`SteamController2026StreamPacket::from_output(report_id, &bytes)`; `bytes`
exclude the report ID. This decoder does not make an unbounded copy. A valid
configuration packet records host intent; acceptance is separate.

Every accepted write is observed, including identical consecutive chunks. A
logical FIFO overrun still exposes the original complete write through the raw
callback and increments an explicit overrun counter. Slot order in retained
stream arrays is feedback IDs `[0, 1, 3, 4]`. These counters describe the virtual
compat model, not a measurement of the retail controller's buffers.

## Supported framing and measured behavior

- MIDI/LFO output `0x83`: declared nine-byte body, or the observed 63-byte body
  with 54 trailing zero bytes. Nonzero extensions remain invalid. The callback
  preserves the complete original body, including reviewed padding.
- Output `0x86`: captured three-byte body, operation 1/2, group 2/5, mode 0.
  Disabling either group yields status `0x02`; internal-group configure yields
  `0x20` for IDs 1/0, while pad-group 8-kHz configure yields `0x10` for IDs 4/3.
  Both kinds of configuration write complete successfully at transport level.
- Output `0x87`: selector 0/4 and an even 2..62 data bytes. The physical client
  used both 64-byte and 12-byte wire writes, the latter carrying ten final data
  bytes. Unconfigured data is rejected atomically.
- Output `0x88`: 63-byte body; even length 2..30 specifies meaningful bytes in
  each 31-byte slot. Unused slot bytes are not audio samples. The physical
  stereo captures used length 30. Every raw byte remains visible in callbacks.
- Input `0x44`: five-byte body, target/status/three zeros. These are stream
  statuses, **not one acknowledgement per data write**.

Other modes/targets and output `0x89` remain unsupported. No float/8-bit/u-law,
trackpad PCM, or inferred calibration is silently accepted. A repeated enable
requires an explicit stop first under the compat policy; the unmeasured retail
already-running status is not fabricated. Invalid requests have explicit
Invalid/Unsupported/Busy completions and no partial state mutation.

## Explicit compat buffering policy

Retail FIFO size, thresholds and exact timing remain unmeasured. The opt-in
model uses 1024 logical bytes per channel, an 8-ms startup allowance, high/low
watermarks 128/64 bytes, 16000 bytes/s draining and a 4-ms feedback polling
deadline while playing. Input servicing may carry current `0x42` state with
feedback; no standalone periodic state stream is added when PCM is inactive.
The feedback queue is bounded to 16 entries; backpressure fails atomically.

The observed status bytes provide acceptance/rejection, stopped, low/high and
overrun indications. The model's numerical buffer policy is implementation
compatibility, not physical fidelity. Stop/reset clears active queues and
pending feedback. Handle close cancels future servicing and removes the owned
device; cached counters/last intent remain diagnostics. There are no worker
threads or persistent resources owned by the protocol.

## Physical reference and automated end-to-end check

The supervised 2026-11-05 D9 study builds on research #105 and implementation
#305. It records MIDI channel mapping/padding, PCM framing, exact waveform byte
correspondence, configuration rejection and qualitative physical feedback. The
initial low-level trial was quiet; a separate peak-4096, equal-440-Hz repeat was
clearly heard/felt on both sides. This is not calibrated acoustic testing.

The source pins are Singer `d809a28`, Pixel player `e06e48d` / TritonLib
`d470e0d`, and iczero `83c46d7`. See research records
`steam-d9-midi-lfo-framing`, `steam-d9-midi-channel-routing`,
`steam-d9-pcm-framing-feedback` and `steam-d9-pcm-stimulus-correspondence` in the
paired audio/MIDI evidence branch. No raw capture or identifying value is
imported into this implementation.

The opt-in Linux test `steam_haptics_live` runs those upstream executables
against **only an owned virtual controller**, with physical Valve controllers
and competing clients absent. It generates its own MIDI/WAV/PCM stimuli,
checks all four MIDI channels, compares every dual/mono PCM sample byte,
verifies the upstream client's acceptance/low/high feedback consumption, tests
short final packets, explicit stops, recreation and node cleanup. It uses the
existing installed UHID/udev setup; it does not run sudo or change permissions.
The creation-open retry has a deterministic regression for the udev race.

Set these variables to reviewed local executable paths, then run:

```sh
export VIRTUALGAMEPAD_STEAM_CLIENT_E2E=1
# Set STEAM_HAPTICS_SINGER, STEAM_HAPTICS_PLAYER, STEAM_HAPTICS_ICZERO.
# Optional: VIRTUALGAMEPAD_STEAM_E2E_ARTIFACTS selects an existing private directory.
cargo test --locked -p gr-curated-controllers --test steam_haptics_live -- \
  --ignored --nocapture --test-threads=1
```

The explicit D9 run passed with four MIDI channels and 79990 received PCM bytes
per internal channel: 39990 dual-client bytes plus 40000 mono-client bytes,
including two short final mono writes. Both clients' transformations matched
exactly. An additional configured/data-receiving instance is closed without a
host stop to check shutdown. Stop/cleanup left no owned hidraw/input nodes and zero write failures.
The ordinary workspace suite retains the ignored hardware requirement; the
separate explicit run supplies measured live evidence. This verifies a virtual
protocol path, not sound emitted by a virtual speaker or physical equivalence.
