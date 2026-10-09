# Provider security and recovery acceptance

Gate 3 passed on this VM at candidate
`e5402d24c7ca8e0c209260c879feb205a95f8dae`, tree
`6894a9c13cb9a6931b22115e06452ec3ccf03fd1`, against base
`11284f01c58cb80be0d187efa2fca95641513fbf`.
This supersedes earlier statements that construction failure, combined errors
or all-family client-exit coverage remain missing. Historical failures remain
in the chronology. Other gates and final release validation remain open.

The seven phases below ran individually, sequentially and with fixed deadlines.
Every phase exited zero; each separately retrieved root-owned receipt reports
`passed` at this exact revision. No failed or skipped phase counts as acceptance.

| Required boundary | Phase and supporting evidence |
| --- | --- |
| Peer authorization, malformed/truncated/oversized frames, unexpected descriptors and descriptor ownership | `rejection`: 33 authorized-client checks, including connection admission, plus unauthorized UID rejection. Unexpected transferred descriptors close; eight connections fill admission, an excess connection rejects, disconnect releases capacity and siblings continue. Unsupported gadget operations reject before resource creation. |
| Positive attachment, FD handoff, normal/repeated close, abandoned close and admission release | `provider-lifecycle`: nine sessions across DualSense, DS4 and Xbox 360. Compiled serial, VHCI/card ancestry, worker diagnostics and client descriptor baseline are verified. |
| Client exit after handoff | `provider-client-exit`: separate process exits for each audio family, no application cleanup. The supervisor verifies all authorized ports free, journals empty and no worker children; each exit is followed by nine positive/repeated/abandoned sessions. |
| Client disconnect before handoff | `provider-client-before-handoff`: valid create request and socket shutdown for each family, then the same supervisor cleanup checks and positive followups. A zero broker PID is pending startup rather than cleanup success. |
| Worker death after readiness and during construction | `provider-worker-death`: three established worker kills plus three pre-handoff construction kills using the staged production executable and validated pidfds. Exact terminal replies and EOF are required; successful handoff rejects the construction test. |
| Initiating and cleanup errors, safe operator restoration | The same worker phase additionally replaces the inode of one held test-owned construction journal for each family, then kills the worker. Each reply preserves the exact SIGKILL error and `attachment cleanup: audio ownership record changed; retained`. Three explicit restorations verify both independently held inode/content identities and attachment removal. Nine subsequent positive/repeated/abandoned sessions pass. |
| Broker death, stale records, restart rejection and restoration | `provider-broker-death`: three established-session broker kills; worker exit/connection EOF, attachment removal, twelve pending/truncated/malformed/identity-replaced startup-rejection checks, held-identity operator restoration and nine successful followup sessions. Unprovable records remain untouched. |
| Forced sibling survival, worker-session admission and capacity recovery | `provider-siblings-admission`: normal four-session capacity/removal checks, then one forced worker failure per audio family while three siblings retain attachment identity and answer fresh diagnostics. A replacement fills the recovered fourth slot; a fifth open rejects exactly. Final owned cleanup and nine followup sessions pass. |

Broker SHA-256:
`f1b6f56178e5a971c42b28ee5d8d5a4026e820850687bc37d0abe91c38d35cfa`.
Worker SHA-256:
`bf3e795fb1d9759c39898884fb6dcc6b7fc92b5d68955c9029c90815076d0c37`.
The candidate package records compiler, source/tree, lockfile and payload hashes.
The production worker changed to validate USB interrupt outputs with the
controller setter policy. All seven phases were rerun on that image.
Ordinary-root contracts remain unchanged. The test clients ran non-root; workers used a separate non-root
identity. The unauthorized identity was distinct from both.

The lab retained the real ownership lock, isolated cards before attachment using
compiled session serial and VHCI ancestry, preserved installed images/config,
and restored the original broker service inactive/socket active state. Its
identity-checked cleanup completed; no initiating lab failure was reported.
Available memory stayed around 18 GiB with zero observed memory pressure. This
functional recovery evidence establishes neither sustained audio nor latency.

## Reproduction and reviewer checks

With the matching immutable candidate installed, run each predefined phase:

```sh
sudo -n -k /usr/local/libexec/virtualgamepad-codex-lab status
sudo -n -k /usr/local/libexec/virtualgamepad-codex-lab run rejection
sudo -n -k /usr/local/libexec/virtualgamepad-codex-lab receipt
```

Repeat `run`/`receipt` for the six remaining phase names in the table. Retrieve
receipts after each terminal phase, before starting another phase. Keep raw
receipts outside tracked source. Status alone is not an acceptance receipt.
Do not run a maintenance trial while unexplained clients/resources exist.

Review `scripts/run-alpha-provider-lab.py` and
`scripts/validate-broker-lifecycle-live.py`, especially authentication, pidfd
identity validation, exact pre-handoff replies, refusal of late injection,
independently held journal identities, startup rejection, all-thread child
inspection, sibling progress and restoration. Their deterministic regressions
include changed identities, occupied backups, refusal before arming, combined
errors, missing restoration acknowledgements and per-family sequencing.
All 329 Python tooling tests and the five mandatory repository checks passed
for the implementation increment.

Reopen affected acceptance cells when provider code, worker/broker binary hashes,
protocols, admission, isolation or cleanup behavior changes. Final exact-head CI,
SBOM and release validation remain separate requirements. Gates 1, 2, 4, 5 and 6
remain open; this is not an alpha readiness recommendation.

The preceding pass at `7ed863a219a343b90fe2171f96aaafde17f4a550` remains historical evidence. Its worker SHA-256 was `8d698dd974b117ad677f42762aa3e9566be95440411e0a9a74ff4db34542be0c`; that receipt was not reused for the changed worker.
