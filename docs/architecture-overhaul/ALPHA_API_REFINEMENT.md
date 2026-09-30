# Alpha API refinement: current execution record

## Baseline and scope

The current branch was fast-forwarded from PR #110 (`476056a`) to PR #121,
`f8a10d00ab7d060a8fcaa78b452d2d2ea136677f`, without rewriting history or deleting
branches. Its tree matched the retained audio implementation branch. Baseline
remote CI, CodeQL, SBOM, Scorecard and full-history secret scanning passed.

Workspace MSRV is Rust 1.85, edition 2024. Local review used Rust/Cargo 1.95.0.
Default features remain empty; optional features are `experimental`,
`audio-pipewire` and `audio-usbip`. Corpus pin remains
`a1789d6ed92b2325016dd78be765342f3ca19aa4`. Ordinary consumers need no corpus access.

This package covers baseline/docs cleanup, read-only topology and realization/audio
API hardening. It is a working candidate, not the final alpha freeze. Changes are
reviewable in those three groups on the current branch. No new external dependency,
release workflow, host permission or physical-device change is made. The unpublished
`gr-hid` supporting crate is now explicitly marked `publish = false`.

## Decisions and evidence

- Topology containers and surfaces are read-only. Curated construction records
  live in supporting-crate SPI, which the normal root does not export. Small
  numeric/spatial leaf values remain ordinary values. GUI pixel sizing is renderer
  policy and has deterministic label/viewport regression coverage.
- [ADR-0017](decisions/ADR-0017-primary-realization-and-audio-components.md) defines
  exact primary realization plus explicit required associated components.
- Component kinds are typed. HID requested identities do not contain audio node
  or ALSA card identities. Endpoint selectors are valid only for their creation.
- Matching remains unavailable; its speculative exposure variant is removed.
  Direction-wide ownership and borrowed audio are retained.
- Root-owned reads preserve complete frames, positions and discontinuities.
  Sample alignment, ownership mismatch, unsupported profile and terminal state
  have typed errors. Diagnostics are retained application health; graph timing
  and bridge scheduling are opt-in experimental instrumentation.
- Microphone consumed-frame progress remains an optional sample-pacing capability.
  It describes consumption including silence, not successful host delivery.
- Required-component failure closes the logical controller when discovered by
  servicing or controller operations. No fallback or automatic recreation occurs.
- Explicit creation rollback retains its initiating typed cause and cleanup
  failures. Existing PipeWire registration, broker handoff, port reservation and
  worker/session cleanup tests remain; root and owned-attachment rollback tests
  extend the failure boundaries.
- Root-handle deterministic consumers cover three audio families, sample/native
  access, repeated polling, typed metadata, neutralization independent of flushing,
  HID/audio failures, simultaneous cleanup failures, sibling removal, recreation,
  mixed no-audio sessions and bounded PCM/service work on two threads. Their
  synchronization proves progress/lifetime behavior, not timing guarantees.

## Future-controller stress

| Case | Existing owner / extension | Candidate conclusion |
| --- | --- | --- |
| Wii speaker and variable rate | Native package operations and new stream generations | No generic physical-audio router or filesystem API |
| Steam/Puck/accessories | Typed package state, deadlines, required compound components | No player/slot manager or new production family |
| Xbox/proprietary audio | Controller package profile and explicit realization | Emulated standard HID does not claim XInput compatibility |
| Independent speaker/headset or capture groups | Additive group-specific overrides; direction policy remains default | No speculative ownership map today |
| Synchronized audible/haptic playback | Existing single group/clock and exclusive owner | Preserve channel roles and alignment |
| Compound controllers | Existing preflight, ordered creation and reverse rollback | Associated audio is a required component, not manager policy |

## Support and remaining sequence

Issues #112 (native continuity/duplex latency), #115 (installed broker security and
recovery), #116 (microphone/matching fidelity), and #117 (GUI/API/alpha sequence)
remain open. USB/IP is explicitly opt-in WIP; accepted support requires #115.
Preserve all historical failed runs and confirmed physical output/reconnect evidence.
Ignored hardware tests are not passes. No new physical-reference actuation occurs.

Next: demo audio integration and actual maintainer feedback; incorporate findings;
fresh independent whole-root review; exact hard-freeze commit and API drift
protection; separate quality/security review; exact-revision Git consumer and
cached offline validation; separately authorized release/tag. `.github/` changes
for future drift enforcement require explicit authorization.

## Local live probe scope

On the prepared Linux host, a private PipeWire graph passed root creation/service,
neutralization, duplicate/sibling independence and terminal cleanup for all three
families. A second short root-only probe ran 64 concurrent PCM/service cycles per
family and closed cleanly. It had no playback producer (zero received playback
frames); microphone writes were bounded/partial. This is worker/concurrency lifetime
verification, not stream throughput, continuity, physical comparison or latency
acceptance. Private graph policy startup/removal notices are not a sustained stream
result. Desktop graph settings and physical devices were untouched.

The downstream consumer imports only `virtualgamepad`. It compiles default,
experimental, PipeWire-only, USB/IP-only and combined audio features, including
single-owner and shared-owner thread examples, then rebuilds from cached dependencies
offline. This source-path check is not exact Git release-candidate validation.

## Completed validation

Validation was performed on the refinement tree based on the exact PR #121
revision above, before incremental commit/PR publication. All local work stays on `main`;
remote review branches carry separate baseline, topology and audio blocks. The
results below are local evidence, not remotely tested freeze/release evidence.
The local compiler is Linux/aarch64 Rust 1.95; Rust 1.85 root compilation also passed.

