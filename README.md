# virtualgamepad

`virtualgamepad` is a pre-1.0 Rust foundation for reviewed, compiled virtual
controllers. It has no runtime profiles, YAML configuration, plugin registry,
or currently shipped controller constructors.

The core separates controller semantics from host realization. Future
controller packages choose exact Linux targets and may independently implement
`HostCompatible` (uinput), `IdentityAccurate` (UHID), and
`HardwareFaithful` (USB gadget) presentation modes. No mode implies another
and no provider fallback occurs.

The active workspace contains controller-neutral realization/contracts/runtime
crates plus Linux uinput, UHID, and USB gadget providers. Retired profile-era
code is preserved only on archival branches.

## Development

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```
