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
three independent targets:

| Realization target | Linux mechanism | Product role |
| --- | --- | --- |
| `Evdev` | uinput | Normal local deployment through Linux evdev. |
| `Hid` | UHID | Normal local deployment with HID identity and report behavior. |
| `UsbGadget` | ConfigFS HID gadget + `dummy_hcd` | Normal, privileged same-host deployment through the Linux USB/HID stack. |

These three targets are peer realization levels, not an ordered ladder, and
do not imply each other. A controller may implement any non-empty subset;
creation selects one exact level. `UsbGadget` is intentionally a same-host
software USB device: it binds the controller's ConfigFS gadget to `dummy_hcd`,
whose software UDC and host make Linux enumerate it as USB in the same kernel.
It works on bare-metal Linux as well as in a VM; it does not require VM
features, a physical UDC, or an external cable. No selection falls back to
another provider or target.

The currently named `UsbTransportValidation` API is transitional. It must
migrate to the deployable `UsbGadget` target without changing controller
semantic state or allowing a controller to bypass target-specific validation.
Until that migration lands, it must not be described as the intended product
architecture.

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
the targets. An operation that the controller family never implements returns
`UnsupportedControl`. An operation the controller implements but whose chosen
realization cannot faithfully expose returns `UnavailableInRealization`; its
candidate state is discarded and the
controller remains usable.

Examples of target mechanisms are intentionally descriptive rather than
prescriptive:

- uinput may realize controller-oriented evdev input and output capabilities
  wherever they faithfully represent the controller. A controller-declared
  keyboard or pointer companion is permitted only through that controller's
  explicit opt-in creation option; the root library exposes no standalone
  keyboard or mouse injection constructor.
- UHID may realize local HID descriptors, identity, input reports, output
  reports, and feature-report exchanges. It is not a USB or Bluetooth
  device-role claim.
- `UsbGadget` realizes actual same-host USB enumeration, endpoint/interface
  topology, HID feature exchanges, and controller-specific reports. It may
  expose native behavior that evdev cannot faithfully express, including HID
  feature reports, firmware/pairing identity, adaptive triggers, LEDs, and
  Steam's dedicated HIDAPI controller path.

Audio and attached accessories are separate from ordinary controller input and
output. A native HID report may represent jack presence, mute, volume, audio
routing controls, or an attached accessory's protocol. It does *not* create a
playback/capture endpoint. Usable headset or controller audio requires a
separate, controller-declared audio realization with a host audio service and
its own streams, lifecycle, and permissions. Similarly, a controller-attached
keyboard is a controller-native accessory protocol, not permission to inject
arbitrary host keyboard events. A realization may expose either only where it
can faithfully do so; otherwise the typed operation is target-unavailable.

## Creation and lifecycle invariants

Creation prepares and validates the exact controller/target realization before
opening host I/O. It verifies the controller manifest, target pairing,
provider capabilities, realization shape, host prerequisites, and required
reverse output. Invalid or unavailable realization produces an actionable
creation/preflight error and no handle.

Updates are local, cloned-candidate edits. A rejected update does not mutate
state or mark it dirty. `commit()` submits a full encoded state; a failed
commit leaves valid state dirty and retryable. Close is terminal even if host
cleanup reports an error. Reverse output is bounded and isolated from the
commit path.

## Linux provider verification boundary

The uinput and UHID crates keep their Linux file-descriptor and ioctl work in
private live I/O implementations. Each has a separate private factory and
already-open I/O interface used only to inject deterministic fakes in that
crate's tests. This is not a public provider extension point and does not
change controller or runtime contracts. Hermetic tests use it to exercise
open/configure, short-write, would-block, malformed reverse-data, reply, and
teardown failures. Ignored host tests remain the separate confirmation that
the same live implementation works with an operator-provisioned Linux node.

## Extension rule

Adding a curated controller must not require controller-family changes to the
core or providers. Its package supplies typed state/features, a non-empty
independent manifest, target-specific realization specs/codecs, reverse-event
decoding, and conformance tests. If it declares host audio streams, it also
supplies an audio sidecar requirement through the backend-neutral audio
contract; controller-native audio controls and attachment semantics remain in
the package. It must document its host prerequisites and
feature availability for every declared target.

See [deployment and hardware validation](DEPLOYMENT_AND_VALIDATION.md) for
the `dummy_hcd` service boundary and operator responsibilities.

## Controller-native state and target surfaces

Curated controllers do not inherit from a mutable base-gamepad state. Each
compiled controller package owns its semantic state, native controls, numeric
domains, validation, codecs, reverse-output decoding, and target declarations.
This is intentional: a controller's physical controls and report semantics are
not an optional feature bag. A DualSense touch surface, a Joy-Con IR camera,
a Wii Remote expansion port, and an Atari Jaguar keypad must remain native
types rather than nullable fields on another controller.

The only shared input vocabulary is digital spatial convenience: face-button
position and D-pad direction. It maps to controller-native labels such as
`Cross` and `A`, but it never provides generic sticks, triggers, touch,
motion, or sensor values. Those values use controller-native, range-validated
newtypes and their documented native numeric domains. This library does not
normalize numeric values across controller families.

Each created concrete controller exposes an immutable typed target surface.
It describes the selected Linux presentation: evdev codes, axis minimum and
maximum, neutral value, flat/dead-zone value, outputs, and documented target
restrictions. The surface lets an embedding application adapt a controller's
native values to its actual Linux presentation without treating presentation
metadata as a second mutable state API. A small common read-only surface view
is available for heterogeneous inspection; controller-specific surface detail
remains typed and concrete.

State changes are transactional. A concrete handle edits a cloned candidate,
validates it against its selected target, and replaces live state only on
success. Rejected edits leave both state and dirty status unchanged. A commit
encodes the complete native state; failed sends retain a valid dirty state for
retry. Core/runtime/provider crates must not branch on a controller family or
interpret controller-native values.

Shared helper types are permitted only when their semantics and units are
identical. Examples include bounded values, timestamps, or a proven common
transport primitive. They must never impose state layout, feature availability,
or numeric conversion policy on a controller package.

## Compound controller presentations and reverse transactions

A curated controller may have one primary host device plus explicitly enabled
companion devices. The runtime owns the ordered provider sessions, preflights
all of them before opening any, rolls back a partial open in reverse order,
and closes all components exactly once. A logical commit sends complete frames
in deterministic component order. This preserves retry safety but does not
claim atomic operating-system visibility across multiple devices.

Companions are controller-declared and controller-owned: callers choose only
the typed companion options exposed by that controller package. They cannot
supply arbitrary event codes, mappings, device paths, or a standalone generic
keyboard/pointer device. The library still never changes host permissions,
udev policy, modules, or configuration.

Reverse events are delivered through bounded typed callback subscriptions
outside the commit path. Each subscription is isolated, so a slow or panicking
consumer is recorded and contained without blocking input or other
controllers. Reply-required reverse requests use typed one-shot reply tokens;
duplicate, closed, or full replies fail recoverably. Controller packages own
request decoding, attachment protocols, and reply payload semantics.
