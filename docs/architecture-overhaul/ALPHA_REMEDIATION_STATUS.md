# Alpha remediation status

Alpha remediation landed through PR #149 at
`81cd4289602ba830d17f47d3521f00a8d276aaf7`, from PR #133 under the release owner's
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
| 3: provider security/recovery | All seven scoped provider phases passed on the frozen broker/worker images (#115 closed); not streaming qualification. |
| 4: USB/IP audio/routing | Playback and microphone marker flow observed for DualSense, DS4 and Xbox 360 with clean close; strict zero-loss assertions still fail and are a known limitation. |
| 5: isolated Steam | Deferred to x86-64 follow-up #144; [confirmed by the release owner](https://github.com/anderstvoss/virtualgamepad/issues/144#issuecomment-6157276710). |
| 6: sustained audio continuity | Deferred post-alpha (#135, #145). |

Physical-controller comparisons are deferred to #136.

## Known limitations

- No screen-reader support in the demo (eframe accessibility is not enabled).
- Steam/FEX compatibility is unverified.
- Physical fidelity and native-host timing are unverified.
- **Sustained audio continuity and latency are not validated in alpha; occasional
  audio dropouts may occur.** USB/IP audio remains opt-in WIP.

## Closeout and evidence boundary

The ordinary-root API audit, enforced 110-item snapshot and separate quality
review have landed. The root defaults remain empty; experimental exports and
supporting crates remain outside the compatibility boundary. The four-family
Linux scope does not imply physical equivalence or non-Linux provider support.
Controller matching remains unavailable (#116); broader hands-on GUI review is
post-alpha #126. Full routing/zero-loss qualification remains #143/#145.

The [recorded functional disposition](https://github.com/anderstvoss/virtualgamepad/issues/143#issuecomment-6153143810)
retains strict zero-loss failures, including native duplex playback silence of
0/1632/5168 frames and microphone silence of 16/224/480 frames for
DualSense/DS4/Xbox respectively. Functional marker flow does not turn these
failed assertions into continuity passes. Steam is deferred, not passed; the
release owner [confirmed the #144 deferral](https://github.com/anderstvoss/virtualgamepad/issues/144#issuecomment-6157276710).

The proposed Git tag `0.1.0-alpha.1` carries workspace crate version `0.1.0`
(`publish = false`). Consumers should pin by tag or full SHA. No release is
declared here: #117 remains open until a separately authorized tag/release is
verified. Exact consumer evidence for a PR head is not merge-SHA qualification.

The committed Git-consumer report is historical evidence for `76b7b3fc`; the
#133 final report tested `bc4f60ca`. Neither is a main revision. No main-SHA
consumer evidence existed at the start of this closeout. Release qualification
for the eventual tagged SHA remains pending; [#117](https://github.com/anderstvoss/virtualgamepad/issues/117)
tracks the exact-revision evidence and release procedure.
