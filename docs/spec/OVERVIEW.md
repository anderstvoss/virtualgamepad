# VirtualGamepad specification overview

The authoritative product and architecture contract is the
[controller-native API specification](implementation/CONTROLLER_NATIVE_API_SPEC.md).
The implemented public API creates curated, compiled controller types with an
explicit Linux target; it does not expose profile registration, YAML runtime
configuration, planning, or automatic fidelity degradation.

## Active documents

- [Controller-native API specification](implementation/CONTROLLER_NATIVE_API_SPEC.md)
- [Adding a curated controller](implementation/ADDING_A_CURATED_CONTROLLER.md)
- [Demo reference consumer](../../demo/README.md)
- [Repository setup and validation](../REPO-SETUP.md)

## Historical documents

Files under `specs/`, the older implementation-plan documents, and the
profile-oriented validation plans describe the pre-redesign architecture.
They are retained as design history only and are non-normative. They must not
be used to add public APIs or runtime behavior. New design decisions belong in
the controller-native specification.

## Current realization boundary

- Generic Gamepad and Xbox 360: Linux uinput compatibility realization.
- DualSense: Linux UHID identity-aware and USB gadget transport realization.
- Steam Controller: compiled typed API, with creation rejected until a Linux
  provider can realize the complete surface.
- Windows/macOS: no creation API.

Every supported path is selected explicitly and validated before a provider
session opens.
