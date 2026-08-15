# Three-level realization direction

## Decision

`virtualgamepad` will provide three peer selectable controller realization
levels:

| Level | Linux mechanism | Privilege model | Purpose |
| --- | --- | --- | --- |
| `Evdev` | uinput | Ordinary process with `/dev/uinput` access | Generic Linux input/output. |
| `Hid` | UHID | Ordinary process with `/dev/uhid` access | Local HID descriptors and reports. |
| `UsbGadget` | ConfigFS HID gadget bound to `dummy_hcd` | Privileged gadget service; ordinary callers use IPC | Same-host USB/HID topology and controller-specific host behavior. |

`dummy_hcd` is deliberately the product backend for `UsbGadget`; a physical
USB Device Controller is not required. It creates a software UDC and USB host
inside the same Linux kernel and works on bare metal as well as in a VM. A
later physical-UDC backend may reuse gadget codecs, but is a separate
external-device feature and not a prerequisite for this level.

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

## Security posture: one-time privileged loader, unprivileged clients

The USB-gadget realization must not be implemented by giving the GUI, library,
or end user passwordless `sudo`. A machine administrator performs a one-time
installation that creates a dedicated service account and a privileged
`virtualgamepad-gadget` service. That service is the only process allowed to
load the required modules, mount/check ConfigFS, create the project-owned
gadget tree, bind `dummy_hcd`, and open `/dev/hidgN`.

The service's authority is intentionally narrow:

- It manages only modules required for this backend (`dummy_hcd`,
  `libcomposite`, and HID gadget support), and must not unload modules it did
  not load.
- It creates and removes only gadgets beneath a project-owned ConfigFS prefix,
  using per-session names and locally administered ephemeral serials.
- It accepts only compiled controller kinds, validated realization data, and
  bounded report payloads. Callers cannot submit arbitrary ConfigFS paths,
  descriptors, module names, shell commands, UDC names, or filesystem paths.
- It authenticates IPC peers using local OS credentials and authorizes only a
  configured user/group. Its Unix socket is not world-writable and is never
  exposed over the network.
- It treats client disconnect, malformed frames, report-write failure, and
  service restart as terminal session events, then performs idempotent cleanup.
- It emits auditable lifecycle records without recording controller input,
  private host logs, or report payloads by default.

The GUI and controller library connect to that Unix socket as an ordinary user.
They can request `create`, `commit input`, `poll reverse output`, and `close`
for their own opaque session handles only. They cannot gain a shell, invoke
`sudo`, alter system policy, access another client's session, or control
physical USB hardware through this interface.

Service installation, upgrades, and removal remain administrator actions.
The service should start on demand or boot, preflight the kernel capabilities,
and return typed unavailability errors when `dummy_hcd` or ConfigFS is absent.
It must fail closed rather than falling back to UHID/evdev or broadening client
authority.

The existing `UsbTransportValidation` naming and pre-provisioned-endpoint model
is transitional. Replace it with a first-class `UsbGadget` target and provider
that creates the same-host software gadget. Keep USB protocol/report encoders controller
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
