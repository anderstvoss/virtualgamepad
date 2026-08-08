# Adding a curated controller

This is a first-party compiled-code checklist, not a plugin guide.

1. Create a controller package with typed state, native controls, normalized
   mappings, deterministic validation, codecs, and typed reverse events.
2. Declare a non-empty independent realization manifest. For every advertised
   mode, define exact target(s), host prerequisites, fidelity claim, prepared
   OS realization, and feature surface. Do not infer another mode.
3. Keep mode-gated features typed. Expose a typed capability query and return
   `UnavailableInRealizationMode` before state mutation when necessary.
4. Add one root creation function and, if needed, a closed heterogeneous-handle
   variant. Do not change the generic runtime or add provider family branches.
5. Add unit, property, state-machine, fault-injection, compile-fail, codec,
   reverse-output, and provider integration coverage. Add sanitized fixtures
   only for report behavior.
6. Document target matrix, host prerequisites, feature availability by mode,
   and supported-host validation evidence before advertising the controller.
