# Initial Linux controller audio

Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.

This is an opt-in alpha implementation with bounded functional acceptance, not
a completed fidelity or latency qualification. The ordinary-root API review and
110-item snapshot are complete. Existing constructors keep audio disabled by
default. Current dispositions are in the [alpha remediation status](history/alpha-1/ALPHA_REMEDIATION_STATUS.md).

| Family | Emulated playback / microphone channels | Controller matching |
| --- | --- | --- |
| DualSense | 4 synchronized channels: audible L/R, haptic L/R / 2 channels | Unavailable |
| DualShock 4 | Stereo / mono | Unavailable |
| Xbox360 standard HID | Stereo / mono | Unavailable; no proprietary XInput audio |

These profiles use interleaved signed 16-bit samples at 48 kHz. DS4 and Xbox
profiles are functional approximations. DualSense channel roles have physical
evidence, but its emulated UAC2 topology is not the physical UAC1 topology.

## Pinned evidence boundaries (Brief 16 R1)

The evidence crosswalk below applies to the compiled functional profiles and
HID paths at [the R0/R1 audit base](https://github.com/anderstvoss/virtualgamepad/commit/324d250bb01a5ddbfc1dd189513d4c9160429ee1), using
the adopted research revision [`a67781e`](https://github.com/anderstvoss/controller-protocol-research/commit/a67781e2412eceea9440d20dbb350ad5d3352c25).
It supplements source-generic implementation comments without changing code,
wire bytes, or the research gitlink. Source-supported consumer behavior is not
device-side measurement.

| Implementation surface | Pinned claim records and limits |
| --- | --- |
| `gr-curated-controllers/src/audio.rs`, DualSense functional channel grouping | [audio-dualsense-alsa-ucm-roles](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-alsa-ucm-roles.json) describes the ALSA UCM mapping; [audio-dualsense-observed-playback-order](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-observed-playback-order.json) cites the historical maintainer's qualitative grip/headphone observation. Neither quantifies isolation or microphone fidelity. |
| DualSense HID output and route/mute/level intent | [audio-dualsense-hid-framing](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-hid-framing.json), [audio-dualsense-hid-levels](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-hid-levels.json), [audio-dualsense-hid-routing](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-hid-routing.json), [audio-dualsense-hid-preamp-mute](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-hid-preamp-mute.json), [audio-dualsense-kernel-jack-detect-routing](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-kernel-jack-detect-routing.json), [audio-dualsense-haptics-consumers](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-haptics-consumers.json). The [Linux consumer, routing and mute](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L1361-L1429) and [USB jack detection](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/hid/hid-playstation.c#L1529-L1552) describe host intent, not PCM delivery. [audio-dualsense-bluetooth-inband-pcm-unknown](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-bluetooth-inband-pcm-unknown.json) remains unknown. |
| DS4 functional stereo/mono profile | [audio-dualshock4-hid-output](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualshock4-hid-output.json) and [audio-dualshock4-sdl-audio-fields](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualshock4-sdl-audio-fields.json) describe different consumer effect layouts, not a PCM path. [audio-dualshock4-usb-audio-unknown](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualshock4-usb-audio-unknown.json) and [audio-dualshock4-bluetooth-inband-audio-unknown](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualshock4-bluetooth-inband-audio-unknown.json) prohibit physical topology or Bluetooth audio inference. No dedicated onboard-speaker matching is established. |
| Xbox360 functional profile and Xbox Series refusal | [audio-xbox360-xpad-no-headset-audio](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-xbox360-xpad-no-headset-audio.json) and [audio-xbox-series-xpad-disables-gip-audio](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-xbox-series-xpad-disables-gip-audio.json) concern the Linux xpad consumer; [its GIP startup path](https://github.com/torvalds/linux/blob/4056b3c9d3c92ea45955641dbafeb9b960e146ef/drivers/input/joystick/xpad.c#L1494-L1505) does not implement headset streaming. The standard-HID Xbox360 approximation is not proprietary headset transport; Series has no declared audio profile. |
| USB/IP functional UAC2 profile (`gr-usbip`) | [audio-dualsense-usb-uac1-topology](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-dualsense-usb-uac1-topology.json) reports a maintainer-documented UAC1 reference, not a corpus-controlled descriptor experiment. The compiled UAC2 profile is deliberately not a UAC1 match. DS4/Xbox360 physical descriptor matching is likewise unqualified. |
| Independent UHID/PipeWire nodes | [audio-consumer-sidecar-hid-sound-association-unknown](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-consumer-sidecar-hid-sound-association-unknown.json) and [audio-consumer-proton-steam-audio-association-unknown](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-consumer-proton-steam-audio-association-unknown.json) remain unknown; independent nodes do not imply automatic consumer routing. |
| Switch Pro, Wii and Steam | [audio-switch-pro-hid-consumers-no-audio](https://github.com/anderstvoss/controller-protocol-research/blob/a67781e2412eceea9440d20dbb350ad5d3352c25/records/claim/audio-switch-pro-hid-consumers-no-audio.json) distinguishes native HD rumble from PCM. Wii speaker playback is refused pending S1; see [Wii Remote](WII_REMOTE.md). Steam's opt-in in-band PCM is not UAC or MIDI enumeration; see [Steam Haptics](STEAM_HAPTICS_STREAMING.md). |

The source lineages include ALSA UCM configuration, Linux and SDL consumer
implementations, and a VirtualGamepad maintainer narrative; these are not
independent physical experiments. No sustained audio, native matching, or host
test was run for this documentation audit.

## Reproducing the real USB topology

The **Generic**, **Native HID** and **Real USB** level labels describe the host
presentation, not proof of controller equivalence, sustained audio, physical
actuation or application routing. Real radio is reserved. The existing
realization IDs remain unchanged; acceptance manifests record these levels in
`reason` text only until #408 defines a schema field.

| Level and realization | Topology the host receives | Audio/haptic boundary |
| --- | --- | --- |
| **Generic** — `linux.uinput` | Generic evdev input/FF nodes, not a USB HID/UAC composite. | Conventional force feedback where declared; no emulated speaker, microphone or USB audio endpoint. |
| **Native HID** — `linux.uhid.usb` plus opt-in `audio-pipewire` | Controller-specific HID on the kernel USB *bus type*, plus independent caller-session PipeWire playback/capture nodes. The sound nodes do **not** inherit the HID device's USB ancestry. | The application may explicitly select the associated sound nodes; automatic HID–sound routing by SDL, Steam or Proton is **unknown**, not guaranteed. |
| **Native HID** — `linux.uhid.bluetooth` | Controller-specific UHID on the Bluetooth *bus type*; no physical Bluetooth radio or USB audio composite is created. | Native rumble and controller in-band speaker commands are separate from UAC audio. Wii speaker playback is still refused; Bluetooth-bus metadata do not imply radio pairing. |
| **Real USB** — `linux.usbip.usb-audio` plus `audio-usbip` | A USB/IP/VHCI device with **one composite USB parent**, HID interface and UAC2 audio-control/playback/capture interfaces; the Linux sound subsystem identifies its ALSA card by that VHCI USB ancestry. | Needs the provisioned root broker and unprivileged worker. Optional `audio-pipewire` supports NativeClient bridging. This is **functional UAC2**, not a physical controller descriptor match; the DualSense reference described by the research corpus is UAC1. |
| **Real USB (experimental)** — `linux.dummy_hcd.usb-hid` | Experimental USB HID presentation without the compiled USB/IP UAC2 audio function. | Does not reproduce the HID-plus-audio composite and remains `not-supported` in the affected acceptance cells. |

For the USB/IP descriptor layout, see the
[compiled UAC2/HID configuration](https://github.com/anderstvoss/virtualgamepad/blob/62f98b628839d173e3b894111544eea6b62163a7/crates/gr-usbip/src/profile.rs#L158-L176);
for Linux's USB-parent/ALSA-card check, see the
[VHCI sound-card resolver](https://github.com/anderstvoss/virtualgamepad/blob/62f98b628839d173e3b894111544eea6b62163a7/crates/gr-root-audio/src/usb_audio.rs#L193-L233).
[ADR-0017](https://github.com/anderstvoss/virtualgamepad/blob/62f98b628839d173e3b894111544eea6b62163a7/docs/adr/ADR-0017-primary-realization-and-audio-components.md#L9-L24)
defines the distinction between associated PipeWire components and a single USB
composite. The controller-native Steam haptic PCM/MIDI commands and Wii speaker
protocol are **in-band HID** subsystems, not standalone USB Audio Class or USB
MIDI interfaces; see
[ADR-0023](https://github.com/anderstvoss/virtualgamepad/blob/62f98b628839d173e3b894111544eea6b62163a7/docs/adr/ADR-0023-controller-subsystems.md#L8-L27) and
[ADR-0024](https://github.com/anderstvoss/virtualgamepad/blob/62f98b628839d173e3b894111544eea6b62163a7/docs/adr/ADR-0024-host-output-stream.md#L85-L96).

The pinned research claims
[`audio-consumer-sidecar-hid-sound-association-unknown`](https://github.com/anderstvoss/controller-protocol-research/blob/4684fce0dc2dd993dd8c3e8846c86c537fdcf7e1/records/claim/audio-consumer-sidecar-hid-sound-association-unknown.json)
and
[`audio-consumer-proton-steam-audio-association-unknown`](https://github.com/anderstvoss/controller-protocol-research/blob/4684fce0dc2dd993dd8c3e8846c86c537fdcf7e1/records/claim/audio-consumer-proton-steam-audio-association-unknown.json)
do **not** establish automatic association or routing of independent HID and
sound nodes. Neither does common USB ancestry alone prove how a proprietary
Steam client selects an audio endpoint. The
[`audio-dualsense-usb-uac1-topology`](https://github.com/anderstvoss/controller-protocol-research/blob/4684fce0dc2dd993dd8c3e8846c86c537fdcf7e1/records/claim/audio-dualsense-usb-uac1-topology.json)
claim cites a historical physical-reference description, not a new controlled
descriptor experiment.

**Proposed host checks — not run in this documentation change:** On a separately
provisioned Linux host, inspect the USB/IP device's interface descriptors and
common VHCI parent, then resolve ALSA playback/capture endpoints and exercise
both directions with bounded markers. Independently enumerate UHID and
PipeWire sidecar nodes, confirm their separate ancestry, and test explicit
selection versus automatic selection in each targeted SDL/Steam/Proton consumer.
Record successes, failures, disconnect cleanup and permissions per realization
and ownership mode. None of these proposed checks are acceptance evidence.

## Choose the presentation at creation

- `LINUX_UHID_USB` with `audio-pipewire`: HID plus associated PipeWire sink/source
  nodes in the caller's audio session. Association does not imply USB ancestry.
- `LINUX_USBIP_USB_AUDIO` with `audio-usbip`: a local USB composite HID/UAC2 device
  through stock Linux VHCI. An administrator provisions the broker and dedicated
  unprivileged worker first. Native-client ownership additionally requires
  `audio-pipewire` for the caller-session bridge.

There is no automatic fallback, module loading, physical microphone access or
physical speaker connection. Callers explicitly connect native endpoints.
Changing topology or sample ownership requires close and recreation.

```rust,no_run
use virtualgamepad::{
    AudioExposure, AudioOptions, CreationOptions, DualSenseOptions, RealizationId, create_dualsense,
};

let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO)
    .with_audio(AudioOptions::new(AudioExposure::Emulated));
let mut controller = create_dualsense(options, DualSenseOptions::default())?;
let audio = controller.audio().expect("enabled audio");
let mut playback = [0_i16; 128 * 4];
let read = audio.read_playback(&mut playback)?;
// Process read.frames complete frames; inspect its positions/discontinuity.
let accepted = audio.write_microphone(&[0_i16; 128 * 2])?;
// Retry any unwritten suffix: accepted counts frames, not individual samples.
controller.close();
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Samples` is the default ownership. `AudioOptions::with_playback_access` and
`with_microphone_access` can instead select `AudioAccess::NativeClient` per
stream group. Library sample methods reject access to native-owned groups.
`audio().endpoints()` exposes direction, format, group, clock identity and typed
PipeWire/ALSA selectors. Resolve them for each creation; never remember ALSA card
numbers. Host-to-controller means playback/haptics; controller-to-host means
microphone return. The synchronized DualSense audible/haptic channels stay together.

Audio and required USB HID replies run independently of caller polling. Continue
controller servicing for native output observations. Buffers are bounded; writes
may be partial, underruns supply silence, and loss is counted. Flush audio
explicitly; input neutralization does not flush it. Closure invalidates sample
access and removes owned endpoints while retaining diagnostics. Graph timing is
not an end-to-end latency measurement. Emulated PCM preserves raw sample values;
it does not apply HID volume/routing controls a second time or claim matching
physical control behavior.

## Cooperative Native HID sidecar association

For `linux.uhid.usb`, PipeWire audio endpoints are separate session-graph
nodes, not USB audio interfaces. An opt-in cooperating application can correlate
them with the owning HID presentation using VirtualGamepad association metadata;
stock SDL, Steam and WirePlumber do not perform this association. Automatic
HID/audio device association requires the Real USB realization
(`linux.usbip.usb-audio`) rather than PipeWire properties alone.

The borrowed `controller.audio()?.sidecar_association()` accessor is available
only for Native HID/PipeWire audio; it returns the stable-per-creation
`AudioSidecarAssociation::key()` and exact `node_names()`. It does not return
PipeWire numeric node IDs, which are assigned by the graph and can change.
All host and NativeClient caller nodes carry the same key.

A cooperating client enumerates its current PipeWire graph, looks up nodes
whose `virtualgamepad.association.key` equals the accessor value, and connects
them using the appropriate declared `media.class` (sink or source). Each
registered node publishes:

- `device.vendor.id` and `device.product.id` (hexadecimal USB HID IDs),
  `device.product.name` (owning HID product name), and `device.serial`
  (the **exact UHID uniq**, not a manufactured physical device serial).
- `virtualgamepad.realization.id` (`linux.uhid.usb`) and
  `virtualgamepad.association.key` (creation-scoped shared correlation key).
- Existing `node.name`, `media.class`, and `node.virtual` properties.

The key differs for distinct creations, including a recreation with restored
HID identity. Graph names and property metadata neither attach a Linux sound
card to the HID input nor grant physical USB ancestry. Do not persist the key
or use it as a controller's long-term hardware identity.

**Live host check plan — not run:** on an explicitly provisioned Linux test
host, create two UHID controllers with PipeWire audio, enumerate current nodes
with `pw-dump`, verify every sink/source and optional caller node has the
expected matching key, HID uniq, VID/PID and direction; verify sibling isolation,
close/removal and recreation with a changed key. Then test a cooperating client
routing playback and microphone explicitly. This agent has no live host and
reports no host, physical, SDL, Steam or WirePlumber acceptance result.

## Build and provision

PipeWire builds need its development headers, SPA headers, pkg-config and libclang
for bindgen. Direct ALSA acceptance examples also need ALSA development headers.
On Debian these are provided by `libpipewire-0.3-dev`, `libspa-0.2-dev`,
`pkg-config`, `libclang-dev` and `libasound2-dev`.

For UHID, use the existing [UHID access setup](DEPLOYMENT_AND_VALIDATION.md) and
a running caller-session PipeWire server. For USB, follow the
[explicit broker installation procedure](history/alpha-1/USB_AUDIO_STOCK_LINUX.md#explicit-audio-broker-installation-and-ordinary-user-probe).
Administrator-selected VHCI ports must be reserved for this broker. Provisioning
is separate from ordinary-user controller creation. The lab executable is not the
installed production worker. Do not grant broad sound-device or sudo access.

Root-API examples (these create virtual devices when run):

```sh
cargo run --features audio-pipewire --example controller_audio -- dualsense 10
cargo run --features audio-usbip --example usb_audio_root
cargo run --features audio-usbip,audio-pipewire --example usb_audio_root -- native
```

## Acceptance and remaining gates

Deterministic coverage includes queue bounds, partial transfers, state retries,
exact HID acknowledgements, malformed IPC, descriptor rejection, connection-owned
lifetime and rollback. Recorded live tests demonstrate ordinary-caller creation
and selected streaming/lifecycle paths. They do not complete the sustained matrix
or physical fidelity. Separately, seven scoped installed broker security/recovery
phases passed (#115 closed); this is not streaming qualification.

Three consecutive 60-second full-duplex trials per family, transport and ownership
mode, with exact patterns and directional p99 below 20 ms, remain a post-alpha
closure target (#135/#145), not a bounded alpha requirement. Recorded
native-client continuity and simultaneous latency failures persist on the
Parallels test VM; isolated PipeWire failures justify native-host qualification,
but do not prove the VM is the sole cause. Follow
[closed issue #112](https://github.com/anderstvoss/virtualgamepad/issues/112),
whose closure does not supply the missing native-host matrix.

Physical DualSense headphone L/R, grip L/R, explicit onboard-speaker routing and
USB reconnect playback are observed. Meaningful headset-microphone response,
microphone routing/fidelity and physical-versus-virtual equivalence remain open.
Matching profiles stay unavailable. Further physical tests require operator
readiness; adaptive-trigger actuation is excluded.

The [execution ledger](history/alpha-1/AUDIO_IMPLEMENTATION.md),
[USB acceptance record](history/alpha-1/USB_AUDIO_STOCK_LINUX.md) and
[physical evidence](history/alpha-1/experiments/EXP-0023-controller-audio.md)
retain successful and failed runs. GUI integration, API review and the separate
quality review have landed. Broader hands-on feedback is post-alpha #126;
exact release-SHA qualification and authorized tagging remain outstanding.


## Current candidate API

Component kinds distinguish input/audio without role-string parsing. Use
`pipewire_node()` or `alsa_pcm()` for creation-scoped endpoint selection.
`AudioRead` belongs to the root contract; incomplete interleaved buffers return
`InvalidSampleBuffer`, native-client-owned sample calls return `OwnershipMismatch`,
and closed sessions return `Closed`. Matching has no public exposure variant.
`diagnostics()` snapshots retained health/loss. Graph timing and scheduling require
`experimental::audio_instrumentation(audio)` and are outside the alpha contract.
`microphone_consumed_frames()` remains optional sample-pacing progress, including
silence; it is not proof that canceled USB transfers reached a host consumer.

Direction ownership applies to every exposed group in that direction. Keep audio
borrows and PCM batches short when sharing a controller owner between threads.
HID readiness/deadlines still require caller service; PCM is worker-backed, and
service detects required audio failure without promising a fixed polling cadence.
USB/IP remains opt-in WIP despite the seven scoped broker security/recovery
phases passing (#115 closed); streaming and full routing remain deferred.
See [migration](ALPHA_API_MIGRATION.md) and the
[current remediation status](history/alpha-1/ALPHA_REMEDIATION_STATUS.md).

## Current release evidence boundary

The remediation native playback smoke test still loses synthetic markers. Graph
discontinuities, queue drops and producer/client accounting are reported
separately; timing observations do not explain away missing frames. No acceptance
assertion is relaxed. Three measured 60-second trials per sustained cell and
duplex/mixed ownership remain post-alpha requirements (#143/#145).
VM observations cannot establish native
host latency or physical microphone fidelity. See
[the remediation status](history/alpha-1/ALPHA_REMEDIATION_STATUS.md).

Host-device discovery belongs to the GUI coordinator, with one active command
and one coalesced refresh, generation filtering, a two-second per-command deadline
and 256 KiB output limit. Initialization, backend changes and the explicit refresh
button initiate discovery; controller service does not wait for it. Errors remain
visible until refresh. Demo defaults enable ALSA and WIP USB/IP; add PipeWire
explicitly. Library defaults remain empty.
