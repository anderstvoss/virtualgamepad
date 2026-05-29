# Manual Gate Guides

This directory holds step-by-step user guides for the manual portion of
each implementation phase gate.

Each file is phase-specific and is meant to be followed by a human
reviewer after the automated phase-gate checks are green.

Files in this directory:

- [Phase 1 Manual Gate](phase-1.md): core domain model review and
  manual-to-automation checks for `gr-core`
- [Phase 2 Manual Gate](phase-2.md): profile registry, capability
  review, and registry-consistency checks for `gr-profiles`
- [Phase 3 Manual Gate](phase-3.md): config validation, session-option
  compilation, and reviewer-facing policy checks for Phase 3
- [Phase 4 Manual Gate](phase-4.md): fake backend sessions, trace
  record/replay, and runtime gate review for Phase 4
- [Phase 5 Manual Gate](phase-5.md): planner selection, degradation,
  rejection, and plan-snapshot review for Phase 5
- [Phase 6 Manual Gate](phase-6.md): translators, descriptor-backed
  HID shaping, reverse-event decoding, and replay-trace reviewer
  checks for Phase 6
- [Phase 7 Manual Gate](phase-7.md): session runtime orchestration,
  reverse-output delivery, diagnostics, and concurrent fake-session
  review for Phase 7
- [Phase 8 Manual Gate](phase-8.md): Linux `uinput` device visibility,
  evdev event flow, EV_FF rumble delivery, and teardown review
- [Phase 9 Manual Gate](phase-9.md): Linux UHID identity surface,
  reverse-path coverage, support evidence, and deferred Tier D review
- [Phase 10 Manual Gate](phase-10.md): Linux transport planning,
  transport trace replay, and planner portability review
- [Phase 11 Manual Gate](phase-11.md): DualSense USB transport provider
  realization, hardware-faithful evidence, and deferred validation review
- [Phase 12 Manual Gate](phase-12.md): Windows and macOS planning-only
  provider foundations, deployment requirements, and cross-build review
