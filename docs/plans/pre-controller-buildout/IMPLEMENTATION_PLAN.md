# Pre-controller buildout: implementation plan for an executing agent

Date: 2026-10-15 (revised 2026-10-15 for autonomous execution). Repositories: `anderstvoss/virtualgamepad` and `anderstvoss/controller-protocol-research`.
Tracking issue: **#176**. Detailed briefs: `docs/plans/pre-controller-buildout/01-…12-*.md` next to this file. When this plan and a brief disagree, this plan wins on order and process, and the brief wins on technical content.

You are the implementing agent, and you drive this work. You build each step, prove it meets the step's objective, open a PR, review it yourself, and move on. Project Claude threads may check in and review on GitHub when they are available, but **they are not a gate**. They can be unavailable for days. Plan as if no reviewer will ever answer, and treat a review that does arrive as a bonus to act on.

Anders is the only gate, and only for the things listed in 1.6 and 1.8. GitHub is your only inbox.

---

## Part 1. How you work

### 1.1 Rules that always apply

Read `ground-rules.md` and the repository's `AGENTS.md` before your first step. In short:

- The ordinary-root API snapshot (110 items) does not change unless the step names an approved decision allowing it.
- No `.github/`, `SECURITY.md`, license or release changes without an explicit owner comment approving that exact change.
- No broker wire-format or protocol-version change.
- No runtime profiles, plugins, descriptor injection, generic mutable gamepad state, provider-owned protocol semantics or silent realization fallback.
- Every bug fix gets a regression test at the lowest seam. Never weaken, skip, quarantine or delete a test to get green.
- One step (or sub-step) = one PR. Do not stack unrelated work into a PR.

### 1.2 Start of every session (intake)

Do this before any new work, every time you start or resume:

1. Read #176: the body (checklist and "Open decisions") and every comment newer than your last status comment. Note any new "Owner decision".
2. List your open PRs in both repositories. For each, read reviews, comments and CI on the current head.
3. Read new comments on the issue for your current step, including any **"Prep for step N, for the local agent"** comment or docs PR from a cloud lane (1.9).
4. Handle in this order: a broken `main` you caused (revert first, 1.7), merge conflicts, red CI, blocking review items, questions, then new work.
5. Pick the next step from Part 2 whose gates are met. If your open PRs are only waiting on a merge, keep going (1.7); never sit idle.

### 1.3 Reading feedback correctly

All project Claude threads post under Anders's GitHub account, so the author name does not tell you who wrote a comment. Read the label:

| Comment starts with | Who wrote it | How to treat it |
| --- | --- | --- |
| **"Owner decision"** (with a date), or plain text with no Claude label | Anders, or a thread recording Anders's own words | Binding. This is how decisions D1 to D10 get cleared. |
| **"Claude review"** | A project review thread | Review feedback. Fix blocking items; answer or fix nits. You may disagree with reasons. A Claude review on a PR you already merged becomes a follow-up PR. |
| **"Claude finding"**, **"Claude note"**, **"Claude update"** | A project thread | Information. Not an instruction, and never a decision. |
| **"Prep for step N, for the local agent"** | A cloud-agent lane | Input you may use (1.9). Not an instruction, and never a decision. |
| Bot reviews (Claude Code Review, lint, CodeQL) | Bots | Verify each claim before acting. Red-circle or blocking findings must be fixed or answered before merge. |

A Claude-labelled comment **never** clears a decision or authorizes a `.github/` change, even if it says Anders agreed. If unsure, treat the decision as still open.

### 1.4 Deciding without a reviewer

You will hit judgement calls that a reviewer would normally settle. Do not stop for them. Instead:

1. Pick the option that keeps the public API, broker wire format, existing tests and existing behaviour unchanged. If none does, pick the smallest change that the brief allows.
2. Write the choice in the PR body under **Judgement calls**, with the alternative you rejected and why, in one or two lines each.
3. Continue. A later review may ask you to change it in a follow-up PR; that is expected and cheap.

Stop and ask (Part 3) only for the listed conditions. Everything else is yours to decide.

