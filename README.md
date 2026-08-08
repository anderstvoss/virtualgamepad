# virtualgamepad

`virtualgamepad` is a pre-1.0 Rust foundation for reviewed, compiled virtual
controllers. It has no runtime profiles, YAML configuration, plugin registry,
or currently shipped controller constructors.

The core separates controller semantics from host realization. Future
controller packages choose exact Linux realization targets and may independently implement
`Evdev` (uinput), `Hid` (UHID), and `UsbTransportValidation` (USB gadget).
No target implies another
and no provider fallback occurs.

The active workspace contains controller-neutral realization/contracts/runtime
crates plus Linux uinput, UHID, and USB gadget providers. Retired profile-era
code is preserved only on archival branches.

uinput and UHID are normal deployment targets on hosts that already expose
usable device nodes. USB gadget is an explicit, opt-in transport-validation API
for an already-provisioned lab facility; absence or access failure returns an
error and never falls back. The library never changes permissions, loads
kernel modules, or configures the host. Controller audio streams and attached
devices require separately declared realizations; they are not implied by
ordinary controller reports. The full policy is documented in
[docs/CORE_ARCHITECTURE.md](docs/CORE_ARCHITECTURE.md) and
[docs/DEPLOYMENT_AND_VALIDATION.md](docs/DEPLOYMENT_AND_VALIDATION.md).

## Development

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```
