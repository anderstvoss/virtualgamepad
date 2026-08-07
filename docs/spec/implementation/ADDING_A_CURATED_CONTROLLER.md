# Adding a curated controller

This is a checklist for adding reviewed, compiled Rust support. It is not a
runtime-extension or plugin guide.

1. Model the controller's actual state and native controls in `gr-controllers`.
   Use normalized controls only where their semantics are genuinely shared.
2. Map spatial face buttons to native labels bidirectionally. Do not export
   ambiguous printed-label names without a controller type.
3. Implement `ControllerDefinition` and `ControllerDriver`, including an
   immutable `PreparedControllerFrame` variant and deterministic validation.
4. Define descriptor/identity, forward report encoding, reverse decoding, and
   typed output events in the controller module.
5. Declare exact identity, transport, and reverse-output requirements. Extend
   a provider only for OS realization mechanics—not controller control logic.
6. Add one root creation function and closed-handle variants. Concrete APIs
   should make absent features impossible to call.
7. Add mapping, range, property, state-machine, fault-injection, compile-fail,
   codec fixture, and Linux integration tests. Document any privileged gate.
8. Update the canonical API spec with supported targets, host prerequisites,
   fidelity claim, and all native-only features.

Runtime registration, profile IDs, and YAML controller definitions are
intentionally forbidden.
