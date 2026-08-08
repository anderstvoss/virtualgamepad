# Provider-neutral core architecture

The active core has four boundaries: realization API, controller contract,
controller runtime, and Linux providers. Controller packages own typed state,
native features, report codecs, and realization manifests. Providers receive
prepared evdev/HID/USB realization data and raw frames only; they never select
or inspect controller families.

Every creation selects one exact Linux target. Targets map to independent
host-presentation modes: uinput is host-compatible, UHID is identity-accurate,
and USB gadget is hardware-faithful. An unavailable mode or feature fails
recoverably without fallback or state mutation.

The project currently ships no controller package. Adding one requires typed
state/control APIs, a non-empty realization manifest, mode-aware validation,
codecs, provider integration tests, and documented host prerequisites.
