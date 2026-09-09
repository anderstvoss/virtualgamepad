# Host readiness record

Status: `surveyed`; live B/P/G remain blocked. Historical worksheet entries below are superseded by the dated surveys and EXP-0005. This is not a host-setup script. Do not load modules, change permissions/drivers, or create devices merely to fill it out. Record public-safe aliases; keep machine names, serials, MACs, and raw logs out of Git.

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

## Scoped preparation — 2026-09-06

The refreshed read-only inventory and exact build outcomes are in
[EXP-0005](experiments/EXP-0005-host-access.md). ConfigFS is mounted, but its gadget
subtree is absent. UHID's node alone does not establish misc registration.
Private SDL input tooling is now built; desktop/Steam acceptance is not established.
Administrator access remains unavailable, so no temporary ACLs, module loads,
or service changes were applied. Existing uinput creation/cleanup passed again.

## Updated-helper live result

The reviewed installed helper successfully loaded UHID, waited for udev, granted
temporary access, and verified its lease. The ordinary-user provider
creation/input/close test passed. Restoration and repeated restoration passed;
both helper leases are inactive and the named-user ACL is absent. Existing
host group policy remains unchanged; the newly loaded module is retained.
The prior failed lease was already absent before this run, so its special recovery
path was not validated live. See [EXP-0005](experiments/EXP-0005-host-access.md).
Controlled B/P consumer comparisons and gadget prerequisites remain outstanding.

## Current SDL/core state

The previous temporary SDL directory disappeared. Pinned SDL 3.2.0 console/input
tooling has now been recreated in a private user-owned cache. Scoped Linux/SDL
DualSense baseline/rewrite acceptance and failure cleanup pass as recorded in
[EXP-0008](experiments/EXP-0008-sdl-core.md). The installed helper is working;
no additional provisioning was required for these runs. Earlier survey entries
are historical and do not describe current access. Steam, physical references
and reserved gadget resources remain independently unvalidated.

## Current conventional-feedback preparation

Uinput registration was absent at the start of EXP-0010. The installed helper
loaded only uinput through its existing policy. Existing ordinary-user creation
and experiment-node access sufficed; no ACL or group changes were needed.
Four-family conventional-feedback ioctls and cleanup passed. The module remains
loaded for administrator review and both helper leases remain inactive. This
supersedes the missing-uinput entry in the earlier core matrix. SDL evdev input
acceptance remains a separate next step.

The current run reports kernel `6.12.107+deb13-arm64`, unlike the earlier
`6.12.105` UHID/SDL records. No kernel change or reboot was performed by this task.
Recheck the recorded UHID/SDL configurations before claiming acceptance on the
current kernel; preserve the older results with their original host version.

## Final core review input tooling

EXP-0017 rebuilt pinned SDL 3.2.0 in a fresh private prefix with input-only console
configuration (audio/camera disabled after an old PipeWire API build failure).
All four current UHID families passed individual controls and cleanup on the
recorded 6.12.107 arm64 host. Existing creation access was reused; no permissions,
modules or services changed for these runs. Raw build/experiment records remain
private. Earlier “not surveyed” and missing-cache entries above are historical;
this does not establish interactive Steam/Eden, isolated touch or gadget readiness.
