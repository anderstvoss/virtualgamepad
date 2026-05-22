# Phase 3 Manual Gate

This guide expands the manual portion of Phase 3 into concrete,
repeatable reviewer steps.

Use it only after the automated Phase 3 checks are green.

Related docs:

- [Rust Implementation Plan](../RUST_IMPLEMENTATION_PLAN.md)
- [Configuration Specification](../../specs/CONFIGURATION_SPEC.md)

## Before you start

1. Make sure you are on the Phase 3 work branch.
2. Run the automated gate first:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 3
```

3. If the automated checks fail, stop here and fix them before
   continuing.

## Check 1: valid config acceptance

Goal: confirm the happy-path sample config validates cleanly and the
output is structured enough to review.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- validate-config samples/configs/dualsense-identity.yaml
```

2. Confirm the output shows:
   - the parsed `config`
   - the compiled session-option summary
   - no validation errors
3. Confirm the sample still names:
   - profile `dualsense`
   - fidelity `identity-aware`
   - provider preference `linux-uhid`

### What to record

- Any confusing field names
- Any missing summary information you expected to see during review

## Check 2: invalid config rejection

Goal: confirm a reviewer-facing invalid config fails with a targeted
message instead of a generic crash.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- validate-config samples/configs/broken-mode.yaml
```

2. Confirm the command exits non-zero.
3. Confirm the output points to
   `outputHandling.callbackNamespace` and explicitly explains that the
   config is invalid because `outputHandling.mode` is `callback` while
   the required callback namespace is missing.

### What to record

- Whether the error feels specific enough to fix quickly

## Check 3: provider preference warnings vs errors

Goal: confirm provider preference remains a hint unless strict rejection
is requested.

### Steps

1. Copy `samples/configs/dualsense-identity.yaml` to a temporary file.
2. Change:
   - `profileId` to `xbox360`
   - `fidelityTier` to `compatibility`
   - `backendPreference` to `evdev`
   - `providerPreference` to `mystery-provider`
   - `validation.rejectUnsupportedProviderPreference` to `false`
3. Run `cargo run -p virtual_gamepad_demo -- validate-config <temp-file>`.
4. Confirm the command succeeds with a warning about
   `session.providerPreference`.
5. Flip `validation.rejectUnsupportedProviderPreference` to `true`.
6. Re-run the command and confirm it now fails.

### What to record

- Whether the warning/error difference is obvious from the output

## Check 4: unknown-field policy

Goal: confirm additive config drift warns by default and can be made
strict when requested.

Important distinction: this check is controlled by
`validation.rejectUnknownConfigFields`, not by
`validation.rejectUnsupportedProviderPreference`. Provider-preference
strictness belongs to Check 3.

### Steps

1. Copy `samples/configs/dualsense-identity.yaml` to a temporary file.
2. Add an unknown key inside `session`, for example
   `unexpectedHint: keep-reviewing`.
3. Run the validator and confirm the config succeeds with a warning for
   `session.unexpectedHint`.
4. Add `validation.rejectUnknownConfigFields: true`.
5. Re-run and confirm the same unknown key now causes a failure.
6. Add an unknown top-level section such as:

```yaml
mystery:
  enabled: true
```

7. Confirm unknown top-level sections are rejected regardless of the
   strictness toggle.

### What to record

- Any unknown-field message that feels ambiguous

## Check 5: sample/doc consistency

Goal: confirm the sample configs and gate text still match the Phase 3
implementation surface.

### Steps

1. Review `samples/configs/`.
2. Confirm `dualsense-identity.yaml` is the happy-path review sample.
3. Confirm `broken-mode.yaml` is the intentionally failing sample used
   by the manual gate.
4. Confirm the commands shown by `vgpd-demo phase-gate 3` match the
   current implementation plan wording.

### What to record

- Any wording drift between plan, guide, and command output
