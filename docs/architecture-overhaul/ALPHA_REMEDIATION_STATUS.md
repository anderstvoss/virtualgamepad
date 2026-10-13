# Alpha remediation status

Alpha remediation landed from PR #133 under the release owner's
[narrowed alpha scope](https://github.com/anderstvoss/virtualgamepad/pull/133#issuecomment-6148689485).
The VM acceptance lab tooling, raw evidence ledgers, gate plans and reviewer
handoff are preserved on the `alpha-lab-archive` branch and are not part of the
main tree.

## What changed

- Callback subscriptions close without deadlocking on a full queue.
- Audio-device discovery in the demo runs in a bounded worker and cannot block
  controller service or shutdown.
- UHID Start/Open/Close/Stop lifecycle observations reach application callbacks.
- Rust 1.85 compatibility is restored and the MSRV CI job checks the real
  minimum compiler; the Provider Tier B workflow and the 17-package SBOM are fixed.
- DS4 and DualSense evdev realizations own a separate touch companion node with
  transactional rollback.
- dummy_hcd HID is rejected before any resource is created; cleanup errors are
  retained alongside the initiating error.
- USB interrupt OUT reports follow the controller setter policy; the USB audio
  worker acknowledges shutdown before PCM channels are released.
- The Xbox 360 standard-HID identity matches its wire descriptor.
- Demo: host audio routing and channel mapping, touchpad lockout timer and full
  keyboard operation of the controls.

## Alpha gate disposition

| Gate | Disposition |
| --- | --- |
| 1: keyboard/accessibility | Four-family keyboard smoke passed; accepted two-hour neutral soak reused. |
| 2: controller outputs/isolation | Changed cells passed (Xbox HID/evdev sweep, USB output requests including Xbox EPIPE, Sony companion removal/contacts, DS4 rollback). |
| 3: provider security/recovery | All seven provider phases passed on the frozen broker/worker images. |
| 4: USB/IP audio/routing | Playback and microphone marker flow observed for DualSense, DS4 and Xbox 360 with clean close; strict zero-loss assertions still fail and are a known limitation. |
| 5: isolated Steam | Deferred to x86-64 follow-up #144. |
| 6: sustained audio continuity | Deferred post-alpha (#135, #145). |

Physical-controller comparisons are deferred to #136.

## Known limitations

- No screen-reader support in the demo (eframe accessibility is not enabled).
- Steam/FEX compatibility is unverified.
- Physical fidelity and native-host timing are unverified.
- **Sustained audio continuity and latency are not validated in alpha; occasional
  audio dropouts may occur.** USB/IP audio remains opt-in WIP.
