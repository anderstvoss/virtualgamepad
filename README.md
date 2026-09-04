# virtualgamepad

`virtualgamepad` provides reviewed, compiled virtual controllers. There are no runtime profiles, descriptors, plugins, or generic controller constructors. Each controller selects one exact peer realization target; selection never falls back.

- `Evdev`: a simple Linux gamepad through uinput.
- `Uhid`: a local HID controller with advanced HID behavior.
- `DummyHcd`: complete USB attachment emulation for curated controllers.

The curated controllers are Xbox 360, DualSense, DualShock 4, and Switch Pro.
Each has a compiled `DummyHcd` USB profile; the Xbox 360 profile is explicitly
standard HID rather than a claim of proprietary XInput/xpad emulation.
`DummyHcd` uses only the root-owned local broker at
`/run/virtualgamepad/broker.sock`; applications cannot provide host paths,
descriptors, modules, command lines, identities, or report formats. Bluetooth
realization is deferred work on the `wip/btvirt` branch and is not currently
available.

See [deployment](docs/DEPLOYMENT_AND_VALIDATION.md) for installation and the privilege boundary, and [architecture](docs/CORE_ARCHITECTURE.md) for the target model.

## Development

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```
