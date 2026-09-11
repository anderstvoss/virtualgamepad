# virtualgamepad

`virtualgamepad` provides reviewed, compiled virtual controllers. There are no runtime profiles, descriptors, plugins, or generic controller constructors. Each controller selects one exact peer realization target; selection never falls back.

- `linux.uinput`: controller-owned Linux evdev controls through uinput.
- `linux.uhid.usb`: local HID presentation with controller-owned stateful USB protocols.
- `linux.dummy_hcd.usb-hid`: experimental research SPI only; excluded from normal application constructors (Gate G).

The curated controllers are Xbox 360, DualSense, DualShock 4, and Switch Pro.
Each retains a compiled experimental `DummyHcd` USB profile; the Xbox 360 profile is explicitly
standard HID rather than a claim of proprietary XInput/xpad emulation.
`DummyHcd` uses only the root-owned local broker at
`/run/virtualgamepad/broker.sock`; applications cannot provide host paths,
descriptors, modules, command lines, identities, or report formats. Bluetooth
realizations remain gated research and are not currently available.

See [deployment](docs/DEPLOYMENT_AND_VALIDATION.md) for installation and the privilege boundary, and [architecture](docs/CORE_ARCHITECTURE.md) for the target model.

## Servicing controllers

UHID personalities own calibration/feature replies, output validation, report timing, and Switch handshake behavior. Call `service` on `readiness()` and at `next_service_in()`, including while semantic state is unchanged. `commit()` accepts and submits edited state but is not the only service point. Submission is not proof that a host consumer observed the report. `dropped_output_events()` reports optional observation queue overflow. UHID transport identities distinguish repeated creations independently of application correlation IDs; oversized identity strings are rejected rather than truncated.

Curated uinput sessions also require polling while input is idle. They complete
conventional force-feedback upload/erase requests internally and expose typed
`ForceFeedbackEvent` observations and replay commands. Applications no longer
send manual force-feedback replies. See the [ownership contract](docs/architecture-overhaul/decisions/ADR-0006-conventional-feedback.md).

Use `CreationOptions::new(RealizationId::LINUX_UINPUT)` or `CreationOptions::new(RealizationId::LINUX_UHID_USB)`. The root API generates fresh session IDs internally. Ambiguous target aliases and `poll_output` are removed from the application contract. Required replies remain controller-owned; applications observe native outputs rather than sending manual replies.

See the [application API and migration guide](docs/APPLICATION_API.md),
[public inventory](docs/architecture-overhaul/API_INVENTORY.md), and
[active pre-alpha ledger](docs/architecture-overhaul/PRE_ALPHA_STATUS.md).
The former broad root surface is available only under `virtualgamepad::experimental`
with the `experimental` feature, outside the intended alpha stability boundary.


For explicit DualSense/DS4 USB/UHID recreation, generate a `DualSenseIdentity` or
`DualShock4Identity`, store its `to_bytes()` in your application, and restore it
with `from_bytes()`. Pass it to `create_dualsense_with_identity` or
`create_dualshock4_with_identity`. The pairing identity and UHID `uniq` stay stable;
transport sessions are fresh. Other targets reject this option. Existing creation
functions remain ephemeral. See the [identity contract](docs/architecture-overhaul/decisions/ADR-0008-explicit-sony-identity.md)
for consumer-association limits and concurrent-identity policy.

The [controller support matrix](docs/CONTROLLER_SUPPORT.md) currently marks all
entries WIP. The [active pre-alpha plan](docs/architecture-overhaul/PRE_ALPHA_STATUS.md)
requires controller/demo refinement and hands-on feedback before the final API
review, separate quality pass and Git-based alpha release.

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

The [core acceptance matrix](docs/architecture-overhaul/CORE_ACCEPTANCE.md) records
measured family/provider results. SDL evdev controls and conventional rumble pass
for DualSense, Switch Pro and Xbox on the recorded profile. DS4's combined evdev
touch/gamepad node currently fails SDL gamepad discovery; use its measured UHID
profile while a separate touch presentation is developed. Full evdev touch and
physical fidelity remain unvalidated.

Physical reference testing currently targets the available DualSense, Xbox Series
and Steam Controller hardware. Other families remain best-effort with explicit
limitations and deterministic/virtual tests. Xbox Series hardware does not
validate Xbox 360 protocol fidelity. See the
[physical validation policy](docs/architecture-overhaul/PHYSICAL_VALIDATION_POLICY.md).

The demo services controllers on background workers even while the window is not
repainting. Each worker owns its controller; the GUI submits bounded native edits
and waits for an applied snapshot before accepting another batch. Its output log is bounded; older optional display messages can be
omitted during UI stalls. Removing a controller stops its worker and closes its
session independently of the remaining controllers.

For repeatable manual experiments, see the [demo lab guide](docs/DEMO_LAB.md).
The GUI includes session-ID reuse, service timing counters, copyable lab notes
and stop-all cleanup. “Release all inputs” uses the acknowledged edit queue;
lab records include consumer build/backend/mapping and association/cleanup
diagnostics. Measurements remain separate from physical acceptance.
