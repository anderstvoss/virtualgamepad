# EXP-0003 — UHID production integration and family regression

Owner: Codex. Execution session: 2026-09-05–06. Inputs: baseline `9b466e0`, corpus `9d0d56e`, contract foundation `7202732` plus lint correction `b2f67b2`. The production revision is the commit containing this record. Branch: `architecture/protocol-session-rewrite`; review: PR #106.

Scope: E4 deterministic DualSense USB/UHID slice and E6 UHID migration of DS4, Switch Pro, and standard-HID Xbox 360. No live UHID, Steam, or physical acceptance is claimed.

Predeclared criteria: preserve semantic/evdev regressions; move feature responses and SET acceptance out of the provider; handle startup requests without callbacks; preserve report classes and exact error acknowledgements; preserve retry identity, lifecycle, cleanup, and session isolation; service idle cadence and Switch handshake in the personality.

Results: the workspace suite passes. New real-personality/fake-provider tests cover all families' feature and SET completion, repeated kernel request IDs, output validation before success, reply backpressure, callback-free Switch handshake and cadence, malformed SET completion/cancellation, STOP cancellation followed by START, and nonterminal consumer CLOSE. Provider tests cover all START numbering flags, single-event writes, duplicate reply rejection, and destruction. Existing family layouts, uinput parity limitations, touch/motion regressions, compound rollback, and UI selection tests remain.

A review found that `write_all` is unsuitable for UHID event framing after a short write. The provider now attempts one event write and treats a short write as uncertain delivery; only WouldBlock is retryable. A regression asserts that no suffix is submitted as another event. Cleanup failure is terminal and no longer retried by the curated sink's Drop.

`RealizationId` replaces the closed target enum with cohesive compiled IDs; the old names remain aliases. Static target sets use declared slices and test unordered membership/hash consistency. Unknown IDs still fail manifest/provider preparation without fallback.

Validation commands: required workspace format/check/Clippy/test commands, `gitleaks detect --redact`, and `cargo test -p gr-provider-linux-uinput creates_and_destroys_a_process_owned_device -- --ignored`. The available live uinput creation/destruction test passed. UHID access, SDL3 setup, and a usable interactive reference comparison are still missing; B/P remain blocked.

Migration boundary: uinput and dummy_hcd retain their existing frame runtime. Gate G still blocks broker ownership replacement; full compound/audio/Bluetooth extensions are separately gated. No obsolete second UHID implementation remains active, but retiring the remaining frame runtime would prematurely remove ungated providers. See ADR-0005.
