# EXP-0005 — Host access separation and broker ownership

Owner: Codex. Date: 2026-09-06. Scope: the approved minimal-permission preparation
plan. This record does not promote B/P/G or hardware fidelity.

## Implemented and tested

Read-only per-realization preflight distinguishes missing mechanisms, denied
access, identity mismatch, occupied authorized UDCs, and unvalidated live criteria.
Unreadable binding inventories fail closed instead of reporting a free UDC.
No device node is opened and no provisioning subprocess is available in that tool.
Tests check read-only behavior with synthetic host trees and denied access.

Runtime broker module loading has been removed. Administrator config now requires
an instance and explicit allowed dummy UDCs as well as allowed UIDs. Selection
never falls back to an unauthorized controller. The broker holds a cross-process
exclusive lock; recovery validates instance journals, inode/device identity, and
UDC binding before any cleanup. Missing ConfigFS retains records. Ambiguous or
symlink records fail closed. Failed root creation cannot trigger cleanup of a
pre-existing research resource. Records survive restarts and are forgotten only
after cleanup. Existing unauthorized-client, disconnect, rollback, and host-write
failure tests remain; new tests cover denied opens without host calls, instance
locking/restart, scoped recovery, binding/identity changes, and idempotence.

The optional service removes CAP_SYS_MODULE and ambient capabilities and enables
ProtectKernelModules. CAP_SYS_ADMIN is explicitly retained pending live testing;
its necessity is not established. An empty-capability experimental drop-in is
provided without claiming it is validated. Unit syntax verification passed with
only the unavailable installed executable replaced in a private test copy.

The SDL launcher uses a fresh private build directory and cleans it on compile
failure, probe failure, and success. A regression verifies a pre-existing shared
filename is neither executed nor changed. Producer, consumer, installation, and
experiment permissions are documented separately, with opt-in per-provider udev
rules. No broad input-group grant is required.

## Host evidence and blockers

The VM's existing uinput ACL was reused; its process-owned live creation/destruction
test passed. UHID has a root-only node but lacks visible misc registration; the
installed module vermagic matches the running kernel. ConfigFS is mounted but the
gadget subtree and dummy UDCs are absent. Broker configuration/socket are absent
and units inactive. Noninteractive sudo reports that a password is required.
Consequently temporary UHID ACL provisioning, B/P comparisons, and reduced-
capability gadget execution could not run. No privileged host changes occurred.

Private development tooling installed: CMake 3.31.6 and Ninja 1.11.1.4 in a virtual
environment, plus extracted (not system-installed) Debian libudev development
headers 257.13-1~deb13u1. SDL 3.2.0 source commit
`535d80badefc83c5c527ec5748f2a20d6a9310fe` built in a private prefix with
SDL_UNIX_CONSOLE_BUILD=ON, tests disabled, HIDAPI enabled, and libudev headers
found. The default desktop configuration initially failed because X11/Wayland
headers were absent. The input/sensor probe subsequently compiled with strict C
warnings; it was not run against uncontrolled devices. This is build evidence,
not SDL/Steam acceptance. No Steam process was observed; a display environment
and launcher exist, but interactive operation/reference hardware remain unverified.

Private ACL/mount/module/unit inventory and build logs remain outside Git. No
permissions, group membership, module state, persistent service configuration,
consumer settings, or shared kernel configuration were changed. Audio/Bluetooth
and capture preparation remain deferred. The repository's corpus-read CI secret
is absent, confirmed by name-only inspection; no credential was created or exposed.

## Resume criteria

An administrator follows [HOST_PROVISIONING](../HOST_PROVISIONING.md) for temporary
UHID access and an explicitly reserved gadget instance. Run B/P sequentially on
baseline and rewrite, then the actual socket-activated broker with the empty-cap
candidate. Identify any failing syscall before deciding capability retention.
Gate G's source metadata/completion mismatch remains an independent blocker.
No permission increase is an acceptable substitute for the required protocol API.

Validation: workspace formatting, all-target/all-feature check, strict Clippy,
workspace tests, Gitleaks, ten Python workflow/tool regressions, private SDL probe
compilation, and systemd unit syntax verification pass. Hardware-dependent tests
remain ignored except for the explicitly rerun uinput test. No Rust dependencies
were added. Configured commit/push hooks are required for delivery.
