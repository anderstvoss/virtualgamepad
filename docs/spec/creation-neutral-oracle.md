# Creation-associated first-valid-neutral oracle (#166)

Status: deterministic core, **not a host-qualified consumer measurement**. This
note extends [the C8 runner disposition](acceptance-runner-consumer-gaps.md)
without amending any run previously published for #166. The owner decision on
2026-11-11 is that this does **not** gate Alpha 2 (#365); a later release policy
still requires owner review.

## Measurement contract and timeline

A sample belongs to **one creation**, one declared controller family, and one
selected realization. Every timestamp is monotonic nanoseconds from the same
runner process; wall-clock UTC is provenance only. Do not mix clock domains.

| Stage | Meaning / authority | Common missing condition |
| --- | --- | --- |
| `api_create` | Immediately before issuing the creation call (existing runner: `api_call`) | Creation was not attempted |
| `provider_ready` | Selected provider's preparation/open completed (existing `api_returned` is the current approximation, not a kernel or consumer observation) | Provider error |
| `kernel_node_present` | `/sys/class/input/event*` or `/sys/class/hidraw/hidraw*` exists **and** resolves to this creation's requested physical path | No node, unmapped companion, missing sysfs identity |
| `consumer_discovered` | The selected consumer actually enumerated the **owned node** | SDL probe provides no discovery event |
| `consumer_opened` | Exact owned event/hidraw node was opened for reading | `EACCES`/`EPERM` retained as failures, not neutral or skipped |
| `first_report` | Earliest report / completed evdev state from the opened **owned** device, not a submitted controller state | Read/decoder unavailable |
| `first_valid_neutral` | Earliest consumer-observed, decodable input whose controls are neutral against its family reference | Initial non-neutral; no decoded report; no permitted open |
| `sdl_probe_completed` | External SDL probe process finished; **not** discovery or first-neutral time | Probe not requested/failed |
| `removal` | Application begins closing that exact controller (`close_call`) | Close not attempted |
| `node_gone` | All associated kernel nodes have disappeared | Timeout, leaked node |

`consumer_discovered` and `sdl_probe_completed` are independent, explicitly
nullable timestamps. Completion can bound discovery but cannot replace it.
Never synthesize `first_valid_neutral` from `commit()`, `api_returned`, a
readable fd, a controller name, `mapping_ready`, or SDL completion. For evdev,
assemble complete key/ABS/MT state through `SYN_REPORT` before testing neutral;
a single delta with no keys pressed is not proof of neutral. For HID, decode
its controller-native input fields and distinguish volatile counters, battery,
revision, timestamp, report IDs, and reply/status reports from controls.

## Exact owned device, including concurrent creations

The runner already retains each creation's
`controller.association().components().requested_physical_path()` and matches
it against `/sys/class/{input,hidraw}/.../device/phys` or `HID_PHYS` in
`uevent` (`xtask/src/linux.rs::matching_nodes`). It records the corresponding
`/dev/input/eventN` or `/dev/hidrawN` node and driver. Those are **starting
observations**, not permission to accept a device with the same display name.

The oracle's `DeviceIdentity` binding consists of:

- a unique runner **creation ID** (distinct even when application controller IDs
  are reused), attached **to every observed event including events before node
  association**; exact selected **family ID** and **realization ID**, and requested physical path;
- resolved **sysfs device** path and current **kernel node** path, associated by
  the runner **while the creation is alive**; for multi-node/companion devices,
  establish a separate bound component identity per stream;
- the kernel identity verified again at consumer-open, and after node churn,
  to prevent stale path or recycled `eventN`/`hidrawN` attribution.

The `DeviceIdentity` proof must be constructed from the actual creation's
association, sysfs identity and node. No searching by VID/PID, name, order,
`eventN` alone, or reported bus type. The pure core ignores observations
labelled with another creation ID (even SDL-completion markers lacking a node), but reports `WrongDevice` if an observation
**claims the owned creation ID** with a conflicting realization, physical path,
resolved sysfs device, or node. Missing bindings remain `Unavailable`, never a
successful neutral measurement. The existing `matching_nodes` function alone
is not the final proof: the host adapter must record and revalidate canonical
sysfs identity at open. There is no device capture adapter in this PR.

## Neutral sources and interpretation

`xtask/src/neutral_oracle.rs` contains a seven-family matcher table. Its native
reference states use the controller-native `Default` that the existing
`*Definition::neutral_state()` encoder path uses. These typed values represent
**decoded, normalized consumer input**, not an application snapshot or outgoing
API commit. Full-state equality is suitable for deterministic canonical
fixtures; a host decoder must normalize or exclude changing non-input metadata
before constructing this representation. In particular, do not conclude that
a legitimate neutral packet with an advancing timestamp is non-neutral.

| Family | Reference in checked-in source | Additional checked-in neutral wire fixture |
| --- | --- | --- |
| DualSense | `DualSenseDefinition::neutral_state` (session default), `DualSenseInputs::default` | `tests/fixtures/protocol-research/ds-neutral.hex` (0x01) |
| DualShock 4 | `DualShock4Definition::neutral_state`, `DualShock4State::default`, `ds4_evdev_frame` / `ds4_frame` | No dedicated neutral hex; `ds4-active-layout.hex` is **not** neutral |
| Switch Pro | `SwitchProDefinition::neutral_state`, `SwitchProState::default`, `switch_pro.rs` encoding | No neutral hex |
| Xbox 360 | `Xbox360Definition::neutral_state`, `Xbox360State::default`, `xbox_hid_frame` / evdev encoder | No neutral hex |
| Wii Remote | `WiiRemoteDefinition::neutral_state`, `WiiRemoteState::default` (Original revision) | `wii-c5-neutral-{30..37,3d..3f}.hex` (exact canonical fixture comparison; matching a report ID to the active Wii mode still requires the host adapter) |
| Steam Controller 2026 | `SteamController2026Definition::neutral_state`, `SteamController2026State::default` | `steam-c6-neutral-42.hex`, `-45.hex`, `-47.hex` |
| Xbox Series | `XboxSeriesDefinition::neutral_state`, `XboxSeriesState::default`, `xbox_series/definition.rs` | `xbox-series-xpad-evdev-golden.json` supplies capability layout **but no neutral frames**, so it cannot by itself prove neutral |

The raw-HID fixture matcher is **exact canonical fixture equivalence**. It is
not a replacement for a semantic HID decoder: changing sequence, sensor time,
status metadata, optional input layout, or legitimate neutral variants must
not be mistaken for button activity. An unsupported report ID/representation
results in `Unavailable`, not `NonNeutralFirst`. The Wii revision/expansion,
Steam format and Sony USB/Bluetooth distinctions require the live adapter to
select the correct reference. For evdev the adapter supplies a full neutral
snapshot **generated from that realization's encoder**; no speculative
hard-coded axis centers or ignored held keys. The deterministic core can compare
those snapshots, but does not itself build a decoder or read an fd.

## Outcome and first-report semantics

`Outcome` is one of `ValidNeutral`, `NonNeutralFirst`,
`PermissionDenied { error }`, `WrongDevice`, `Timeout`,
`Unavailable { reason }`, and `ConsumerOpenError { error }`. The permission
error carries the exact open-error text. The first *decodable* owned input
is evaluated for neutral; an initial active report produces
`NonNeutralFirst` even if a later neutral is also timestamped. The timeline
retains **both** `first_report` and `first_valid_neutral` so reviewers can
recover the delay and distinguish malformed/unsupported packets. Wrong-device
claims are errors, while interleaved reports from another known creation are
ignored and counted. A deadline with no decodable owned neutral yields
`Timeout`; an unavailable decoder, incomplete identity, or missing permission
is not silently converted to timeout/success. Outcome priority is permission
failure, identity mismatch, other open failure, first non-neutral, valid
neutral, explicit unavailable, then timeout. The host must never mask a real
`EACCES` by retrying until deadline and reporting a neutral success.

## New evidence format; do not rewrite historical observations

Keep existing `acceptance/schema/run-v1.schema.json` records, their stage
names, source SHA, host labels, sample counts, and missing/error cells **as
published**. A new future observation is a separate run or sidecar with a new
schema/version and a link to the exact original `run-v1` sample only when it
truly came from that same creation. A new run is **not** a retrofit of old
samples. Suggested versioned sidecar:

```json
{
  "kind": "creation_neutral_oracle",
  "schema_version": 1,
  "provenance": {
    "virtualgamepad_sha": "<actual run SHA>",
    "runner_revision": "<actual run SHA>",
    "host": "<actual host label, kernel, CPU and permissions>",
    "sdl_version_backend_hints": "<actual probe configuration or unavailable>",
    "fixture_or_decoder_revision": "<actual source and checksum>"
  },
  "creation": {
    "id": "<unique run-local creation ID>",
    "family": "virtualgamepad.dualsense",
    "realization": "linux.uinput",
    "requested_physical_path": "<actual path>",
    "resolved_sysfs_device": "<actual canonical path>",
    "kernel_node": "<actual event or hidraw path>"
  },
  "observation": {
    "outcome": "<actual outcome enum>",
    "error": null,
    "neutral_source": "<encoder or fixture reference>",
    "timestamps_ns": {
      "api_create": null,
      "provider_ready": null,
      "kernel_node_present": null,
      "consumer_discovered": null,
      "consumer_opened": null,
      "first_report": null,
      "first_valid_neutral": null,
      "sdl_probe_completed": null,
      "removal": null,
      "node_gone": null
    },
    "ignored_foreign_events": 0
  },
  "historical_run_link": null
}
```

All placeholder values above are **a proposed record shape, not measured
observations**. Store `null` plus explicit reason for every unobserved stage.
Record monotonic clock origin, source/decoder revision, poll interval and
consumer scope, and keep raw byte dumps only in authorized local evidence.
Existing timing percentile output is not a correctness verdict or a latency
budget. No physical-report cadence or USB/radio-fidelity claim follows.

## Kalux run: host-only follow-up

After a *separate* reviewed Linux consumer-capture adapter is wired to the
runner (this PR adds only a deterministic core), run exactly the following
from a tracked-clean checkout on `kalux` using its ordinary-user uinput/UHID
permissions, recording every command's exit status:

1. Record `git rev-parse HEAD`, `git status --porcelain`, `uname -a`, CPU
   model/count, `rustc --version`, `cargo --version`, `pkg-config --modversion
   sdl3`, `sha256sum scripts/sdl3-gamepad-probe.c`, the research pin, SDL
   hints/backend, `id`, and exact consumer-node ACLs. Record `udev`/kernel
   versions and whether `/dev/uinput`, `/dev/uhid`, and selected event/hidraw
   nodes are accessible; do not change host policy without approval.
2. Run the same four lifecycle configurations for each **declared** realization
   the host can create (start with DualSense `linux.uinput`, Xbox 360
   `linux.uinput`, Wii `linux.uinput`/`linux.uhid.bluetooth`, and Steam
   Controller 2026 `linux.uinput`/`linux.uhid.usb`; add DS4, Switch Pro,
   Xbox Series as supported), using the existing CLI:

   ```bash
   cargo run --locked -p xtask -- acceptance \
     --controller virtualgamepad.dualsense --realization linux.uinput \
     --cycles 10 --concurrency 1 --out /tmp/dualsense-n1.json
   cargo run --locked -p xtask -- acceptance \
     --controller virtualgamepad.dualsense --realization linux.uinput \
     --cycles 10 --concurrency 4 --out /tmp/dualsense-n4.json
   ```

   Repeat for each declared pair. These commands **currently only collect
   existing lifecycle stages**: the host adapter must first arrange for
   owned-device reads, decoding and the new versioned sidecars; passing these
   commands alone does **not** fill `first_valid_neutral`.
3. Capture each read-open result and exact `EACCES`/`EPERM` message; identify
   physical-path-to-sysfs-to-node association before each open. Capture both
   first raw report and first decoded neutral (or explicit failure), while N=4
   interleaves other creations. If SDL is requested, run `--sdl` separately;
   distinguish its actual discovery event, if any, from wrapper completion.
4. Record warm/cold samples, N, cycles, consumer/revision and decoder hashes,
   error/timeout/unavailable cells, removal/node-gone, resource before/after
   and all raw report digests. Preserve the historical research #10, #17 and
   C6 D9 records unchanged. Publish new run/sidecar separately and request
   owner review before any timing threshold or release-gating decision.

No host run, kernel read, SDL probe, permission change or timing claim is made
by this PR.
