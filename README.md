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

| Controller | `linux.uinput` | `linux.uhid.usb` | `linux.uhid.bluetooth` | Emulated audio (opt-in) | Actual Bluetooth |
| --- | --- | --- | --- | --- | --- |
| DualSense | WIP | WIP | Undeclared | UHID/PipeWire, USB/IP (WIP) | Deferred |
| DualShock 4 | WIP | WIP | Undeclared | UHID/PipeWire, USB/IP (WIP) | Deferred |
| Switch Pro | WIP | WIP | Undeclared | Undeclared | Deferred |
| Xbox 360 (standard HID, not XInput) | WIP | WIP | Undeclared | UHID/PipeWire, USB/IP (WIP) | Deferred |
| Wii Remote | WIP | Undeclared (fixture only) | WIP | Undeclared | Deferred |
| Steam Controller (2026), wired | WIP (reduced) | WIP | Undeclared | Undeclared | Deferred |
| Xbox Series | WIP (reduced) | Undeclared | WIP | Undeclared | Deferred |

- `linux.uinput`: Linux evdev controls through `/dev/uinput`.
- `linux.uhid.usb`: a HID device with USB bus metadata through `/dev/uhid`. It is
  not an actual USB device.
- `linux.uhid.bluetooth`: local HID with Bluetooth bus metadata through UHID;
  it does not provide radio discovery, pairing or BlueZ integration. Actual
  Bluetooth is deferred (#164).
- `linux.usbip.usb-audio`: opt-in composite HID/UAC2 device through a root-owned
  local broker, declared only for DualSense, DualShock 4 and Xbox 360.
- `linux.dummy_hcd.usb-hid`: declared for DualSense, DualShock 4, Switch Pro
  and Xbox 360, but creation is refused before resources are opened.

No implemented cell is promoted beyond WIP; undeclared cells are not compiled
production targets. See the [support matrix](docs/CONTROLLER_SUPPORT.md) for
status definitions and restrictions. The ordinary root includes all seven
families above; the Steam Puck/receiver is deferred, not part of the wired
Steam realization.

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

See [CONTRIBUTING.md](CONTRIBUTING.md), the [architecture](docs/CORE_ARCHITECTURE.md)
and the [decision records](docs/adr/). Alpha 1 planning and evidence records are
kept in [docs/history/alpha-1](docs/history/alpha-1/README.md).

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
