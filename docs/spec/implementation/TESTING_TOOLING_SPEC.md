# Controller-native testing and tooling

The hermetic test suite covers:

- normalized/native equivalence and controller-specific range validation;
- atomic rejected updates, dirty-state preservation, commit retry, and close;
- controller-owned identity, descriptor, feature-report, evdev, and USB specs;
- deterministic frame encoding and robust typed reverse decoding;
- provider/open/send/read/close fault injection and callback containment;
- bounded subscriptions, drop cleanup, diagnostics, and heterogeneous handles;
- compile-fail controller-native type boundaries with `trybuild`;
- property-generated control and lifecycle sequences;
- no-default-feature and individual-provider feature compilation.

[`fuzz/`](../../../fuzz/README.md) contains isolated `cargo-fuzz` targets for
raw reverse reports and generated control sequences. Privileged `/dev/uinput`,
`/dev/uhid`, and configfs gadget checks remain opt-in gates on prepared Linux
hosts. YAML is allowed only as sanitized fixture or snapshot serialization; it
must never define runtime controller behavior.

The required local gates are listed in the repository [README](../../../README.md).