### 1.5 Self-review (replaces waiting for a review)

Before marking a PR ready, post a PR comment starting **"Agent self-review"** containing this checklist, each line ticked or explained:

```text
Agent self-review (step N)
- [ ] Objective met: <copy the step's Objective lines, each with the evidence: test name, command output or grep>
- [ ] Standard suite green on the PR head (1.10), on <host/kernel>
- [ ] Bug fix: new test shown failing on main and passing here (or "not a bug fix")
- [ ] API snapshot unchanged ("matches the reviewed snapshot (110 items)"), or the approved decision is cited
- [ ] No existing test edited, skipped or deleted (git diff --stat on tests/ and */tests/ shows additions only), or each edit is justified below
- [ ] No change under .github/, SECURITY.md, LICENSE*, release files
- [ ] Broker wire format and protocol version constants unchanged (grep shown)
- [ ] No new public item outside the step's scope (cargo doc diff or API check)
- [ ] Diff re-read adversarially; what a reviewer would object to and how it is handled:
- [ ] Judgement calls listed in the PR body
- [ ] Docs touched where behaviour changed (docs/STATUS.md, CORE_ARCHITECTURE.md, the issue)
```

A box you cannot tick blocks the merge unless the line says why it does not apply. Be honest: the self-review is what Anders and later reviewers read instead of watching you work.

### 1.6 Merging

There are two merge modes. **D10** on #176 decides which applies.

**Mode A (until D10 is granted): Anders merges.** Mark the PR ready when the merge gate below passes, and keep working. If the next step depends on the unmerged one, branch from the unmerged step's branch (a stack) rather than waiting. Keep stacks at most **three PRs deep**; when a stack is full, work a step that does not depend on it (Part 2 marks parallel steps). When a parent merges, merge `main` into the child and re-validate.

**Mode B (after D10 is granted): you merge your own PRs** with a squash merge when the merge gate passes, except for the owner-merge list.

Merge gate (both modes):

1. Every required check is green on the current head, and the branch is up to date with `main`.
2. The "Agent self-review" comment is posted and complete.
3. No unresolved blocking item from a "Claude review", a human comment, or a red-circle bot finding.
4. The PR has been marked ready for at least **2 hours**, or a "Claude review" on the current head says it is ready to merge. This gives a check-in the chance to catch something without making it a gate.

Owner-merge list (Anders merges these in either mode; mark them ready and move on):

- any change to the API snapshot (step 10c);
- marking an ADR "Accepted" (ADRs may self-merge as "Proposed");
- removing a provider or realization (step 12b, dummy_hcd);
- anything touching `.github/`, `SECURITY.md`, the license or release files (step 17);
- any PR that edits or deletes an existing test.

### 1.7 When something you merged is wrong

