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
