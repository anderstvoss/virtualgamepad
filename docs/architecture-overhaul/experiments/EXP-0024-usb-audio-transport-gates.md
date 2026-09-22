# EXP-0024: USB audio transport and FunctionFS completion gates

Status: blocked by identified kernel-interface limitations; no live USB audio pass.
The observed host runs 6.12.107+deb13-arm64. This review inspected the corresponding
upstream stable v6.12.107 sources, not a reconstructed Debian binary/source match.

## dummy_hcd cannot carry the planned PCM transport

The upstream [dummy_hcd source at v6.12.107](https://github.com/gregkh/linux/blob/v6.12.107/drivers/usb/gadget/udc/dummy_hcd.c)
excludes isochronous endpoints from its capabilities (`TYPE_BULK_OR_INT`). In
`dummy_timer`, the `PIPE_ISOCHRONOUS` branch returns `-EINVAL` for transfers.
Consequently, loading dummy_hcd and UAC2 functions does not establish a usable USB
audio transport. An ALSA gadget-side device or successful descriptor preparation
would not demonstrate host PCM flow.

The approved plan's stock dummy_hcd/UAC2 combination therefore needs revision or
a separately reviewed kernel implementation. Do not advertise it, quietly replace
isochronous audio with bulk transfers, or label associated PipeWire nodes as a USB
composite device. Existing interrupt/control HID experiments remain separate.

## FunctionFS does not provide the assumed post-payload SET acknowledgement

The [FunctionFS source at v6.12.107](https://github.com/gregkh/linux/blob/v6.12.107/drivers/usb/gadget/function/f_fs.c)
preserves the setup packet, unlike the legacy HID-gadget report-ID ioctl. However,
`ffs_ep0_read` receives OUT data via `__ffs_ep0_queue_wait`, which waits for request
completion and changes the setup state to `FFS_NO_SETUP` before returning to the
caller. `ffs_ep0_write` then rejects a missing pending setup. It does not provide a
separate userspace operation to accept/reject that OUT transfer after interpreting
its payload. A mismatch with the required personality-owned SET result remains.

Inference: a userspace forwarding state machine cannot restore this missing
completion authority. Pre-payload validation can reject setup metadata but cannot
reject invalid payload contents after reception with the required USB reply.
A desired-contract model was not integrated into production because its success
would not prove FunctionFS implements those semantics. Gate G remains open.

## Required decision and validation

Keep the emulated UHID/PipeWire work independent. USB composite support needs a
transport capable of isochronous transfers and an explicit decision about SET
completion: an adequate kernel interface, or a documented change to the required
contract. Real USB gadget hardware may address isochronous transport but does not
by itself fix FunctionFS completion semantics.

Before ordinary callers can use any replacement: verify actual kernel/transport
capabilities, execute full enumeration requests through an unprivileged personality,
measure exact success/error completions and timeouts, and complete broker ownership,
authorization, cleanup and sustained PCM tests. No kernel modifications, module
replacement, broad sound permissions or new broker operation were installed here.
