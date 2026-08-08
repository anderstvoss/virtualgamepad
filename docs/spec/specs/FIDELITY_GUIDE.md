# Realization-mode guide

Fidelity is an exact host-presentation claim selected by `LinuxTarget`, not a
best-effort fallback ladder.

| Mode | Typical Linux target | Claim |
|---|---|---|
| `HostCompatible` | uinput | Generic Linux input/force-feedback presentation. |
| `IdentityAccurate` | UHID | Local HID identity, descriptor, and report behavior. |
| `HardwareFaithful` | USB gadget transport | Device-role transport and topology behavior. |

Modes are independent. A controller may support any subset, and a request for
one mode never falls back to another. Native and normalized controller commands
retain their meaning in every supported mode. Controller-specific operations
that are unavailable in a selected mode return a recoverable error without
mutating state.

No curated controllers are exposed while the independent-mode core migration
is in progress. Controller packages will publish their own exact target matrix
only after reviewed implementation and validation.
