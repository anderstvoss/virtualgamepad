# Host readiness record

Status: `not_run`. This is a survey worksheet, not a host-setup script. Do not load modules, change permissions/drivers, or create devices merely to fill it out. Record public-safe aliases; keep machine names, serials, MACs, and raw logs out of Git.

| Area | Record before live work | Current result |
| --- | --- | --- |
| Toolchain | Installed Rust/Cargo, declared pin and MSRV check strategy | Not surveyed |
| Linux | Kernel release/config, architecture, UHID/uinput/gadget API availability | Not surveyed |
| Access | Whether expected device nodes exist and ordinary-user access is provisioned | Not surveyed |
| USB gadget | ConfigFS availability, broker version/profile set, dummy UDC capacity and ownership | Not surveyed |
| SDL | SDL3 dev/runtime and HIDAPI version/settings; probe build prerequisites | Not surveyed |
| Steam | Version/settings, interactive availability, session-specific identification, safe log handling | Not surveyed |
| Reference hardware | DualSense model/firmware alias, USB/BT availability; SC2/Puck if available | Not surveyed |
| Capture | usbmon/btmon tooling and authorized capture workflow; restricted raw storage | Not surveyed |
| Audio | ALSA/PipeWire and playback/capture observation tools, only for F/H | Not surveyed |
| Bluetooth | BlueZ/btvirt versions and isolated experiment environment, only for L/M | Not surveyed |

Use existing `docs/SDL_ACCEPTANCE.md` and operator scripts after inspecting their current interfaces. Freeze experiment-specific kernel/driver/consumer settings and verify cleanup before reuse. Testing driver binding is a separate controlled variable from bus identity.

Missing reference hardware blocks physical validation. Missing Steam blocks Steam evidence. Neither prevents fake-clock protocol work. Do not label absence a provider defect or automatically install/provision dependencies.

## Read-only execution survey — 2026-09-05

Owner: Codex. Linux arm64, kernel `6.12.105+deb13-arm64`. The ordinary process cannot read/write the root-only UHID node. uinput exists with group/ACL access; actual opening remains a separate test. No UDC class or ConfigFS gadget directory was visible. SDL3 was not discoverable through pkg-config. A Steam launcher and btmon are installed; interactive Steam operation, physical devices, firmware, and capture authorization are unverified. btvirt was not found.

B/P and live G are blocked pending a provisioned host. Proposed setup: confirm kernel UHID/uinput/dummy_hcd/ConfigFS support, provision narrowly scoped device access and broker service using the repository deployment procedure, install SDL3 development prerequisites, and supply an interactive Steam session plus a reference-controller alias. Review concrete commands before changing host policy. No modules, permissions, drivers, packages, or devices were changed by this survey.

The live uinput process-owned creation/destruction test subsequently passed using existing ordinary-user access. No host configuration changed. The [provisioning proposal](HOST_PROVISIONING.md) is prepared for review for the remaining UHID/gadget work.
