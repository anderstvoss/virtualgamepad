# Post-GUI root API audit

## Candidate and scope

Reviewed against merged baseline `ec4c2ff90e58478793873f7b6c5c59a64338db0e`
(PRs #122–125, #127 and #128), plus the scoped changes in this audit. The
maintainer's GUI findings and reviewed approval are recorded in
[the checkpoint](ALPHA_DEMO_REFINEMENT.md). This is a fresh whole-root review of
`src/lib.rs`, its application/controller/audio/output modules, and the re-exported
controller, realization and audio contracts. Earlier inventories are inputs, not
approval of the current contract. No hard freeze or release is declared.

The regenerated [inventory](API_INVENTORY.md) lists all 110 ordinary root items,
including re-exported methods, public fields, associated constants and concrete/
auto trait implementations. Group decisions below apply to every member of their
listed inventory groups. Supporting crates and `experimental` remain unstable SPI.

## Disposition

| Boundary / inventory group | Decision and rationale | Evidence / limitation |
| --- | --- | --- |
| Four controller handles and six creation functions | Keep: exact creation, native editing, commit and caller-driven service are the application boundary | `src/controllers.rs`; all-family lifecycle and native-control tests; no manager or fallback |
| Creation options, Sony identities, realization/controller IDs and target sets | Keep: immutable creation policy, persisted Sony bytes and exact descriptive IDs; ID/set constructors do not install providers | `src/application.rs`; `src/controllers.rs`; `gr-realization-api`; unsupported combinations preflight before I/O |
| Native state, axes, triggers, contacts, motion, battery and control enums | Keep: family-native accepted state and validated numeric leaf values; fixed physical slots are deliberate | Curated family contracts; controller parity, range, neutralization and compile-fail family tests |
| Input topology, clusters, surface wrappers and `ControllerSurfaceInfo` | Keep: private extensible containers with accessors; supporting SPI constructs curated topology | `gr-controller-contract`; no root construction module; no renderer pixel widths |
| Range/placement/scale, label/control/axis/output/restriction leaf records | Keep public value fields: copyable descriptive entries do not mutate a supplied static controller surface | Migration notes; inspectability preserved; future descriptor/container growth uses private containers |
| Typed output events, HID payloads, lifecycle and force feedback | Keep: optional bounded native observations; required host replies remain internal | `src/output.rs`; GET/SET acknowledgement and output-conversion regressions; raw payloads do not confer protocol ownership |
| Service readiness, diagnostics, association and component kinds | Keep: borrowed descriptors, retained terminal state and opaque creation association | `src/application.rs`; typed Input/Audio; labels/observed paths are not ownership proof |
| PCM format, options, directions, semantic channels and `AudioRead` | Keep: complete S16 interleaved frames, direction-wide exclusive owner and explicit segment discontinuity | `gr-audio-contract`; root frame-alignment/ownership tests; no matching variant |
| Audio handle, endpoint/selectors and diagnostics | Keep borrowed access: metadata is creation-scoped, health is retained and required-component failure is fail-closed | `src/audio.rs`; fake mixed/sibling/close/recreation/thread consumers; no demonstrated handle-contention defect |
| Optional microphone consumption/drop observations | Keep: application pacing/loss capability; `None` means unavailable | Consumption includes silence and is not successful host delivery; no exact health-poll or latency promise |
| Controller/control/commit/audio errors | Keep typed actionable categories and extensible enums; strings supplement causes | Creation rollback preserves initiating plus cleanup errors; callers need not parse messages for normal flow |
| Experimental graph/bridge telemetry and factories | Exclude from ordinary alpha contract | Root negative-boundary tests; no graph timestamp is advertised as end-to-end latency |
| Empty root `audio-alsa` feature | Remove: no library code uses it; host ALSA routing is a demo capability | Demo retains local `audio-alsa`, including existing defaults; manifest regression protects the separation |

Trait compatibility is recorded and exercised by root-only compile checks.
Controller handles are `Send`, not `Sync`; `Arc<Mutex<Controller>>` is shared safely.
The audio handle is `Send + Sync`, but remains borrowed from its controller.
PCM operations still require an exclusive mutable borrow; sharing an owned handle
uses application synchronization. Worker PCM does not service HID requests.
Callbacks and PCM work must remain bounded and readiness/deadlines recomputed.
An audio borrow cannot survive closing its controller. Lifetime failures terminate
only the owning logical controller, not siblings. Neutralization and audio flushing
remain separate. There is no root controller/backend plugin interface.

## Feature and platform review

The library defaults are empty. The three root features are `experimental`,
`audio-pipewire` and `audio-usbip`; downstream checks exercise all eight subsets.
The demo separately defaults to USB/IP and ALSA routing. Audio exposure is still
an explicit creation choice; a compiled backend does not automatically create it.

| Creation | Required availability | Accepted claim |
| --- | --- | --- |
| Local emulated audio | Linux, `audio-pipewire`, UHID USB and a compiled family profile | Associated endpoints in one required logical controller session |
| USB/IP sample audio | Linux, `audio-usbip`, explicit composite and prepared broker/VHCI | Opt-in WIP pending #115 |
| USB/IP native client | Above plus `audio-pipewire` for caller-session endpoints | WIP; no continuity/latency guarantee |
| Switch Pro audio / controller matching | Unimplemented | Reject before external construction; no fallback |
| Non-Linux providers | Current providers are Linux implementations | Types compile; unsupported creation is not platform support |

Strict rustdoc ordinary declarations and trait implementations agree between Linux
(default and all features), Windows MSVC and macOS x86_64 (all features). Owner
source links can differ in cross-target rustdoc and are excluded from the semantic
comparison. Cross-target compilation/doc generation is not native runtime testing;
merged CI supplies Windows/macOS build/test evidence at the baseline.

## Tooling findings and corrections

The old rustdoc extractor silently omitted trait implementations and could assign
a standard-library trait's source link as the owner of a re-export when the actual
source link was absent. Both have deterministic sanitized HTML regressions.
Generic compiler blanket implementations are excluded; concrete and auto traits
(including negative implementations) are retained. The root consumer matrix also
omitted experimental-plus-audio combinations; it now discovers root feature names
from the manifest and checks their full powerset.

After building matching root docs, these commands are read-only checks:

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --locked -p virtualgamepad --no-deps --all-features
python3 scripts/generate-api-inventory.py --check
python3 scripts/check-root-consumers.py
python3 scripts/generate-api-inventory.py --compare-doc-root target/x86_64-pc-windows-msvc/doc/virtualgamepad
python3 scripts/generate-api-inventory.py --compare-doc-root target/x86_64-apple-darwin/doc/virtualgamepad
```

The comparison target docs must first be built with the corresponding `--target`.
The inventory is an audit aid, not a complete semver checker: inherited trait
methods, compiler blanket implementations, behavioral contracts and experimental
exports are not a frozen semantic snapshot. Toolchain changes can change auto
traits. Final drift enforcement needs a deliberate toolchain/baseline policy and
an enforced check entry point; no `.github/` or release workflow changes are made.

## Remaining alpha conditions

- Reconcile #126's broad exercise checklist with the reviewed GUI scope; actual
  feedback exists, but unreported family/ownership/hardware cases are not passes.
- #112 is closed after the maintainer's local UHID/audio completion clarification.
  Its unchecked continuity/latency matrix and historical failures remain bounded;
  no latency guarantee follows from issue closure.
- #115 remains the installed USB/IP security/recovery gate. Keep accepted alpha
  support explicitly excluding that WIP claim until measured acceptance closes.
- #116 remains physical microphone/matching fidelity work. Emulated profiles do
  not require inventing physical equivalence; matching remains unavailable.
- Review this post-GUI inventory/disposition and resolve API-affecting findings
  before declaring the exact hard-freeze revision. Add enforced drift protection;
  workflow changes require explicit authorization under `AGENTS.md`.
- Then perform the separate ownership/unsafe/OS/concurrency/dependency quality
  pass. Public-semantic changes reopen the affected API review.
- Validate the exact release candidate through Git consumption, cached offline
  rebuild, final CI and reconciled support/migration/changelog statements.
  Tagging/publication requires a separate authorized release step. #117 remains
  the sequence umbrella, not completed by this audit.

## Validation

See the PR's completion evidence for the final candidate. Ignored host probes are
not acceptance; no desktop configuration, privilege or physical-device operation
is part of this audit. Baseline post-merge CI/CodeQL/SBOM/Scorecard/Gitleaks passed,
and the four virtualenv dependency alerts are now cleared.
