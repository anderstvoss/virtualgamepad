# EXP-0023: controller audio baseline and first PipeWire transport

Status: partial evidence; USB composite and controller matching are not accepted.
Baseline: GUI PR #110 (`476056a`), audio work on its descendant branch.

## Physical DualSense observation

On 2026-09-23 the reference device was visible again in this Linux VM as
054c:0ce6, bcdDevice 1.00, high-speed, configuration 1. Read-only inspection
again showed audio-control interface 0, two audio-streaming interfaces 1/2,
HID interface 3, and an ALSA playback/capture card. The ALSA mixer exposed
headphone-jack and headset-microphone-jack indicators, both reporting `off` at
inspection time, plus enabled playback/capture switches and volume values 97/31.
No audio stream, mixer control or HID effect was exercised during this check.
The `off` indicators are a snapshot, not evidence that the headset or mic is
absent; jack routing and capture remain subject to an interactive test after
the maintainer confirms readiness.

The attached device was identified by USB VID/PID 054c:0ce6, bcdDevice 1.00.
No serial number or microphone recording is included in this record. A complete
firmware revision was not established. Cached descriptors, ALSA stream information
and mixer controls were inspected; direct descriptor access was permission-limited.

| Property | Observed value |
| --- | --- |
| USB audio class | UAC1, version 1.00 |
| Interfaces | Audio control 0; playback 1; capture 2; HID 3 |
| Playback | Alternate 1, four channels, S16_LE, 48000 Hz |
| Playback channel mask | 0x0033 (FL, FR, RL, RR) |
| Playback endpoint | 0x01, adaptive, maximum 392 bytes, interval 4 at high speed |
| Capture | Alternate 1, two channels, S16_LE, 48000 Hz |
| Capture channel mask | 0x0003 (FL, FR) |
| Capture endpoint | 0x82, asynchronous, maximum 196 bytes, interval 4 |
| Playback feature unit | ID 2, mute and volume |
| Capture feature unit | ID 5, mute and volume |

Four separate 250 ms, 110 Hz playback bursts at amplitude 300/32767 were
submitted, activating one channel at a time. All four ALSA transfers completed.
A one-second stereo capture completed with 192000 bytes. Its samples were discarded
without saving a recording. Mixer controls were read but not changed; no HID
adaptive-trigger commands were sent.

These results establish stream availability and successful transfers. They do
**not** establish audible response, physical haptic channel isolation, headset
routing, mute/gain semantics, timing fidelity, or reconnect behavior. The physical
controller's reported trigger and stick defects are excluded from acceptance.
UAC2 functional emulation must not be described as matching this UAC1 topology.

### Headset and grip channel observation follow-up

With a headset containing a microphone connected to the physical DualSense's
3.5 mm jack, the maintainer confirmed left headphone output followed by right
headphone output during sequential channel playback. A grip-only repeat used
110 Hz, S16_LE, 48000 Hz bursts at amplitude 1600/32767, lasting 750 ms each,
with three seconds of silence between channels 3 and 4. Channels 1 and 2
contained only zeros during that repeat. Playback completed successfully and
the maintainer reported **clear left grip followed by right grip**.

This establishes the observed playback mapping for this device and configuration:

| PCM channel | Observed destination |
| --- | --- |
| 1 (FL) | Left headphone |
| 2 (FR) | Right headphone |
| 3 (RL) | Left grip |
| 4 (RR) | Right grip |

This is human-observed channel separation, not a measurement of acoustic or
mechanical crosstalk, gain, latency, or synchronization. No adaptive-trigger
commands were sent. Headset microphone routing, speaker/headset switching,
reconnect behavior and physical-versus-virtual comparison remain open; this
observation alone does not enable controller-matching support.

### Reattached physical controller, five-output and microphone check

On 2026-09-23 the maintainer confirmed a fresh four-channel sequence through
the physical controller: left headset audio, right headset audio, left grip,
right grip, in that order. Playback was 48 kHz, four-channel S16_LE through
the physical DualSense PipeWire sink, with independent 750 ms tones and
one-second silence gaps. Headset channels used 440 Hz at 1000/32767; grip
channels used 110 Hz at 1600/32767. The stream returned successfully. The
result confirms human-observed routing again, not quantitative crosstalk or
latency.

