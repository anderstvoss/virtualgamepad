# virtualgamepad spec overview

`virtualgamepad` is a Rust library for a curated set of tightly integrated
virtual controllers. The controller-native API specification is the source of
truth; legacy profile/session documents below are retained only to migrate the
kernel-facing provider stack.

The crate at the repo root is an early scaffold (see [../../README.md](../../README.md) and [../../CHANGELOG.md](../../CHANGELOG.md)); the runtime API will be built out against the architecture and crate layout defined here.

## What lives here

- [Controller-native implementation](implementation/CONTROLLER_NATIVE_API_SPEC.md): current product and architecture contract
- [Curated controller checklist](implementation/ADDING_A_CURATED_CONTROLLER.md): requirements for first-party additions
- [Legacy references](specs/): prior profile/session planning material, not public API guidance

See [README.md](README.md) for the per-document index.

## Current status

- The crate scaffold exists but exposes no public API yet.
- The active source of truth for design intent is this spec package.

## Next expected step

Scaffold the Rust workspace beyond the current single-crate stub and start implementing the crate layout described in [RUST_IMPLEMENTATION_SPEC.md](implementation/RUST_IMPLEMENTATION_SPEC.md), following the sequencing in [RUST_IMPLEMENTATION_PLAN.md](implementation/RUST_IMPLEMENTATION_PLAN.md).
