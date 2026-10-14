# Proposed Git alpha release notes

Candidate tag for maintainer review: `0.1.0-alpha.1`. Workspace packages remain
`0.1.0` and unpublished (`publish = false`). The proposed Git tag
`0.1.0-alpha.1` carries crate version `0.1.0`; consumers should pin by tag or
full SHA. Consumers need no access to the private protocol corpus: it is pinned
by `protocol-corpus.rev` for CI validation, not by a submodule. This package prepares a Git-based alpha, not a published tag or
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

Known limits: USB/IP remains opt-in WIP. Seven scoped installed broker
security/recovery phases passed (#115 closed); this does not qualify streaming
continuity or the full routing matrix. Playback and microphone marker flow was
observed for DualSense, DS4 and standard-HID Xbox, including close/restoration;
strict zero-loss assertions failed and remain failed evidence (#143/#145).
Controller matching and physical microphone equivalence remain unavailable (#116).

Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.

No screen-reader support is provided. Broader hands-on GUI acceptance is
post-alpha #126. Steam is deferred to x86-64 #144, as confirmed by the release
owner; no Steam compatibility is claimed. Physical
comparisons (#136) and native-host timing (#135) remain deferred. Windows/macOS
CI establishes build/test results, not functioning controller providers.
`linux.dummy_hcd.usb-hid` creation is rejected before resource allocation because
`f_hid` lacks adequate GET_REPORT metadata and explicit negative replies. Ignored host
probes and compilation do not imply physical acceptance. Host capture beyond the
reviewed routing/channel scope and native scheduling/shutdown remain explicit
limits. The existing unmaintained ttf-parser dependency warning is retained for
review; no new advisory suppression was added.

Alpha gate dispositions and known limitations are in the
[remediation status](architecture-overhaul/ALPHA_REMEDIATION_STATUS.md).

Review [the historical acceptance handoff](architecture-overhaul/ALPHA_FINAL_REVIEW.md),
[API disposition](architecture-overhaul/ALPHA_ROOT_API_AUDIT.md),
[freeze candidate](architecture-overhaul/ALPHA_FREEZE_CANDIDATE.md),
[quality review](architecture-overhaul/ALPHA_QUALITY_REVIEW.md),
[support](CONTROLLER_SUPPORT.md) and [audio](CONTROLLER_AUDIO.md).
