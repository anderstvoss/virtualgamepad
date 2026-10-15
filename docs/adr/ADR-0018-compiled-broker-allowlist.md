# ADR-0018: Compiled family registry and broker allowlist

Status: Proposed. Owner acceptance is pending checkpoint C1 review.

## Decision

The privileged broker serves only reviewed, compiled controller families. Adding
an audio or gadget profile is a broker release, including review of its fixed
metadata and tests. A client selects an existing identifier; it cannot submit a
report descriptor, USB function layout, executable path or protocol handler.

One compile-time family list in `gr-controller-wire::compiled_families!` supplies
shared immutable IDs, USB specifications and wire-tag metadata. The typed
`gr-curated-controllers::registry::FAMILIES` expands from that list and links the
controller-owned topology, manifest and surfaces. The lower metadata layer avoids
a dependency cycle through the curated controllers' dummy_hcd provider and broker.
The list is compiled Rust, never runtime registration, plugins or data injection.

The USB/IP builder takes a compiled `UsbProfileSpec`. Its semantic playback and
microphone layouts determine channel acceptance; numbered-report policy and
strings are declared alongside the descriptor. Golden tests preserve device,
configuration, report and session-string bytes for the three existing profiles.

Audio broker tags remain DualSense 1, DualShock 4 2 and Xbox 360 3. Switch Pro has
no audio broker tag or USB audio profile. Legacy gadget tags remain DualSense 1,
DualShock 4 2, Switch Pro 3 and Xbox 360 4. These are distinct existing mappings;
neither wire format nor protocol version changes. Unknown IDs and tags fail
closed. Gadget creation remains unavailable because Linux f_hid lacks complete
GET_REPORT metadata and explicit negative replies; the registry does not lift
that gate or decide whether the provider should be parked (D5).

Controller-native report semantics and typed state stay in the controller
implementation. The worker uses generated typed dispatch; `NativeState` remains
an enum to preserve typed snapshot validation without introducing dynamic state.
Family metadata reserves crate-internal topology/output/subsystem/resource slots
for later contracts. It does not add ordinary-root API items or grant D2.

## Alternatives and consequences

A transport-only broker accepting descriptors or executable protocol handlers
from callers would move the security boundary to runtime input. It is rejected:
that would permit descriptor injection and provider-owned protocol semantics,
contrary to the compiled curated architecture. Generic transport adapters remain
controller-neutral; the privileged allowlist is explicitly reviewed policy.

Maintaining separate handwritten lists in the broker, worker and USB builder
would require synchronized edits for each family and permit identity/layout
mismatches. The compile-time list generates those dispatch tables. Family-specific
code still implements its own state, topology, outputs and supported realizations;
a test-only fifth family needs one module and one typed registry entry to exercise
curated creation on Bluetooth bus 5. It is never exported or broker-enabled.

Registry tiers describe intended realization mechanisms, not accepted physical
fidelity. Existing support cells remain WIP. Live driver binding, consumer-ready
creation/removal timing, sustained audio and physical comparison require their
own evidence. This ADR does not claim them.
