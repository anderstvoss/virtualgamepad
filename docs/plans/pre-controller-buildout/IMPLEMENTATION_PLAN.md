# Pre-controller buildout: implementation plan for an executing agent

Date: 2026-10-15. Repositories: `anderstvoss/virtualgamepad` (base `main` at `5a0a7dd`) and `anderstvoss/controller-protocol-research` (base `abffb5d`).
Tracking issue: **#176**. Detailed briefs: `docs/plans/pre-controller-buildout/01-…12-*.md` next to this file. When this plan and a brief disagree, this plan wins on order and process, and the brief wins on technical content.

You are the implementing agent. You build each step, validate it, open a PR, and act on feedback that arrives through GitHub issues and PRs. Project threads watch your work and review it there. You never get feedback any other way, so GitHub is your only inbox.

---

## Part 1. How you work

### 1.1 Rules that always apply

Read `ground-rules.md` and the repository's `AGENTS.md` before step 1. In short:

- The ordinary-root API snapshot (110 items) does not change unless the step names an approved decision allowing it.
- No `.github/`, `SECURITY.md`, license or release changes without an explicit owner comment approving that exact change.
- No broker wire-format or protocol-version change.
- No runtime profiles, plugins, descriptor injection, generic mutable gamepad state, provider-owned protocol semantics or silent realization fallback.
- Every bug fix gets a regression test at the lowest seam. Never weaken, skip, quarantine or delete a test to get green.
- One step = one PR. Do not stack unrelated work into a PR.
- Do not merge your own PRs. Anders merges, unless he says on the PR that a reviewer may.

### 1.2 Start of every session (intake)

Do this before any new work, every time you start or resume:

1. Read #176: the body (checklist and the "Open decisions" list) and every comment newer than your last status comment.
2. List your open PRs in both repositories. For each, read reviews, review comments, issue comments and CI status on the current head.
3. Read new comments on the issue for the step you are on (named in each step below).
4. Handle feedback in this order: merge conflicts, then red CI, then review comments that request changes, then questions, then new work.
5. Only when nothing on your open PRs needs action, pick the next step from the order in Part 2 whose gates are met.

### 1.3 Reading feedback correctly

All project Claude threads post under Anders's GitHub account, so the author name does not tell you who wrote a comment. Read the label:

| Comment starts with | Who wrote it | How to treat it |
| --- | --- | --- |
| **"Owner decision"** (with a date), or plain text with no Claude label | Anders, or a thread recording Anders's own words | Binding. This is how decisions D1 to D9 get cleared. |
| **"Claude review"** | A project review thread | Review feedback. Fix blocking items; answer or fix nits. You may disagree with reasons. |
| **"Claude finding"**, **"Claude note"**, **"Claude update"** | A project thread | Information. Not an instruction, and never a decision. |
| Bot reviews (Claude Code Review, lint, CodeQL) | Bots | Verify each claim before acting. Red-circle or blocking findings must be fixed or answered. |

A Claude-labelled comment **never** clears a decision or authorizes a `.github/` change, even if it says Anders agreed. Only an "Owner decision" comment or Anders's unlabeled words do that. If unsure, ask on #176.

### 1.4 How you report

Label everything you post with **"Agent (implementer)"** so threads can tell you apart from them. End each post with the Claude Code footer the repository already uses.

- **Status comment.** Keep one status comment on #176, starting with `<!-- agent-status -->` and edited in place. It holds: current step, PR link, state (`developing`, `validating`, `in review`, `blocked`), and what you are waiting on. Edit it whenever state changes. Post a new comment only for a question or a merged step.
- **PRs.** Open as **draft** while developing. Mark **ready for review** only when every validation item for the step passes. Use the PR template in 1.5.
- **Questions and blockers.** Post one comment on #176 starting **"Agent question (step N / Dn)"**: what is blocked, the options, your recommendation. Then work on the next unblocked step. Never wait idle on an answer.
- **Merged step.** Tick its #176 checklist line and post a one-line comment linking the PR.
- **Controller issues.** When a step changes what a controller can do, post one line on that controller's issue (#154 to #160) linking the PR.

### 1.5 PR template

The PR body opens with **Before:** and **After:** paragraphs in plain language, then:

