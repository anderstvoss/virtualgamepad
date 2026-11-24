# virtualgamepad

Sustained audio continuity and latency are not validated in alpha; occasional audio dropouts may occur.

`virtualgamepad` is a Rust library for creating reviewed, compiled virtual game
controllers on Linux. Each controller keeps its own native controls, units and
outputs; there is no generic gamepad model, runtime profile format or plugin
system. The application picks one exact realization and the library never falls
back to another.

Current release: [`0.1.0-alpha.1`](https://github.com/anderstvoss/virtualgamepad/releases/tag/0.1.0-alpha.1)
(Git-only, `publish = false`). See [status](docs/STATUS.md) for what is and is not
validated.

## Controllers and realizations

| Controller | `linux.uinput` | `linux.uhid.usb` | `linux.uhid.bluetooth` | `linux.usbip.usb-audio` |
| --- | --- | --- | --- | --- |
| DualSense | WIP | WIP | WIP | Opt-in WIP |
| DualShock 4 | WIP | WIP | WIP | Opt-in WIP |
| Switch Pro | WIP | WIP | WIP | — |
| Xbox 360 (standard HID; not XInput) | WIP | WIP | — | Opt-in WIP |
| Wii Remote | WIP | — | WIP | — |
| Steam Controller 2026 (wired; not Puck) | WIP, reduced | WIP | — | — |
| Xbox Series | WIP, reduced | — | WIP | — |

- `linux.uinput`: Linux evdev controls through `/dev/uinput`.
- `linux.uhid.usb`: a HID device with USB bus metadata through `/dev/uhid`. It is
  not an actual USB device.
- `linux.uhid.bluetooth`: local HID with Bluetooth bus metadata; this does
  **not** implement Bluetooth discovery, pairing or a radio transport.
- `linux.usbip.usb-audio`: opt-in real USB/IP composite HID/UAC2 for
  DualSense, DualShock 4 and Xbox 360, using the current installed root broker.
- `linux.dummy_hcd.usb-hid`: declared for four families but **refused**
  before resource creation (incomplete Linux f_hid request semantics).

All compiled family realizations remain WIP in the family support overview;
individual acceptance-manifest functions may have narrower evidence-backed
statuses. An em dash means **undeclared**, not an unsupported implementation.
Actual Bluetooth radio and the Steam Puck are deferred. See the
[support matrix](docs/CONTROLLER_SUPPORT.md) for exact claims and restrictions.

## Usage

```rust
use virtualgamepad::{create_dualsense, CreationOptions, DualSenseControl, DualSenseOptions, RealizationId};
let mut pad = create_dualsense(
    CreationOptions::new(RealizationId::LINUX_UHID_USB),
    DualSenseOptions::default(),
)?;
pad.set_native(DualSenseControl::Cross, true)?;
pad.commit()?;
pad.service(&mut |_output| {})?; // keep servicing, even while input is idle
pad.close();
```

A controller is a live protocol endpoint: call `service` when its readiness
descriptor is readable and at `next_service_in()`, even when nothing changed.
See [`examples/controller_loop.rs`](examples/controller_loop.rs) and the
[application API guide](docs/APPLICATION_API.md).

Features: `audio-pipewire` and `audio-usbip` enable emulated controller audio
(off by default; see the [audio guide](docs/CONTROLLER_AUDIO.md)). `experimental`
exposes unstable research interfaces outside the API stability boundary.

## Host setup

Building needs no device permissions. Creating controllers needs access to
`/dev/uinput` or `/dev/uhid`; USB/IP audio needs the administrator-installed
broker. Inspect a host without changing it:

```bash
python3 scripts/host-preflight.py all
```

See [deployment and validation](docs/DEPLOYMENT_AND_VALIDATION.md).

## Demo

```bash
cargo run -p virtualgamepad-demo
```

The demo creates and drives controllers interactively; see the
[demo lab guide](docs/DEMO_LAB.md).

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
```

Start with the [documentation index](docs/README.md) for integration,
controller implementation and acceptance paths. See [CONTRIBUTING.md](CONTRIBUTING.md), the [architecture](docs/CORE_ARCHITECTURE.md)
and the [decision records](docs/adr/). Alpha 1 planning and evidence records are
kept in [docs/history/alpha-1](docs/history/alpha-1/README.md).

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
