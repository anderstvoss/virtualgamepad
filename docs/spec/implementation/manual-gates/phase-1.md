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

Goal: confirm the checked-in sample fixture validates cleanly, the
public-facing names are consistent, the digital/analog split is obvious,
and the YAML looks easy for a human to edit.

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
6. Confirm the **inner `profile:` tag** inside the payload (alongside
   `fields:`) is the literal word `dualsense` — no hyphen, no
   `dual-sense`. The same word must appear in the outer `profile_id:`
   field and in the demo's reported `payload_type` from step 3. A
   fixture author should never have to remember two different
   spellings of the same concept.
7. Confirm the `dpad:` block is a single nested map with four keys
   (`up`, `down`, `left`, `right`) rather than four flat `dpad_*`
   fields at the payload level.
8. Confirm a non-Rust contributor could reasonably copy this file and
   edit it without understanding internal implementation details.
9. Confirm digital values are visually distinct from analog ones:
   - `buttons.face.*`, `buttons.shoulders.*`, `buttons.stick_clicks.*`,
     `buttons.system.*`, `dpad.*`, and `touchpad.contact_*.active` use
     `true` / `false`
   - `sticks.*`, `triggers.*`, and `touchpad.contact_*.(x|y)` use numbers
10. Confirm the DualSense touchpad is present as two named contacts:
   - `touchpad.contact_1`
   - `touchpad.contact_2`
11. Note for review that the DualSense touch surface is modeled as
    absolute multitouch X/Y contacts here; the current Linux evidence
    uses a 1920x1080 coordinate space, but those ranges belong to the
    profile contract layer rather than the payload file itself.

### What to record

- Any field names that feel too Rust-centric or confusing
- Any neutral values that are not obvious to a human reviewer
- Any spelling drift between the outer `profile_id` and the inner
  `profile` tag, in either direction

## Check 3: user-authored Xbox 360 fixture workflow

Goal: confirm a reviewer can create a valid custom fixture by editing
YAML only, with no parser or Rust-code changes.

### Steps

1. Open `crates/gr-core/fixtures/payload-dualsense-neutral.yaml`.
2. Open `tests/fixtures/xbox360-neutral.yaml`.
3. Compare the two files.
4. Confirm the Xbox 360 fixture is clearly derived from the checked-in
   template rather than requiring a different authoring workflow.
5. Confirm the Xbox 360 fixture also uses the nested `dpad:` map
   (same `up`/`down`/`left`/`right` keys as the DualSense sample) —
   the shared `dpad` shape should look identical across profiles.
6. Confirm the Xbox 360 fixture uses:
   - booleans for digital inputs
   - numeric values for `sticks.*` and `triggers.lt` / `triggers.rt`
   - `lb` / `rb` / `ls` / `rs` naming rather than the older long-form names
7. Run:

```bash
cargo run -p gr-cli -- validate-fixture tests/fixtures/xbox360-neutral.yaml
```

8. Confirm the command succeeds.
9. Confirm the output reports:
   - `kind: input-frame`
   - `profile_id: xbox360`
   - `payload_type: xbox360`
10. Run the fixture-loading test that uses that exact file:

```bash
cargo test -p gr-core workspace_xbox360_fixture_loads_as_profile_input_frame
```

11. Confirm the test passes.
12. Review the Xbox 360 fixture contents and confirm the neutral shape
    is still easy to infer by eye.

### What to record

- Whether the copy/edit workflow felt straightforward
- Any fields that were hard to translate from the DualSense sample to
  the Xbox 360 sample
- Whether the `dpad:` block looked obviously analogous across the two
  fixtures

## Check 4: sparse delta fixture authoring

Goal: confirm that a `ProfileInputDelta` carries only the fields a
fixture author actually sets, and that the authoring workflow stays
usable from YAML alone.

### Steps

1. Open `tests/fixtures/dualsense-delta-sparse.yaml`.
2. Read it top to bottom and confirm only the fields that should be
   "changed" appear:
   - `dpad.left: true`
   - `triggers.l2: 66`
   - `touchpad.contact_1.active/x/y`
   - nothing else under `fields:`
3. Confirm there is no `cross:`, `sticks.right_x:`, `triggers.r2:`, or any other
   field the author did not deliberately set — absent fields must
   stay absent in the YAML, not appear with default values.
4. Run:

```bash
cargo run -p gr-cli -- validate-fixture tests/fixtures/dualsense-delta-sparse.yaml
```

