# EXP-0007 — Baseline/rewrite Linux bus startup comparison

Owner: Codex. Baseline: `9b466e0`. Rewrite: `1525534` plus the test-only
`dualsense::host_probe` module committed with this record. Kernel:
Linux arm64 `6.12.105+deb13-arm64`. No device-driver override was applied.
The DualSense descriptor source is unchanged between these revisions.

## Controlled procedure

The same test source was compiled against both revisions. The baseline was
extracted with `git archive` into a private user-owned directory, with only the
probe file and a cfg(test) module declaration added. No baseline production code,
Git branch, kernel, permissions, services or consumer settings were changed.
The probe source SHA-256 is
`746d04cd9de60d2978790df7181fd48c1d11d21d0638971f58ff9de617ac9ad8`.

Each revision ran three sequential repetitions of USB (`0x03`), then virtual
(`0x06`) bus metadata. Each condition ran for five seconds, with initial neutral
commit followed by the same changing motion script, commit and output servicing
at a requested four-millisecond caller cadence. Scheduling jitter and the rewrite's
autonomous report scheduling were not measured or equated to actual wire cadence.
Within each revision only bus metadata changed. IDs, descriptor, feature logic
and the caller script were held fixed. Process-owned identity prefixes prevented
selection of physical siblings; each run verified its node disappeared before the
next condition. The baseline/rewrite transport suffix implementations differ,
as do the protocols/runtimes under comparison; exact byte equivalence is not claimed.

## Observations

| Revision | Bus | Repetitions | Driver | Input / hidraw children | Service error | Removal |
| --- | --- | --- | --- | --- | --- | --- |
| baseline | USB | 3/3 | playstation | present / present | none | observed |
| baseline | virtual | 3/3 | hid-generic | present / present | none | observed |
| rewrite | USB | 3/3 | playstation | present / present | none | observed |
| rewrite | virtual | 3/3 | hid-generic | present / present | none | observed |

This apparatus did not reproduce a basic Linux enumeration failure in either
revision. It supports retaining USB as the compiled presentation for the current
DualSense protocol, without proving specialized binding is required by consumers.
A virtual sysfs topology alone still says nothing about the configured bus field.

## Remaining evidence and reproduction

Run the checked-in probe on a prepared, isolated host as the ordinary user:

```bash
cargo test -p gr-curated-controllers controlled_bus_startup_probe -- --ignored --nocapture
```

For baseline reproduction, extract the pinned revision, copy the exact
`src/dualsense/host_probe.rs` into its curated-controller package and append a
Linux cfg(test) module declaration with `#[path = "dualsense/host_probe.rs"]` to
`dualsense.rs`; run the same command. Preserve source hash and every run result.
The test is instrumentation only, not a public BUS_VIRTUAL realization.

Changing motion was submitted, not independently observed from evdev/hidraw.
SDL sensors/HIDAPI, Steam gyro/output, physical comparison and dummy_hcd reference
conditions remain untested. Gate P additionally needs separately controlled
binding and consumer evidence; changing the bus and thereby its driver together
cannot establish causality. B/P remain incomplete. No full compatibility or
realization-support promotion follows from the successful measurement runs.
