# Controller audio execution ledger

Baseline: GUI PR #110, `476056a`. Audio is newly authorized pre-alpha work; final
API review, quality review and release still follow audio and later GUI feedback.

## Contract

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
| Creation policy, profiles, bounded transport | Implemented, validation in progress; no backend is enabled by these contracts alone |
| UHID/PipeWire | Pending: real sample flow, native-client endpoints, lifecycle and headless acceptance |
| Hardened broker/FunctionFS | Pending: connection ownership, quotas, deadlines, restricted resources, exact requests, failure cleanup |
| USB composite/UAC2 | Pending: actual enumeration, PCM flow, HID coexistence, independent removal |
| DualSense physical lab/matching | Pending physical attachment and descriptor/audio experiments |
| Separate GUI integration | Subsequent maintainer-directed branch/PR |

No audio exposure is currently advertised as working. Root constructors reject
non-disabled audio before any controller I/O until an implemented backend enables
an exact combination. Supporting workspace profile/queue APIs are SPI, not root
application factories.

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
