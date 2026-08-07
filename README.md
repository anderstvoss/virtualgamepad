# VirtualGamepad

`virtualgamepad` is a Rust library for creating a small, curated set of Linux
virtual controllers through controller-specific, strongly typed APIs. It is a
compiled controller emulator—not a YAML profile engine, runtime registry, or
plugin host.

The public API is intentionally breaking before 1.0. Its source of truth is
the [controller-native API specification](docs/spec/implementation/CONTROLLER_NATIVE_API_SPEC.md).

## Supported creation paths

Callers must choose a Linux target explicitly. Creation never falls back to a
different target or a less faithful device.

| Controller | `Uinput` | `Uhid` | `UsbTransport` |
|---|---:|---:|---:|
| Generic Gamepad | compatibility | rejected | rejected |
| Xbox 360 | compatibility | rejected | rejected |
| DualSense | rejected | identity-aware | USB gadget |
| Steam Controller | rejected | rejected | rejected |

Steam Controller has a typed compiled API, but creation currently returns an
actionable error because no Linux provider realizes its complete declared
surface. Windows and macOS do not expose controller creation APIs.

## Using the API

State changes are local, validated, and atomic. `commit()` submits the complete
current state. A rejected update preserves the prior state; a failed commit
keeps the controller dirty and available for retry.

```rust,no_run
use virtualgamepad::{
    ControlUpdate, CreationOptions, DualSenseControl, FaceButton, LinuxTarget,
    create_dualsense,
};

let mut controller = create_dualsense(CreationOptions::new(LinuxTarget::Uhid))?;

// Normalized labels are spatial and portable.
controller.apply(ControlUpdate::FaceButton {
    button: FaceButton::South,
    pressed: true,
})?;

// Native labels are explicit controller-specific types.
controller.set_native(DualSenseControl::Cross, true)?;
if let Err(error) = controller.commit() {
    eprintln!("commit remains retryable: {error}");
}

let diagnostics = controller.diagnostics();
assert_eq!(diagnostics.controller, virtualgamepad::ControllerKind::DualSense);
controller.close()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Normalized names use physical positions such as
`FaceButton::{North, South, East, West}`. Native names use types such as
`DualSenseControl::Cross` and `XboxControl::A`; ambiguous methods such as
`button_x` are deliberately absent.

Reverse output is delivered through bounded typed subscriptions. Callback
panics cancel only that subscription, slow callbacks do not run on the commit
path, and delivery health is available through `diagnostics()`.

## Linux prerequisites

- `Uinput` requires access to `/dev/uinput`.
- `Uhid` requires access to `/dev/uhid`.
- `UsbTransport` requires a peripheral-capable USB controller, configfs gadget
  support, an available UDC, and the permissions needed to configure it.

Missing Cargo provider features, unsupported controller/target pairs, and host
open failures produce distinct creation errors. See the
[demo](demo/README.md) for a reference consumer.

## Development

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```

Compile-fail tests enforce controller-specific API boundaries. Property tests
exercise mapping and lifecycle invariants. Raw reverse reports and generated
control sequences have dedicated [`cargo-fuzz` targets](fuzz/README.md).
Privileged Linux device tests remain separate from the hermetic suite.

Record user-visible changes in [CHANGELOG.md](CHANGELOG.md). Repository setup
and security checks are documented in [docs/REPO-SETUP.md](docs/REPO-SETUP.md),
[docs/HARDENING-CHECKLIST.md](docs/HARDENING-CHECKLIST.md), and
[SECURITY.md](SECURITY.md).

## License

[AGPL-3.0-only](LICENSE)
