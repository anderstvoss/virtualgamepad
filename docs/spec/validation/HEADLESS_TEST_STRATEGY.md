# Headless test strategy

Headless CI runs formatting, all-target/all-feature compilation, warnings-as-
errors clippy, the complete workspace test suite, doc and `trybuild`
compile-fail tests, property tests, feature-minimal checks, and secret scanning.
Controller and provider tests use synthetic identities, reports, and injected
kernel/backend boundaries; no real user data is permitted.

Actual device creation is intentionally separate because `/dev/uinput`,
`/dev/uhid`, configfs, UDC hardware, and an observing host are environment
prerequisites. A privileged result may strengthen a documented support claim;
its absence must never be converted into silent fallback or a weakened
hermetic assertion.

See [TESTING_TOOLING_SPEC.md](../implementation/TESTING_TOOLING_SPEC.md) and the
[`cargo-fuzz` guide](../../../fuzz/README.md).
