# EXP-0025: local USB/IP audio feasibility

Status: stock-kernel feasibility demonstrated for the functional DualSense profile;
production broker and full milestone acceptance remain incomplete.

## Setup and scope

The maintainer prepared stock `usbip_core` and `vhci_hcd` on
`6.12.107+deb13-arm64` and explicitly launched the bounded administrator lab
harness. No custom kernel/module, network listener, sound-device ACL change or
persistent privilege delegation was used. The worker ran as the invoking ordinary
lab user, using an anonymous AF_UNIX stream pair connected to VHCI.

This lab authorization is distinct from production broker isolation. Production
workers must use a dedicated unprivileged identity, separate from application
clients, so a client cannot ptrace a worker or replace its compiled USB behavior.
The root-owned installed executable and its launch policy remain security gates.

The compiled emulated device exposes separate HID and UAC2 interfaces. Linux
bound `snd-usb-audio`, `usbhid`, and the PlayStation HID driver; three HID input
nodes appeared. ALSA reported:

| Direction | Format | Channels | Endpoint | Interval |
| --- | --- | --- | --- | --- |
| Host to controller | S16_LE, 48 kHz | FL, FR, RL, RR | OUT 0x01, adaptive | 1 ms |
| Controller to host | S16_LE, 48 kHz | FL, FR | IN 0x82, asynchronous | 1 ms |

These are emulated descriptors. The physical reference uses UAC1; this observation
does not enable controller-matching mode or establish audio/haptic fidelity.

## Observed transfers

The ALSA card was resolved through the just-created VHCI USB ancestry before
opening `hw` playback and capture. Playback used a synthetic constant four-channel
pattern; capture received the worker's deterministic two-channel ramp, with no
physical microphone. No raw recordings or private identity were retained.

| Trial | Simultaneous playback/capture | Captured frames | Exact pattern | Silence | Pattern gaps | ALSA errors |
| --- | --- | --- | --- | --- | --- | --- |
| Short | 2 seconds | 96,000 | 96,000 | 0 | 0 | none |
| Sustained 1 | 60 seconds; 60.148 s command interval | 2,880,000 | 2,880,000 | 0 | 0 | none |
| Sustained 2 | 60 seconds; 60.150 s command interval | 2,880,000 | 2,880,000 | 0 | 0 | none |

Both ALSA commands exited successfully in every trial. Command intervals include
process startup/drain and are **not** latency measurements. Exact capture data
establishes the microphone-return path. Playback completed successfully, but the
supervisor's worker-side playback counters were not collected by the client test;
this is not yet exact end-to-end playback-loss acceptance.

After the bounded session ended, the port returned to its free state, the virtual
USB device disappeared, and its ALSA card disappeared. Cleanup used the owned
socket, with no detach-by-port operation.

## Deterministic and process checks

The `gr-usbip` suite covers bounded framing, ISO packet layouts, report classes,
exact success/stall acknowledgements, cancellation and sequence reuse, control
ordering after slot reuse, large capture URBs, silence underruns, playback loss
and discontinuities, and terminal closure. A separate unprivileged process test
exercises all three actual compiled personalities over anonymous sockets, including
invalid SET payload rejection and patterned PCM. Those tests do not involve VHCI.

## Open gates

- Third sustained trial, measured latency and exact worker-side playback checks.
- Live DS4 and Xbox profiles, duplicate/mixed sessions and independent removal.
- Live invalid SET status, timeout and cancellation fault injection through VHCI.
- Connection-owned production broker, dedicated worker identity, immutable worker
  installation, descriptor/data-channel controls, quotas and crash/restart tests.
- Library sample ownership and native caller integration for the USB realization.
- Physical DualSense comparison and controller-matching acceptance.

USB audio remains unavailable through the ordinary root creation API until these
production gates are met. This result resolves the anonymous-socket/stock-VHCI
feasibility question; it does not close the alpha or complete audio milestone.


## Subsequent DS4 and Xbox runs

The same bounded harness then attached the DS4 emulated profile. Three consecutive
60-second duplex trials passed: each captured 2,880,000 exact mono ramp frames,
with zero silence or pattern gaps and no ALSA errors. Command intervals were
61.484, 61.481 and 61.473 seconds; these include startup/drain, not latency.

The subsequent Xbox emulated profile enumerated and began duplex transfer, but its
first trial lost the device after 27.674 seconds. Both ALSA commands failed with
`File descriptor in bad state`. The 1,296,000 captured frames before disappearance
all matched the mono pattern, with no silence or gaps. The worker/supervisor cause
is unresolved pending its terminal output. This is a retained failed trial, not a
completed streaming pass or accepted Xbox USB support. The port was subsequently
free and its virtual device absent.

A fresh user-run Xbox session then completed one 60-second trial: 2,880,000
exact frames, zero silence/gaps and no ALSA errors (61.562-second command
interval). Its second trial failed after 47.191 seconds with `Input/output error`
from both ALSA clients. All 2,154,000 captured frames still matched, with zero
silence/gaps. Inspection after this failure found the worker running and the VHCI
device and associated ALSA card present. This failure therefore does not establish
worker termination; its cause remains unresolved. Three consecutive Xbox trials
have **not** passed. The later timed teardown returned the port to its free state.

The user subsequently supplied the supervisor counters for that retry. Playback
consumption plateaued at 5,050,656 frames and USB completions at 122,765 while the
process continued printing until its time limit. Queue loss, discontinuities and
microphone silence all remained zero. This excludes an observed queue overflow or
microphone underrun in these counters, but does not prove host delivery or explain
the ALSA failure. The old worker did not report request stalls or scheduling
lateness, so those causes cannot be distinguished from this run alone. Only these
aggregate observations are retained here; terminal output is not a test fixture.

The worker now exposes retained counts for stalled requests, inactive audio
requests, processed playback/capture frames and maximum software scheduling
lateness. These are diagnostics for the next reproduction, not measured audio
latency or proof of kernel delivery. The live checker also records whether its
original session remains present after each trial, preserving PCM failure details
when the device disappears.

## Packet-timed capture follow-up

After introducing packet-timed microphone collection and a 192-frame operating
fill, another Xbox trial completed both ALSA commands successfully and retained
its device. Of 2,880,000 captured frames, 2,879,760 matched the pattern and 240
were silence; pattern gaps were zero. The command interval was 61.088 seconds.
This remains a failed run, not proof that the earlier interruption is fixed or
that low-latency acceptance is complete.

That checker version did not separate warm-up from measurement. The updated
checker reports the first two seconds separately and still requires every measured
frame to match. It also reports first/last unexpected frame positions. A subsequent
single-trial run was started but its process result became unavailable across the
tool-session interruption; no pass is recorded for that run.
