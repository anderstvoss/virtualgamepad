# Phase 1 Manual Gate

This guide expands the manual portion of Phase 1 into concrete,
repeatable reviewer steps.

Use it only after the automated Phase 1 checks are green.

Related docs:

- [Rust Implementation Plan](../RUST_IMPLEMENTATION_PLAN.md)
- [Testing Tooling Specification](../TESTING_TOOLING_SPEC.md)

## Before you start

1. Make sure you are on the Phase 1 work branch.
2. Make sure the workspace builds cleanly.
3. Run the automated gate first:

```bash
cargo run -p virtual_gamepad_demo -- phase-gate 1
```

4. If the automated checks fail, stop here and fix them before
   continuing.

## Check 1: `show-types` naming and readability

Goal: confirm the public-facing type names are readable, stable, and
match the spec.

### Steps

1. Run:

```bash
cargo run -p virtual_gamepad_demo -- show-types
```

2. Read the `fidelity-tiers` section.
3. Confirm the output contains exactly:
   - `compatibility`
   - `identity-aware`
   - `hardware-faithful`
4. Read the `backend-levels` section.
5. Confirm the names are concise public terms, not Rust enum spellings
   or debug output.
6. Read the `backend-families` section.
7. Confirm each family name is distinct and unambiguous.
8. Read the `capability-categories` section.
9. Confirm the category names look appropriate for docs, snapshots, and
   fixtures.
10. Run the same command a second time.
11. Confirm the ordering is unchanged between runs.

### What to record

- Whether any name looks awkward, misleading, or too implementation-specific
- Whether the ordering stayed stable

## Check 2: built-in DualSense fixture acceptance and authoring ergonomics

Goal: confirm the checked-in sample fixture validates cleanly and looks
 easy for a human to edit.

### Steps

1. Run:

```bash
cargo run -p gr-cli -- validate-fixture crates/gr-core/fixtures/payload-dualsense-neutral.yaml
```

2. Confirm the command succeeds.
3. Confirm the output reports:
   - `kind: input-frame`
   - `profile_id: dualsense`
   - `payload_type: dualsense`
4. Open `crates/gr-core/fixtures/payload-dualsense-neutral.yaml`.
5. Read through the YAML structure from top to bottom.
6. Confirm a non-Rust contributor could reasonably copy this file and
   edit it without understanding internal implementation details.
7. Confirm the neutral values are obvious at a glance:
   - buttons use `released`
   - sticks are centered at `0`
   - triggers are `0`

### What to record

- Any field names that feel too Rust-centric or confusing
- Any neutral values that are not obvious to a human reviewer

## Check 3: user-authored Xbox 360 fixture workflow

Goal: confirm a reviewer can create a valid custom fixture by editing
YAML only, with no parser or Rust-code changes.

### Steps

1. Open `crates/gr-core/fixtures/payload-dualsense-neutral.yaml`.
2. Open `tests/fixtures/xbox360-neutral.yaml`.
3. Compare the two files.
4. Confirm the Xbox 360 fixture is clearly derived from the checked-in
   template rather than requiring a different authoring workflow.
5. Run:

```bash
cargo run -p gr-cli -- validate-fixture tests/fixtures/xbox360-neutral.yaml
```

6. Confirm the command succeeds.
7. Confirm the output reports:
   - `kind: input-frame`
   - `profile_id: xbox360`
   - `payload_type: xbox360`
8. Run the fixture-loading test that uses that exact file:

```bash
cargo test -p gr-core workspace_xbox360_fixture_loads_as_profile_input_frame
```

9. Confirm the test passes.
10. Review the Xbox 360 fixture contents and confirm the neutral shape
    is still easy to infer by eye.

### What to record

- Whether the copy/edit workflow felt straightforward
- Any fields that were hard to translate from the DualSense sample to
  the Xbox 360 sample

## Check 4: snapshot readability

Goal: confirm the snapshot files are useful review artifacts instead of
noisy serialized dumps.

### Steps

1. Run:

```bash
cargo insta test --check
```

2. Confirm the command succeeds without creating or updating snapshot
   files.
3. Open the snapshot directory: `crates/gr-core/src/snapshots/`
4. Review each snapshot file.
5. Confirm serialized values use canonical public names.
6. Confirm each snapshot is concise enough to read comfortably in code
   review.
7. Confirm payload snapshots make variant identity obvious.
8. Confirm there is no debug-only noise such as Rust type wrappers or
   unstable formatting artifacts.

### What to record

- Any snapshot that feels too noisy to review
- Any snapshot whose naming or structure would make regressions hard to
  interpret later

## Check 5: test-suite coverage confidence

Goal: confirm the automated tests cover the same behaviors you just
manually inspected.

### Steps

1. Run:

```bash
cargo test -p gr-core
```

2. Watch the test names and results.
3. Confirm there are tests covering:
   - fidelity-tier parse/display behavior
   - serde round-trip behavior
   - snapshot behavior
   - fixture loading
   - property-test execution
4. Confirm the fixture-loading tests cover both:
   - the checked-in DualSense fixture
   - the workspace-level Xbox 360 fixture
5. If a manual concern from Checks 1–4 does not seem represented by a
   test, note that gap before signing off.

### What to record

- Any manual observation that does not appear to be backed by a test
- Any test area that feels too narrow for the behavior it is meant to protect

## Sign-off

After all five checks pass and any concerns are resolved:

1. Run the full required validation set if you have not already:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
gitleaks detect
```

2. Sign off the phase gate:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 1 gate passed"
```
