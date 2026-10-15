# Pre-controller buildout: plan index

Date: 2026-10-15. Base: virtualgamepad `main` at `5a0a7dd`; research repo `main` at `abffb5d`.
Tracking issue: #176. **A single executing agent should follow [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md)**, which orders these briefs and defines validation and the issue/PR feedback loop. Canonical copy: `docs/plans/pre-controller-buildout/` in virtualgamepad. This folder extends `plans/post-alpha-1-cleanup-agent-plan.md`; it does not replace it.

Each numbered file is a self-contained agent brief. To dispatch one, give an agent this sentence: *"Execute `docs/plans/pre-controller-buildout/<file>` in anderstvoss/virtualgamepad (or controller-protocol-research), following `ground-rules.md`; stop and ask on #176 at any open decision."* Briefs say what blocks them; nothing with an open blocker should be started.

## Briefs

| # | Brief | Issue | Lane | Blocked by |
| --- | --- | --- | --- | --- |
| 01 | [Switch Pro SPI fix and hygiene](01-fixes-switch-pro-spi-and-hygiene.md) | #174 | Fixes | D1 |
| 02 | [Compiled controller registry](02-controller-registry.md) (steps 1a to 1e) | #173 | Core | D1 |
| 03 | [Steam UHID consumer probe](03-steam-uhid-consumer-probe.md) (step P) | #159 | Lab | P0 none; P1 D1, D9 |
| 04 | [Contract ADRs 0019 to 0023](04-contract-adrs.md) | #176 | Design | D1 (docs); D3 for 0021; D4 for 0023 |
| 05 | [Retained outputs and typed kinds](05-retained-outputs-and-kinds.md) (B, C) | #176 | Core | 02, step 2, ADR-0020; D2 for public part |
| 06 | [Live topology](06-live-topology.md) (A) | #163 | Runtime | 02/1a; ADR-0019 for 06b |
| 07 | [Persistent resources seam](07-persistent-resources.md) | #162 | Runtime | 02/1a |
| 08 | [Presentation epochs and receivers](08-epochs-and-receivers.md) (E, F) | #165 | Runtime | 02, ADR-0021 (D3), ADR-0022, 03 result |
| 09 | [Acceptance manifests, runner, timing](09-acceptance-runner.md) (step 4) | #166 | Validation | 02/1a; D9 for live runs |
| 10 | [Facade, crate tidy, demo](10-facade-tidy-demo.md) (steps 2, 3, 5) | #176 | Core | 02; D5 for dummy_hcd |
| 11 | [Research ledgers](11-research-ledgers.md) (Wii, Steam, Xbox Series) | #167 | Research | D1 (evidence start) |
| 12 | [Windows CI downloads](12-windows-ci-downloads.md) | #150 | Fixes | D8 |

## Order

Lanes can run in parallel; within a lane, run in the order shown. "Core" is strictly serial because its steps touch the same crates.

```text
Wave 0 (now)     01 Fixes ─┐   03-P0 (source)   04 ADRs (docs)   11 Research
                           ▼
Wave 1           02 Registry 1a ──► 1b ──► 1c ──► 1d ──► 1e
                       │
Wave 2 (after 1a)      ├─► 09 Runner        ├─► 06 Topology   ├─► 07 Resources
                       │
Wave 3 (after 1e)      └─► 10 step 2 ──► 05 Outputs ──► 10 step 3 ──► 10 step 5
                                         08 Epochs/receivers (after 06/07, ADRs, 03)
```

12 slots in whenever D8 is given. 03-P1 runs whenever a host is ready.

## What each new controller needs first

| Controller | Must land before package code merges | Needed later in that controller's work, not to start it |
| --- | --- | --- |
| **Xbox Series** (#160), uinput first | 02 | 05 (retained LEDs/rumble), 09 for timing, 11d evidence |
| **Wii Remote** (#158) | 02 (with its Bluetooth-bus proof), D6 production gate, D7 first realization | 06 (extensions), 07 (EEPROM), 05c point-set kind for IR, 11b evidence, 09 |
| **Steam Controller, wired** (#159) | 02, 03 result = UHID accepted | 05c capacitive/pressure kinds, 11c, 09 |
| **Steam Controller, Puck and lizard companions** | 08, plus #168 USB function list if 03 says USB/IP | 03 kernel pin |

## Decisions for Anders

| ID | Decision | Recommendation | Unblocks |
| --- | --- | --- | --- |
| D1 | Lift the hold on code changes ("wait before beginning changes", 2026-10-14), per lane or all at once | Start Fixes, Core (02), Design (04) and Research (11) now | 01, 02, 04, 11, then everything |
| D2 | Grow the API snapshot for `host_outputs()` and the typed output/input kinds (`OutputSurface`, `InputTopology` are in the snapshot today) | Yes | 05b, 05c |
| D3 | Accept the logical-controller vs presentation split (epochs) for mode switching | Yes | ADR-0021, 08 |
| D4 | Displays and bulk subsystems: ADR now, or wait for a controller that needs them | ADR now, no code | ADR-0023 |
| D5 | dummy_hcd: keep in tree, or park on `archive/dummy-hcd` | Park (it is unavailable and adds a closed enum to every refactor) | 10 step 3, shrinks 02/1d |
| D6 | Wii production gate: the one-line OK on #158 that four docs still require | Give it once 02 is merged | Wii package code |
| D7 | Wii first realizations now that Bluetooth-bus UHID is available | uinput (playable, reduced) **and** `linux.uhid.bluetooth` (the only path that binds `hid-wiimote`) together; drop USB-bus UHID to a fixture-only experiment | Wii package plan; updates the #158 proposal, which predates the 04:08 decision |
| D8 | Authorize the `.github/` change for #150 (cache plus bounded retry on Windows only) | Yes | 12 |
| D9 | Which host runs live lab work (Steam probe P1, acceptance runs): your machine through a session on your device, or another prepared x86-64 host | Your machine, if it has `/dev/uhid` and a kernel with `hid-steam` IBEX support | 03-P1, 09 live runs |

Not blocking now: Wii vs Steam order (the table above suggests Xbox Series, then Wii, then wired Steam once 03 reports); Steam Machine receiver `1305` scope (default listed, out of scope); reference hardware for each controller.

## Corrections to the earlier plan, applied here

- The Python test exclusion is `test_protocol_research.py` (it was `test_protocol_corpus.py` before #178).
- Step 4 no longer re-enables a CI runner: #177 is deferred indefinitely.
- Step 1a now stubs the re-assessment's registry fields and proves the Bluetooth-bus UHID path end to end.
- `.gitignore` for `__pycache__` moves from step 3 to brief 01b.
- The #158 Wii proposal treats Bluetooth-bus UHID as deferred; that was superseded on 2026-10-15 (D7 settles what replaces it).
