# Brief 15: Xbox Series package (#160), checkpoint C9

Read `ground-rules.md` first, then #322's owner decisions, #160's research
and scope decisions, and #168's transport feasibility notes. The alpha API
policy on #322 supersedes the snapshot approval rule in the ground rules.

**Lane:** Controller. **Checkpoint:** C9, branch `buildout/c9-xbox-series`.
**Blocked by:** C7 API convergence merged (#329), this brief merged to
`main`, and merged research records for each implemented behavior. Live
acceptance in 15d needs an eligible D9 host.

## Goal

Add Xbox Series as the seventh curated family, with
`create_xbox_series(CreationOptions, XboxSeriesOptions)` in the curated
crate and the ordinary root. Use the post-C7 Wii and Steam registry shape
for subsystems, refused operations and creation toggles.

- `linux.uinput` first. Present the playable evdev surface the Linux
  `xpad` driver exposes for Xbox Series, including Guide and Share,
  signed sticks, 10-bit triggers, D-pad and conventional two-motor
  force feedback. Independent impulse triggers and extra motors are
  explicit deviations because `FF_RUMBLE` has only two magnitudes.
  Keep the native output type at two motors.
- `linux.uhid.bluetooth` second, only with a recorded descriptor and
  a supported consumer path (`hid-microsoft`, SDL or Steam Input).
  The current ledger has SDL decoder and rumble evidence but no
  qualifying report map and binding evidence. Status is **No evidence**.
  Actual Bluetooth radio and pairing remain with #164.
- Wired Xbox Series uses GIP. USB HID realizations are inapplicable.
  GIP and USB/IP feasibility remain with #168.

## Evidence rule for autonomous work

Use the Xbox Series L0 ledger in the
`controller-protocol-research` submodule
(`docs/ledgers/xbox-series-l0.md`, from #167). Cite the research record
next to each table, byte and constant. If a record is missing, merge the
source and claim in a research PR first and adopt that revision before
using it. Leave unresolved behavior unimplemented and record **No evidence**.

The pinned `xpad` and SDL sources describe consumer behavior. Cite the
kernel commit, file and lines and restate the behavior in original
wording. The published evdev artifact supplies observations after the
driver processes input. Keep that separate from device-side evidence
such as firmware identity, raw reports and motor actuation.

## Sub-steps (one PR each into `buildout/c9-xbox-series`)

**15a. Family, native state and target contract.** Register `xbox-series`
with the evidenced realization, subsystems, refused operations and
creation toggles. Add typed buttons, signed sticks, validated 10-bit
triggers and D-pad. Fill the research gaps for identity, complete evdev
capabilities, axis parameters and force feedback first.

**Done when:** registry tests show the exact targets and restrictions, and
state tests cover neutral, boundaries, rejected edits and independent
Guide/Share. Existing family behavior stays covered.

**15b. Uinput realization and host output.** Open the evidence-backed
gamepad surface and map native input and conventional force feedback.
Document impulse-trigger and extra-motor refusal on the selected surface.
If the Bluetooth evidence gate clears, add the evidenced HID codec in
this step after uinput. Otherwise carry the Bluetooth gap into 15d.

**Done when:** a deterministic golden capability fixture from the ledger
matches the fake-provider device, including identity, names, key and
axis codes, ranges, flat and supported FF capabilities. Document and
test each deviation from the golden fixture, including the provider’s
zero axis fuzz because the evdev contract exposes range and flat. Tests
exercise each input and the two-motor reverse path, repeated polling, rollback,
individual removal, shutdown and idempotent cleanup. If HID is admitted,
compare the descriptor with the research fixture and assert the exact
success or error acknowledgment for every declared host request type
within its polling cycle.

**15c. Converged constructors and application wiring.** Expose both
constructors with `XboxSeriesOptions`, using the C7 session, association,
typed service and retained-output conventions. Wire registry-driven
selection to the ordinary root wherever supported. Update affected API
snapshots and list each delta in the PR and `docs/ALPHA_API_MIGRATION.md`.

**Done when:** curated and ordinary-root callers compile against the
same two-argument shape, the snapshot check passes, and tests show
unsupported target and audio requests are rejected before opening a
provider. Test registry-driven selection beyond the visible viewport
where it applies.

**15d. Acceptance, timing and host audit.** Add
`acceptance/xbox-series.toml` with native function and realization
evidence. Run the #166 creation-to-ready and removal-to-node-gone
measurements for repeated single and concurrent controllers on an
eligible host. Amend the #258 Alpha 2 audit with Xbox Series rows,
following the C8 audit's existing structure.

**Done when:** the manifest validates and every cell cites a test or
records an explicit status and reason. Record the exact candidate,
host/kernel, consumer version, commands and results for each live run.
Compare the virtual evdev capabilities with the xpad reference and run
the reviewed SDL probe for controls and conventional FF where available.
Track Steam Input separately as manual unless run. Record timing
distributions and a budget where the measurements support one. If the
host is ineligible, record the missing permission or facility and leave
the affected cells untested. Keep #258 open for Anders's sign-off.

## Checkpoint C9 PR

Follow IMPLEMENTATION_PLAN.md 1.6. Include the before/after, 15a through
15d PRs and objective evidence, API deltas, judgement calls, manifest,
timing, and the #258 amendment. **Needs your attention** lists every
**No evidence** cell, each uinput deviation, and pending live or manual
acceptance. Link the checkpoint from #160 and update the agent-status
comment on #322. Anders merges the checkpoint to `main`.
