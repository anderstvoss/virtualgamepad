# ADR-0005 — Integrate proven UHID personalities and preserve gated providers

Status: accepted for deterministic implementation, host acceptance pending. Evidence: EXP-0003.

All curated UHID controllers use `gr-hid` with controller-owned personalities and the controller-neutral Linux transport. Native HID definitions contain identity, descriptor, and initial numbering hints only. Authoritative START flags and request IDs remain adapter concerns. Required replies cannot be overridden through application callbacks; compatibility reply methods explicitly reject such attempts on migrated HID sessions.

Use complete `RealizationId` strings declared by compiled manifests. Keep old target names as aliases and use static slices for extensible target sets, avoiding a closed bit registry or a fixed-capacity public set. Breaking const/set-builder and Switch protocol-status APIs is acceptable in early development. The demo and documentation use the new status accessors.

STOP cancels definitely-unsent input for the stopped presentation and preserves desired semantic state for START. Consumer CLOSE/OPEN does not destroy the library session. Invalid framing gets a defined error; if that completion is blocked or uncertain, destroy the transport in the same consuming cycle. No malformed request is left pending indefinitely.

Retain the existing uinput and compiled broker paths until their own migrations are valid. In particular, the current gadget control API does not establish the UHID request contract; Gate G needs staged startup and live capability/latency evidence. Removing these working paths now would violate controller realization parity. Shared wire encoders remain reusable; do not add duplicated mutable protocol semantics to the broker.

Revisit after G passes for a supported kernel/profile and after compound/audio/Bluetooth gates establish their specific host behavior. No physical or consumer support promotion follows from deterministic tests alone.
