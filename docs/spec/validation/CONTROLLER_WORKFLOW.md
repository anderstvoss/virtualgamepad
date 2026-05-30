# Interactive Controller Workflow

This document defines the controller-support workflow used to add or
expand device support in `virtualgamepad`.

The workflow is primarily designed for interactive development on a
Linux validation bench with access to physical devices for direct
comparison. It also supports document-backed implementations for any
target endpoint when physical validation is unavailable, unnecessary,
or disproportionate to the intended support level.

## Branch model

- workflow/tooling changes land on `workflow/controller-support-toolchain`
- new device work defaults to `device/<profile-id>/buildout`

Each device branch should keep its own support report, evidence notes,
and controller dossier so partial work remains inspectable.

## Validation origin

Support status must distinguish how a behavior was validated. At
minimum, track:

- `implemented`: code/tests/fixtures exist
- `document-backed`: behavior is derived from public documentation,
  drivers, checked-in descriptors, or comparable evidence
- `physically-validated`: compared against a real device
- `host-validated`: exercised against real host software or host tools
- `claimable`: eligible to be claimed as supported at the requested
  tier

Document-backed support is valid for any endpoint, but it must never be
presented as equivalent to physically validated support.

## Iteration loop

1. gather public evidence first
2. create or refresh the controller dossier
3. implement the next required tier surface
4. compare against a physical device when available and warranted
5. turn findings into fixtures, tests, and support-report evidence
6. update claimability and remaining blockers
7. feed newly discovered gaps back into core/runtime/demo/provider code

Higher-tier implementation may begin before lower tiers are fully
signed off, but claims remain linear:

- `compatibility` must be claimable before `identity-aware`
- `identity-aware` must be claimable before `hardware-faithful`

## Capability families

The workflow treats richer device behavior as modular capability
families rather than assuming a universal controller surface.

Examples:

- gameplay input
- identity/descriptors
- reverse output commands
- feature reports
- transport session behavior
- touch surfaces
- motion sensors
- adaptive triggers
- audio routes
- expansion/accessory ports
- vendor-specific side channels

Profiles may declare these families independently, and each family can
remain absent, document-backed, or physically validated on its own.
