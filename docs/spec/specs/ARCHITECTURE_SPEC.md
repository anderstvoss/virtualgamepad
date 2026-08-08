# Architecture specification

The authoritative architecture is
[CONTROLLER_NATIVE_API_SPEC.md](../implementation/CONTROLLER_NATIVE_API_SPEC.md).

`virtualgamepad` is divided into four one-way layers:

1. `gr-realization-api` defines controller-neutral Linux targets, independent
   realization modes, prepared OS realizations, and provider contracts.
2. `gr-controller-contract` defines normalized controls, lifecycle errors,
   realization manifests, and controller contracts.
3. `gr-controller-runtime` implements atomic local updates, dirty tracking,
   retry-safe commits, and terminal closure without controller-family logic.
4. Future controller packages own compiled controller state, native controls,
   validation, report codecs, reverse events, and realization data.
5. Linux providers own kernel or transport I/O. They receive an immutable
   controller realization and encoded frames; they do not choose controller
   semantics.

The current root crate is core-only while controller packages are rebuilt. A
future constructor selects `LinuxTarget` explicitly. Creation proves that the
controller supplies the exact independent realization mode and target; there
is no automatic fallback.

The normal root dependency graph excludes the legacy profile registry,
planner, session actor, translator dispatch, and YAML configuration. Retained
pre-redesign workspace crates are compatibility-isolated and are not part of
the controller-native product path.

See
[ADDING_A_CURATED_CONTROLLER.md](../implementation/ADDING_A_CURATED_CONTROLLER.md)
for the extension boundary and [TEST_PLAN.md](../validation/TEST_PLAN.md) for
the required assurance layers.
