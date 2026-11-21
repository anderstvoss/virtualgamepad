# Device data: synthetic defaults and caller-provided overrides

This audit covers controller-reported identity, device-data replies and their exposed realization metadata. Identity is per instance unless restored explicitly; fixed calibration and firmware responses may be synthetic constants. Runtime imports of private data must remain outside both public repositories.

## Audited emission matrix

| Family | Field and realization | Emission | Default / per-instance | Caller override | File import |
| --- | --- | --- | --- | --- | --- |

## Conformance and follow-up

Tests run against a deterministic provider, without a physical controller or host. Unimplemented family overrides are tracked by the P10b/P10c gates.
