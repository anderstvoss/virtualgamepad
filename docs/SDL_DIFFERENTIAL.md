# SDL differential observations

Use the same pinned SDL build and probe for reference and virtual observations.
Choose the exact SDL device path, the same duration and the same existing script
mode/control case. `--reopen` optionally adds a close/open/100ms service/close
sequence. This closes consumer handles only; it never removes physical devices.

```sh
scripts/run-sdl3-gamepad-probe.sh "$REFERENCE_PATH" 1000 --reopen > reference.jsonl
scripts/run-sdl3-gamepad-probe.sh "$VIRTUAL_PATH" 1000 --reopen > virtual.jsonl
python3 scripts/compare-sdl3-observations.py reference.jsonl virtual.jsonl
```

No new dependencies: existing C compiler/SDL development build and Python standard
library. Use caller-provided build environment paths, never committed local paths.
The baseline remains SDL 3.2.0 revision
`535d80badefc83c5c527ec5748f2a20d6a9310fe`. Newer decoder research is separate.

## Contract version 2

Each terminal record retains legacy identity, mapping, script, counts, masks and
sensor fields. `schema_version` is 2, including mapping_ready records. Current
Rust mapping orchestration selects readiness by record_type and success by process
status, so no version-dependent reader needs migration. External consumers must
accept v2 before assuming v1 fields alone describe the observation.

The `observations` object always includes these dimensions:

- identity and build (SDL runtime version/revision);
- backend, requested HIDAPI hint, mapping string and mapping source;
- capabilities (named buttons/axes, touchpad count, rumble/RGB LED);
- controls (down/up/axis masks and final state), sensor summaries and touch edges;
- output-call results and separately recorded controller reverse evidence;
- opened, closed, reopened, reclosed and device_removed.

Each is `{ "value": ..., "reason": null }` when measured, or
`{ "value": null, "reason": "explanation" }` when unmeasured. Backend identity,
mapping source, controller reverse evidence and provider removal are currently
explicitly unmeasured by this standalone probe. Requested hints are not evidence
of the chosen backend. Consumer close is not device removal. Reopen is not measured
unless requested and attempted. Physical/reference role is assigned by the caller
and must be retained in the surrounding experiment provenance.

Sensor summaries compare availability, enablement, change and validity; raw sample
counts remain diagnostic rather than exact timing equivalence. Touch masks record
first-pad contact down/up and motion; out-of-range contacts produce an explicit
missing measurement. Control masks support up to 32 SDL buttons; larger SDL builds
are rejected rather than silently truncating observations.

## Comparison and expected limitations

The comparator accepts one terminal record as JSON or JSONL with mapping_ready
records. It reads v1 as partial evidence and never trusts v1 consumer_closed.
It reports match, expected realization limitation, unexpected difference, and not
measured per field. Missing evidence cannot become agreement. Incompatible script
profiles/control cases suppress behavioral comparison; different builds remain a
visible difference. Preserve the exact build/backend/scenario in experiment records.

An optional `--limitations limitations.json` file contains exact exceptions:

```json
[
  {
    "field": "capabilities.rgb_led",
    "reference": true,
    "virtual": false,
    "reason": "The selected target does not present an RGB LED output",
    "evidence": "controller-specific experiment and restriction locator"
  }
]
```

A rule matches only those exact values. It cannot suppress missing data or failed
captures. Exit 0 means no unexpected differences, **not** complete compatibility;
exit 1 means unexpected differences and exit 2 invalid input. Always inspect the
structured counts and differences. Source agreement and successful output API
calls never substitute for physical capture or physical effects.

## Attaching independently measured harness evidence

The existing live controller harness already records reverse events and verifies
owned-node cleanup. To attach those observations, preserve the exact single-capture
file and create a separate JSON evidence record after verifying the session/device
association. Use `--reference-evidence` or `--virtual-evidence`:

```json
{
  "schema_version": 1,
  "capture_sha256": "SHA256 of the exact capture file bytes",
  "evidence": "controlled experiment locator for this exact session",
  "observations": {
    "controller_reverse": { "rumble_seen": true },
    "device_removed": true
  }
}
```

Only missing backend, mapping_source, controller_reverse and device_removed may
be attached. Hash mismatch and overwriting existing measurements are rejected.
The evidence locator is retained in comparison output. Do not attach a historical
cleanup record merely because its controller name matches. A physical controller
remaining connected is not a failed consumer close; record its removal only when
that scenario actually measured removal.
