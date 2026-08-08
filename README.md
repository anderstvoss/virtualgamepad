# VirtualGamepad

`virtualgamepad` is a Rust library for creating a small, curated set of Linux
virtual controllers through controller-specific, strongly typed APIs. It is a
compiled controller emulator—not a YAML profile engine, runtime registry, or
plugin host.

The public API is intentionally breaking before 1.0. Its source of truth is
the [controller-native API specification](docs/spec/implementation/CONTROLLER_NATIVE_API_SPEC.md).

## Realization-mode core migration

The public product is temporarily core-only while curated controller packages
are rebuilt. It exposes no production controller constructors during this
pre-1.0 migration. Steam Controller 1 is not an active target; its preserved
source lives on `archive/steam-controller-1`.

Future controllers select an exact Linux target and therefore one independent
host-realization mode: `HostCompatible` (uinput), `IdentityAccurate` (UHID),
or `HardwareFaithful` (USB gadget transport). These are not fallback tiers.
They affect how the host sees a controller, not the controller's typed
normalized/native command vocabulary.

## Controller contract

Restored controllers will use mutable typed state and explicit `commit()`.
Normalized labels use spatial positions; native labels are explicit
controller-specific types. The same operations retain their meaning in every
supported realization mode. A feature unavailable in the selected mode returns
a recoverable error without changing state; a failed commit stays dirty and is
retryable.

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
