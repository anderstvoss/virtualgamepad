# Brief 09: acceptance manifests, runner and timing harness (cleanup step 4, #166)

Read `ground-rules.md` first.

**Lane:** Validation (new files mostly; parallel with the Core lane after 1a).
**Blocked by:** D1; brief 02 step 1a merged (the runner enumerates `FAMILIES`).
**Needed by:** the next release goal itself ("validated controller creation performance per controller") and every acceptance issue #154 to #160.

**Changed from the original step 4:** no CI runner. #177 is deferred indefinitely and the privileged job in `provider-tier-b.yml` stays `if: false`. Live runs happen on a recorded local host; CI covers only the deterministic fake-provider path. Do not edit `.github/`.

## 09a. Manifests

`acceptance/<controller-id>.toml`, one per registry family: one row per native function (from `docs/CONTROLLER_SUPPORT.md`), one column per realization. Each cell: status in Anders's categories (`not-supported`, `wip`, `source-evidence`, `physical-evidence`), test ID, and evidence as research-repo record IDs plus the research repo revision (not paths in this repo). Rows also list declared refusals and persistent resources once the registry carries them. A test checks every registry family has a manifest and every manifest cell names a realization the family declares.

## 09b. Runner

`cargo xtask acceptance --controller <id> --realization <id> [--reference <file>]` (add an `xtask` crate; none exists yet):

1. create through the public API, recording #166's timestamps: API call, preparation/open, kernel node present, named consumer discovery/open, first valid neutral report;
2. drive a scripted input sequence derived from the family's `InputTopology`;
3. run `scripts/run-sdl3-gamepad-probe.sh` and evdev/hidraw checks; for UHID and USB realizations, record which kernel driver bound (sysfs), since acceptance names the in-kernel driver as a consumer;
4. remove and record time to observed disappearance;
5. write JSON with SHA, host, kernel, CPU, toolchain, SDL version/backend/hints, devices, probe hash.

Also: repeated create/remove cycles, 1 and N concurrent sessions, warm/cold, leak checks. No universal ms threshold (#166 says budgets come after measured distributions and Anders's review).

`cargo xtask acceptance-diff <run.json> <reference.json>` wraps `scripts/compare-sdl3-observations.py`.

## 09c. Evidence

Run records use the research repo's experiment format, so they can be filed there by PR. A deterministic test runs the runner against the fake providers in ordinary CI.

## Done when

Runs for at least two existing controllers on a recorded local host (D9 names it), missing cells marked honestly, `docs/STATUS.md` updated, #166 and #176 updated.
