# Configuration specification

The controller-native API has no runtime YAML configuration, profile name,
profile search path, dynamic registry, or automatic backend preference.

Applications configure creation with the strongly typed `CreationOptions`:

- `LinuxTarget` is mandatory and selects exactly one provider;
- output subscription capacity is bounded explicitly;
- the slow-callback diagnostic threshold is explicit;
- future host setup controls must be typed fields with documented defaults.

The current core-only product exposes no controller creation functions.
Restored controller packages will select type through a typed creation function
and state through normalized/native typed methods followed by explicit
`commit()`.

YAML is permitted only for sanitized test fixtures and human-readable report
snapshots. Loading YAML must never affect runtime controller definitions,
realization selection, descriptors, or control mappings.

Exact lifecycle and failure rules are specified in
[CONTROLLER_NATIVE_API_SPEC.md](../implementation/CONTROLLER_NATIVE_API_SPEC.md).
