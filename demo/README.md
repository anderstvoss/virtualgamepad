# virtualgamepad demo

`vgpd-demo` is a small reference consumer for the controller-native public
API. It deliberately has no profile browser, YAML configuration, session
planner, or automatic provider selection.

It lets a developer choose one curated controller and one explicit Linux
target, apply normalized controls to local mutable state, and call `commit()`.
Creation errors are shown verbatim: a target that cannot exactly realize the
selected controller is rejected rather than downgraded.

## Running

```bash
cargo run -p virtual_gamepad_demo -- info
cargo run -p virtual_gamepad_demo -- gui
```

The GUI is Linux-only and needs the appropriate provider-host permissions for
successful creation. It is a manual reference client; its source demonstrates
the same constructors and `ControllerHandle` operations available to library
users.

## Non-goals

- It is not an embeddable component or a compatibility wrapper.
- It does not define controller behavior through YAML or profiles.
- It does not claim a realization works when the selected Linux target lacks
  the controller's complete declared surface.

## License

[AGPL-3.0-only](../LICENSE), same as the library.
