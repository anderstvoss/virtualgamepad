# EXP-0008 — DualSense creation identity and SDL core acceptance

Owner: Codex. Baseline `9b466e0`; rewrite based on `f750835`, with the changes
committed alongside this record. SDL 3.2.0 source
`535d80badefc83c5c527ec5748f2a20d6a9310fe`; Linux arm64
`6.12.105+deb13-arm64`, playstation binding, USB metadata. Corpus pin unchanged.

## Implementation

DualSense pairing identity now comes from six OS entropy bytes before provider
open, with local/unicast bits applied. The personality keeps it immutable. Caller
session reuse/high bits do not derive the address. Entropy open/read failure
rejects creation without a weak fallback. Other personalities do not acquire an
entropy requirement unless their controller opts in. This is ephemeral virtual
identity with probabilistic collision resistance, not a physical serial guarantee.

The SDL probe requires an exact path and bounded numeric duration (100–60000 ms).
Zero/multiple matches, open failures, missing required sensors, nonfinite values,
non-increasing sensor timestamps and insufficient changes fail. `--dualsense-script`
also requires press/release of all 15 standard SDL buttons, full-range movement of
all six axes, touch activity and successful rumble/LED submission. JSON schema 1
reports selection, GUID/vendor/product, SDL version, observations, elapsed time,
errors and consumer close. The Rust runner separately checks decoded rumble and
RGB output, reaps the child, verifies node removal, and reports cleanup.

The private pinned SDL build uses console input/HIDAPI/libudev, CMake 3.31.6 and
Ninja 1.11.1.4. Extracted libudev development headers are version
257.13-1~deb13u1. These are private development tools, not system installs. Existing
workspace libc is now a test-only direct dependency for readiness polling; no new
runtime dependency was introduced. Paths, full consumer records and build logs
stay outside Git. No permissions, modules, shared services or settings changed.

## Results

Three exploratory rewrite runs passed motion/output checks, followed by three
stronger ten-second rewrite runs and three baseline runs with the full control
sweep. The same compiled SDL consumer and semantic script were used. The archived
baseline runner substitutes 1 ms polling for unavailable readiness/deadline APIs
and exact baseline session-path selection for the newer process prefix; baseline
production source is unchanged. Actual scheduling and wire-rate equivalence were
not assumed.

| Revision | Strong runs | Buttons down/up | Axes | Distinct samples per sensor | Output / removal |
| --- | --- | --- | --- | --- | --- |
| rewrite | 3/3 | all 15 | all 6, range checks passed | 1947–1953 | expected rumble/RGB, cleanup passed |
| baseline | 3/3 | all 15 | all 6, range checks passed | 2035–2042 | expected rumble/RGB, cleanup passed |

All strong runs observed touch activity and increasing sensor timestamps. Rumble
matched low/high magnitudes 0x4000/0x8000 within one 8-bit wire unit; RGB matched
32/64/128 exactly. Counts are observations, not a latency/rate guarantee.

A concurrent rewrite test created three controllers with IDs 7, 7 and 65543,
observed three distinct kernel identities and input devices, removed the middle
controller while the other two continued service, and removed all remaining nodes.
Consumer termination and controller-removal injections both passed: expected
consumer failure, child reaping and idempotent controller cleanup. Invalid CLI and
missing-device probe invocations failed with valid JSON. Deterministic tests cover
fixed-identity requests, entropy failure, exact selection, argument bounds, JSON
escaping, constant samples and invalid sensor timestamps/values without SDL.

## Reproduction and remaining limits

Build the probe with `scripts/run-sdl3-gamepad-probe.sh` under scoped SDL pkg-config
and library paths, or compile it once in the private prefix. Set
`VIRTUALGAMEPAD_SDL_PROBE` to that executable and run the ignored
`dualsense_sdl_observes_motion_and_returns_output`,
`sdl_failure_boundaries_reap_consumer_and_remove_controller` and
`repeated_application_ids_keep_three_kernel_sessions_independent` tests in
`dualsense_uhid_live`. No root compiler/test invocation is needed.

The measured DualSense Linux/SDL core path passes. This does not reproduce a
baseline SDL failure, prove specialized binding necessary, validate Steam, or
establish physical fidelity. Microphone mute, adaptive triggers, speaker/audio and
other outputs without a portable SDL contract are not promoted by the standard
control sweep. Other families, evdev parity acceptance, corpus delivery checks and
Gate G remain separate work. PR #106 remains draft pending the full core matrix.

## Acceptance review follow-up

The initial sweep proved control activity but did not explicitly assert observed
neutral state. Its trigger minima also began at zero, permitting a high-only
trigger stream to satisfy the low-end criterion. The probe now tracks whether each
axis has actual samples and computes extrema only from those samples, with a
regression rejecting high-only and absent streams. The script includes neutral
holds and SDL asserts all standard buttons released and axes near zero.

A new three-run rewrite sequence passed with these stronger checks, 1827–1857
distinct samples per sensor, observed neutral state, full control masks and typed
output/cleanup checks. The corresponding three-run baseline sequence also passed, with 1910–1919
distinct samples per sensor and observed neutral state. The earlier six-run results
remain evidence for their earlier criteria rather than being silently upgraded.