| Command / evidence | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo check --locked --workspace --all-targets --all-features` | Passed |
| `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` | Passed |
| `cargo test --locked --workspace --all-features` | Passed: 478 passed, 54 ignored; ignored probes are not acceptance |
| `RUSTDOCFLAGS="-D warnings" cargo doc --locked -p virtualgamepad --no-deps --all-features` | Passed |
| `cargo test --locked -p virtualgamepad --no-default-features` and separate experimental, PipeWire, USB/IP and combined feature runs | All five passed |
| `python3 -m unittest discover -s scripts/tests` | Passed: 89 tests |
| `python3 scripts/check-root-consumers.py` | Passed: root-only feature matrix and cached offline rebuild |
| `python3 scripts/generate-api-inventory.py` | Regenerated current root declarations, methods and public fields |
| `python3 scripts/check-protocol-corpus.py` | Passed at the pinned revision above |
| `cargo +1.85 check --locked -p virtualgamepad` | Passed |
| Locked all-target/all-feature Windows GNU and macOS ARM cross-checks | Passed compilation; native platform execution was not performed |
| `gitleaks detect --redact` and `gitleaks detect --no-git --redact` | Passed history and working-tree scans |
| `cargo deny check` | Passed with existing duplicate/license-policy warnings |
| `cargo audit` | Passed with an existing allowed unmaintained `ttf-parser` advisory, RUSTSEC-2026-0192; no advisory ignore added |
| Private-graph `root_audio_live` ignored probes, explicitly opted in | Both passed; scope is described above |
| `git diff --check` | Passed |

PipeWire compilation needed a libclang runtime and builtin headers extracted into
an ephemeral build-tools directory. No system install or package dependency was
added. The demo gains only a dev dependency on the existing unpublished
`gr-controller-contract` workspace crate for synthetic topology fixtures.

The dependency warning remains for the later quality review. Installed USB/IP
security/recovery, sustained/native audio and physical microphone fidelity remain
unaccepted. Existing remote platform CI is baseline evidence; changed code has only
the local and cross-compilation results above. This record does not close #112,
#115, #116 or #117 and does not declare the API frozen.

## Reviewable file groups

This lists the public working-tree changes; private memory status was updated
separately and historical records retained. No branches, caches or evidence were
removed. Baseline, topology and realization/audio changes are committed incrementally
for separate stacked review. No release or workflow edit was performed.

### Baseline, guidance and inventory

- `AGENTS.md`
- `README.md`
- `crates/gr-hid/Cargo.toml`
- `docs/ALPHA_API_MIGRATION.md`
- `docs/APPLICATION_API.md`
- `docs/CONTROLLER_AUDIO.md`
- `docs/CONTROLLER_SUPPORT.md`
- `docs/CORE_ARCHITECTURE.md`
- `docs/CURRENT_CONTROLLER_REFINEMENT.md`
- `docs/DEMO_LAB.md`
- `docs/DEPLOYMENT_AND_VALIDATION.md`
- `docs/architecture-overhaul/ALPHA_API_REFINEMENT.md`
- `docs/architecture-overhaul/API_INVENTORY.md`
- `docs/architecture-overhaul/AUDIO_IMPLEMENTATION.md`
- `docs/architecture-overhaul/GATE_STATUS.md`
- `docs/architecture-overhaul/PRE_ALPHA_STATUS.md`
- `docs/architecture-overhaul/decisions/ADR-0017-primary-realization-and-audio-components.md`
- `scripts/check-root-consumers.py`
- `scripts/generate-api-inventory.py`
- `scripts/tests/test_generate_api_inventory.py`

### Read-only topology and renderer

- `crates/gr-controller-contract/src/construction.rs`
- `crates/gr-controller-contract/src/lib.rs`
- `crates/gr-controller-contract/tests/control_exposure.rs`
- `crates/gr-curated-controllers/src/dualsense.rs`
- `crates/gr-curated-controllers/src/dualshock4.rs`
- `crates/gr-curated-controllers/src/switch_pro.rs`
- `crates/gr-curated-controllers/src/xbox360.rs`
- `crates/gr-curated-controllers/tests/dualsense_uhid_live.rs`
- `demo/Cargo.toml`
- `demo/src/gui.rs`
- `demo/src/gui/input_clusters.rs`

### Realization, audio, cleanup and consumers

- `Cargo.lock`
- `Cargo.toml`
- `crates/gr-audio-contract/src/lib.rs`
- `crates/gr-audio-contract/src/pcm.rs`
- `crates/gr-audio-contract/src/queue.rs`
- `crates/gr-audio-linux/src/pipewire_backend.rs`
- `crates/gr-audio-linux/tests/pipewire_live.rs`
- `crates/gr-curated-controllers/src/audio.rs`
- `crates/gr-privileged-broker/src/audio_host.rs`
- `crates/gr-realization-api/src/lib.rs`
- `examples/controller_audio.rs`
- `examples/usb_audio_acceptance.rs`
- `examples/usb_audio_latency.rs`
- `examples/usb_audio_latency_alsa.rs`
- `examples/usb_audio_latency_reverse.rs`
- `examples/usb_audio_native_probe.rs`
- `src/application.rs`
- `src/audio.rs`
- `src/audio/backend.rs`
- `src/audio/fake.rs`
- `src/controllers.rs`
- `src/controllers/tests.rs`
- `src/creation.rs`
- `src/experimental.rs`
- `src/lib.rs`
- `src/usb_audio.rs`
- `src/usb_audio_bridge.rs`
- `tests/root_audio_live.rs`
- `tests/ui/alpha_api_boundaries.md`
- `tests/ui/root_audio_consumer.rs`
