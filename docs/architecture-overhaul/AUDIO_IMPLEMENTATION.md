# Controller audio execution ledger

Baseline: GUI PR #110, `476056a`. Audio is newly authorized pre-alpha work; final
API review, quality review and release still follow audio and later GUI feedback.

## Contract

The accepted closure target is measured p99 below 20 ms in each direction on the
prepared host, after warm-up, for sample and native-client access. Report maximum
latency and outliers separately. This supersedes the earlier measure-first policy;
it is not a hard real-time guarantee. Queue capacity is not the operating fill
target. The USB lab microphone now targets 192 frames (4 ms at 48 kHz), retaining
2048-frame capacity. This change needs fresh live validation and is not itself
latency evidence.

For reproducible low-latency host preparation, use the private graph harness:

```sh
python3 scripts/run-pipewire-audio-lab.py --timeout 35 -- \
  cargo test -p gr-audio-linux --features pipewire --test pipewire_live \
  latency_graph_source_to_library_samples -- --ignored --nocapture
```

It leaves desktop audio settings unchanged. The current desktop minimum is 1024
frames, which cannot satisfy the target at 48 kHz. The isolated graph defaults to 512 frames (10.67 ms); `--quantum` permits
128, 256 or 512 frames. The callback-driven source separates graph delivery from blocking client
pipes. The latest short four-path matrix passed exact marker continuity, channel
integrity and p99 below 20 ms for all three curated emulated profiles. Sustained
acceptance remains open; earlier graph-driven runs failed continuity. Earlier latency-only passes are not accepted
paths. EXP-0023 retains all failures; full latency/continuity acceptance remains
pending. The existing pw-cat probes remain available for independent-client
comparison. Run live probes sequentially, without concurrent builds or tests.

USB capture now collects each 1 ms packet on its scheduled deadline instead of
requiring a whole URB of microphone samples at completion. A deterministic 32-packet
test supplies only one millisecond at a time and verifies exact assembled output;
partial cancellation discards its collected generation without leaking it into a
reused request. Collected frames abandoned by cancellation/teardown are counted.
The root audio handle now uses a private backend interface and supports microphone
flush. Producer-side flush leaves cursor ownership with the consumer: the next
backend read skips previously published frames; already executing/delivered audio
is unaffected. Flush is not spontaneous queue loss.

The broker accepts administrator VHCI allowlists and requires a non-root worker
UID/GID, with the UID distinct from every authorized client. Staged port admission
has bounded inventory parsing, concurrent reservations and pre-attach revalidation.
These are tested prerequisites, not an installed USB attachment operation. The
daemon/provider integration, privileged lifecycle and full security matrix remain
open.

Audio exposure and per-group sample ownership are selected in `CreationOptions`.
Disabled remains default. Emulated and controller-matching are distinct requests;
changing exposure requires closing/recreating the controller. No implicit fallback
or resurrection. Native-client ownership and library-sample ownership are exclusive.
HID controls and protocol interpretation remain controller-owned; providers only
transport samples and expose endpoints. Physical routing remains caller policy.

The initial functional profiles use 48 kHz signed-16 PCM. DualSense retains four
playback and two capture channels, with audible and haptic pairs sharing one queue.
DS4/Xbox functional approximations use stereo playback and mono microphone. These
are named emulated profiles, not claims about physical formats or routing.
Matching profiles fail explicitly until their evidence and acceptance are complete.

## Increment status

