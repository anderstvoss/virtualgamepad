# ADR-0012 — Exact target configuration for shared UHID

Accepted after [EXP-0018](../experiments/EXP-0018-multi-uhid.md).

Carry the complete realization ID in native HID specifications. Configure each
UHID provider for one supported exact ID. Validate selection/capability/native
shape/bus before preflight or creation. Keep bus metadata controller-owned and
reject mismatches instead of rewriting metadata. USB default/value aliases remain.
Curated session dispatch recognizes native HID shape rather than the USB ID.

This extends cohesive realization selection, not a public transport/bus product.
Providers contain no controller-family decisions. Adding a controller does not
implicitly admit any new realization. Bluetooth bus metadata over local UHID is
not a Bluetooth link. Existing Sony restoration remains restricted to USB UHID.