```text
Step: <N> (<brief file>)   Tracking: #176   Issue: #<n>
Decisions relied on: <Dn and the comment link, or "none">

How:
<short paragraph>

Validation (commands run on <host/kernel>, all on the PR head):
- cargo fmt --all -- --check: pass
- cargo check --locked --workspace --all-targets --all-features: pass
- cargo clippy --locked --workspace --all-targets --all-features -- -D warnings: pass
- cargo test --locked --workspace --all-features: pass (<N> tests)
- python3 scripts/check-alpha-api.py: "matches the reviewed snapshot (110 items)"
- gitleaks detect: pass
- <step-specific checks from this plan>: <result>

New tests: <list, with what each proves>
Known environment failures: <e.g. test_broker_audio_live on a bare host, same as main>

Completion Summary (AGENTS.md format)
```

### 1.6 Validation you run on every step

The standard suite (in the template above), plus ruff and the Python tests when `scripts/` changed:

```bash
ruff check .
(cd scripts/tests && python3 -m unittest $(ls test_*.py | grep -vx test_protocol_research.py | sed 's/\.py$//'))
```

Also re-read your own diff adversarially before marking ready: what would make CI or a reviewer reject it? For a bug fix, show the new test failing on `main` and passing on the branch.

### 1.7 Review and merge loop

