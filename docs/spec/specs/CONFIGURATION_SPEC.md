# Configuration specification

The controller-native API has no runtime YAML configuration, profile name,
profile search path, dynamic registry, or automatic backend preference.

Applications configure creation with the strongly typed `CreationOptions`:

- `LinuxTarget` is mandatory and selects exactly one provider;
- output subscription capacity is bounded explicitly;
- the slow-callback diagnostic threshold is explicit;
- future host setup controls must be typed fields with documented defaults.

Controller type is selected by the creation function, such as
`create_dualsense` or `create_xbox360`. Controller state is configured through
that handle's normalized and native typed methods, then submitted with an
explicit `commit()`.

YAML is permitted only for sanitized test fixtures and human-readable report
snapshots. Loading YAML must never affect runtime controller definitions,
realization selection, descriptors, or control mappings.

Exact lifecycle and failure rules are specified in
[CONTROLLER_NATIVE_API_SPEC.md](../implementation/CONTROLLER_NATIVE_API_SPEC.md).
