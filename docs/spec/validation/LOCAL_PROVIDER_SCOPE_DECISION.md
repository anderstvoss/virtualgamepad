# Decision: local provider scopes are explicit

**Status:** accepted

## Decision

The library and demo distinguish three Linux provider scopes:

| Scope | Inventory | Host requirement | Intended use |
| --- | --- | --- | --- |
| Standard | `uinput` | `/dev/uinput` access | Default local virtual gamepad use |
| Identity-aware | `uinput`, UHID | `/dev/uinput` and `/dev/uhid` access | Explicit DualSense/HID identity testing |
| Transport lab | USB transport only | Peripheral-capable UDC, configfs, delegated privilege, observing host | Dedicated hardware-faithful validation |

`linux_standard_backends()` is the default for the graphical debugger.
The GUI lets a user select the two broader scopes before creating any
controller, then keeps that selection fixed until all active sessions are
removed. This makes required host privileges visible before a provider can be
opened.

## Rationale

The Rust library does not require OS configuration for planning, translation,
fake-backed sessions, or consumers that do not open a Linux virtual device.
Real provider sessions do require kernel device access, which cannot be solved
by adding a Rust dependency. Treating UHID and USB gadget requirements as a
universal library prerequisite would incorrectly raise the adoption cost of the
standard `uinput` use case.

## Consequences

- Ordinary GUI users need only the standard Linux `uinput` permission setup.
- UHID and USB gadget setup remain optional, provider-specific operations.
- Bluetooth transport stays excluded because it has no live realization.
- The lab setup guide is evidence-oriented and is not installation guidance for
  general library users.

## Reconsideration criteria

Revisit this decision only if a provider gains a live implementation with no
additional system privilege beyond the standard scope, or if a portable,
least-privilege mechanism makes identity-aware/transport providers suitable as
ordinary desktop defaults. Any revision must document its security model,
supported host classes, and migration path for the GUI scope selector.
