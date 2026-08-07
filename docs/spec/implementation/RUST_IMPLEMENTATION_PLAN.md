# Historical Rust implementation plan

The phase-based profile, planner, translator, and YAML-session plan that
previously lived here is non-normative. The clean-break implementation is
specified by [CONTROLLER_NATIVE_API_SPEC.md](CONTROLLER_NATIVE_API_SPEC.md),
with controller additions governed by
[ADDING_A_CURATED_CONTROLLER.md](ADDING_A_CURATED_CONTROLLER.md).

The maintained validation gates are the workspace format, check, clippy,
tests, doc/compile-fail tests, feature-minimal checks, secret scan, optional
fuzz targets, and separately privileged Linux device checks.
