# Six-gate acceptance on the current VM

Continue in PR #133 on its existing branch. Native timing and physical comparisons
remain post-alpha #135/#136. These six gates remain alpha requirements. Compilation
and short functional trials close only their stated cells. Keep historical failures
and raw artifacts outside tracked source.

## Input isolation prerequisite

`scripts/run-alpha-input-lab.py` defaults to a plan. Apply requires reviewed,
root-owned code and a hash-pinned test binary. The gated child drops privileges and
capabilities; client display/probe/library environment is applied only after that
drop. The parent installs a temporary rule for the child's exact creation-label
PID, assigns a separate `ID_SEAT` and `LIBINPUT_IGNORE_DEVICE=1`, verifies syntax
and reloads rules without triggering existing devices. Active Sony tests verify
input properties before touch injection and service required host probes while
waiting. Neutral scripts do not acquire this requirement.

The leader is observed with `WNOWAIT` and remains reserved through group/device
cleanup. Restoration refuses changed rule identity or remaining devices. A newly
created rule directory is removed only when empty. Regressions cover preparation
failure, timeout, interruption, gate EOF, repeated restoration and output quotas.
The installed scoped helper cannot replace its own root-owned code: administrator
installation is required. Preserve three fixed actions and no broad sudo.

## Acceptance cells and closure paths

| Gate | Work and evidence | Closure criterion |
| --- | --- | --- |
| Audio continuity | Compare C/Rust graph clocks, buffer handoff, channels, callback gaps and marker accounting. Prebuild and avoid competing tests. Correct demonstrated seams with regressions. | Three independent 60-second controls pass, then all 72 product trials and three-family slow-consumer/recovery checks pass without relaxed loss/duration assertions. |
| Provider recovery | Stage exact broker/worker hashes; preserve idle guards, original services and the global lock. Test successful attachment, worker/client death, bounds, pending-journal restart and identity-matched cleanup. | Bounded completion and owned cleanup, or truthful fail-closed rejection with tested operator restoration. Empty restart alone is insufficient. |
| Contacts/outputs | Prove PID isolation first. Exercise Sony HID and associated evdev nodes, both contacts/release, frames, feedback, partial construction, siblings and removal. | Exact observations and cleanup for representable controls/outputs. No contacts reach the shared desktop. Gamepad-only evdev results do not prove companion touch. |
| USB/IP audio/routing | Prepare attachment-identity-bound `ACP_IGNORE` before attach; verify the shared ALSA monitor ignores the lab card and shared defaults stay unchanged. Use direct ALSA/private graph access. | Supported three-family directions, routing transitions, disappearance/reconnect and teardown pass without touching foreign cards/routes/ports/journals. |
| Keyboard/accessibility | Owned display/state; keyboard traversal/activation, viewport selection, lockout, discovery loading/error/retry and output/error display. Regress demonstrated defects. | Interactive receipts plus regressions and owned cleanup. Neutral soak remains separate evidence. |
| Isolated Steam | Fresh data/config/state and display; inspect launcher's storage behavior first. Never copy/modify the real profile. Any login occurs in the disposable session. | Exact-candidate recognition and available input/output pass, with profile/device/process cleanup. No unspecified game compatibility claim. |

## Progress and final checks

The input runner and active-contact gate are committed with Python/Rust regressions.
Administrator installation and live isolation are pending. No active contacts were
injected by this continuation. Audio qualification remains failed; other cells stay
open until their receipts pass. This plan is not an acceptance result.

Run the five required Rust/Gitleaks checks, forced Rust 1.85, strict rustdoc/API/
inventory, Python/corpus/dependency/worker validators, root feature consumers and
exact-Git/offline rebuild. Verify final-head platform/security workflows and 17 SBOM
identities. Keep the PR draft until all six gates have supported dispositions.
Commit validated increments without rewriting history and retain exact revision,
tree, binary and lockfile receipts.

## Sequential continuation: keyboard touch

The touch canvas now supports focused Space activation/release and bounded WASD
movement. Hold retains a contact; a new Space press toggles it off. Relative movement
uses the existing composition path. Focus loss and selection changes release only
the keyboard-owned momentary contact. Reset/lockout suppression requires key release
before rearming. The canvas has a descriptive label, visible focus and keyboard hint;
the lockout checkbox itself is labelled.

Five new regressions cover exact quick-press/release events, focus loss, held sibling
preservation, relative edge clamping, reset/removal, selection and suppression, and
actual egui Space/D events beside an adjacent focus target. All 14 touch tests pass.
The five mandatory workspace commands passed before this documentation increment.
No dependencies were added. This is deterministic evidence, not completion of the
interactive keyboard inventory or the final two-hour soak. Gate 1 remains open;
the new GUI image requires a fresh soak. Gates 2–6 retain their previous dispositions.
