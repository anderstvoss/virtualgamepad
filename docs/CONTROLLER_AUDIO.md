# Initial Linux controller audio

This is an opt-in pre-alpha implementation, not a completed fidelity or latency
qualification. The intended API still needs the fresh API review after GUI
integration. Existing constructors keep audio disabled by default.

| Family | Emulated playback / microphone channels | Controller matching |
| --- | --- | --- |
| DualSense | 4 synchronized channels: audible L/R, haptic L/R / 2 channels | Unavailable |
| DualShock 4 | Stereo / mono | Unavailable |
| Xbox360 standard HID | Stereo / mono | Unavailable; no proprietary XInput audio |

These profiles use interleaved signed 16-bit samples at 48 kHz. DS4 and Xbox
profiles are functional approximations. DualSense channel roles have physical
evidence, but its emulated UAC2 topology is not the physical UAC1 topology.

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
    AudioExposure, AudioOptions, CreationOptions, RealizationId, create_dualsense,
};

let options = CreationOptions::new(RealizationId::LINUX_USBIP_USB_AUDIO)
    .with_audio(AudioOptions::new(AudioExposure::Emulated));
let mut controller = create_dualsense(options)?;
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

## Build and provision

PipeWire builds need its development headers, SPA headers, pkg-config and libclang
for bindgen. Direct ALSA acceptance examples also need ALSA development headers.
On Debian these are provided by `libpipewire-0.3-dev`, `libspa-0.2-dev`,
`pkg-config`, `libclang-dev` and `libasound2-dev`.

For UHID, use the existing [UHID access setup](DEPLOYMENT_AND_VALIDATION.md) and
a running caller-session PipeWire server. For USB, follow the
[explicit broker installation procedure](architecture-overhaul/USB_AUDIO_STOCK_LINUX.md#explicit-audio-broker-installation-and-ordinary-user-probe).
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
or the complete privileged security/recovery acceptance.

Three consecutive 60-second full-duplex trials per family, transport and ownership
mode, with exact patterns and directional p99 below 20 ms, remain the closure
target. Native-client continuity and simultaneous latency failures persist on the
Parallels test VM; isolated PipeWire failures justify native-host qualification,
but do not prove the VM is the sole cause. Follow
[issue #112](https://github.com/anderstvoss/virtualgamepad/issues/112).

Physical DualSense headphone L/R, grip L/R, explicit onboard-speaker routing and
USB reconnect playback are observed. Meaningful headset-microphone response,
microphone routing/fidelity and physical-versus-virtual equivalence remain open.
Matching profiles stay unavailable. Further physical tests require operator
readiness; adaptive-trigger actuation is excluded.

The [execution ledger](architecture-overhaul/AUDIO_IMPLEMENTATION.md),
[USB acceptance record](architecture-overhaul/USB_AUDIO_STOCK_LINUX.md) and
[physical evidence](architecture-overhaul/experiments/EXP-0023-controller-audio.md)
retain successful and failed runs. GUI integration, user feedback, final API
review, the separate quality review and alpha release remain subsequent gates.
