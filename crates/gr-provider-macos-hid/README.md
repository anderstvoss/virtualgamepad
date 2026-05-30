# `gr-provider-macos-hid`

Phase 12 ships this crate as a planning-only macOS HID provider foundation. It
exists so the planner can select a macOS-native backend family, report
entitlement and deployment prerequisites, and prove the cross-platform
architecture does not need core-crate rewrites to admit macOS support.

What Phase 12 includes:

- inventory entry and backend identity (`macos-hid`)
- `can_realize()` support reporting for planning
- deployment-requirement reporting for a macOS realization path
- explicit "not implemented yet" behavior for session opening

What full realization will require in a later phase:

- a notarized DriverKit system extension
- the matching app entitlements and user approval flow
- provider-local device/session lifecycle management
- real reverse-path handling and diagnostics for the chosen realization stack
- supported-host validation on a macOS system

This crate must keep macOS-specific implementation details local to the
provider and out of the platform-neutral core crates.
