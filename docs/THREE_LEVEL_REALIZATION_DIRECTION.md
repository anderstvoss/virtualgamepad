# Three-level realization direction

## Decision

`virtualgamepad` will provide three peer selectable controller realization
levels:

| Level | Linux mechanism | Privilege model | Purpose |
| --- | --- | --- | --- |
| `Evdev` | uinput | Ordinary process with `/dev/uinput` access | Generic Linux input/output. |
| `Hid` | UHID | Ordinary process with `/dev/uhid` access | Local HID descriptors and reports. |
| `UsbGadget` | ConfigFS HID gadget bound to `dummy_hcd` | Privileged gadget service; ordinary callers use IPC | In-VM USB/HID topology and controller-specific host behavior. |

`dummy_hcd` is deliberately the product backend for `UsbGadget`; a physical
USB Device Controller is not required. It creates a software UDC and USB host
inside the same VM. A later physical-UDC backend may reuse gadget codecs, but
is a separate external-device feature and not a prerequisite for this level.

## Controller and creation requirements

Every curated controller must be eligible to implement any of the three
levels. Its manifest declares the supported subset, full typed control surface,
report/evdev codecs, reverse-output handling, target restrictions, and
provider-specific limitations. A controller is never silently downgraded:
creation selects exactly one requested level and returns a typed error when it
is not declared or cannot be created.

All three levels preserve the controller's user-facing controls and outputs
where their provider can represent them. A limitation must state its technical
reason and have a regression test. USB gadget may additionally expose native
USB/HID capabilities that evdev cannot model faithfully: feature reports,
USB identity/release/serial, firmware and pairing metadata, native LEDs,
adaptive triggers, advanced haptics, composite interfaces, and Steam HIDAPI
behavior.

## Required implementation boundary

The public GUI/library must not require root. A privileged gadget service owns
only `dummy_hcd`, `libcomposite`, HID gadget module checks, ConfigFS resources,
and `/dev/hidgN`; it communicates through a narrow typed IPC API. It must use
unique process-owned gadget names and serials, report lifecycle and cleanup
failure explicitly, and never remove resources it did not create.

The existing `UsbTransportValidation` naming and pre-provisioned-endpoint model
is transitional. Replace it with a first-class `UsbGadget` target and provider
that creates the in-VM gadget. Keep USB protocol/report encoders controller
owned; do not couple controller semantic state to ConfigFS mechanics.

## Acceptance bar

For every USB-gadget controller realization, test deterministic descriptor,
identity, input/output/feature report fixtures, repeated motion cadence,
creation, host probe, removal, shutdown, and idempotent cleanup. When the host
supports it, live tests must prove `dummy_hcd` enumeration, expected hidraw and
sensor devices, feature replies, and controller-specific host acceptance.

The DualSense POC established the initial acceptance evidence: Steam detected
live gyro through the `dummy_hcd` USB path while UHID did not, with the same
controller report protocol. This makes USB topology a first-class realization
requirement.
