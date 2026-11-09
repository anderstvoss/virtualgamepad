# Pre-alpha API refinement migration

This is a deliberate breaking candidate update before the final alpha freeze.

- Inspect topology descriptors and `ControllerSurface` through same-named
  accessors (`topology.sticks()`, `stick.id()`, `surface.restrictions()`). Small
  range/placement/scale and label values retain public fields. Curated construction
  records are supporting-crate SPI; normal applications inspect supplied metadata.
- Remove `FaceButtonCluster::button_width`. UIs measure labels and available space.
- `RealizationId` names the exact primary realization. Explicit audio options add
  required associated endpoints; USB/IP remains its own composite realization.
- Use `ComponentAssociation::kind()` for input/audio classification. Treat `role()`
  as a display label. Audio components have no requested HID physical/unique IDs.
- Matching is unavailable. Use `AudioExposure::Disabled` or `Emulated`; do not
  reinterpret emulation as a controller-matching claim.
- Replace audio-selector `identity()` with `pipewire_node()` or `alsa_pcm()`.
  Resolve selectors anew after recreation; cached selectors do not keep nodes alive.
- `AudioRead` is root-owned and keeps `frames`, `first_frame`, `discontinuity`.
- `ControllerAudio::diagnostics()` returns opaque retained application health.
  Existing meaningful loss/error getters remain available. Graph timing and
  bridge scheduling now require `experimental::audio_instrumentation(audio)` and
  the opt-in `experimental` feature; acceptance examples declare that requirement.
- Rename `microphone_host_frames()` to `microphone_consumed_frames()`. It is
  optional sample-pacing progress, including silence, not proof of host delivery.
- Match `AudioError::InvalidSampleBuffer`, `OwnershipMismatch`, `Unsupported`,
  `Unavailable`, `Closed`, and `Backend` for their respective conditions.
  `AccessDenied` retains OS/backend permission meaning.
- Failed creation may return `ControllerError::Cleanup { cause, cleanup }`.
  Inspect the typed initiating cause; cleanup details supplement it.
- Audio is required when requested. Failure discovery closes the logical controller;
  recreate without audio explicitly if desired. No reopening or silent degradation.

The unused old audio session/factory traits are removed from internal SPI. Manifest
sidecar requirements remain distinct descriptive contracts, not another live backend.

Post-GUI audit: the empty root `audio-alsa` feature is removed. ALSA host-device
routing belongs to the demo's `audio-alsa` feature, whose defaults are unchanged.
Library users enable `audio-pipewire` for local UHID audio or `audio-usbip` for the
WIP composite; ALSA routing does not add a library realization.

## C7 step 20a · Curated controller creation

The curated crate and corresponding `virtualgamepad::experimental` exports
now use common realization options plus family options. The ordinary root,
Wii and Steam snapshots are unchanged in this step.

| Before | After |
| --- | --- |
| Curated `create_dualsense(creation)` | `create_dualsense(creation, DualSenseOptions::default())` |
| Curated `create_dualshock4(creation)` | `create_dualshock4(creation, DualShock4Options::default())` |
| Curated `create_switch_pro(creation)` | `create_switch_pro(creation, SwitchProOptions::default())` |
| Curated `create_xbox360(creation)` | `create_xbox360(creation, Xbox360Options::default())` |
| Curated `create_dualsense_with_identity(creation, id)` | `create_dualsense(creation, DualSenseOptions::default().with_identity(id))` |
| Curated `create_dualshock4_with_identity(creation, id)` | `create_dualshock4(creation, DualShock4Options::default().with_identity(id))` |
| Curated `create_switch_pro_with_identity(creation, id)` | `create_switch_pro(creation, SwitchProOptions::default().with_identity(id))` |
| Curated Sony identity `to_bytes()` | `bytes()` |
| Curated Sony identity `from_bytes(bytes) -> Option<_>` | `from_bytes(bytes) -> Result<_, ProviderError>` |

Curated Sony handles expose their local USB/UHID pairing identity via
`identity()`. Sony address bytes use pairing-feature order. Switch Pro and Wii
use display order. All require locally administered unicast flags. The wired
Xbox 360 options carry no persistent identity for the existing standard-HID
presentation (research `x360-l0-consumer-sdl360-vs-generic-hid`).

Curated USB worker constructors still bind an already opened bridge. The
application-side USB/IP path resolves identity before opening that bridge.
Registry entries now describe the older families' existing audio components
and realization restrictions. Sony input-lab isolation is an environment
setting for acceptance runs, so it has no family option or demo toggle.

## C7 step 20b · Ordinary-root controller creation

Root `CreationOptions` still selects the realization and audio.
The family options and identity types are re-exported from the curated crate.
The ordinary API remains open to changes during alpha.

| Before | After |
| --- | --- |
| Root `create_dualsense(creation)` | `create_dualsense(creation, DualSenseOptions::default())` |
| Root `create_dualshock4(creation)` | `create_dualshock4(creation, DualShock4Options::default())` |
| Root `create_switch_pro(creation)` | `create_switch_pro(creation, SwitchProOptions::default())` |
| Root `create_xbox360(creation)` | `create_xbox360(creation, Xbox360Options::default())` |
| Root `create_dualsense_with_identity(creation, id)` | `create_dualsense(creation, DualSenseOptions::default().with_identity(id))` |
| Root `create_dualshock4_with_identity(creation, id)` | `create_dualshock4(creation, DualShock4Options::default().with_identity(id))` |
| Root Sony identity `to_bytes()` | `bytes()` |
| Root Sony identity `const from_bytes(bytes) -> Option<_>` | `from_bytes(bytes) -> Result<_, ProviderError>`. Use `.ok()` for an option. Validate at runtime if migrating a const initializer. |
| Root Sony identity `generate() -> Result<_, ControllerError>` | `generate() -> Result<_, ProviderError>` from the shared curated type. Handle the error at the persistence boundary. |

The root adds `SwitchProIdentity` and `SwitchProController::identity()`.
Restore a stored Switch Pro address via `SwitchProOptions::with_identity`.
The existing Sony root handle identity accessors still return an option.
Wii and Steam supporting snapshots are unchanged.
