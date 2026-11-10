# Retained accepted host outputs

Curated and ordinary family handles expose a typed `host_outputs()` snapshot
with its revision. In-process UHID personalities
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

## USB worker recovery

DualSense and DualShock 4 use the same retained report semantics as UHID.
The standard-HID Xbox 360 worker returns an empty snapshot at revision zero.
The application validates an initial snapshot when opening the worker and
refreshes it during `service()`. The getter reads the cached typed snapshot.
On orderly close the worker stops both processing threads before returning the
final snapshot. If shutdown fails or the worker dies, the last validated
snapshot remains available and diagnostics retain the cause.

The private worker control channel adds operation 8, version 1. Its request
body is the little-endian u64 generation followed by the version byte and a
mode byte (0 for live, 1 for final and close). A successful operation 8 reply
echoes these fields and adds a one-byte family (1 for DualSense, 2 for DualShock
4, 3 for Xbox 360), a little-endian u64 revision, and the family fields below.
Each optional field uses a one-byte 0/1 presence flag followed, when present,
by its fixed-size value. The maximum reply body is 64 bytes.

- DualSense fields in order are motors (2 bytes), right trigger (11), left
  trigger (11), mute LED (1), microphone muted (1, boolean), audio path
  (1, enum 0–3), speaker volume (1), microphone volume (1), speaker preamp
  (1, range 0–7), player LEDs (1), and lightbar RGB (3).
- DualShock 4 fields in order are motors (2 bytes), lightbar RGB (3), and
  blink (2).
- Xbox 360 has no fields and its revision is zero.

The client checks the operation, generation, version, mode, family, field
shape and revision before replacing its cache. Tag 2 continues to return
surviving events, tag 3 continues to report loss independently, and older
clients can still close with tag 4. A new application requires operation 8 v1
at USB/IP creation and reports an installation error with older workers.
The public broker framing and USB report bytes are unchanged.

Native dummy_hcd retains its f_hid limitation. Its incomplete request metadata
and reply support cannot establish a complete validated retained view.

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
