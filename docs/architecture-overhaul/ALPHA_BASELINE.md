# Historical alpha continuation baseline

The current `main` was fast-forwarded from PR #110 (`476056a`) to PR #121,
`f8a10d00ab7d060a8fcaa78b452d2d2ea136677f`. Remote main and the retained local audio
branch had identical trees. No historical branch or evidence was removed.

The verified PR #121 CI, CodeQL, SBOM, Scorecard and secret checks passed. Workspace
MSRV is Rust 1.85 (edition 2024); the local compiler is Rust 1.95. The corpus pin is
`a1789d6ed92b2325016dd78be765342f3ca19aa4`. Default features are empty;
`experimental`, `audio-pipewire` and `audio-usbip` are optional.

At this baseline, issues #112, #115, #116 and #117 were open. #112 is now closed
without complete continuity/latency acceptance; consult
[remediation status](ALPHA_REMEDIATION_STATUS.md) for current gates. USB/IP is opt-in WIP until installed
security/recovery acceptance. Physical microphone/matching, sustained native audio,
maintainer hands-on refinement, final API review, quality review and release gates
remain distinct requirements. Existing audio implementation and partial physical
output/reconnect evidence do not close those gates.

Local completion guidance uses locked Cargo commands. The supporting `gr-hid`
crate is explicitly unpublished. Root publication policy is unchanged.

Next review blocks are topology/surface hardening, realization/audio API hardening,
then demo audio integration and real maintainer feedback. This baseline does not
freeze the API or authorize a release.
