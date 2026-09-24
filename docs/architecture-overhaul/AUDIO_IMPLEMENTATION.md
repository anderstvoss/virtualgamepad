# Controller audio execution ledger

Baseline: GUI PR #110, `476056a`. Audio is newly authorized pre-alpha work; final
API review, quality review and release still follow audio and later GUI feedback.

Historical checkpoint (2026-09-23; later updates below supersede it): the installed USB broker/worker passed an
ordinary-user mixed-session, quota, client-death and idle-client worker-death
isolation probe. The USB sample-access harness passed three consecutive
60-second full-duplex trials for DualSense and Xbox360 at an 8 ms microphone
fill, and DS4 at a 12 ms fill. Earlier failed continuity runs are preserved in
the [USB evidence ledger](USB_AUDIO_STOCK_LINUX.md#current-acceptance-ledger-2026-09-24).
This establishes neither directional end-to-end p99 nor sustained native-client
and root USB acceptance. The root USB API and native-client bridge now compile
and pass deterministic tests; short isolated PipeWire transfers pass in both
bridge directions. Physical DualSense headset L/R and grip L/R routing were confirmed;
the onboard speaker was silent with the headset removed, and two microphone
captures had near-zero signal. The physical DualSense is now disconnected.
The maintainer requires confirmation before any additional physical DualSense
run. Matching remains unavailable.

Physical update (2026-09-24): the maintainer again confirmed left/right
headphones and left/right grips, and clearly heard the onboard speaker after a
bounded audio-only HID route selected it. The probe restored the headphone
route; the prior speaker volume/preamp values cannot be read back on this
kernel. Headset and built-in microphone captures both remained near the noise
floor despite speech, so microphone response and routing are unaccepted.
The [physical evidence](experiments/EXP-0023-controller-audio.md#audio-only-speaker-route-and-speech-follow-up-2026-09-24)
records the exact scope. Further interactive physical runs still require
maintainer readiness; matching remains unavailable.

Further authorized physical checks compared headset capture at ALSA gain 31
and 101, then compared quiet and speech windows at gain 101 with an audible
timing cue. Gain returned to 31 after each run; speech did not produce a
measurable change from quiet at maximum gain. A physical USB unplug/replug
removed and restored its audio/HID interfaces, and a fresh left-headphone
tone was heard afterward. Microphone response and controller matching remain
unaccepted; see the same EXP-0023 record for exact aggregates and limits.

A later maximum-gain built-in-microphone comparison, with the headset removed
and a left-grip timing cue, showed a small left-channel speech-versus-quiet
RMS increase in one run. The right channel remained flat. This does not close
microphone fidelity or headset-routing acceptance.

Update (2026-09-24): the rebuilt installed worker passed short ordinary-caller
USB duplex checks for all three families, and one 60-second DualSense
sample-access trial passed exactly after the client playback queue gained
bounded spare capacity and the example refilled microphone credit promptly.
The lower-fill trials and a 16 ms-fill p99 21.766 ms trial still failed.
Native playback in a private PipeWire graph missed marked frames both in
debug and optimized tests without USB/IP, the broker or worker; the
native-microphone graph direction passed one short trial. The native-client
and simultaneous directional latency gates therefore remain open, with
native-host qualification tracked in
[issue #112](https://github.com/anderstvoss/virtualgamepad/issues/112).
No physical DualSense run was performed in this update.

After VHCI reload, ordinary-caller root USB creation passed for all three
families in sample and native-client mode. Short root full-duplex sample trials
also passed for all three. Sustained root DualSense attempts exposed 32
measured microphone-silence frames at both 12 and 16 ms operating fill, and
one worker termination. Direct broker IPC at 16 ms fill passed one 60-second
DualSense trial without measured-interval silence, so the extra root
sample/control path remains under investigation. Root audio now requests
host-frame progress, including silence, in one versioned worker exchange;
the matching worker must be installed before further root live testing. No
sustained root or native-client matrix is claimed complete.

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
The installed broker has passed ordinary-client attachment and scoped lifecycle
probes; the complete security matrix and post-reboot recovery acceptance remain
open. The installer now provisions stock VHCI module loading at boot. This
host's earlier `service-start-limit-hit` was cleared by the maintainer, and
the installed socket passed subsequent ordinary-caller creation checks.

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
| Hardened broker / USB prerequisite | Installed fixed-profile broker passed scoped ordinary-client lifecycle checks; boot loading is provisioned; remaining security/restart acceptance pending |
| USB composite/UAC2 | Local USB/IP worker, root USB realization, sample API and native-client bridge implemented; deterministic and scoped live checks pass; root full-duplex/live latency acceptance pending |
| DualSense physical lab/matching | UAC1 4-out/2-in at 48 kHz S16_LE and headset/haptic channel mapping observed; all five output routes confirmed; microphone fidelity unresolved; matching unavailable |
| Separate GUI integration | Subsequent maintainer-directed branch/PR |

Emulated audio is available for the three scoped families on `LINUX_UHID_USB`
with the opt-in `audio-pipewire` feature. The `audio-usbip` feature exposes
`LINUX_USBIP_USB_AUDIO` for installed-broker USB audio, with `audio-pipewire`
also required for native-client ownership. This is pre-alpha and full live
acceptance remains open. Unsupported combinations fail before controller I/O.
Controller-matching remains unavailable.
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
these assumptions. The optional production API is now implemented for explicit
USB/IP selection, but full live transport and broker security acceptance remain
release blockers.

## Current validation boundary

Short sample/native-client checks and root-only three-family lifecycle checks pass.
Earlier DS4/Xbox sustained PipeWire trials failed with unexpected silence and/or
queue loss, and one DS4 run showed host graph scheduling errors. Later installed
USB sample-access full-duplex continuity trials passed at the operating fills
recorded above. The earlier failures remain evidence, not waived acceptance.
Full-duplex/native sustained acceptance, steady-state latency,
full clock-domain association and interpreted controller audio-control state remain
open. `stream_timings()` provides retained stream-local graph ticks, rate, monotonic
observation time and graph discontinuity counters. Graph delay is an estimate,
not measured end-to-end latency; graph loss and PCM queue loss are separate.

A fresh isolated 60-second `pw-cat` playback soak on all three families
observed 512–1,024 all-zero frames around 10 seconds, with no queue loss
reported under the old handling. An EMPTY PipeWire chunk now advances the
playback frame position as explicit loss; a deterministic regression prevents
EMPTY chunks from being presented as valid silent samples. The cause of the
specific `pw-cat` zero blocks still needs correlation. That
correction improves diagnostics but does not make the failed soak a pass.
Graph-driven source trials isolate whether the gap originates in the external
client or controller endpoint. A 62-second graph-driven DualSense trial at
512 frames missed 7,168 marker frames; at 256 frames it missed 29,952. The
256-frame trial reported zero application-queue and position gaps while graph
clock observations reported missed cycles. Thus merely lowering the graph
quantum does not establish continuity on this prepared VM; the lost markers
are upstream of the root sample queue. Both failed runs are retained, and
neither supports a sub-20 ms accepted path despite low p99 among received
markers.

The broker now admits authenticated connections and sessions under shared per-UID
and global quotas, rejects/closes unsolicited descriptors, bounds partial-frame
read time and reply writes, and terminally closes clients after framing/I/O failure.
These checks are prerequisites only; they do not establish complete USB broker
security acceptance or permit new composite operations.

The headless root-only `controller_audio` example prints endpoint metadata, drains
playback, services HID and reports retained teardown diagnostics. The
`usb_audio_root` smoke example exercises root USB creation; the
`usb_audio_acceptance` example pairs root sample access with an owned-ALSA
full-duplex checker. Their sustained installed-broker result is pending. A
separate root-only consumer checked successfully from Git revision `58fb8ae`
with a fresh initial dependency fetch and then an offline rebuild after caching.

## Stock Linux revision

The maintainer requires stock Linux. The revised candidate is a local USB/IP VHCI
transport with a trusted unprivileged device worker, described in
[USB_AUDIO_STOCK_LINUX](USB_AUDIO_STOCK_LINUX.md). A new internal `gr-usbip` crate
provides allocation-free, bounded data-phase framing and completion encoding with
synthetic tests. The worker reuses workspace audio/HID/wire contracts and the lab
example reuses curated controller personalities; this adds no external packages.
It now backs the opt-in root USB realization.

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
detaches by a reusable port number. This helper remains separate from the
installed production broker. The installer provisions VHCI module loading at
boot. After the latest reboot, VHCI was loaded manually and the maintainer
reset the failed socket. The updated worker protocol still needs installation.


The subsequent administrator-run DualSense VHCI probe successfully enumerated HID
and UAC2 audio on stock Linux, passed short plus two 60-second duplex ALSA trials,
and removed its USB/ALSA resources on timeout. See [EXP-0025](experiments/EXP-0025-local-usbip-audio.md)
for exact evidence and the remaining limits. It predates public USB integration.
An earlier DS4 PipeWire trial failed: 7,168 queued playback frames lost,
3,072 unexpected frames (2,048 all-zero), and one discontinuity over 60 seconds.
That failed sustained acceptance remains open alongside the earlier observations.


Native DualSense output now exposes a validity-gated `microphone_muted` observation,
independent of the mute LED, while preserving the complete payload. The mapping is
supported by the [Linux v6.12.107 PlayStation driver](https://github.com/gregkh/linux/blob/v6.12.107/drivers/hid/hid-playstation.c).
It reports a host request; it does not automatically transform PCM. Gain, routing,
other audio bytes and physical mute behavior still require their own evidence and
application/transport integration.

## Cleanup and buffer-validation follow-up

Ordinary registration/backend failures now explicitly disconnect every created
endpoint and retain cleanup errors alongside the initial cause. Drop remains the
unwinding fallback. Interleaved playback validates byte bounds, frame alignment
and stride before queue delivery; missing mapped buffers terminate the backend.
Regression tests cover malformed layouts and multiple cleanup failures. The
headless root example prints retained graph timing after closure.

At `9dc68a0`, the longer four-path DualSense check passed exact continuity with
p99 1.087–11.476 ms (maximum 15.007 ms). See EXP-0023 for distributions and scope.
Three-trial/full-duplex/mixed-session acceptance and root USB live validation
remain open.

## Remaining production closure sequence

Continue on the current branch with tested local commits. The installed worker,
strict descriptor handoff, staged cleanup and root ordinary-caller realization
are implemented. The remaining exit is measured live acceptance, including
recovery after the host reboot.

1. **Daemon attachment and recovery.** The version-two connection handler,
   journal, staged worker owner and fixed-profile handoff are implemented.
   Recheck installed restart recovery and remaining setup/death boundaries.
   Unverifiable attachments must remain untouched and reported.
2. **Installation.** The explicit administrator installer and fixed worker are
   implemented, including dedicated UID/GID and boot VHCI module loading. Verify
   installed restart recovery and remaining privilege/crash boundaries on this
   prepared host. Module loading and permission grants remain outside creation.
3. **Application integration.** The optional explicit USB audio realization,
   worker state/output client, bounded PCM pumps and native-client bridge are
   implemented. Run the root-only full-duplex consumer against the installed
   broker, then resolve any live endpoint or terminal-diagnostic defects.
4. **Reliability acceptance.** Run the full family/transport/access matrix: three
   consecutive 60-second full-duplex trials after readiness, exact patterns and
   channel isolation, no unexplained loss, and measured p99 below 20 ms separately
   in each direction. Include mixed/duplicate sessions, HID load and bounded stall
   recovery. Preserve failures and measurement boundaries.
5. **Physical comparison and handoff.** Complete DualSense microphone selection,
   mute/gain, routing and reconnect observations with controls restored and no raw
   recordings retained. Preserve the confirmed four-channel mapping. Matching
   remains unavailable until comparison acceptance; GUI/API/quality/alpha remain
   subsequent gates.

The private version-two connection handler has deterministic socket tests for
malformed operations, admission before setup, setup/handoff rollback, stale
requests, version changes, disconnects and independent same-UID clients. A
connection owns at most one audio session and cannot name another connection's
resources. It is now connected to staged privileged VHCI attachment and the explicit
administrator installer. Ordinary-user installed probes have passed creation,
native updates and closure for all three families, plus duplicate/mixed sessions,
port exhaustion, independent removal, recreation, client death and worker death.
These results cover parts of steps 1 and 2, not the complete security acceptance
matrix. Restart recovery remains fail-closed for unverifiable journal records.

The latest lifecycle correction releases reservations on worker death even when
the client retains an idle broker connection. Its deterministic regression and
updated installed isolation probe passed. The earlier consumption-based
microphone refill attempts failed after two successful trials; later retests
and their operating-fill conditions are recorded in the
[current evidence ledger](USB_AUDIO_STOCK_LINUX.md#current-acceptance-ledger-2026-09-24).
The physical DualSense is disconnected. Further interactive physical tests
require a new confirmation from the maintainer before they begin.

An unprivileged `SampleStreams` pump now handles the production worker's two
anonymous PCM channels independently of application polling. It uses fixed
capacity queues and preallocated packet buffers, preserves frame positions and
discontinuities, reports slow-reader loss, accepts partial microphone writes,
offers explicit per-direction flushing and retains a terminal channel error.
Deterministic three-family round-trip, slow-reader overflow and worker-death
tests pass. Root USB creation and native-client bridging now use this building
block; their sustained live acceptance and directional latency measurements
remain open.

The application endpoint descriptor now uses typed `PipeWireNode` and `AlsaPcm`
selectors and reports its stream-group and clock-domain identities. The previous
generic node-string getters were migrated in the root-only example. `AlsaPcm`
is resolved from current owned VHCI ancestry; it does not imply that the broker
has passed its remaining security and latency acceptance.

The reverse PipeWire sample-access harness now paces its microphone producer
against consumed queue frames. A free-running nominal-rate timer accumulated
1,100–1,400 queued frames and yielded 25–52 ms latency despite spare queue
capacity being intended only for stalls. The first queue-paced version capped
fill below the 512-frame callback size. Its exact-marker, 18.5–18.9 ms p99
short runs for the three families were misleading: eight seconds of samples
took about sixteen wall-clock seconds. The harness now communicates the lab
quantum to its producer, keeps one graph block ready, reports wall-clock rate
and rejects delivery below 90% of nominal real time. At 512 frames, a DualSense
run took 8.1 seconds but missed 512 markers and had 20.7 ms p99. At 256 frames,
it missed 3,072 markers with 13.3 ms p99. Graph timing reported discontinuities
in both runs. Neither path meets the combined continuity, throughput and p99
target. Sustained real-time throughput, full-duplex latency and both USB access
modes remain open.

The graph-driven native-client bridge also remains unaccepted. On the private
512-frame graph, one DualSense microphone run missed 512 measured marker frames
with zero bridge-queue underruns; both endpoint timing records showed graph
discontinuities and 1,024 missed graph frames. The playback-direction run also
missed 512 markers, while the caller source reported 1,536 underrun frames and
both nodes reported graph discontinuities. Delivered-frame p99 was below 20 ms
in these runs, but a latency percentile over surviving frames does not cancel
the continuity failure. The native graph harness now emits endpoint timing and
underrun counters with each result.

The USB native caller bridge now publishes its PipeWire graph timing and a
distinct caller-playback source-underrun count through the root audio API. The
headless `usb_audio_native_probe` uses only root controller creation and typed
endpoint selectors, and checks ALSA microphone capture together with a native
PipeWire playback capture. Short runs for all three families passed the measured microphone pattern
but failed strict playback continuity because interior silence remained. This
is a measured native-client failure, separate from the nine successful root
USB sample-access trials recorded in the stock-Linux ledger.
