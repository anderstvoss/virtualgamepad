# ADR-0017: Exact primary realizations and explicit associated audio

Status: accepted for the pre-alpha candidate; not the final API freeze.
Baseline: PR #121, `f8a10d00ab7d060a8fcaa78b452d2d2ea136677f`.

## Decision

`RealizationId` selects the exact primary controller realization. Typed creation
options explicitly request controller-owned associated components. Every requested
component is required for that creation; no implicit fallback or audio degradation
occurs. `ControllerAssociation::components()` describes the actual component set.

`linux.uhid.usb` presents local HID with USB bus metadata. It may additionally own
caller-session PipeWire audio endpoints when emulated audio is explicitly requested.
Those endpoints do not share USB ancestry merely because they share a controller.

`linux.usbip.usb-audio` remains a distinct composite realization: one stock-Linux
USB/IP/VHCI presentation contains compiled HID and functional UAC2 functions.
Native-client ownership may request explicit caller-session PipeWire companions.
The implementation is opt-in WIP, not accepted support pending issue #115.

Invalid realization/feature/platform/profile combinations reject before external
I/O wherever possible. Providers remain controller-neutral. Role strings are
labels; typed component kinds and audio endpoint selectors drive application logic.

## Alternatives and consequences

Encoding every sidecar/ownership combination in realization strings creates a
combinatorial naming policy without making ownership clearer. The primary model
preserves exact selection while allowing typed creation options to evolve.

Audio ownership is direction-wide for current profiles. Borrowed audio access is
retained: private fake-provider root consumers cover bounded PCM work alongside
HID service, multiple families, native/sample owners and terminal shutdown. These
are deterministic ergonomics/lifecycle results, not live latency qualification.

Independent same-direction groups could receive additive per-group overrides;
existing direction-wide settings would remain defaults for all groups. Wii's
controller-native speaker operations and proprietary Xbox audio remain controller
package concerns. No evidenced near-term controller requires a new shared root
primitive. A concrete contention or topology counterexample must reopen this
candidate decision before freeze.

Gate I closes in architectural/deterministic scope. Gates F/H retain separate
implementation, live, consumer and physical evidence; this ADR does not close them.
