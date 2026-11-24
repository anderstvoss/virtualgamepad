# Switch Pro Bluetooth-bus UHID (P9c)

This realization presents a **local** Nintendo Switch Pro HID node with Bluetooth bus metadata (bus 5). It has no radio, pairing, BlueZ, or real wireless transport. Creation must explicitly select `linux.uhid.bluetooth`; no automatic fallback to or from `linux.uhid.usb`.

The behavioral target is source-backed Linux `hid-nintendo` and SDL Switch HIDAPI, not verified physical-equivalence. See the draft [P9c research brief](https://github.com/anderstvoss/controller-protocol-research/blob/e223aceb8e47b5a772941f14f4fc92e77fabb052/docs/p9c-switch-pro-bt-uhid.md) and its `swpro-bt-*` claim IDs; the vg research pin remains unchanged until that PR merges.

### Realization levels

| Realization ID | Level | Mechanism | Permission |
| --- | --- | --- | --- |
| `linux.uhid.bluetooth` | Native HID | UHID | needs UHID access |

## Scope and limitations

- Bluetooth startup must not use the proprietary USB 0x80 handshake; input begins in source-described simple 0x3F mode and changes to full 0x30 following subcommand 0x03.
- Source-backed 0x01 subcommand and 0x10 rumble-only outputs, 0x21 acknowledgements, device info, synthetic SPI and retained ordinary output semantics are in scope.
- NFC/IR MCU reports 0x11/0x31, console pairing, Bluetooth audio, actual Bluetooth discovery/reconnection and physical descriptor equivalence remain explicitly unsupported.
- Simple report 0x3F uses a D-pad hat and cannot represent simultaneous opposing directions. Full 0x30 input preserves individual direction bits. The SDL simple-axis parser requires a raw neutral value of `0x8000`; the native signed axis is offset to this code, rather than emitting zero as neutral (source claim `swpro-bt-simple-axis-neutral` in research PR #161).
- P5b host testing on kalux observed successful SDL Bluetooth mapping and bounded N1/N3 create, open, SDL-open and remove sequences (50/50 N1 and 150/150 N3 per Bluetooth family). The report shape remains a source-derived emulation interface, **not** a measured retail Bluetooth descriptor or real radio.

The [controller support matrix](CONTROLLER_SUPPORT.md) remains family-level WIP despite those scoped host results. See [Alpha 2 cleanup evidence](ALPHA2_POLISH_CLEANUP.md) and [#365 P5b findings](https://github.com/anderstvoss/virtualgamepad/issues/365#issuecomment-7073966945).

TODO(alpha2-final): P8 support acceptance and physical Bluetooth comparison.
