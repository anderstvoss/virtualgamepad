# Documentation Index

This directory is the source-of-truth spec package for the `virtualgamepad` template repo.

## Specs

- [Architecture Specification](specs/ARCHITECTURE_SPEC.md): product goals, architecture boundaries, planning model, and target component model
- [Configuration Specification](specs/CONFIGURATION_SPEC.md): session-oriented configuration shape and runtime policy rules
- [Fidelity Guide](specs/FIDELITY_GUIDE.md): externally named fidelity tiers and support-claim rules

## Implementation

- [Gamepad Emulation Framework](implementation/IMPLEMENTATION_FRAMEWORK.md): language-agnostic module model
- [Rust Implementation Plan](implementation/RUST_IMPLEMENTATION_PLAN.md): phased sequencing for the Rust buildout
- [Rust Implementation Specification](implementation/RUST_IMPLEMENTATION_SPEC.md): authoritative crate ownership, runtime contracts, and acceptance criteria

## Validation

- [Test Plan](validation/TEST_PLAN.md): test inventory and coverage expectations
- [Headless Test Strategy](validation/HEADLESS_TEST_STRATEGY.md): remote-first automation and fixture replay strategy
- [Device Spec Validation Plan](validation/DEVICE_SPEC_VALIDATION_PLAN.md): evidence dossiers, capture workflow, and reverse-engineering minimization