- `main` red after your merge: open a revert PR at once and merge it under the same rules (in Mode A, flag it on #176 as **"Agent question (revert)"**). Then fix forward on a new branch.
- A review arrives after the merge: treat its blocking items as a follow-up PR in your next session, linked from the original PR.

### 1.8 Gates and decisions

| ID | Decision | Gates | While it is open, do this instead |
| --- | --- | --- | --- |
| D1 | Lift the hold on code changes | everything | **Cleared 2026-10-15 05:54 UTC** |
| D2 | API snapshot may grow for `host_outputs()` and typed kinds | 10c | Land 10a and 10b. Open 10c as a draft with the snapshot diff listed, and do not merge it. |
| D3 | Logical-controller vs presentation split (epochs) | 5c, 13a | Merge ADR-0021 as "Proposed"; skip 13a. |
| D4 | Bulk subsystems as an ADR now | 5e | Leave ADR-0023 as a draft PR. |
| D5 | dummy_hcd keep or park | 12b | Do 4d in full (keep dummy_hcd working); skip 12b. |
| D6 | Wii production gate (#158) | after this plan | Nothing in this plan. |
| D7 | Wii first realizations | after this plan | Nothing in this plan. |
| D8 | `.github/` change for #150 | 17 | Skip 17; re-run Windows once per failure as in 1.10. |
| D9 | Host for live lab runs | 7c, 16 | Use the machine you run on if it meets the step's host requirements, and record the host in the PR. Otherwise skip, and say why in the status comment. |
| D10 | You may merge your own PRs (Mode B) | merge mode | Mode A (stack and keep going). |

Record skipped steps and why in the status comment. Never wait idle on a decision.

### 1.9 Using cloud-lane prep

Cloud agents prepare some steps ahead of you (specs, ADR drafts, manifests, research ledgers, test sketches). They are labelled "Prep for step N, for the local agent" on the step's issue or opened as docs PRs. Before starting a step, check for its prep:

| Step | Prep, if any | Where |
| --- | --- | --- |
| 4 registry | implementation spec | #173 comment 6185758742 |
| 5 ADRs | ADR-0019 to ADR-0023 as Proposed docs PRs | #176 comment 6185737019, PRs labelled "Agent (lane: adrs)" |
| 6, 8 live topology | seam audit and test sketches | #163 comment 6191783744 |
| 7 runner | manifest schema, manifests as data, timing method | #166 comment 6185734497 |
| 9 resources | implementation spec | #162 comment 6191791649 |
| 11 ledgers | research-repo PRs | #167 comment 6185726279 |
| 16 Steam P1 | live-run procedure, Valve udev rules | #159 comment 6185727138 |

Prep is a head start, not a spec you must follow. Verify it against the code (cloud agents cannot build the workspace); where it is wrong, do what the brief and the code require and note the difference in the PR. If a lane's docs PR covers your step (for example an ADR), review and merge it under 1.6 instead of writing a second one, or supersede it with yours and close it with a link. Never edit a lane's branch.

### 1.10 Validation you run on every step

```bash
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
python3 scripts/check-alpha-api.py      # "matches the reviewed snapshot (110 items)"
gitleaks detect
# when scripts/ changed:
ruff check .
(cd scripts/tests && python3 -m unittest $(ls test_*.py | grep -vx test_protocol_research.py | sed 's/\.py$//'))
```

Research-repo steps run `tools/publication_guard.py`, `tools/validate.py` and the repo's tests instead.

Known environment failures: on a bare host about five Python tests (`test_broker_audio_live`, one in `test_host_helper`) fail as they do on `main`; list them in the PR instead of skipping them. The Windows job sometimes fails crate or git downloads (`CRYPT_E_REVOCATION_OFFLINE`, "Connection was reset"; #150). Re-run it once per failure. If the re-run also fails with the same network error, say so on the PR and re-run once more later; never change revocation checking.

### 1.11 How you report

Label everything you post **"Agent (implementer)"** and end with the Claude Code footer the repository already uses.

- **Status comment.** Keep one comment on #176 starting `<!-- agent-status -->`, edited in place: merge mode, current step, open PRs with state (`developing`, `ready`, `waiting on Anders`, `stacked on #N`), skipped steps and why, and judgement calls you want a check-in to look at. Edit it whenever state changes.
- **PRs.** Draft while developing; ready when the merge gate items 1 to 3 pass. Use the template in 1.12.
- **Questions.** Only for Part 3 conditions: one comment on #176 starting **"Agent question (step N / Dn)"** with options and your recommendation, then work on something else.
- **Merged step.** Tick its #176 checklist line and post a one-line comment linking the PR.
- **Checkpoints.** After steps **4e**, **9**, **10b** and **14**, post one comment on #176 starting **"Agent checkpoint"**: the PRs merged since the last checkpoint, every judgement call in them, anything you would like reviewed, and what comes next. Check-ins review in batches from these, so make them complete.
- **Controller issues.** When a step changes what a controller can do, post one line on that controller's issue (#154 to #160) linking the PR.

### 1.12 PR template

The PR body opens with **Before:** and **After:** paragraphs in plain language, then:

```text
Step: <N> (<brief file>)   Tracking: #176   Issue: #<n>
Decisions relied on: <Dn and the comment link, or "none">
Prep used: <lane comment or PR link, and where you departed from it, or "none">
Stacked on: <#PR, or "main">

How:
<short paragraph>

Judgement calls:
- <choice> (rejected: <alternative>, because <reason>)

Validation (commands run on <host/kernel>, all on the PR head):
- <the 1.10 suite, one line each with the result>
- <step objective checks>: <result>

New tests: <list, with what each proves>
Known environment failures: <as in 1.10>

Completion Summary (AGENTS.md format)
```

---

## Part 2. Steps

Each step lists an **Objective**: checks you can run yourself that prove the step is done. If every Objective line holds and the merge gate passes, the step is done without anyone else's review. Steps marked *(parallel)* do not touch the Core lane's files and are what you work on while a stack is full or a gate is closed.

Progress at this revision: steps 1 and 2 are ready (#180, #181, waiting on Anders's merge); step 3 is in progress (Puck test question answered on #176 comment 6189359458: assert per transport).

### Step 1. Switch Pro SPI fix (#174) — brief 01a

**Develop:** explicit SPI ranges in `switch_spi_byte` (IMU calibration `0x6020..0x6038`, factory stick calibration `0x603d..0x604F`, colors `0x6050..0x605B`, else `0xFF`); clear the "colors in SPI" flag unless a source-backed retail color record exists.
**Objective:** tests on the subcommand `0x10` reply cover each range, both boundaries of each, a straddling read and a read outside all ranges; a test ties the device-info color flag to the color range; the stick calibration test fails on `main`.
**Report:** #174 closed with a comment, one line on #156.

### Step 2. Hygiene — brief 01b *(parallel)*

**Develop:** ignore `__pycache__/` and `*.pyc`; bump `protocol-research` to the research repo's `main`.
**Objective:** `scripts/tests/test_protocol_research_pin.py` passes; `git status` is clean after the Python tests run; fixture manifest hashes match the new pin.

### Step 3. Steam probe, source pass (P0) — brief 03 *(parallel, research repo)*

**Develop:** pin the first kernel release carrying `STEAM_CONTROLLER_IBEX` (`28de:1302`); record from `hid-steam.c` at that release whether the wired path needs a USB interface or parent (file:line); record whether a pinned SDL3 HIDAPI Steam driver supports `0x1302` and filters on interface number or usage page.
**Objective:** research checks pass; no GPL excerpts; every kernel and SDL fact cites commit and file:line; registry tests assert per transport rather than on whole dicts.
**Report:** one line on #159 with the provisional answer (UHID likely accepted or not), labelled as an inference until step 16.

### Step 4. Registry (#173) — brief 02, five PRs in order (Core, stackable)

The critical path. Each PR leaves all four families working. Prep: #173 comment 6185758742.

**4a. `ControllerFamily` trait and `FAMILIES`, additive.** The trait (`ID`, `usb_profile()`, `BROKER_TAG`, `topology()`, per-realization tier and permission class) and crate-internal stub fields (`max_topology`, `output_kinds`, `subsystems`, `persistent_resources`, `refused_operations`, `kind`) for the four families.
Objective: tests that IDs and broker tags are unique, declared realizations match surfaces, and tier/permission pairs are consistent; a `cfg(test)` fifth family declaring `linux.uinput` and `linux.uhid.bluetooth` creates on bus 5 through the curated session with a fake provider; API check unchanged; no existing test edited.

**4b. `gr-usbip` profiles from the registry.** `Profile::new(&UsbProfileSpec)`; channel layouts from the spec.
Objective: the first commit adds a test recording today's descriptor bytes for all three profiles, and it passes unchanged after the switch; `ProfileId` no longer exists (grep).

**4c. Broker and audio worker through the registry.** Generated tag table replacing the hand-written `match`; `gr-audio-worker` dispatches through the registry.
Objective: `git diff main -- <broker security tests>` is empty; a test proves the generated table equals today's tags; wire-format and protocol-version constants unchanged (grep shown).

**4d. Replace `CompiledControllerKind`.** `ControllerId` lookups in `NativeDummyHcdRealization` and `dummy_hcd.rs` (if D5 has parked dummy_hcd, do 12b first and shrink this to leftovers).
Objective: the enum no longer exists (grep); dummy_hcd creation returns the same error as before (test).

**4e. ADR-0018, broker allowlist.** ADR in `docs/adr/`; update `docs/STATUS.md` and `docs/CORE_ARCHITECTURE.md`.
Objective: the 4a test family needs only its module plus one registry line (diff shown); ADR merged as "Proposed" unless Anders accepts it.

**Report:** #173 closed; #176 lines ticked; **checkpoint** after 4e.

### Step 5. Contract ADRs — brief 04 *(parallel, docs only; one PR each)*

5a ADR-0019 live topology · 5b ADR-0020 retained outputs and kinds · 5c ADR-0021 epochs (D3 for "Accepted") · 5d ADR-0022 receivers · 5e ADR-0023 subsystems (stays draft until D4).
Prep: the ADR lane may already have PRs open. Review them against brief 04 and the code, fix or supersede them, and merge as "Proposed".
**Objective:** each ADR answers every question brief 04 lists for it (checklist in the PR); links resolve; `docs/STATUS.md` links each ADR; any proposed root API addition is listed explicitly.

### Step 6. Live topology seam audit (#163) — brief 06a *(parallel after 4a)*

**Develop:** tests in `crates/gr-hid/tests/internal_topology.rs` for #163's cases: queued bytes never rewritten, rapid plug/unplug, edge saturation, invalid transitions, malicious register writes, probe overlap, close mid-change, no ambiguous retry of non-repeatable sends. Change core code only for a failing test. Prep: #163 comment 6191783744.
**Objective:** one named test per #163 case (table in the PR mapping case to test); any core change has its failing test shown first.
**Report:** #163 comment saying "no core change needed" with the test list, or naming the fix.

### Step 7. Acceptance manifests and runner (#166) — brief 09 *(parallel after 4a)*

Prep: #166 comment 6185734497.
**7a. Manifests.** `acceptance/<id>.toml` per family, cells in the four evidence categories with research record IDs. Objective: a test that every family has a manifest and every cell names a declared realization.
**7b. Runner.** An `xtask` crate with `acceptance` and `acceptance-diff`, recording #166's timestamps, kernel driver binding and environment. Objective: a deterministic test runs the runner against fake providers in ordinary CI; JSON output passes a schema test.
**7c. Live runs (D9).** At least two existing controllers on the recorded host, 1 and N concurrent, repeated cycles. Objective: run JSON filed as research-repo experiments; missing cells marked honestly; no budgets set.
**Report:** `docs/STATUS.md` updated; #166 comment with results.

### Step 8. Live topology generations — brief 06b *(after 4e and 5a)*

**Develop:** crate-internal `topology_generation()`, `current_topology()` and a retained counter; edits to absent controls fail with `UnavailableInRealization`.
**Objective:** a test family with one optional module covers attach, detach, rejected edit while absent, monotonic counter, the ADR's uinput policy, and close during a change; API check unchanged.

### Step 9. Persistent resources seam (#162) — brief 07 *(after 4a; agree file ownership with step 8 in the PR)*

Prep: #162 comment 6191791649.
**Develop:** validated import before any provider opens; explicit `KeepSnapshot` / `Discard` after deliberate or error close; first disposition wins; resource-dirty separate from `is_dirty()`.
**Objective:** every item on #162's "Minimum regressions" list (comment 6163240564, minus Wii address maps) has a named test; a rejected import opens nothing (fake provider asserts it); `Drop` never exports; a stateless family carries no new fields at runtime (test or size assertion).
**Report:** #162 comment with the integration recipe Wii will follow; **checkpoint** after 9.

### Step 10. Root facade, then retained outputs — briefs 10 (step 2) and 05 (Core)

**10a. Facade.** Crate-private `Controller<F>`, newtype public types, `macro_rules!` forwarding. Objective: API check passes (doc comments identical); `src/controllers/tests.rs` unchanged (`git diff` empty) and green; line count before/after reported.
**10b. Retained outputs below the root (05a).** Objective: a burst larger than the 32-entry queue keeps the latest value; the revision increases once per accepted change; malformed outputs leave it unchanged. **Checkpoint** after 10b.
**10c. Root `host_outputs()` and typed kinds (05b, 05c).** Needs D2 and is on the owner-merge list. Objective: the snapshot is regenerated and every added item listed in the PR body; new types are `#[non_exhaustive]`.

### Step 11. Research ledgers (#167) — brief 11 *(parallel, research repo)*

Prep: the research-ledgers lane opens these PRs (#167 comment 6185726279). Your part: review each lane PR against brief 11 and merge it under 1.6, write any ledger the lane has not started, then bump the `protocol-research` pin in virtualgamepad.
**Objective:** guard, validator and tests pass; every kernel fact cites commit and file:line with no excerpt; Wii revisions and Xbox GIP vs Bluetooth kept apart; the pin bump passes `test_protocol_research_pin.py`.

### Step 12. Crate tidy — brief 10 (step 3)

**12a.** `gr-root-audio` crate and wire-crate fold. Objective: API check unchanged; `--no-default-features`, each feature alone and `--all-features` all build.
**12b. dummy_hcd (D5 = park; owner-merge).** Create `archive/dummy-hcd` from `main` first; remove the provider, broker module, per-family surfaces and docs rows; keep `RealizationId::LINUX_DUMMY_HCD_USB_HID` rejecting. Objective: creation with that ID returns the same error (test).

### Step 13. Presentation epochs and receivers (#165) — brief 08 *(needs 5c, 5d and a Steam tier answer from step 3 or 16)*

**13a. Epochs (D3).** Objective: tests for swap success, prepare failure leaving the old presentation intact, failure mid-swap, close during swap, identity kept, counters reset.
**13b. Receiver aggregate.** Objective: #165's completion evidence as tests: deterministic 0-to-4-slot fixture with optional companions, simultaneous changes, rollback, required requests, close/failure, sibling survival.

### Step 14. Demo from topology — brief 10 (step 5)

**Objective:** GUI lifecycle tests unchanged and green; a new test that every registry family produces an editor; `grep` shows no per-family branches left in `demo/src/gui.rs` except declared-output panels. **Checkpoint** after 14.

### Step 15. Readiness report (no PR)

Post one comment on #176 and on each of #158, #159, #160: what is now in place for that controller, what remains from the "What each new controller needs first" table in `README.md`, and which decisions are still open. This hands off to controller work.

### Step 16. Steam probe on a host (P1) — brief 03 *(D9; any time after step 3)*

Host needs `/dev/uhid` and kernel **v7.3-rc1 or newer** (first release with the IBEX ID). Throwaway probe on a branch, never merged. Cases: `28de:1302` UHID USB bus, `28de:1304` single node, DualSense control. Prep: #159 comment 6185727138.
**Objective:** experiment record in the research repo with driver binding, hidraw, hidapi interface number and bus, SDL3 detection, Steam client listing and first report delivery per case; brief 03's decision rule applied in one line on #159 and #176.

### Step 17. Windows CI downloads (#150) — brief 12 *(D8; owner-merge)*

**Objective:** a cold-cache and a warm-cache run linked in the PR; revocation checking untouched; egress allowlist changes listed.

---

## Part 3. When to stop and ask (on #176)

Ask, then work on something else:

- A step would change the API snapshot without D2, or the broker wire format at all.
- A step needs `.github/`, security, license or release files.
- A test you did not write fails for a reason that is not environmental, and the fix is not obviously within your step.
- Two feedback sources conflict (for example a Claude review and an owner decision): the owner decision wins; say which you followed.
- You are blocked on every remaining step: post a status comment listing each block, then stop.

A judgement call (1.4), a missing review, or prep that disagrees with the code is **not** a reason to stop.

---

## Part 4. For project threads checking in

Check-ins are optional and asynchronous. To check in: read the `<!-- agent-status -->` comment and the latest "Agent checkpoint" on #176, then post "Claude review" comments on the PRs or checkpoint. Blocking items on a merged PR become a follow-up PR (1.7). Do not ask the agent to wait for a review, and do not use a review to clear a decision.
