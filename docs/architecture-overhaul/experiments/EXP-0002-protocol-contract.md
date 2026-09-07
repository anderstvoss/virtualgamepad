# EXP-0002 — Deterministic protocol contract

Owner: Codex. Date: 2026-09-05. Inputs: reviewed kit `e3bd06f`, corpus gitlink adopted in `18ca773`; implementation revision is the commit containing this record. Gates C/D/J/O and K, scoped to the in-memory contract and DualSense USB subset.

Acceptance declared before production adoption: synchronous explicit-time service; independent neutral/Cross fixtures; numbered/unnumbered report framing; exact GET/SET success/error; stable retry bytes and sequence wrap; bounded queue and submission work; autonomous cadence/watchdog; non-repeatable edges; independent session removal; terminal/idempotent cleanup; request deadlines and uncertain-delivery failure.

Harness: `gr-hid` private fake transport and clock, plus a test-only DualSense USB personality. A cycle reads at most one event and spends a bounded submission budget. The synthetic SC2-inspired personality is a scheduling stress case, not a physical implementation.

Commands: `cargo test -p gr-hid -p gr-curated-controllers --lib`; workspace strict Clippy and tests before commit. Tests compare two manually constructed corpus frames independently of the encoder. Existing controller layout and uinput regressions remain. Framing tests cover all three report classes, unnumbered ID zero, numbered zero rejection, empty payloads and size limits. Live envelopes and host-driver behavior remain separate provider gates.

Review found and corrected two prototype issues: a minimum submission budget could starve input when an old reply and new request occupied the same cycle; typed output from an acknowledged SET could be lost if subsequent input generation hit pressure. Regression tests cover both. Invalid runtime construction closes its acquired transport.

Result: deterministic contract selected by ADR-0004. No Linux, physical, SDL, Steam, audio, or Bluetooth acceptance is established by these tests. Gate K remains limited to checked-in test artifacts from ADR-0003.