| Increment | Status / exit |
| --- | --- |
| Creation policy, profiles, bounded transport | Implemented; bounded queues and deterministic tests |
| UHID/PipeWire | Implemented behind `audio-pipewire`; short sample/native and three-family lifecycle checks passed; sustained acceptance pending |
| Hardened broker / USB prerequisite | Existing connection quotas, framing deadlines and descriptor rejection tested; FunctionFS superseded by the stock-Linux USB/IP design (EXP-0024); VHCI broker acceptance pending |
| USB composite/UAC2 | Local USB/IP worker and compiled profiles implemented; deterministic and three-family process tests pass; DualSense VHCI/ALSA feasibility passed (EXP-0025); full acceptance and production integration pending |
| DualSense physical lab/matching | Attached: UAC1 4-out/2-in at 48 kHz S16_LE observed; short ALSA transfers passed; matching still unavailable |
| Separate GUI integration | Subsequent maintainer-directed branch/PR |

Emulated audio is available for the three scoped families on `LINUX_UHID_USB`
with the opt-in `audio-pipewire` feature. Unsupported combinations fail before
controller I/O. Controller-matching and USB composite audio remain unavailable.
Supporting workspace profile/queue APIs are SPI, not root application factories.

The optional Linux `pipewire` 0.10.1 binding provides native stream registration
and safe stream-clock queries (minimum native API 0.3.50). Control callbacks use
the dedicated main loop; a separate process listener owns the data-loop state.
Disconnect quiesces processing before its listener is removed. It adds its SPA/sys/build dependencies, including
bindgen. Linux all-feature builds need pkg-config, PipeWire/SPA development headers
and libclang (Debian/Ubuntu packages `pkg-config libpipewire-0.3-dev libclang-dev`).
Ordinary default builds do not enable the audio backend. CI prerequisite changes
remain outstanding; the local build was validated with these prerequisites.

Read-only audio component metadata distinguishes associated PipeWire endpoints
from the HID component; associated nodes do not claim common USB ancestry.
`ComponentAssociation::surface()` now returns an option because an audio component
has no controller input surface. Existing consumers should inspect components by
role and handle absent input surfaces.

## Queue guarantees

Preallocated single-producer/single-consumer queues publish complete signed-16
frames using release/acquire ownership. Endpoints are non-cloneable and require
exclusive access. Operations contain no locks, allocation, user callbacks or
unsafe code. Capacity is validated and bounded; unread slots cannot be overwritten.
Microphone writers retain the unaccepted suffix. Host producers explicitly mark
unretryable frames discarded; readers stop at discontinuity boundaries and expose
frame positions. Flush is explicit; close/drop is terminal, idempotent and prevents
later reads of stale buffered data. New formats require a fresh queue/generation.

## Evidence and acceptance