With the headset unplugged, a one-second 440 Hz tone on the right audio
channel transferred successfully, but the maintainer heard **no onboard
speaker output**. The headset was then reconnected. This host runs Linux
6.12.107. The upstream [DualSense audio-jack routing patch](https://www.spinics.net/lists/linux-input/msg101514.html)
explains that the default path stays on headphones even when unplugged and
that speaker output needs HID path selection plus speaker volume/preamp setup.
The current result is therefore an exact blocker for a speaker claim on this
host; USB transfer success alone is insufficient. No experimental HID routing
report was sent to the physical device, and adaptive triggers were untouched.

The USB HID reverse-output decoder now exposes validity-gated audio path,
speaker and microphone volume, and speaker preamp fields alongside the full
native report bytes. Its four path values follow the upstream Linux
`hid-playstation` mapping: stereo headset, dual-mono headset, headset-left
plus speaker-right, and speaker-right only. This is an observation of a host
request, not proof of physical routing. The decoder does not alter PCM; a
backend must not also apply those controller-native levels to samples unless
it explicitly owns that transformation.

Two bounded three-second, 48 kHz, stereo S16_LE headset-microphone captures
returned 144,000 frames each, but their levels stayed near zero: RMS about
0.5/32767 per channel, with peaks at most 8/32767. The maintainer confirmed
the headset was seated and unmuted. The ALSA headset-jack indicators still
reported off. Capture availability is established; headset microphone routing
and meaningful signal response are **not** established. The checker retained
only frame counts and per-channel RMS/peak aggregates, no PCM recording.

### Audio-only speaker route and speech follow-up (2026-09-24)

The reference controller was attached over USB with a 3.5 mm headset. A new
four-channel, one-at-a-time ALSA test again produced the maintainer-confirmed
order: left headphone, right headphone, left grip, right grip. This reconfirms
channel routing, without measuring acoustic crosstalk or latency. No trigger
controls were used.

With the headset removed, ordinary right-channel ALSA playback completed, but
the listener could not determine whether the onboard speaker played. A bounded
follow-up used the audio-only USB HID fields from the upstream
[DualSense jack-routing patch](https://www.spinics.net/lists/linux-input/msg101514.html):
speaker-only output path (right source), speaker volume `0x64` and preamp `0x02`.
It played a quiet one-second right-channel tone and restored the stereo-headphone
route in a `finally` block. The maintainer **clearly heard the onboard speaker**.
The probe's deterministic tests assert that all motor, trigger and LED fields
remain zero. The old kernel provides no readback for the previous speaker volume
or preamp values; those settings may persist while the restored headphone route
is active. The headset was requested to be reconnected afterward. This confirms
five independently addressed physical outputs in the tested USB configuration,
not automatic jack switching by this kernel or matching-mode acceptance.

Two further three-second microphone captures completed at 48 kHz stereo, one
with the headset seated and unmuted and one after it was removed. The maintainer
spoke during both. Each returned exactly 144,000 frames; headset RMS was
0.50/0.51 with peaks 2/2, and built-in-microphone RMS was 0.50/0.51 with peaks
1/2, all relative to 32767 full scale. The orange mute light was off during
the built-in test. These near-zero levels do **not** demonstrate either
microphone's signal path or gain/mute behavior. No audio recording was retained.
Physical USB reconnect, quantitative physical/virtual comparison and microphone
routing remain open; controller-matching support is still unavailable.

### Physical versus emulated profile comparison (static)

The compiled `DualSenseEmulated` USB profile and the observed physical device
share the 054c:0ce6 USB identity and a 48 kHz, S16_LE, four-channel playback /
two-channel capture stream shape. The emulated playback roles are left/right
audible and left/right haptic; those roles match the human-observed physical
output order. These facts establish a useful comparison target, **not** a
matching USB topology:

| Property | Physical reference | Compiled emulated USB profile |
| --- | --- | --- |
| Audio class | UAC1 | UAC2 |
| Playback/capture channels | 4 / 2 | 4 / 2 |
| PCM format | S16_LE, 48 kHz | S16_LE, 48 kHz |
| Playback role order | Headset L/R, grip L/R observed | Audible L/R, haptic L/R declared |
| Microphone signal path | Near-zero captures; unverified | Functional synthetic microphone return |
| Internal speaker selection | Explicit audio-only HID route heard | Not physically compared with a host consumer |

The virtual profile's different Audio Class descriptors, clock model and
terminal topology cannot be described as physical matching. A controlled
consumer comparison of endpoint association, per-channel routing, HID audio
controls and teardown is still required. No capture waveform or device serial
number is retained in this record.

The gain check below was prepared to narrow the microphone failure without
retaining audio. Even if it remains quiet, headset compatibility, HID
microphone volume/mute state and this host's USB/audio routing require
separate evidence before assuming a broken microphone. USB reconnect and an
identical-consumer physical/virtual comparison require their own readiness
confirmation.

### Headset microphone gain check (2026-09-24)

With the 3.5 mm headset reconnected, the maintainer confirmed speech during
both three-second captures. The physical controller's `Headset Capture Switch`
was on. The lab helper captured exactly 144,000 stereo frames at the existing
ALSA capture gain 31/101, then at 101/101, and verified restoration to 31 in
its `finally` path. It retained aggregate levels only:

| ALSA capture gain | Left RMS / peak | Right RMS / peak |
| --- | --- | --- |
| 31/101 | 0.50 / 1 | 0.51 / 2 |
| 101/101 | 7.16 / 36 | 17.22 / 83 |

Values are relative to 32767 full-scale PCM. The gain change measurably
amplified the captured signal, but the result does not establish intelligible
speech, source selection or headset microphone fidelity. No recording was
saved. A same-gain silence-versus-speech comparison followed.

The first maximum-gain comparison was invalidated when the maintainer missed
the transition between its quiet and speech windows. The repeat added a quiet
half-second left-headphone cue between two three-second captures at gain
101/101. The maintainer heard the cue and confirmed being quiet before it and
speaking afterward. Each window returned 144,000 stereo frames:

| Maximum-gain window | Left RMS / peak | Right RMS / peak |
| --- | --- | --- |
| Quiet | 13.20 / 64 | 17.37 / 81 |
| Speech | 13.17 / 64 | 17.40 / 85 |

The microphone signal showed no measurable speech-level change in this
configuration. Gain was verified back at 31/101. This narrows the current
headset-capture failure; it does not prove the headset microphone or controller
hardware is defective. The signal path, jack compatibility, HID microphone
controls and this VM's USB/audio presentation remain possible factors. Built-in
microphone response at maximum gain was not tested.

### Physical USB reconnect (2026-09-24)

Before the maintainer unplugged the controller, the physical device was on
USB path `1-6`, ALSA card 1, bcdDevice `0100`, with playback and capture streams
and capture gain 31. After the maintainer reported unplugging it, physical
DualSense discovery found no device or associated ALSA card. After the
maintainer reconnected USB, the device returned on path `1-6`, card 1, with
the same revision, playback/capture streams and gain 31. Its four interfaces
again reported audio control, two audio streaming and HID classes. This
establishes clean host disappearance and fresh endpoint enumeration for this
one reconnect. After explicit readiness, a quiet half-second 440 Hz
left-headphone tone transferred through the fresh ALSA device and the
maintainer heard it. A three-second capture immediately afterward returned
144,000 stereo frames; the maintainer spoke, but RMS remained 0.50/0.51 and
peaks 2/2. This verifies the post-reconnect **playback** path and capture
transfer while leaving meaningful microphone response unverified. It does not
establish audio stream continuity *during* disconnection; closure and
recreation are still required.

## PipeWire transport evidence

Ignored live tests were explicitly run on a prepared host. Four backend tests
passed: host playback to library samples with exact four-channel values;
library microphone return to host capture; native-client ownership; and native
sample exchange in both directions. A root-only consumer test passed creation,
servicing, neutralization, independent removal, repeated closure and duplicate
sessions for DualSense, DS4 and standard-HID Xbox 360.

The first playback experiment found that an unspecified channel-position map
produced silence. Explicit positions corrected this. A deterministic format-POD
regression now checks S16_LE, 48000 Hz and the four channel positions. Semantic
haptic roles remain explicit in the API; callers must not treat those channels
as ordinary rear speakers or enable automatic remixing.

The live tests are short functional checks, not the required three 60-second
acceptance trials. No USB PCM path has passed; no controller-matching profile is
enabled. Physical speakers and microphones are not automatically connected.

## Host preparation and remaining gates

The administrator ran `scripts/prepare-audio-lab.py --apply`. ConfigFS and kernel
FunctionFS, UAC1/UAC2 and dummy_hcd modules are available; two dummy UDCs were
verified. This grants no broker authorization or sound-device permissions.

Remaining: FunctionFS enumeration through unprivileged personalities, broker
security acceptance and deployment, UAC2 ALSA bridge, sustained three-family
trials on both transports, physical control/routing comparison, matching decision,
and subsequent GUI integration and fresh API review.

## Sustained playback results

### Closure follow-up

A fresh isolated-in-time DS4 playback run completed three 60-second trials, each
with 2,879,488 frames and zero queue loss, unexpected frames or discontinuities.
Each deliberate 200 ms stall discarded 8,192 frames and surfaced a gap. This is
new playback evidence, not an explanation or erasure of the historical failures,
nor full-duplex/native-client acceptance.

A new synthetic host-to-library latency test timestamps 128-frame blocks just
before handing them to `pw-cat` and measures arrival at the library sample read.
After excluding the first two seconds of source publication, 288,000 measured
frames had p50 23.283 ms, p95 32.621 ms, p99 33.543 ms and maximum 37.374 ms,
with zero invalid markers or queue drops. **The sub-20 ms gate failed.** These
numbers include the source pipe and graph buffering; they do not measure the
reverse direction or physical hardware converters. Zero-filled intervals are not
latency markers; independent continuity tests remain required.

The host settings metadata reported `clock.min-quantum=1024` and 48,000 Hz:
21.333 ms per minimum graph cycle. The endpoint now requests `node.latency`
corresponding to approximately 2.67 ms (128/48000), without forcing graph quantum
or changing desktop defaults. PipeWire defines this property as a request, so
host-policy constraints must be resolved and the resulting path remeasured; this
property alone cannot establish latency acceptance. See the
[PipeWire node keys](https://docs.pipewire.org/group__pw__keys.html).

An isolated graph with 128-frame minimum/default quantum and WirePlumber's
policy-only profile initially measured p99 25.940 ms, maximum 26.136 ms. Inspection
identified persistent startup backlog in the paced test source: it began filling
the client pipe before the graph was ready, then produced at the same rate as
consumption. The harness now drains an acknowledged startup block before pacing
the measured source. With that correction, the isolated short probe measured
290,176 frames, p50 10.062 ms, p95 10.775 ms, p99 10.948 ms and maximum 11.111 ms,
with zero invalid markers and queue drops. This is short host-to-library sample
evidence only, not reverse/native access or three-family sustained acceptance.
The earlier numbers remain valid observations of the prefilled test pipeline;
they must not be represented as backend-only latency.

`scripts/run-pipewire-audio-lab.py` creates a private runtime/config/state directory,
starts an unprivileged daemon and policy-only session manager, and runs a bounded
test command. It does not launch hardware monitors. Desktop settings were checked
afterward and retained their original 1024-frame minimum. The maintainer delegated
the choice of preparation method; isolated validation avoids modifying shared audio
settings. Ordinary desktop use remains subject to its graph's timing policy.

Further short isolated probes passed the same p99 threshold:

| Path | Measured frames | p50 | p95 | p99 | Maximum |
| --- | ---: | ---: | ---: | ---: | ---: |
| Library microphone to host | 289152 | 4.758 ms | 5.199 ms | 5.266 ms | 6.163 ms |
| Native caller to host | 289152 | 10.058 ms | 10.522 ms | 10.584 ms | 10.699 ms |
| Host to native caller | 276352 | 7.776 ms | 8.273 ms | 8.420 ms | 8.553 ms |

All reported zero invalid markers. Only playback paths report playback queue-loss
counters; a missing reverse-direction loss metric must not be read as zero loss.
The capture-side test client uses `stdbuf --output=0` to avoid measuring its stdio
buffer as backend latency; collection still includes up to a 128-frame read block.
Zero-filled intervals are excluded from marker latency distributions. These probes
do not replace the full-duplex continuity, mixed-controller or sustained matrix.

### Strict continuity follow-up: acceptance remains failed

The latency-only results above are not continuity passes. After adding an exact
post-warm-up marker-frame requirement, the four-path rerun passed only library
microphone to host (289280/289280 frames, p99 6.197 ms). Host to library received
271360/290048 frames; native caller to host 287744/289664; host to native caller
288512/289664. All three failed despite their sub-20 ms latency distributions.

A subsequent experiment with separate control/process listeners and RT_PROCESS
failed all four paths. It also relied on callback-removal synchronization that
is present in PipeWire 1.4.2 but absent from the supported 0.3.49 implementation.
That experiment was removed. All callbacks again execute on the dedicated
worker's main loop with a single listener; no cross-thread mutable callback
userdata or newer teardown guarantee is assumed.

The startup handshake previously sent one finite marker block before link
activation and waited for every frame. That can time out if activation discards
initial buffers. The harness now continues bounded, unmeasured warm-up until the
receiver acknowledges sample flow. A deterministic regression simulates initial
buffers without a consumer. Measured traffic still requires exact frame counts;
per-marker accounting also rejects equal totals containing both loss and repeats.

Later isolated runs still failed. At 128 frames, a host-to-library run observed
254208/298752 measured frames, 4608 invalid channel frames, p99 263.521 ms and
maximum 270.921 ms, despite zero application queue drops. A 256-frame experiment
also failed (275328/290176 frames, p99 215.198 ms, maximum 222.552 ms). Removing
playback-client stdio buffering did not resolve the failure. Samples sometimes
contained audible channels with zero haptic channels, or only one nonzero channel.
These synthetic observations are retained as failures, not physical evidence.

A correlated 128-frame run of pw-top showed increasing scheduling-error counters
for the dummy driver, library sink and pw-cat: respectively 22, 216 and 228 at
the fourth snapshot, at 48 kHz. This locates a graph scheduling failure alongside
the sample corruption; it does not establish its complete cause or excuse loss.
No real-time privilege, desktop graph changes, or lowered acceptance threshold
was introduced. The added callback-driven source removes blocking pipe I/O from graph processing.
Its host-to-library run measured p99 10.752 ms, maximum 14.030 ms, but still failed:
256128/307328 measured frames, 51456 missing marker frames, 256 duplicate marker
frames, 5760 invalid channel frames, and zero application queue drops. This
separates the long pipe backlog from the remaining continuity defect. It does
not establish a supported path. Independent capture, graph scheduling diagnosis,
and the sustained acceptance matrix remain outstanding.

### Earlier observations

After serializing PipeWire local callbacks on their dedicated worker, DualSense
completed three 60-second playback trials: 2879488, 2879488 and 2880512 frames,
with zero unexpected frames, queue drops or discontinuities. Startup-to-first-data
was approximately 34–39 ms; this is **not** a steady-state latency measurement.
Deliberate 200 ms caller stalls discarded 7168–8192 frames and surfaced gaps.
The later corrupted/empty-buffer handling change still needs sustained revalidation.

DS4 and Xbox trials failed and remain failed evidence. One separately observed
DS4 trial received 7168 unexpected zero frames with zero caller-queue drops;
PipeWire's driver, sink and pw-cat scheduling-error counters were nonzero during
that run. This supports host graph underruns as a cause, without proving all loss
has been explained. Earlier mixed trials also overflowed the bounded caller queue.
Do not weaken the assertions or count these runs as accepted functionality.

Short microphone/native-client checks passed again after worker serialization.
Sustained microphone/native-client trials, steady-state latency, full three-family
acceptance and post-change playback revalidation remain outstanding.

### Synchronous graph processing follow-up

The replacement implementation uses separate control and process listener state.
The process callback runs synchronously on the data loop without blocking client
pipe I/O. Teardown disconnects the stream before freeing process listener state;
the native disconnect path removes the node through a synchronous data-loop
invocation, rather than relying on newer listener-removal behavior. Intentional
close removes the control listener first and does not become a backend failure.

A subsequent short matrix passed all four directions/access paths for each of
DualSense, DS4 and Xbox360, using their curated emulated profiles. Each probe
lasted approximately eight seconds, with two seconds excluded as warm-up, on an
isolated 512-frame graph. Exact per-marker counts, channel integrity and p99 below
20 ms passed. These are twelve short checks, not three consecutive 60-second
full-duplex trials. A preceding 62-second graph-source trial failed with missing
frames and remains failed evidence. No sustained acceptance is inferred.

Five close-during-processing/recreation cycles also passed with retained clocks,
invalidated handles and an unaffected sibling session. Stream-local graph clock
snapshots now distinguish missed graph frames from bounded queue loss. Upstream
client loss is not automatically included in either counter.

### Longer DualSense graph-path check

At revision `9dc68a0`, four sequential 62-second probes (two seconds excluded as
warm-up) passed on the private 512-frame graph. Each used the curated DualSense
emulated profile. All measured marker frames were present exactly once, with no
invalid channel frames. Host-to-library graph diagnostics reported zero missed
graph frames and discontinuities.

| Path | Measured frames | p50 ms | p95 ms | p99 ms | Maximum ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Library microphone to host | 2881152 | 6.773 | 10.541 | 10.848 | 14.214 |
| Native caller to host | 2881280 | 10.759 | 11.131 | 11.429 | 15.007 |
| Host to native caller | 2880768 | 10.788 | 11.165 | 11.476 | 13.527 |
| Host to library samples | 2880256 | 0.651 | 1.014 | 1.087 | 5.625 |

These application-to-application measurements include client buffering and
scheduling. They do not measure physical converters or prove independent clock
synchronization. This is one trial per path, run sequentially, not the required
three consecutive full-duplex trials across families and transports. Earlier
failed runs remain part of the evidence; this run does not explain them all.
