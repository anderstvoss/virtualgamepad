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

| Controller | `linux.uinput` | `linux.uhid.usb` | Emulated audio (opt-in) |
| --- | --- | --- | --- |
| DualSense | WIP | WIP | UHID/PipeWire, USB/IP |
| DualShock 4 | WIP | WIP | UHID/PipeWire, USB/IP |
| Switch Pro | WIP | WIP | — |
| Xbox 360 (standard HID, not XInput) | WIP | WIP | UHID/PipeWire, USB/IP |

- `linux.uinput`: Linux evdev controls through `/dev/uinput`.
- `linux.uhid.usb`: a HID device with USB bus metadata through `/dev/uhid`. It is
  not an actual USB device.
- `linux.usbip.usb-audio`: opt-in composite HID/UAC2 device through a root-owned
  local broker.
- `linux.dummy_hcd.usb-hid`: unavailable; creation is rejected before any
  resources are opened.

No cell is promoted beyond WIP yet; see the [support matrix](docs/CONTROLLER_SUPPORT.md)
for the status definitions. Wii Remote is available as a WIP
[supporting package](docs/WII_REMOTE.md), with uinput and Bluetooth-bus UHID;
the ordinary-root constructors remain unchanged. Steam Controller (2026) and
Xbox Series remain separate planned work.

## Usage

```rust
use virtualgamepad::{create_dualsense, CreationOptions, DualSenseControl, RealizationId};
let mut pad = create_dualsense(CreationOptions::new(RealizationId::LINUX_UHID_USB))?;
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

See [CONTRIBUTING.md](CONTRIBUTING.md), the [architecture](docs/CORE_ARCHITECTURE.md)
and the [decision records](docs/adr/). Alpha 1 planning and evidence records are
kept in [docs/history/alpha-1](docs/history/alpha-1/README.md).

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