The functional DualSense grouping is consistent with ALSA's
[Sony DualSense configuration](https://github.com/alsa-project/alsa-ucm-conf/blob/master/ucm2/USB-Audio/Sony/DualSense-PS5-HiFi.conf),
which distinguishes four hardware playback channels from two hardware capture
channels. This is source context, not new physical observation or a pinned wire
fixture. Physical lab data must settle format/routing claims before matching is enabled.

Deterministic tests cover queue wrap, partial retry, channel separation, gaps,
closure, invalid frames, capacity limits, concurrent wrap/backpressure and
Wii-like variable-rate speaker generations. Root tests guard creation-time policy
and fail-before-I/O behavior for all families. Profile tests guard explicit matching
rejection and functional limitations.

Live acceptance remains: three 60-second runs per profile and transport after
warm-up, exact sample/channel checks, microphone return, native-client access,
stalls, mixed and duplicate controllers, independent removal and recreation.
Broker tests must precede production USB exposure. Do not promote skipped tests.

The physical lab must identify the attached DualSense exactly, inspect descriptors
and formats, test individual channels, mute/gain/routing and audio-driven haptics,
and compare physical and virtual sessions under the same host settings. Raw audio
and private device identity stay local. Do not actuate the reported broken triggers.

Actual physical and live findings: [EXP-0023](experiments/EXP-0023-controller-audio.md).

## USB plan correction

[EXP-0024](experiments/EXP-0024-usb-audio-transport-gates.md) records two source-level
blockers: stock dummy_hcd cannot carry isochronous PCM, and FunctionFS cannot supply
the assumed separate post-payload SET acknowledgement. Module preparation is not
USB audio readiness. The maintainer selected stock Linux; local USB/IP replaces
these assumptions. Production USB audio remains unavailable until live transport
and broker security acceptance; existing hardening does not override this gate.

## Current validation boundary

Short sample/native-client checks and root-only three-family lifecycle checks pass.
One DualSense three-trial playback run passed before the latest buffer-flag fix;
DS4/Xbox sustained trials failed with unexpected silence and/or queue loss. A DS4
observation also showed host graph scheduling errors. These are retained failures,
not waived acceptance. Full-duplex/native sustained acceptance, steady-state latency,
full clock-domain association and interpreted controller audio-control state remain
open. `stream_timings()` provides retained stream-local graph ticks, rate, monotonic
observation time and graph discontinuity counters. Graph delay is an estimate,
not measured end-to-end latency; graph loss and PCM queue loss are separate.

The broker now admits authenticated connections and sessions under shared per-UID
and global quotas, rejects/closes unsolicited descriptors, bounds partial-frame
read time and reply writes, and terminally closes clients after framing/I/O failure.
These checks are prerequisites only; they do not establish complete USB broker
security acceptance or permit new composite operations.

The headless root-only `controller_audio` example prints endpoint metadata, drains
playback, services HID and reports retained teardown diagnostics. It opens no
physical audio devices and leaves microphone underruns silent.

## Stock Linux revision

The maintainer requires stock Linux. The revised candidate is a local USB/IP VHCI
transport with a trusted unprivileged device worker, described in
[USB_AUDIO_STOCK_LINUX](USB_AUDIO_STOCK_LINUX.md). A new internal `gr-usbip` crate
provides allocation-free, bounded data-phase framing and completion encoding with
synthetic tests. The worker reuses workspace audio/HID/wire contracts and the lab
example reuses curated controller personalities; this adds no external packages.
It is not yet an enabled provider.

The worker bounds USB frames, pending requests, packet counts, storage and deadlines.
Tests cover ordered control requests after slot reuse, exact HID success/stall
replies, capture transfers larger than 4 KiB, playback overflow/discontinuities,
unlink/sequence reuse, and terminal shutdown of both sample directions.
The unprivileged process harness passes descriptor, personality, playback and
patterned microphone checks for all three families. These checks simulate the
host socket; they are not kernel, ALSA, latency or sustained acceptance.

The explicit administrator-run VHCI lab harness starts the worker after dropping
UID/GID/supplementary groups, applies no-new-privileges and resource limits, bounds
readiness/lifetime/output, and shuts down its owned socket for cleanup. It never
detaches by a reusable port number. This is a lab prerequisite, not a deployed
production broker or a persistent privilege grant. The host module preparation
was previously completed but VHCI was absent at the latest inspection; the next
live run must reload it.


The subsequent administrator-run DualSense VHCI probe successfully enumerated HID
and UAC2 audio on stock Linux, passed short plus two 60-second duplex ALSA trials,
and removed its USB/ALSA resources on timeout. See [EXP-0025](experiments/EXP-0025-local-usbip-audio.md)
for exact evidence and the remaining limits. It does not enable the public USB API.
The latest DS4 PipeWire trial still failed: 7,168 queued playback frames lost,
3,072 unexpected frames (2,048 all-zero), and one discontinuity over 60 seconds.
That failed sustained acceptance remains open alongside the earlier observations.


Native DualSense output now exposes a validity-gated `microphone_muted` observation,
independent of the mute LED, while preserving the complete payload. The mapping is
supported by the [Linux v6.12.107 PlayStation driver](https://github.com/gregkh/linux/blob/v6.12.107/drivers/hid/hid-playstation.c).
It reports a host request; it does not automatically transform PCM. Gain, routing,
other audio bytes and physical mute behavior still require their own evidence and
application/transport integration.