5. Confirm the command succeeds.
6. Run the integration test that loads this exact fixture:

```bash
cargo test -p gr-core workspace_dualsense_sparse_delta_decodes_only_set_fields
```

7. Confirm the test passes. It asserts only `dpad.left`,
   `triggers.l2`, and the first touch contact decode as `Some` on the
   resulting delta; everything else is `None`.
8. As an authoring exercise, copy the fixture to a new file (for
   example `tests/fixtures/dualsense-delta-experiment.yaml`), change
   only the value of `dpad.left` from `true` to `false`, save,
   and run `cargo run -p gr-cli -- validate-fixture` against the new
   path. Confirm it accepts and that the workflow remains pure-YAML
   with no parser changes.
9. Delete the experimental fixture so it does not end up tracked.

### What to record

- Whether the YAML reads as a sparse delta or whether it looks like a
  partial full-snapshot in disguise
- Any field that surprises you by appearing in the output despite not
  being declared in the fixture
- Whether step 8's authoring workflow stayed self-contained (no
  changes to Rust code or to `gr-testkit`)

## Check 5: snapshot readability and per-variant coverage

Goal: confirm the snapshot files are useful review artifacts instead
of noisy serialized dumps, and that every Phase 1 enum has full
per-variant snapshot coverage.

### Steps

1. Run:

```bash
cargo insta test --check
```

2. Confirm the command succeeds without creating or updating snapshot
   files.
3. Open the snapshot directory: `crates/gr-core/src/snapshots/`
4. Count snapshot files per enum prefix and confirm each count equals
   that enum's `ALL.len()`:
   - `ls crates/gr-core/src/snapshots/ | grep -c fidelity-tier-` → `3`
   - `ls crates/gr-core/src/snapshots/ | grep -c backend-level-` → `3`
   - `ls crates/gr-core/src/snapshots/ | grep -c backend-family-` → `6`
   - `ls crates/gr-core/src/snapshots/ | grep -c capability-category-` → `9`

   Plus the two DualSense neutral payload/frame snapshots, for a total
   of 23.
5. Review a sample of the per-variant snapshots and confirm serialized
   values use canonical public names (kebab-case where applicable) and
   are easy to tell apart at a glance.
6. Open `gr_core__tests__dualsense-neutral-payload.snap`. Confirm:
   - the on-wire payload tag reads `profile: dualsense` (no hyphen);
   - the `dpad` block reads as a single nested map with four boolean
     entries — not four flat `dpad_*` fields;
   - the `touchpad` block exists with `contact_1` and `contact_2`;
   - digital and analog sections are visually easy to distinguish.
7. Confirm each snapshot is concise enough to read comfortably in code
   review.
8. Confirm there is no debug-only noise such as Rust type wrappers or
   unstable formatting artifacts.

### What to record

- Any enum whose per-variant snapshot count does not match its
  `ALL.len()`
- Any snapshot that feels too noisy to review
- Any snapshot whose naming or structure would make regressions hard
  to interpret later

## Check 6: test-suite coverage confidence

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
   - serde round-trip behavior for **both** frames and deltas
   - snapshot behavior with per-variant coverage
   - fixture loading for **both** frame and delta fixtures
   - sparse-delta absence-of-fields (e.g.
     `sparse_dualsense_delta_only_carries_set_fields`)
   - DualSense touchpad round-trip behavior
   - profile-id-vs-payload mismatch behavior for both frame and delta
     (e.g. `payload_variant_must_match_profile_id` and
     `delta_payload_variant_must_match_profile_id`)
   - property-test execution including `dpad_yaml_round_trip` and
     `dpad_json_round_trip`
4. Confirm the integration tests cover all three workspace fixtures:
   - `dualsense_fixture_loads_as_profile_input_frame`
   - `workspace_xbox360_fixture_loads_as_profile_input_frame`
   - `workspace_dualsense_sparse_delta_decodes_only_set_fields`
5. If a manual concern from Checks 1–5 does not seem represented by a
   test, note that gap before signing off.

### What to record

- Any manual observation that does not appear to be backed by a test
- Any test area that feels too narrow for the behavior it is meant to
  protect

## Sign-off

After all six checks pass and any concerns are resolved:

1. Run the full required validation set if you have not already:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
gitleaks detect
```

2. Sign off the phase gate:

```bash
git commit --allow-empty -m "chore(phase-gate): Phase 1 gate passed"
```
