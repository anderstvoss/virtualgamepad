# EXP-0004 — Gadget control capability review

Owner: Codex. Date: 2026-09-06. Gate G status: blocked. This is source evidence, not a running-kernel or latency result.

The inspected [Linux v6.12 f_hid.c](https://github.com/torvalds/linux/blob/adc218676eef25575469234709c2d87185ca223a/drivers/usb/gadget/function/f_hid.c) has SHA-256 `c192f056cb04640432d1c908ebeb2b928779f8e5e056aa46bd81cc3b842981ff`. The source provides GET_REPORT ioctls; treating these ioctls as absent would be incorrect.

The request-ID read ioctl exposes only an eight-bit report ID. GET setup retains length internally and discards the report-type byte for this interface. Responses are cached by report ID; the response ioctl has no transaction token or explicit protocol-error completion. The workqueue waits up to 2500 ms, then uses a cached response or zero-filled data. SET_REPORT either stalls when the output endpoint is selected or queues reception and completion inside the kernel; it has no matching userspace decision/acknowledgement API.

Inference: this source interface cannot faithfully represent the complete typed GET/SET and exact completion contract in ADR-0004. A staged broker handshake alone cannot restore metadata or completion authority that the kernel interface omits. Do not replace the existing broker with a nominally generic forwarding path and claim contract parity.

The existing broker still binds and services controller-specific startup features before returning its session. Its compiled profile remains the documented implementation boundary. No production IPC replacement or kernel modification was made during this review.

To resume: provision the isolated host described in [HOST_PROVISIONING](../HOST_PROVISIONING.md), identify its actual kernel source/profile, and measure startup ordering, available control metadata, completion behavior, concurrent sessions, client death, restart, cleanup, and latency. Either demonstrate an adequate interface or record a narrowly scoped profile restriction/kernel-interface decision in an ADR before E5 replacement. A fake startup model would test its own ordering, not resolve this capability blocker. B/P host comparisons and other extension gates remain separately required.
