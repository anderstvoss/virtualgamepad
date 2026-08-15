# Deployment and realization levels

This guide defines the project's three selectable realization levels. It does
not authorize an ordinary application process to change the host.

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

## USB gadget through `dummy_hcd` (`UsbGadget`)

`UsbGadget` is the third deployable realization level. It creates a ConfigFS
HID gadget and binds it to `dummy_hcd`, which supplies an entirely software
USB host and UDC in the same VM. Linux and Steam therefore see a USB HID
controller, but no physical device-mode hardware or external cable is used.
It is intended to make USB/HID topology and controller-specific capabilities
available to normal controller creation, subject to the privileged service
boundary below.

A controller may implement evdev, UHID, USB gadget, or any combination. Each
creation request chooses exactly one declared level; missing `dummy_hcd`,
ConfigFS/HID-gadget support, service authority, or device endpoints is a typed
creation error, never a fallback to evdev or UHID. USB gadget does not define a
different controller API: packages retain one typed state and implement the
full target-specific codec and reverse-output surface.

The DualSense proof of concept confirmed Steam gyro input through this level
with the same descriptor, feature fixtures, wire-axis mapping, and 250 Hz
cadence that failed to become visible via UHID. That makes the USB topology a
required product-level alternative, not merely transport validation. See
[the POC finding](POC_DUMMY_HCD_DUALSENSE.md#confirmed-finding-2026-08-15).

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
| In-VM USB topology and controller-specific HID behavior | The `UsbGadget` realization through `dummy_hcd`. |

This is feature-open: the table identifies necessary kinds of realization, not
a ceiling on provider capability. A controller may faithfully implement a
feature on any declared target, and must return the recoverable
target-unavailable error when it cannot.

## Host-security boundary

The controller library and GUI remain unprivileged. `UsbGadget` operations are
performed by a narrowly scoped, privileged virtualgamepad gadget service that
owns module loading, ConfigFS, UDC binding, `/dev/hidgN`, and cleanup. The
service exposes only controller lifecycle/report operations over an internal
IPC boundary; it does not grant callers arbitrary ConfigFS or root access.

An operator is responsible for preparing a host before calling the relevant
API:

| Target | Operator-provisioned prerequisites |
| --- | --- |
| uinput | A usable `/dev/uinput` node, required kernel support, and existing access for the application user. |
| UHID | A usable `/dev/uhid` node, required kernel support, and existing access for the application user. |
| USB gadget | The privileged gadget service, `dummy_hcd`, `libcomposite`, HID gadget support, mounted ConfigFS, and service-owned `/dev/hidgN`. No physical UDC is required. |

The uinput and UHID providers run as ordinary processes after normal device
access is granted. The USB-gadget service is the sole privileged component. A
provider preflight must report typed, actionable errors such as unavailable
service, missing `dummy_hcd`, unsupported HID gadget facility, unavailable
ConfigFS, or cleanup failure. It must never try a different realization.

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
