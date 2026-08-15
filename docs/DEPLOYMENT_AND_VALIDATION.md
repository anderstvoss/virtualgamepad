# Deployment and hardware validation

This guide distinguishes normal library deployment from hardware-transport
validation. It does not authorize the library to change the host.

## Deployable library targets

Applications create local virtual controllers through explicit deployable
targets. A selected target creates a controller only when the controller
declares that realization and the host passes preflight.

After creation, applications should inspect the concrete controller's typed
target surface before translating their own values to Linux-facing ranges. The
surface records exact event codes, axis ranges, neutral values, output
channels, and restrictions for that controller/target pair. It does not change
the controller's typed state API and does not imply a universal numeric range
across controller families.

### uinput (`Evdev`)

uinput is the generic Linux host presentation. A controller may use every
controller-oriented evdev capability that faithfully represents its feature
surface. This can include ordinary buttons, D-pad directions, axes, triggers,
additional axes, multitouch-style input, lighting, motion, haptics, and force
feedback when the prepared realization and Linux mechanism support them. This
list is illustrative, not exhaustive.

The uinput provider configures only the prepared event classes and codes. A
controller package may declare a keyboard or pointer companion only through a
typed, explicit opt-in controller creation option; the library does not expose
standalone generic injection constructors or caller-configured key maps. A
controller package declares and tests each faithful evdev realization.

The deployment provider opens and configures only an already-accessible
`/dev/uinput` node. It creates process-owned devices, writes complete evdev
frames, and handles generic force-feedback upload/erase handshakes. The node
is opened nonblocking, so reverse-event polling cannot stall an application's
input path. It never loads a module or changes host permissions.

uinput is the project’s assumed zero-extra-setup deployment path: a supported
host must make its existing `/dev/uinput` policy available to an ordinary
application user. The library neither installs a project-specific udev rule
nor asks the user to join a project-specific group. If a host does not meet
that baseline, preflight returns an actionable error rather than changing the
host.

### UHID (`Hid`)

UHID presents a local software HID device. A controller may use any faithfully
represented HID capability, including identity, descriptors, input reports,
output reports, feature reports, touch/motion data, lighting, haptics, and
other controller-native exchanges. This list is illustrative, not exhaustive.

UHID does not claim USB or Bluetooth device-role transport. A feature that
requires a physical USB interface, composite topology, or another non-HID
host service is available through UHID only when the controller package has a
reviewed local realization for that complete feature; otherwise it is
target-unavailable.

The deployment provider opens an already-accessible `/dev/uhid` node, creates
the declared HID device, transports input/output reports, and handles static
or controller-supplied feature-report replies. It opens the node nonblocking;
kernel `START`, `STOP`, `OPEN`, and `CLOSE` notifications are diagnostic only,
so they do not invalidate a controller session or change its polling contract.
It never changes host setup.

## Explicit USB gadget API

USB gadget (`UsbTransportValidation`) remains a separate, explicitly named library
API for transport validation. A caller may request it when its environment
already exposes a suitable gadget facility and attached lab hardware. It exists
to establish claims about USB enumeration, descriptors, interface topology,
endpoint behavior, external-host interaction, timing, reconnect, and class
interfaces. It is not an ordinary deployment option and is not required for
normal applications using this library.

A controller may be USB-validation-only. Such a controller is admitted through
the explicit USB API, not normal deployable creation, until it declares uinput
or UHID realization. If the selected host lacks the prepared gadget endpoint,
peripheral-capable hardware, authority, or other declared prerequisite,
creation returns a typed error and no controller/session. It never falls back
to uinput or UHID. USB validation does not define a different controller API or
a universal feature set: controller packages still declare exactly what they
can faithfully represent at that target.

The library only consumes a gadget facility an operator has already prepared.
It opens only controller-declared, pre-existing endpoint paths and reports
their absence as an error. It must not create configfs functions, bind a UDC,
or perform any preparation that may cause host configuration or module loading.

## Audio and attached-device scope

Controller reports and audio streams are distinct capabilities. For example,
DualSense reports carry headset/microphone detection and output controls for
volume, mute, lighting, and haptics; its wired connection also exposes audio
to the host as a separate USB-audio function. Xbox's Gaming Input Protocol
(GIP) treats headset audio as a separate device type and routes streaming
through the host audio driver. A controller package must model these two paths
separately. The core exposes backend-neutral audio sidecar requirements and
session traits only; it currently includes no PipeWire, ALSA, or other audio
backend. A future backend is replaceable without changing controller or Linux
realization contracts.

| Capability | Required realization claim |
| --- | --- |
| Rumble, lighting, mute, volume, jack/attachment state | Controller-native reverse reports, evdev output where faithfully representable, or HID output/feature reports. |
| Headset/controller playback and microphone capture | A separate, controller-declared host audio realization with actual capture/render streams. Neither uinput nor UHID alone provides it. |
| Plug-in keyboard/chatpad | A controller-native attached-device protocol and a faithful transport realization; never generic host keyboard injection. |
| Physical accessory topology | An explicit USB transport-validation realization, when the prepared lab facility supports it. |

This is feature-open: the table identifies necessary kinds of realization, not
a ceiling on provider capability. A controller may faithfully implement a
feature on any declared target, and must return the recoverable
target-unavailable error when it cannot.

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
| USB gadget validation | A pre-provisioned, usable gadget endpoint backed by a peripheral-capable USB Device Controller, plus existing access for the application user. |

The uinput and UHID providers can run as ordinary processes only after that
access has been granted by existing host policy. USB gadget validation commonly
requires a lab host, but the library itself still operates only with the access
already delegated to its caller. A provider preflight must report typed,
actionable errors such as access denied, missing prepared endpoint, unsupported
kernel facility, missing UDC, or insufficient authority. It must never try a
different provider or weaken the requested realization.

## Acceptance requirements

Every provider implementation must prove its stated feature surface through
hermetic tests and supported-host validation. Tests cover realization-shape
validation, host preflight, I/O/reverse-output behavior, failed cleanup,
terminal closure, and no fallback. A controller package adds a target matrix
that documents and tests the controller-specific features faithful at each
declared target.

uinput and UHID hermetic tests use private scripted I/O only within their
provider crates. They verify normal and failure behavior without requiring a
device node or modifying the host. This seam is test infrastructure, not a
library-facing backend or plugin API.

The live provider smoke tests are intentionally ignored by default because
they require operator-provisioned device-node access. On a prepared Linux host,
run `cargo test -p gr-provider-linux-uinput -- --ignored` and
`cargo test -p gr-provider-linux-uhid -- --ignored`. Each test creates only a
process-owned device, sends one minimal valid frame, and destroys it.
