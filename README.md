# virtualgamepad

`virtualgamepad` is a pre-1.0 Rust foundation for reviewed, compiled virtual
controllers. It has no runtime profiles, YAML configuration, or plugin
registry. The current curated packages are Generic Gamepad, Xbox 360, and
DualSense; their constructors require explicit Linux target selection.

The core separates controller semantics from host realization. Future
controller packages choose exact Linux realization targets and may independently implement
`Evdev` (uinput), `Hid` (UHID), and `UsbGadget` (a software USB gadget bound to
`dummy_hcd`).
No target implies another
and no provider fallback occurs.

The active workspace contains controller-neutral realization/contracts/runtime
crates plus Linux uinput, UHID, and USB gadget providers. Retired profile-era
code is preserved only on archival branches.

The project direction is three peer creation levels: evdev, UHID, and an
in-VM USB-gadget backend that uses `dummy_hcd`, not a physical UDC. Each
controller must be able to declare any subset of those levels, and creation
must select one exact level with no fallback. The current `UsbTransportValidation`
API is transitional and will be replaced by that deployable `UsbGadget` level.
The full policy is documented in
[docs/CORE_ARCHITECTURE.md](docs/CORE_ARCHITECTURE.md) and
[docs/DEPLOYMENT_AND_VALIDATION.md](docs/DEPLOYMENT_AND_VALIDATION.md).

Each concrete controller has native typed state and numeric domains. The
library does not normalize stick, trigger, touch, or sensor values across
families. After creation, a controller's typed `surface()` describes its exact
Linux presentation—event codes, axis ranges, neutral values, outputs, and
target restrictions—so embedding applications can adapt without guessing.
Common spatial face-button and D-pad labels are available only for digital
convenience; native controller controls remain explicit types.

See the [controller-package architecture](docs/spec/implementation/CONTROLLER_PACKAGE_ARCHITECTURE.md)
and [family modeling guide](docs/spec/implementation/CONTROLLER_FAMILY_MODELING.md)
before adding a curated controller.

## Development

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```
