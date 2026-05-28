# Phase 10 Manual Gate

This guide is the reviewer checklist for Phase 10
(`gr-provider-linux-transport`). Phase 10 closes the transport
foundation tier at the architecture / state-machine level: enumeration
and protocol state machines, USB and Bluetooth packet models, and
skeletal transport-tier translators registered against the planner.
Phase 10 does **not** realize a real OS-level USB/BT gadget API or
exercise real-device traffic; the first hardware-faithful target lands
in Phase 11.

Unlike Phase 8 (uinput) and Phase 9 (UHID), Phase 10 has no `/dev/*`
prerequisites and no Tier D deferred-validation queue. Sign-off uses the
standard `chore(phase-gate): Phase 10 gate passed` wording — neither
`pending-linux-host` nor `pending-supported-host` evidence statuses are
expected here.

Host prerequisites:

- A Linux host with a working Rust toolchain (for the cross-compile
  manual check, the host must also have the `x86_64-pc-windows-msvc`
  target installed via `rustup target add x86_64-pc-windows-msvc`).
- The workspace must be clean (`git status` reports no modifications).
- No `/dev/*` device nodes, no privileged runner, no real controller —
  this gate exercises planner contracts and state-machine fixtures
  only.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 10
```

## Check 1: planner selects a transport family for `--goal hardware-faithful`

Goal: confirm the planner routes `--goal hardware-faithful` requests at
the `linux-transport-usb` or `linux-transport-bluetooth` family when
the inventory advertises transport-tier backends.

### Steps

1. Run the planner against the stub transport inventory:

```bash
cargo run -p virtual_gamepad_demo -- plan-session dualsense \
    --goal hardware-faithful \
    --inventory samples/inventories/linux-transport-stub.yaml
```

2. Confirm:
   - the command exits 0
   - the rendered plan reports `selected_backend_family:
     linux-transport-usb` (or `linux-transport-bluetooth`, depending on
     the planner's tie-break ordering)
   - the `deployment_requirements.requirements` section lists the
     Phase 11 deferral note (`phase-10 transport backend is plannable;
     live USB/Bluetooth gadget realization lands in Phase 11`),
     making the deferral self-evidencing rather than an out-of-band
     contract

3. Re-run with the alternate bus preference if the planner exposes the
   hint to confirm both families are reachable.

## Check 2: replay-trace consumes a transport enumeration fixture

Goal: confirm captured enumeration steps replay through the transport
state machine and reach the documented "ready" state.

### Steps

1. Replay the captured enumeration trace:

```bash
cargo run -p virtual_gamepad_demo -- replay-trace \
    crates/gr-provider-linux-transport/fixtures/dualsense-usb-enumeration.yaml
```

2. Confirm:
   - each captured step is consumed in order
   - the final state matches the documented `ready` state for the
     transport state machine
   - no spurious transitions are emitted

## Check 3: malformed transport-trace fixture surfaces a specific transition error

Goal: confirm the state machine rejects malformed traces with a
diagnostic naming the missing transition, rather than a generic parse
or panic.

### Steps

1. Author a custom transport-trace fixture omitting a mandatory startup
   step (drop the `configure-endpoints` step from a copy of the
   dualsense-usb fixture).

2. Run the replayer:

```bash
cargo run -p virtual_gamepad_demo -- replay-trace path/to/broken-trace.yaml
```

3. Confirm:
   - the command exits non-zero
   - the error message names the specific missing state transition
     (not a generic "fixture invalid" or parse failure)
   - the message points the reviewer at the fixture step index for the
     failed transition

## Check 4: planner stays portable; transport crate is Linux-only

Goal: confirm `gr-provider-linux-transport` does not leak into the
planner's compilation graph on non-Linux targets — the cross-build of
`gr-planner` for Windows must still succeed even though the transport
provider crate is Linux-gated.

### Steps

1. Confirm the workspace declares the transport crate as Linux-only at
   the root `Cargo.toml` (the optional `provider-linux-transport`
   feature is the gate; the dep is not pulled in by `gr-planner`).

2. Cross-compile the planner:

```bash
cargo check --target x86_64-pc-windows-msvc -p gr-planner
```

3. Confirm:
   - the command exits 0
   - no transport-related compile errors appear in the output
   - the planner's public surface compiled without the transport
     provider as a dependency

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 10 gate passed"
```

Phase 10 is **not** a deferred-validation gate. If a check above is
flagged "prerequisite-pending" at sign-off time, the Phase 10
implementation PR has not yet landed and the gate is not complete —
unlike Phase 9's `pending-linux-host` / `pending-supported-host`
vocabulary, prerequisite-pending is a temporary marker that the impl
PR clears, not a Tier D deferral that closes with provider-complete
wording.
