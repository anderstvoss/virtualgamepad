# Phase 5 Manual Gate

This guide is the step-by-step reviewer checklist for Phase 5
(`gr-planner`). Run it after the automated gate is green.

Start with:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 5
```

## Check 1: Identity-aware plan on UHID

Goal: confirm the planner picks the expected Linux UHID backend without
degradation when the requested tier is realizable.

Steps:

```bash
cargo run -p virtual_gamepad_demo -- \
  plan-session dualsense \
  --goal identity-aware \
  --inventory samples/inventories/linux-uhid-only.yaml
```

Confirm:

- output is structured YAML
- `outcome: plan`
- `selected_backend_family: linux-uhid`
- `selected_level: hid`
- `degradation.degraded: false`

What to record:

- whether the selected backend and tier match the expected outcome

## Check 2: Hardware-faithful request degrades to identity-aware

Goal: confirm the planner degrades honestly when transport-tier
realization is unavailable.

Steps:

```bash
cargo run -p virtual_gamepad_demo -- \
  plan-session dualsense \
  --goal hardware-faithful \
  --inventory samples/inventories/linux-uhid-only.yaml
```

Confirm:

- `outcome: plan`
- `requested_fidelity_tier: identity-aware`
- `degradation.degraded: true`
- `degradation.reasons` contains `transport-not-realizable`

What to record:

- whether the degradation reason is explicit and readable

## Check 3: Empty inventory rejects cleanly

Goal: confirm planner rejection is structured and actionable.

Steps:

```bash
cargo run -p virtual_gamepad_demo -- \
  plan-session dualsense \
  --goal hardware-faithful \
  --inventory samples/inventories/empty.yaml
```

Confirm:

- `outcome: rejection`
- `reasons` includes `no-backend-supports-profile`
- output is structured YAML, not an ad hoc error string

What to record:

- whether the rejection would be understandable to a user debugging
  their host inventory

## Check 4: Compatibility plan on uinput

Goal: confirm compatibility-tier selection remains available on the
lowest-level Linux backend.

Steps:

```bash
cargo run -p virtual_gamepad_demo -- \
  plan-session xbox360 \
  --goal compatibility \
  --inventory samples/inventories/linux-uinput-only.yaml
```

Confirm:

- `outcome: plan`
- `selected_backend_family: linux-uinput`
- `selected_level: evdev`
- no degradation is reported

What to record:

- whether the selected plan matches the expected fidelity and backend

## Check 5: Custom plan-snapshot fixture

Goal: confirm the planner output can be captured as a stable
`plan-snapshot` fixture for unusual edge cases.

Steps:

1. Run a custom case, for example:

```bash
cargo run -p virtual_gamepad_demo -- \
  plan-session steam-controller \
  --goal identity-aware \
  --inventory samples/inventories/linux-uinput-only.yaml
```

2. Save the output as a `kind: plan-snapshot` fixture under
   `crates/gr-testkit/fixtures/community/`.
3. Run:

```bash
cargo test -p gr-planner --all-features
```

Confirm:

- the fixture decodes successfully if referenced by tests
- the YAML shape is stable and reviewer-readable

What to record:

- the edge case chosen
- whether the resulting snapshot is readable enough to review in a PR

## Check 6: Snapshot readability

Goal: confirm the stored planner snapshots read like intentional user
facing output rather than debug dumps.

Steps:

1. Review `crates/gr-planner/src/snapshots/`.
2. Review any new `plan-snapshot` fixtures added in this phase.

Confirm:

- degradation reasons are typed and human-readable
- warnings explain why hints were ignored
- enabled and unsupported capability lists are easy to scan

## Sign-off

When all checks pass:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 5 gate passed"
```
