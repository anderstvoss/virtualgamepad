# UHID broker decision

## Status

Accepted for the Linux identity-aware provider. `uinput` remains the standard
production target for compatibility-tier virtual controllers.

## Decision

The library does not require UHID setup for ordinary use. A standard Linux
installation with the kernel `uinput` module and permission to open
`/dev/uinput` is sufficient for the compatibility-tier providers.

Native HID identity is an explicit opt-in scope. Currently that means the
DualSense profile's HID identity, descriptor, HID output/feature reports,
touch contacts, and motion reports. Applications that need this scope should
use `linux_brokered_identity_backends`, which contains normal `uinput` plus a
`linux-uhid-broker` provider. The application receives no permission to open
`/dev/uhid`.

The broker process is the only process with UHID access. Its versioned local
Unix-socket protocol permits only:

- creation of the policy-declared profile at identity-aware HID level;
- bounded HID input reports for that created session;
- reading decoded reverse output events and diagnostics; and
- closing the caller's session.

It cannot receive an arbitrary HID descriptor, issue raw UHID operations,
open arbitrary device nodes, or forward HID feature reports from clients.
Sessions are attached to their Unix connection and are closed if that
connection ends.

## Deployment boundary

Install the broker as a separately managed system service. The service
account, or a tightly confined root service, owns `/dev/uhid`; the application
does not join an `uhid` group and is never granted that device node.

The service manager must be the authorization point for the broker socket.
Give the socket only to principals authorized to create virtual native-HID
devices. A narrow `virtualgamepad` service group is acceptable when local
account membership is the product's authorization model: that group grants
only the constrained broker protocol, not raw `/dev/uhid` access. Products
with per-request authorization should place a policy gateway (for example a
desktop D-Bus/Polkit service) in front of the socket and keep the socket
private to that gateway.

Recommended service containment is: no network access, a read-only system
view, a private temporary directory, no home-directory access, no new
privileges where compatible with the selected service identity, and a device
allow-list limited to `/dev/uhid`. The broker is not a replacement for normal
OS authorization; its process and socket permissions must be reviewed by the
integrator.

## Consequences

- Normal applications use `linux_standard_backends()` and need no broker,
  service, or UHID permission.
- Identity-aware applications gain the higher-fidelity surface without being
  able to create arbitrary HID devices.
- USB transport remains a separate lab/provider scope. Bluetooth live
  realization remains unsupported.
- The initial broker accepts the existing DualSense identity-aware provider
  only. Adding a profile requires an explicit policy and protocol review.
- Broker calls are synchronous in this first implementation, so high-rate
  applications should continue to prefer `uinput` unless native HID identity
  is essential. An asynchronous client transport is a future improvement, not
  a reason to grant direct UHID access.

## Reconsideration triggers

Revisit this decision if a new profile needs an identity-aware descriptor, if
the project adopts a system-wide authorization API, if broker throughput is
insufficient for a validated native-HID use case, or if Linux gains a safer
unprivileged virtual-HID mechanism.