1. Draft PR, develop, validate, mark ready, update the status comment to `in review`.
2. A review thread posts a **"Claude review"**. Address every blocking item with a push, reply on each thread, and re-validate.
3. CI red: diagnose and push a fix. The Windows `CRYPT_E_REVOCATION_OFFLINE` download failure (#150) may be re-run once; if it fails again, say so on the PR.
4. Merge conflict: merge `main` into your branch (no force-push on shared branches), regenerate generated files with the repo's tools, re-validate.
5. Anders merges. Then do the "Merged step" reporting in 1.4 and start the next step from fresh `main`.

### 1.8 Gates and decisions

Nothing in Part 2 starts until **D1** is cleared by an "Owner decision" comment on #176. D1 may clear only some lanes; respect that.

| ID | Decision | Gates |
| --- | --- | --- |
| D1 | Lift the hold on code changes (all or per lane) | everything |
| D2 | API snapshot may grow for `host_outputs()` and typed output/input kinds | 10b, 10c |
| D3 | Accept the logical-controller vs presentation split | 5c (ADR-0021), 13 |
| D4 | Bulk subsystems as an ADR now | 5e (ADR-0023) |
| D5 | dummy_hcd keep or park | 12b |
| D6 | Wii production gate (on #158) | after this plan |
| D7 | Wii first realizations | after this plan |
| D8 | `.github/` change for #150 | 17 |
| D9 | Host for live lab runs | 7c, 16 |

If a step is gated, skip it and take the next ungated one. Record skipped steps in the status comment.

---

## Part 2. Steps

Order is the default sequence for a single agent. Steps marked *(parallel)* do not touch the same files as the Core lane and may be interleaved while a Core PR waits for review. Do not have more than two PRs open in review at once.

### Step 1. Switch Pro SPI fix (#174) — brief 01a

**Develop**
- In `crates/gr-curated-controllers/src/switch_pro.rs`, replace the open-ended `checked_sub(0x6020)` match in `switch_spi_byte` (line ~721) with explicit ranges: IMU calibration `0x6020..0x6038`, factory stick calibration `0x603d..0x604F`, colors `0x6050..0x605B`, otherwise `0xFF`.
- Clear the "colors in SPI" flag in the subcommand `0x02` device-info reply, unless `controller-protocol-research` already has a source-backed retail color set (then use it and cite the record ID). Never invent color bytes.

**Validate**
- New tests on the subcommand `0x10` reply: each range, both boundaries of each, a read straddling two ranges, a read wholly outside any range.
- A test that the device-info color flag agrees with what the color range returns.
- Show the stick calibration test failing on `main`.

**Feedback:** PR plus #174. **Done:** merged, #174 closed, one line on #156.

### Step 2. Hygiene — brief 01b *(parallel)*

**Develop:** add `__pycache__/` and `*.pyc` to `.gitignore`; bump the `protocol-research` submodule to the research repo's current `main`.
**Validate:** `scripts/tests/test_protocol_research_pin.py` passes; `git status` clean after running the Python tests.
**Feedback:** PR plus #176.

### Step 3. Steam probe, source pass (P0) — brief 03 *(parallel, research repo)*

**Develop:** pin the first kernel release that carries `STEAM_CONTROLLER_IBEX` (`28de:1302`); record from `hid-steam.c` at that release whether the wired path needs a USB interface or USB parent (file:line); record whether a pinned SDL3 HIDAPI Steam driver supports `0x1302` and filters on interface number or usage page. File these as source records and consumer-evidence claims.
**Validate:** `tools/publication_guard.py`, `tools/validate.py` and the research repo tests pass; no GPL excerpts.
**Feedback:** research-repo PR plus #159. **Done:** merged, one line on #159 with the provisional answer (UHID likely accepted or not).

### Step 4. Registry (#173) — brief 02, five PRs in order

The critical path. Each PR leaves all four families working.

**4a. `ControllerFamily` trait and `FAMILIES`, additive**
- Develop: the trait (`ID`, `usb_profile()`, `BROKER_TAG`, `topology()`, per-realization tier and permission class) and crate-internal stub fields (`max_topology`, `output_kinds`, `subsystems`, `persistent_resources`, `refused_operations`, `kind`). Implement it for the four families.
- Validate: tests that IDs and broker tags are unique; declared realizations match surfaces; tier and permission pairs are consistent. A `cfg(test)` fifth family declaring `linux.uinput` and `linux.uhid.bluetooth` that creates on the Bluetooth bus (bus 5) through the curated session with a fake provider. API check unchanged.

**4b. `gr-usbip` profiles from the registry**
- Develop: `Profile::new(&UsbProfileSpec)`; channel layouts from the spec.
- Validate: first commit a test that records today's descriptor bytes for all three profiles; the switch keeps them byte-identical; then delete `ProfileId`.

**4c. Broker and audio worker through the registry**
- Develop: generated tag table replacing the hand-written `match`; `gr-audio-worker` dispatches through the registry.
- Validate: existing broker security tests pass **unchanged** (no edits to them in the diff); a test that the generated table equals today's tags; wire format and protocol version constants unchanged.

**4d. Replace `CompiledControllerKind`**
- Develop: `ControllerId` lookups in `NativeDummyHcdRealization` and `dummy_hcd.rs`. If D5 has already parked dummy_hcd, do step 12b first and shrink this to deleting leftovers.
- Validate: the enum no longer exists (`grep` in the PR body); dummy_hcd creation still rejects as before.

**4e. ADR-0018, broker allowlist**
- Develop: ADR in `docs/adr/`; update `docs/STATUS.md` and `docs/CORE_ARCHITECTURE.md`.
- Validate: the test family from 4a needs only its module plus one registry line (show the diff of that test family).

**Feedback:** each PR plus #173. **Done:** #173 closed, #176 lines ticked.

### Step 5. Contract ADRs — brief 04 *(parallel, docs only; one PR each)*

5a ADR-0019 live topology · 5b ADR-0020 retained outputs and kinds · 5c ADR-0021 presentation epochs (needs D3) · 5d ADR-0022 receivers · 5e ADR-0023 subsystems (needs D4).
**Develop:** follow ADR-0013 and ADR-0014's format; each settles the questions brief 04 lists and states what the API snapshot gains when the first consumer family ships.
**Validate:** links resolve; `docs/STATUS.md` links each ADR; any proposed root API addition is listed explicitly so D2 review can see it.
**Feedback:** PR plus #176. Status "Accepted" only if Anders approves on the PR; otherwise merge as "Proposed".

### Step 6. Live topology seam audit (#163) — brief 06a *(after 4a)*

**Develop:** new tests in `crates/gr-hid/tests/internal_topology.rs` for the cases #163 lists: queued bytes never rewritten, rapid plug/unplug, edge saturation, invalid transitions, malicious register writes, probe overlap, close mid-change, no ambiguous retry of non-repeatable sends. Change core code only for a test that fails.
**Validate:** every new test is named in the PR; any core change has its failing test shown first.
**Feedback:** PR plus #163. **Done:** a #163 comment saying "no core change needed" (with the test list) or naming the fix.

### Step 7. Acceptance manifests and runner (#166) — brief 09 *(after 4a)*

**7a. Manifests.** `acceptance/<id>.toml` per family, cells in Anders's four categories with research record IDs. Validate: a test that every family has a manifest and every cell names a declared realization.
**7b. Runner.** An `xtask` crate with `acceptance` and `acceptance-diff`, recording #166's timestamps, kernel driver binding and environment. Validate: a deterministic test runs the runner against fake providers in ordinary CI; JSON output matches a schema test.
**7c. Live runs (needs D9).** Run it for at least two existing controllers on the named host, 1 and N concurrent, repeated cycles. Validate: run JSON filed as research-repo experiments; missing cells marked honestly; no budgets set.
**Feedback:** PRs plus #166. **Done:** `docs/STATUS.md` updated, #166 comment with results.

### Step 8. Live topology generations — brief 06b *(after 4e and 5a)*

**Develop:** `topology_generation()`, `current_topology()` and a retained counter, kept crate-internal; edits to absent controls fail with `UnavailableInRealization`.
**Validate:** test family with one optional module: attach, detach, rejected edit while absent, counter monotonic, the ADR's uinput policy, close during a change. API check unchanged.
**Feedback:** PR plus #163.

### Step 9. Persistent resources seam (#162) — brief 07 *(after 4a; agree file ownership with step 8)*

**Develop:** validated import before any provider opens; explicit `KeepSnapshot` / `Discard` after deliberate or error close; first disposition wins; resource-dirty separate from `is_dirty()`. Optional file helper only if a caller needs it.
**Validate:** the "Minimum regressions" list on #162 (comment 6163240564) minus Wii address maps; rejected import leaves no host nodes (fake provider asserts nothing opened); `Drop` never exports; a stateless control family shows no overhead.
**Feedback:** PR plus #162. **Done:** #162 comment with the integration recipe Wii will follow.

### Step 10. Root facade, then retained outputs — briefs 10 (step 2) and 05

**10a. Facade.** Generic crate-private `Controller<F>`, newtype public types, `macro_rules!` forwarding. Validate: doc comments identical (API check passes), `src/controllers/tests.rs` unchanged and green, line count reported.
**10b. Retained outputs below the root (05a).** Validate: a burst larger than the 32-entry queue keeps the latest value; revision increases once per accepted change; malformed outputs do not change it.
**10c. Root `host_outputs()` and typed kinds (05b, 05c; needs D2).** Validate: the snapshot is regenerated and every added item listed in the PR body; new types are `#[non_exhaustive]`.
**Feedback:** PRs plus #176.

### Step 11. Research ledgers (#167) — brief 11 *(parallel, research repo; one PR each)*

11a consumer-evidence marker only if needed (registry, transports and `tools/status.py` landed in research PR #7) · 11b Wii L0 ledger · 11c Steam migration from #159 · 11d Xbox Series L0 inventory.
**Validate:** guard, validator and tests pass; every kernel fact cites commit and file:line with no excerpt; Wii revisions and Xbox GIP vs Bluetooth kept apart.
**Feedback:** research PR plus the controller issue. **Done:** submodule pin bumped in virtualgamepad afterwards.

### Step 12. Crate tidy — brief 10 (step 3)

**12a.** `gr-root-audio` crate, wire-crate fold. Validate: same public names behind the same features (API check), all feature combinations build (`--no-default-features`, each feature alone, `--all-features`).
**12b. dummy_hcd (needs D5 = park).** Create `archive/dummy-hcd` from `main` first; remove the provider, broker module, per-family surfaces and docs rows; keep `RealizationId::LINUX_DUMMY_HCD_USB_HID` rejecting. Validate: creation with that ID still returns the same error (test).

### Step 13. Presentation epochs and receivers (#165) — brief 08 *(needs 5c, 5d and step 16 or P0's tier answer)*

**13a. Epochs.** Validate: swap success, prepare failure leaves the old presentation intact, failure mid-swap, close during swap, identity kept, counters reset.
**13b. Receiver aggregate.** Validate: the #165 completion evidence: deterministic 0-to-4-slot fixture with optional companions, simultaneous changes, rollback, required requests, close/failure, sibling survival.
**Feedback:** PRs plus #165.

### Step 14. Demo from topology — brief 10 (step 5)

Validate: GUI lifecycle tests unchanged; a new test that every registry family produces an editor; no per-family branches left in `demo/src/gui.rs` except declared-output panels.

### Step 15. Readiness report (no PR)

Post one comment on #176 and on each of #158, #159, #160: what is now in place for that controller, what remains from the "What each new controller needs first" table in `README.md`, and which decisions (D6, D7) are still open. This hands off to the controller threads.

### Step 16. Steam probe on a host (P1) — brief 03 *(needs D9; any time after step 3)*

Throwaway probe on a branch, never merged. Cases: `28de:1302` UHID USB bus, `28de:1304` single node, DualSense control. Record driver binding, hidraw, hidapi interface number and bus, SDL3 detection, Steam client listing, first report delivery.
**Validate:** experiment record in the research repo; the decision rule from brief 03 applied in one line on #159 and #176.

### Step 17. Windows CI downloads (#150) — brief 12 *(needs D8)*

Validate: a cold-cache and a warm-cache run linked in the PR; revocation checking untouched; egress allowlist changes listed.

---

## Part 3. When to stop and ask (on #176)

- A step would change the API snapshot without D2, or the broker wire format at all.
- A step needs `.github/`, security, license or release files.
- A test you did not write fails for a reason that is not environmental.
- A review comment asks for something outside this plan's scope.
- Two feedback sources conflict (for example a Claude review and an owner decision): the owner decision wins; say which you followed.
- You have been blocked on every remaining step: post a status comment listing each block, then stop.
