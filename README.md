# virtualgamepad

`virtualgamepad` provides reviewed, compiled virtual controllers. There are no runtime profiles, descriptors, plugins, or generic controller constructors. Each controller selects one exact peer realization target; selection never falls back.

- `linux.uinput`: controller-owned Linux evdev controls through uinput.
- `linux.uhid.usb`: local HID presentation with controller-owned stateful USB protocols.
- `linux.dummy_hcd.usb-hid`: selectable curated USB HID attachment through the broker.

The curated controllers are Xbox 360, DualSense, DualShock 4, and Switch Pro.
Each has a compiled `DummyHcd` USB profile; the Xbox 360 profile is explicitly
standard HID rather than a claim of proprietary XInput/xpad emulation.
`DummyHcd` uses only the root-owned local broker at
`/run/virtualgamepad/broker.sock`; applications cannot provide host paths,
descriptors, modules, command lines, identities, or report formats. Bluetooth
realizations remain gated research and are not currently available.

See [deployment](docs/DEPLOYMENT_AND_VALIDATION.md) for installation and the privilege boundary, and [architecture](docs/CORE_ARCHITECTURE.md) for the target model.

## Servicing controllers

UHID personalities own calibration/feature replies, output validation, report timing, and Switch handshake behavior. Call `poll_output` on `readiness()` and at `next_service_in()`, including while semantic state is unchanged. `commit()` accepts and submits edited state but is not the only service point. Submission is not proof that a host consumer observed the report. `dropped_output_events()` reports optional observation queue overflow. UHID transport identities distinguish repeated creations independently of caller session IDs; oversized identity strings are rejected rather than truncated.

`CreationOptions.target` accepts `RealizationId::LINUX_UINPUT`, `LINUX_UHID_USB`, or `LINUX_DUMMY_HCD_USB_HID`. The old target names remain aliases. Required HID replies are protocol-owned; application reply methods cannot override or duplicate them. Switch stream status is available on the controller handle.

The [architecture gate ledger](docs/architecture-overhaul/GATE_STATUS.md) separates deterministic results from blocked live-host work. The broker's dynamic protocol migration, composite/audio behavior, and Bluetooth extensions are not complete.

## Development

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```

Host access can be inspected without changes using
`python3 scripts/host-preflight.py all` (or a complete realization ID). Builds do
not need device permissions. Opt-in creation access, consumer access, and the
optional gadget broker have separate [deployment requirements](docs/DEPLOYMENT_AND_VALIDATION.md).
