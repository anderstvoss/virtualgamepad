# Acceptance timing runs

Use the local Linux host with permission for `/dev/uinput` and `/dev/uhid`, the
usual workspace native libraries and optional SDL3 development files. This
runner installs nothing and never chooses another realization on failure.

```
cargo xtask acceptance --controller virtualgamepad.dualsense --realization linux.uinput --cycles 50 --concurrency 1 --out run.json
cargo xtask acceptance --controller virtualgamepad.dualsense --realization linux.uinput --cycles 50 --concurrency 4 --sdl --out concurrent.json
cargo xtask acceptance-diff run.json reference.json
```

Repeat for Xbox360 uinput and DualSense UHID USB. In a fresh process the first
cycle is cold, subsequent cycles warm; N sessions coexist before removal.
`--backend fake` produces explicitly synthetic observations for CI/development.
`--reference file` embeds a reference comparison without assigning a verdict.
`acceptance-diff` compares timing records side by side, or invokes the existing
SDL comparator for SDL observations. No universal timing threshold is set.

All timestamps are nanoseconds on one process monotonic clock. `api_returned`
is preparation/open complete, not consumer discovery. Node stages poll at 1 ms
with a two-second bound, matching creation-associated physical paths. Consumer
open is an actual nonblocking evdev/hidraw open. The driver comes from the owned
node's sysfs ancestry; missing drivers carry a reason. SDL is optional and its
observation is retained; its stage measures probe completion as an upper bound.
Earlier sessions continue servicing during a later session's observation waits.

The input script presses/releases declared face buttons, returning neutral;
other native functions require their per-function acceptance cases. First valid
neutral report is explicitly unmeasured until a family-specific consumer oracle
exists; commit success is not report delivery. These are honest missing cells,
not a promoted acceptance result. Identity reuse after removal is also not
inferred from fresh random identities. All sessions close twice to exercise
idempotence and then drop. Failed opens and missing disappearance remain raw
samples. Resource counts before/after cover fds, threads and class nodes; unrelated
host activity can change these counts, so inspect deltas before calling them leaks.

JSON schema: `acceptance/schema/run-v1.schema.json`. The header records exact
workspace SHA/dirty flag, research revision, kernel/CPU/toolchain and available SDL
metadata. Probe source SHA is recorded; binary SHA and unobserved hints/backend
remain explicitly unmeasured. Do not replace missing values with guessed data.
Summaries group cold/warm per run (one family/realization/N), use nearest-rank
p50/p90/p99, population standard deviation and median absolute deviation. Summary
durations are relative to api_call, or close_call for removal. Failed/unobserved
stages stay missing; raw samples remain authoritative.

Review/sanitize local output before filing a research experiment: no real device
serials, raw captures or private host data. Virtual-host runs cannot be filed as
physical evidence. The current research schema lacks a dedicated virtual_host_run
kind; a reviewed research-format change is needed before publication in that
format. Keep unfiled output local and identify this limitation in #166.
