# Realization policy and provider-neutral architecture

`virtualgamepad` is a library for statically compiled, curated virtual
controllers. A controller package owns its typed state, normalized and native
control APIs, report codecs, reverse-event decoding, and realization manifest.
The core owns selection, validation, lifecycle, retryable commits, and
diagnostics. Linux providers own only host I/O for prepared realization data;
they never select or branch on a controller family.

## One controller API; independent host realizations

A controller has one typed semantic state and one control vocabulary. A
realization controls how a host sees that controller; it never selects a
different input API or changes the meaning of a control. Linux targets map to
three independent modes:

| Mode | Linux target | Product role |
| --- | --- | --- |
| `HostCompatible` | uinput | Normal local deployment. |
| `IdentityAccurate` | UHID | Normal local deployment with HID identity and report behavior. |
| `HardwareFaithful` | USB gadget | Explicit hardware-validation transport, not ordinary library deployment. |

The modes are not an ordered ladder and do not imply each other. A controller
may implement any non-empty subset, including only hardware validation. Normal
creation selects an exact deployable target; USB-gadget use is exposed through
a separate validation API. No selection falls back to another provider or
mode.

## Feature-complete intent is controller-defined

Every provider must target the full controller feature surface that it can
faithfully realize. There is no universal reduced feature list for uinput,
UHID, or USB gadget. Additional axes, multitouch, lighting, motion, haptics,
and force feedback are examples of features that a controller may realize at
any target when the target's Linux mechanism can represent them faithfully.
They are not a capability ceiling or a promise that every controller supports
them.

Each controller realization manifest declares, for its exact target, the
prepared OS realization, host prerequisites, codec/report behavior,
reverse-output support, and typed controller feature surface. The same native
or normalized operation may be faithfully available in any, all, or none of
the modes. An operation that the controller family never implements returns
`UnsupportedControl`. An operation the controller implements but whose chosen
realization cannot faithfully expose returns
`UnavailableInRealizationMode`; its candidate state is discarded and the
controller remains usable.

Examples of target mechanisms are intentionally descriptive rather than
prescriptive:

- uinput may realize controller-oriented evdev input and output capabilities
  wherever they faithfully represent the controller; it does not create
  keyboard or mouse injection devices.
- UHID may realize local HID descriptors, identity, input reports, output
  reports, and feature-report exchanges. It is not a USB or Bluetooth
  device-role claim.
- USB gadget validates actual USB enumeration, endpoint and interface topology,
  external-host interaction, and transport behavior. It is required only to
  make those hardware claims, not to unlock a generic controller feature by
  definition.

## Creation and lifecycle invariants

Creation prepares and validates the exact controller/target realization before
opening host I/O. It verifies the controller manifest, target/mode pairing,
provider capabilities, realization shape, host prerequisites, and required
reverse output. Invalid or unavailable realization produces an actionable
creation/preflight error and no handle.

Updates are local, cloned-candidate edits. A rejected update does not mutate
state or mark it dirty. `commit()` submits a full encoded state; a failed
commit leaves valid state dirty and retryable. Close is terminal even if host
cleanup reports an error. Reverse output is bounded and isolated from the
commit path.

## Extension rule

Adding a curated controller must not require controller-family changes to the
core or providers. Its package supplies typed state/features, a non-empty
independent manifest, target-specific realization specs/codecs, reverse-event
decoding, and conformance tests. It must document its host prerequisites and
feature availability for every declared target.

See [deployment and hardware validation](DEPLOYMENT_AND_VALIDATION.md) for
operator responsibilities and the security boundary.
