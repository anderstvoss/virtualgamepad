# Exact-candidate alpha acceptance lab

This is an execution handoff, not a passing acceptance receipt. Run only reviewed
candidate code. Keep recordings, logs, build receipts and local policy outside the
checkout. Do not weaken marker, reply, ownership or cleanup assertions.

## Candidate and native audio

In a clean Git checkout of the full PR head, choose a new external output directory:

```bash
python3 scripts/run-alpha-acceptance.py --prepare-only --report-dir "$BUILD_RECEIPTS"
python3 scripts/run-alpha-acceptance.py --native --report-dir "$NATIVE_RECEIPTS"
```

Required tools: the repository Rust toolchain, C compiler, PipeWire development
headers/tools, WirePlumber policy profile and systemd virtualization detection.
Preparation builds all workspace targets/features before measurement, compiles
the independent C control with warnings denied and runs its marker self-test.
The receipt records full revision/tree, lockfile hash, actual compiler, Cargo flags
and hashes of selected artifacts. `candidate-source.tar` is a portable source
bundle; it does not replace a Git checkout for the driver. Do not reuse VM binaries
on a different architecture. Rebuild from the same SHA and preserve both receipts.

The native run creates a fresh private PipeWire graph per trial. It requires three
60-second independent C controls and three graph-driven Rust controls before
running the 72 family/direction/duplex cells and three slow-consumer family tests.
The C control counts generation separately from successfully queued graph buffers
and received markers, excludes two seconds of warm-up, permits bounded drain, and
verifies the negotiated S16LE/48kHz/stereo format. It makes no latency claim.
Its stream API does not expose a graph xrun total: `graph_xruns: null` is explicitly
unavailable, not zero. Obtain xrun and driver scheduling diagnostics from the Rust
graph control and separately identified graph profiling/environment receipts.
The Rust/product checks retain the existing application-to-application p99 <20ms
threshold. Physical device latency requires a separate timestamped comparison.

Failed controls block the product matrix and remain in `acceptance.json`. Do not
run compiler jobs, profiling or the GUI soak alongside measured controls. An
unqualified VM can remain explicitly unsupported for sustained acceptance if the
exact candidate passes on a qualified native Linux host. This does not establish
the cause of VM loss or prove the product free of independent defects.

## Administrator maintenance window

`python3 scripts/run-alpha-provider-lab.py` prints the proposed phases without
changing the host. Applying the lab requires administrator credentials, a reviewed
root-owned copy of the runner beneath trusted root-owned directories, and approved
binary hashes from the clean build receipt. Do not install or enable a replacement
production service. Do not invoke Cargo as root.

The administrator must provide existing, distinct non-root client, worker and
unauthorized identities with non-root primary groups. Verify an unused high-speed
VHCI port and inspect the original broker's ownership journals. Prepare missing
modules/devices separately using the existing reviewed provisioning tools; only
grant temporary access after matching kernel registration. Record the prior state
and restore only resources created or changed by that preparation. Do not unload
modules now in use, unbind occupied UDCs, detach another attachment or restart
shared PipeWire services. ConfigFS/UDC preparation is unnecessary for rejecting
the currently unavailable dummy_hcd realization.

Using administrator-selected values, invoke the trusted runner:

```bash
sudo python3 -I "$TRUSTED_LAB_RUNNER" --apply \
  --broker "$CANDIDATE_BROKER" --broker-hash "$APPROVED_BROKER_SHA256" \
  --worker "$CANDIDATE_WORKER" --worker-hash "$APPROVED_WORKER_SHA256" \
  --revision "$CANDIDATE_SHA" --client-uid "$CLIENT_UID" \
  --worker-uid "$WORKER_UID" --unauthorized-uid "$UNAUTHORIZED_UID" \
  --port "$FREE_VHCI_PORT" --report "$ROOT_OWNED_NEW_RECEIPT" \
  -- "$ABSOLUTE_UNPRIVILEGED_VALIDATOR" "$VALIDATOR_ARGUMENT"
```

The command is executed as the client identity with supplementary groups cleared,
NoNewPrivileges and a bounded systemd unit. The runner stops only the verified idle
original broker/socket, stages hash-verified candidate images and a separate policy,
and creates temporary socket-activated units. Clients see the private candidate
socket through a mount mapping. The worker binary has a private read-only mapping;
the real global ownership lock and separate candidate journals remain intact.

Preflight rejects clients, children, nonempty journals, occupied ports and unknown
listener/backlog ownership. The original PID is re-read after socket quiescence.
Restoration attempts run after setup/test failure, timeout and interruption, and
retain initiating and cleanup errors. Changed path identities, running candidate
units, remaining attachments or nonempty journals prevent deletion. Failed cleanup
must be investigated; rerunning is not permission to erase retained evidence.

The unauthorized identity is a prerequisite for a separately run peer-denial test;
the runner does not claim that merely supplying its UID executes that test.
Likewise, a successful arbitrary validator is not the complete security matrix.

## Required remaining live matrix

Record one receipt per scenario with exact binaries, startup/drain accounting,
expected result, actual result and owned-resource cleanup:

- Broker: authorized and unauthorized peers; malformed/truncated/oversized frames;
  unexpected descriptors and FD accounting; admission limits; client/worker death;
  restart and stale journals; construction/cleanup failure; sibling isolation.
- USB/IP audio: all three families, sample/native directions and supported duplex
  ownership; ALSA ownership/routing, reconnect, slow clients, required failures,
  HID/audio isolation and teardown. Existing production/USB worker validators are
  deterministic prerequisites, not substitutes for installed acceptance.
- Controllers: all four HID and evdev families, outputs and every declared report
  type. For DS4 verify separate associated gamepad/contact nodes, two contacts and
  release, feedback, partial second-node rollback and sibling removal.
- Unavailable dummy_hcd: preflight/direct-open/broker rejection before construction,
  stable technical reason, no fallback and no resource growth after repetition.
- Physical: controlled USB passthrough of all four reference families. Record model,
  firmware, transport and accessories; use the reviewed SDL probe build. Compare
  controls, representable outputs, touch/motion and applicable audio without claiming
  descriptor or physical equivalence from functional emulation.
- GUI: disposable desktop with only test-owned inputs for touch; keyboard access,
  off-viewport selection, lockout, discovery retry/loading/error, routing, displays
  and shutdown. Use a separate Steam profile for recognition/input/output, without
  extrapolating to unspecified games.

Run the two-hour neutral GUI soak separately from audio timing:

```bash
timeout --signal=TERM --kill-after=5 7260 "$EXACT_GUI_SOAK_BINARY" 7200
```

Record RSS, FD/thread/child counts, remove/recreate cycles and final device cleanup.
The previous two-hour soak is historical evidence and does not certify new code.
An unavailable isolated desktop, reference device, administrator credential or
external native receipt is a blocked cell, never a pass.

## Return package and decision

Return build/environment receipts, raw logs, the acceptance matrix, service-state
restoration proof, sanitized physical observations and native timing methodology.
Include kernel, architecture, virtualization, PipeWire/SDL versions, actual graph
format/quantum, scheduling and xrun diagnostics. Supplemental profiler/scheduling
runs must be identified separately from quiet acceptance trials.

The reviewer must independently resolve PR head/base/tree and require matching
receipt revisions. A source change invalidates affected acceptance; later docs-only
changes require explicit source-tree equivalence and exact-head CI/consumer checks.
Keep PR #133 draft until required acceptance is complete. No merge, tag or release
publication is authorized by this procedure.
