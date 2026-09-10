# EXP-0006 — Production DualSense USB/UHID startup

Owner: Codex. Revision: commit containing this record, based on `d0fe910`.
Kernel: Linux arm64 `6.12.105+deb13-arm64`; driver: `playstation`.
Corpus: unchanged published pin `9d0d56e`. No physical capture used.

## Scope and apparatus

The ignored `dualsense_uhid_live` integration test opens the actual curated
DualSense USB personality, services it every millisecond for five seconds, and
checks its process-owned HID sysfs node for playstation binding and input children.
It explicitly closes twice and waits up to two seconds for that node to disappear.
No global driver rebinding, consumer settings, ACLs, modules or services were
changed for these runs. Existing ordinary-user creation access was sufficient.
The test remains ignored in ordinary builds and needs an isolated prepared host.

Transport identity now uses process ID and an atomic creation ordinal rather
than caller session ID alone. Reusing an application session ID no longer aliases
UHID phys/uniq fields within a process; concurrent processes in the same PID
namespace also differ. This is ephemeral transport identity, not a physical serial
or a guarantee across PID namespaces. Controller feature-reported addresses retain
their existing controller-owned behavior.

## Results, including failed instrumentation

Three initial development probes failed identification assertions. Direct inspection
of the owned test node showed successful playstation binding and input children:
the newly introduced verbose transport identity exceeded UHID's 64-byte field,
so the provider truncated the suffix and the test selector missed it. The failures
are instrumentation/identity failures, not evidence of protocol rejection. Cleanup
assertions in those failed probes were insufficient because they used that same
selector; no cleanup pass is attributed to them.

The corrected compact identity has deterministic maximum-length coverage. The
provider now rejects oversized or embedded-NUL identity fields before submitting
a create event. Synthetic selector tests reject siblings and truncated suffixes.
After one exploratory corrected pass, a declared three-run sequence passed 3/3:
production startup, final driver/input presence, five-second service, and explicit
idempotent close with observed node removal. Required workspace checks pass.

## Limits and next work

This is one Linux USB/UHID configuration. No baseline, BUS_VIRTUAL, unbound-driver,
SDL, Steam or physical-reference comparison was run. Input child enumeration does
not establish changing sensor values or consumer-observed input/output fidelity.
B/P remain blocked on those controlled comparisons, not on creation-device access.
G's control metadata/completion decision is independent. No realization metadata
is promoted to HostValidated or PhysicallyValidated by this experiment.
