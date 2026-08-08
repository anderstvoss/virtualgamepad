# Deployment and hardware validation

This guide distinguishes normal library deployment from hardware-transport
validation. It does not authorize the library to change the host.

## Deployable library targets

Applications create local virtual controllers through explicit deployable
targets. A selected target creates a controller only when the controller
declares that realization and the host passes preflight.

### uinput (`HostCompatible`)

uinput is the generic Linux host presentation. A controller may use every
controller-oriented evdev capability that faithfully represents its feature
surface. This can include ordinary buttons, D-pad directions, axes, triggers,
additional axes, multitouch-style input, lighting, motion, haptics, and force
feedback when the prepared realization and Linux mechanism support them. This
list is illustrative, not exhaustive.

The uinput provider is deliberately limited to controller-oriented devices;
it does not create keyboard or mouse injection devices. The provider must not
assume that a feature is unavailable merely because it is uncommon in a
generic gamepad. A controller package declares and tests each faithful evdev
realization.

### UHID (`IdentityAccurate`)

UHID presents a local software HID device. A controller may use any faithfully
represented HID capability, including identity, descriptors, input reports,
output reports, feature reports, touch/motion data, lighting, haptics, and
other controller-native exchanges. This list is illustrative, not exhaustive.

UHID does not claim USB or Bluetooth device-role transport. A feature that
requires a physical USB interface, composite topology, or another non-HID
host service is available through UHID only when the controller package has a
reviewed local realization for that complete feature; otherwise it is
mode-unavailable.

## Hardware validation target

USB gadget (`HardwareFaithful`) is a separate, explicitly named validation
surface. It exists to establish claims about USB enumeration, descriptors,
interface topology, endpoint behavior, external-host interaction, timing,
reconnect, and class interfaces. It is not an ordinary deployment option and
is not required for normal applications using this library.

A controller may be hardware-validation-only. Such a controller is admitted
through the validation API, not normal deployable creation, until it declares
uinput or UHID realization. Hardware validation does not define a different
controller API or a universal feature set: controller packages still declare
exactly what they can faithfully represent at that target.

## Host-security boundary

The library never escalates privileges or changes host configuration. In
particular, it does not install udev rules, alter ACLs or ownership, load
kernel modules, mount configfs, bind a USB Device Controller, or invoke
`sudo`.

An operator is responsible for preparing a host before calling the relevant
API:

| Target | Operator-provisioned prerequisites |
| --- | --- |
| uinput | A usable `/dev/uinput` node, required kernel support, and existing access for the application user. |
| UHID | A usable `/dev/uhid` node, required kernel support, and existing access for the application user. |
| USB gadget validation | A Linux host with configfs and required gadget functions, a peripheral-capable USB Device Controller, and effective administrative authority to stage and remove the gadget. |

The uinput and UHID providers can run as ordinary processes only after that
access has been granted by existing host policy. USB gadget validation is a
privileged lab operation. A provider preflight must report typed, actionable
errors such as access denied, missing device node, unsupported kernel facility,
missing configfs, missing UDC, or insufficient authority. It must never try a
different provider or weaken the requested realization.

## Acceptance requirements

Every provider implementation must prove its stated feature surface through
hermetic tests and supported-host validation. Tests cover realization-shape
validation, host preflight, I/O/reverse-output behavior, failed cleanup,
terminal closure, and no fallback. A controller package adds a target matrix
that documents and tests the controller-specific features faithful at each
declared target.
