# virtualgamepad spec overview

`virtualgamepad` is a Rust-first design and planning package for a virtual controller framework. This directory is the source-of-truth spec.

The crate at the repo root is an early scaffold (see [../../README.md](../../README.md) and [../../CHANGELOG.md](../../CHANGELOG.md)); the runtime API will be built out against the architecture and crate layout defined here.

## What lives here

- [Product and architecture specs](specs/): target architecture, configuration rules, and fidelity definitions
- [Implementation guidance](implementation/): language-agnostic framework guidance plus Rust build plan and crate-level implementation specification
- [Validation strategy](validation/): test plan, headless automation strategy, and device-spec evidence workflow

See [README.md](README.md) for the per-document index.

## Current status

- The crate scaffold exists but exposes no public API yet.
- The active source of truth for design intent is this spec package.
- Any mentions inside spec documents of removed prototypes from upstream design history are historical context, not active code references.

## Next expected step

Scaffold the Rust workspace beyond the current single-crate stub and start implementing the crate layout described in [RUST_IMPLEMENTATION_SPEC.md](implementation/RUST_IMPLEMENTATION_SPEC.md), following the sequencing in [RUST_IMPLEMENTATION_PLAN.md](implementation/RUST_IMPLEMENTATION_PLAN.md).
