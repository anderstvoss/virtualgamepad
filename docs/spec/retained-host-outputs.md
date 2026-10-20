# Retained accepted host outputs (step 10b)

Each curated family exposes a supporting `host_outputs()` snapshot with its
revision. Nothing is re-exported through the ordinary application root; public
root access and typed kinds remain gated by D2. In-process UHID personalities
update their typed view at validated acceptance, before either bounded
observation queue. uinput retains the latest accepted conventional force-feedback
playback request, preserving full u16 strengths and stop semantics. Upload and
erase acknowledgements remain effect ownership operations, not motor playback.
No elapsed-duration simulation or physical playback is asserted.

Partial report validity bits preserve previously accepted fields. An identical
request or report with no valid retained fields leaves the revision unchanged.
An accepted semantic change advances it once, including a multi-field change.
Rejected/malformed/unsupported reports leave it untouched; revision exhaustion
rejects the candidate rather than wrapping. Transport acknowledgement retries
do not reapply state. Values survive callback consumption, observation eviction
and terminal/error close. Polling never changes the revision by itself.

DualSense retains compatible motor requests, validity-gated raw adaptive-trigger
effect words, mute LED/microphone/audio-route controls, player indicators and
lightbar. DualShock 4 retains motors, lightbar and blink requests. Switch retains
exact encoded motor words (without claiming validated compressed decoding) and
steady/flashing player-light bits; the packet counter alone is not a semantic
change. The Xbox 360 standard-HID fallback rejects HID output reports, preserving
its existing protocol rather than inventing XInput or LED support; its evdev
rumble view is implemented. Native and HID domains are kept separate.

## Unavailable realizations

Owner decision #176 comment6298389648 scopes this step to in-process realizations.
USB-worker getters return `UnavailableInRealization` before and after close,
never a silently incomplete reconstruction: the 128-entry observation channel
drops reports and the current IPC has no retained-state/revision query. #203
tracks a separately approved recovery feature. No IPC/broker frame or version
changes are made. Worker factories explicitly omit snapshot bookkeeping while
reusing the same controller acceptance policy. Native dummy_hcd remains unavailable for a complete curated
view under its existing f_hid protocol-ownership limitation; its experimental
raw event stream is not relabeled as validated retained state.

## Evidence and regression boundary

Existing decoders and accepted report shapes determine retained byte positions.
Linux hid-playstation at adc218676eef25575469234709c2d87185ca223a
(source record `kernel`) supplies DualSense LED/motor and DS4 validity masks.
DualSense trigger enable bits are additionally source-reviewed at
https://github.com/nowrep/dualsensectl/blob/d3ae2fa272e1bf9a5c08a0ded3f374b40f7c73ac/main.c#L64
(right bit2, left bit3); full-file SHA256
0a26da8399b0de010ddc037b5cce42a849f09ce295dd56b66d0c2727d1a4b00b.
This is consumer/compatibility-source evidence, not physical fidelity. No source
excerpts are copied.

Deterministic tests cover >32-report bursts, evicted partial updates, independent
sessions, repeated values, each declared request type and exact error/success
ACKs, retry byte identity, native upload/play/stop and post-close snapshots.
Existing request-ownership, security and protocol regressions remain in place.
