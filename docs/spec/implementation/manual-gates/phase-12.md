# Phase 12 Manual Gate

This guide is the reviewer checklist for Phase 12 (`gr-provider-windows-hid`
and `gr-provider-macos-hid`, Windows + macOS provider foundations). Phase 12
proves the Linux-first runtime admits Windows and macOS providers without
architectural rewrites: both ship as inventory + diagnostics +
deployment-requirement reporting only — **no device realization**.

Like Phase 10 (and unlike Phases 8/9/11), Phase 12 has no `/dev/*`
prerequisites and no Tier-D / `pending-supported-host` deferred-validation
queue. It is cross-build + planner-contract work, fully verifiable on a single
Linux host. This branch still records closure with the provider-complete commit
wording so Phases 9–12 share the same post-implementation review policy on the
current host: `chore(phase-gate): Phase 12 provider-complete closure recorded`.
A check flagged `prerequisite-pending` at sign-off means the Phase 12
implementation PR has not yet landed — that marker is temporary and the impl PR
clears it; it is **not** a Tier-D deferral.

Host prerequisites:

- A Linux host with a working Rust toolchain and both cross-check targets
  installed (`rustup target add x86_64-pc-windows-msvc x86_64-apple-darwin`).
  Both gate cross-builds are `cargo check` only (no linking), so a Linux host
  is sufficient — no Windows or macOS machine is required.
- The workspace must be clean (`git status` reports no modifications).
- No device nodes, no privileged runner, no real controller.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 12
```

## Check 1: planner selects the Windows provider with deployment requirements

Goal: confirm a Windows-host request selects the `windows-hid` family and the
plan surfaces its deployment requirements.

### Steps

1. Plan against the Windows stub inventory:

```bash
cargo run -p virtual_gamepad_demo -- plan-session dualsense \
    --goal identity-aware \
    --host-platform windows \
    --inventory samples/inventories/windows-hid-stub.yaml
```

2. Confirm:
   - the command exits 0
   - `selected_backend_family: windows-hid`
   - `deployment_requirements.requirements` lists the planning-only note and the
     signed virtual-HID bus driver requirement

## Check 2: planner selects the macOS provider with entitlement prerequisites

Goal: confirm a macOS-host request selects the `macos-hid` family and surfaces
the entitlement / system-extension prerequisites.

### Steps

1. Plan against the macOS stub inventory:

```bash
cargo run -p virtual_gamepad_demo -- plan-session dualsense \
    --goal identity-aware \
    --host-platform macos \
    --inventory samples/inventories/macos-hid-stub.yaml
```

2. Confirm:
   - the command exits 0
   - `selected_backend_family: macos-hid`
   - `deployment_requirements.requirements` lists the notarized DriverKit system
     extension + matching app entitlement requirement

## Check 3: macOS hardware-faithful request degrades or rejects with reasons

Goal: confirm an over-tier request (hardware-faithful on a hid-only macOS
inventory) does not silently succeed — it degrades or rejects with explicit
reasoning.

### Steps

1. Plan a hardware-faithful goal against the macOS stub inventory:

```bash
cargo run -p virtual_gamepad_demo -- plan-session dualsense \
    --goal hardware-faithful \
    --host-platform macos \
    --inventory samples/inventories/macos-hid-stub.yaml
```

2. Confirm:
   - the plan is `degraded: true` (or a rejection) with a `reason` naming that
     no transport backend is available and the inventory exposes only hid-tier
     backends — not a silent hardware-faithful claim

## Check 4: no platform-specific dependency leaks into the core crates

Goal: confirm Windows/macOS provider details stay inside their crates and never
leak into the platform-neutral core.

### Steps

1. Run the core-purity check:

```bash
rg 'extern crate (windows|winapi|core_foundation|objc)' \
    crates/{gr-core,gr-profiles,gr-config,gr-session-options,gr-runtime-model,gr-backend-api,gr-planner,gr-translators,gr-session,gr-host-bridge}
```

2. Confirm the search returns nothing (exit non-zero / no matches).

## Check 5: each provider crate documents its realization roadmap

Goal: confirm `gr-provider-windows-hid` and `gr-provider-macos-hid` each carry a
README describing what a full realization will require.

### Steps

1. Confirm each crate has a `README.md` documenting the realization roadmap
   (virtual-HID bus driver for Windows; DriverKit system extension + entitlement
   for macOS).

2. If the READMEs are not yet present, mark this check `prerequisite-pending` —
   the Phase 12 implementation PR adds them; it is not a deferred-validation
   item.

## Sign-off

When all automated checks and all manual checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 12 provider-complete closure recorded"
```

Phase 12 is **not** a deferred-validation gate. Any `prerequisite-pending`
markers at sign-off time mean the Phase 12 implementation PR has not yet landed
and the gate is not complete — there is no `pending-supported-host` queue for
this phase.
