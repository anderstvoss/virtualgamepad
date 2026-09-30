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
