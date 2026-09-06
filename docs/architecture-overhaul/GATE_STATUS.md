# Architecture gate status

This is the current status ledger. Definitions and dependencies live in [the gate register](ARCHITECTURE_DECISION_EXPERIMENTS.md), section 17. Do not mark a gate passed because its design or test harness exists.

| Gate | Question | Execution batch | Status | Owner | EXP / ADR evidence |
| --- | --- | --- | --- | --- | --- |
| A | Corpus evidence model | E1 | passed | Codex | [EXP-0001](experiments/EXP-0001-corpus-seed.md), [ADR-0003](decisions/ADR-0003-corpus-boundary.md); source/synthetic scope only |
| B | Controlled bus/host comparison | E3 | not_run | unassigned | None |
| C | Stateful synchronous protocol contract | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| D | HID framing boundary | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| E | Compound UHID usefulness | E6 compound | not_run | unassigned | None |
| F | Host audio coherence | E6 host audio | not_run | unassigned | None |
| G | Broker capability/startup/latency | Early probe; E5 replacement | not_run | unassigned | None |
| H | USB Audio implementation depth | E6 USB audio | not_run | unassigned | None |
| I | Realization variant granularity | Affected E6 variants | not_run | unassigned | None |
| J | Required replies and deadlines | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| K | Corpus generation boundary | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| L | BT personality over UHID | E6 BT protocol | not_run | unassigned | None |
| M | Actual BT realization viability | E6 BT bus after L | not_run | unassigned | None |
| N | Curated compatibility variants | Affected E6 family | not_run | unassigned | None |
| O | Autonomous cadence and delivery | E2 | passed | Codex | [EXP-0002](experiments/EXP-0002-protocol-contract.md), [ADR-0004](decisions/ADR-0004-synchronous-hid-session.md); deterministic prototype scope |
| P | Specialized driver behavior | E3 with B | not_run | unassigned | None |

## Update rules

Allowed statuses: `not_run`, `running`, `passed`, `failed`, `blocked`, `not_applicable`. The last requires a scoped justification; it is not a pass. Claim an item before execution and name an owner. Link every completed or blocked run to an EXP record and relevant ADR, including negative/mixed outcomes and missing prerequisites.

Record evidence axes independently in each run. A fake-I/O result can pass while live kernel/Steam acceptance remains blocked. When repeated runs disagree, keep every result and report the gate as unresolved rather than selecting the successful run.

E0 preparation has documentation, source/branch inventory, and a passing automated baseline recorded in [BASELINE.md](BASELINE.md) on 2026-09-05. No full batch or architecture gate is signed off. Corpus integration, plan commit, experiment ownership, historical evidence reconciliation, and host survey remain pending; ignored/live acceptance tests were not run.
