# Brief 30: Per-family USB audio profile fidelity (proposal)

**Status:** Research-gated proposal; no implementation or host run.
**Scope:** DualSense, DualShock 4 and Xbox 360; Generic, Native HID, and Real USB realization levels.

## Current compiled functional profiles

At [VG `62f98b6`](https://github.com/anderstvoss/virtualgamepad/blob/62f98b628839d173e3b894111544eea6b62163a7/crates/gr-curated-controllers/src/audio.rs), the S16LE 48 kHz profiles are DualSense 4 playback / 2 capture, DualShock 4 2 / 1, and Xbox 360 2 / 1. The [USB/IP descriptor builder](https://github.com/anderstvoss/virtualgamepad/blob/62f98b628839d173e3b894111544eea6b62163a7/crates/gr-usbip/src/profile.rs) uses functional UAC2 composites, not physical matching.

## Research and proposed disposition

Populate this brief from the evidence corpus without asserting physical observations. The broker and worker IPC remain unchanged, and all host checks are **not run**.
