# Architecture specification

The authoritative architecture is
[CONTROLLER_NATIVE_API_SPEC.md](../implementation/CONTROLLER_NATIVE_API_SPEC.md).

`virtualgamepad` is divided into four one-way layers:

1. `gr-controller-contract` defines normalized controls, lifecycle errors,
   realization requirements, and controller/provider contracts.
2. `gr-controller-runtime` implements atomic local updates, dirty tracking,
   retry-safe commits, and terminal closure without controller-family logic.
3. `gr-controllers` owns every compiled controller's state, native controls,
   validation, report codecs, reverse events, identity, descriptors, and exact
   realization data.
4. Linux providers own kernel or transport I/O. They receive an immutable
   controller realization and encoded frames; they do not choose controller
   semantics.

The root crate supplies the closed public handle enum and four concrete
creation functions. A caller selects `LinuxTarget` explicitly. Creation first
proves that the controller module supplies that exact realization and that the
provider satisfies all declared requirements. There is no automatic fallback.

The normal root dependency graph excludes the legacy profile registry,
planner, session actor, translator dispatch, and YAML configuration. Retained
pre-redesign workspace crates are compatibility-isolated and are not part of
the controller-native product path.

See
[ADDING_A_CURATED_CONTROLLER.md](../implementation/ADDING_A_CURATED_CONTROLLER.md)
for the extension boundary and [TEST_PLAN.md](../validation/TEST_PLAN.md) for
the required assurance layers.
