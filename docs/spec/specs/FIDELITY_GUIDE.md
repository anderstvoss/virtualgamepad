# Fidelity guide

Fidelity is a creation guarantee, not a best-effort runtime tier.

A curated controller may be created on a `LinuxTarget` only when its compiled
module supplies a complete realization for that target and the provider can
honor every declared requirement. The library returns `CreationError` before
returning a handle when identity, transport, reverse output, descriptors,
feature reports, or host prerequisites are incomplete. It never substitutes a
generic evdev device for a requested native controller.

Current supported combinations are:

| Controller | uinput | UHID | USB transport |
|---|---:|---:|---:|
| Generic Gamepad | complete compatibility realization | unsupported | unsupported |
| Xbox 360 | complete compatibility realization | unsupported | unsupported |
| DualSense | unsupported | complete identity-aware realization | complete USB realization where host prerequisites exist |
| Steam Controller | unsupported | unsupported | unsupported |

“Unsupported” is intentional: the corresponding creation function remains in
the stable curated API but returns a precise error until a complete provider
realization is implemented and accepted. Privileged hardware validation is a
separate release gate and never weakens hermetic tests.

The detailed acceptance criteria are in
[CONTROLLER_NATIVE_API_SPEC.md](../implementation/CONTROLLER_NATIVE_API_SPEC.md)
and [DEVICE_SPEC_VALIDATION_PLAN.md](../validation/DEVICE_SPEC_VALIDATION_PLAN.md).
