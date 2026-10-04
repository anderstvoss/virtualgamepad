# Proposed Git alpha release notes

Candidate tag for maintainer review: `0.1.0-alpha.1`. Workspace packages remain
`0.1.0` and unpublished; this package prepares a Git-based alpha, not a tag or
registry release. The exact release SHA and final acceptance are recorded only
after review, merges and final validation.

The ordinary application boundary is the root `virtualgamepad` crate. Applications
select an exact realization, create typed native controller handles, edit/commit
state, service HID readiness/deadlines and retain diagnostics after terminal close.
Four existing families remain: DualSense, DualShock 4, Switch Pro and standard-HID
Xbox360. Provider limitations and evidence levels remain explicit in the support
matrix. No new Wii/Steam/Puck/proprietary Xbox/Bluetooth family is introduced.

Topology containers and controller surfaces are inspected through accessors;
renderer sizing and supporting-crate construction SPI are outside this contract.
Explicit required audio components use direction-wide Samples/NativeClient ownership
and borrowed audio access. PCM reads preserve complete frame positions and gaps;
errors distinguish malformed buffers, unsupported configuration and ownership.
Endpoint selectors are creation-scoped; input neutralization and audio flush are
independent. Failure of a required component terminates its logical controller;
creation/cleanup failures remain inspectable without silently falling back.

The demo includes explicit audio creation/inspection, host routing, meters, sample
monitoring/microphone input/test tone, recreation and retained diagnostics. GUI
refinement bounds long-running service history, avoids meter-driven stream
recreation, improves layout/routing, and adds button feedback and touchpad timeout.
The demo defaults enable WIP USB/IP and ALSA routing; library defaults remain empty.
No routing is automatically enabled by compiling a backend.

The reviewed root snapshot is enforced in Rust lint CI. `experimental` and internal
workspace crates are excluded from the compatibility promise. Breaking changes
need demonstrated necessity, API review and migration rationale; alpha status does
not waive that policy. See [migration notes](ALPHA_API_MIGRATION.md).

Known limits: USB/IP is WIP/excluded from accepted support. Installed broker
security/recovery acceptance is tracked by #115; its closure alone would not
qualify streaming continuity or the full provider matrix.
matching and physical microphone equivalence remain unavailable pending #116.
No sustained/native continuity or latency guarantee is advertised. Ignored host
probes and compilation do not imply physical acceptance. Host capture beyond the
reviewed routing/channel scope and native scheduling/shutdown remain explicit
limits. The existing unmaintained ttf-parser dependency warning is retained for
review; no new advisory suppression was added.

The expanded all-implemented-paths release gate is **not ready**. Current code
resolution and unresolved host/audio gates are in the
[remediation status](architecture-overhaul/ALPHA_REMEDIATION_STATUS.md) and
[reviewer handoff](architecture-overhaul/ALPHA_REMEDIATION_REVIEW_HANDOFF.md).

Review [the historical acceptance handoff](architecture-overhaul/ALPHA_FINAL_REVIEW.md),
[API disposition](architecture-overhaul/ALPHA_ROOT_API_AUDIT.md),
[freeze candidate](architecture-overhaul/ALPHA_FREEZE_CANDIDATE.md),
[quality review](architecture-overhaul/ALPHA_QUALITY_REVIEW.md),
[support](CONTROLLER_SUPPORT.md) and [audio](CONTROLLER_AUDIO.md).
