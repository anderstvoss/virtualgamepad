# Alpha 1 virtual host consumer and timing runs

**Proposed; not run.** This is an operator-run Linux harness for the four
Alpha 1 families, not a physical controller comparison. No host runs were
performed in this PR. Neither an SDL GUID nor a kernel driver binding
promotes the support matrix.

## Prerequisites and commands

Run as the ordinary desktop user on a dev-only Linux host with working
/dev/uinput and /dev/uhid, native Cargo build prerequisites, SDL3 development
files (pkg-config package sdl3), and existing session ACLs for each associated
evdev/hidraw node. Prepare these *before* the run using the
[host audit](ALPHA_2_HOST_AUDIT.md) and [SDL guidance](SDL_ACCEPTANCE.md).
The harness does not install packages, load modules, alter udev policy,
or grant device permissions.

From the repository root, run:

    python3 scripts/alpha1-host-matrix.py --family all --realization all --preflight-only
    bash scripts/alpha1-host-run.sh --out alpha1-results

Optional dev-only comparison with the generic HID driver:

    sudo -v
    bash scripts/alpha1-host-run.sh --out alpha1-results-generic --rebind

This needs noninteractive sudo authorization for **per-device sysfs bind/unbind
writes only**. Never run the harness or SDL probes as root. The program checks
ownership against association metadata from its own public-API controller
instance and refuses ambiguous HID parents. It restores the exact previous
driver binding in a finally path, including after generic probe failure.
If a write or restore fails, stop experiments and have the operator examine
that one verified virtual HID object before repeating the run. The harness
does not unload any driver module or intentionally change a physical device.
A host crash or privilege revocation could prevent restoration; verify the
original binding before leaving the dev session.

Single-condition run:

    python3 scripts/alpha1-host-matrix.py --family switch-pro --realization linux.uhid.usb --out alpha1-switch-results
    python3 scripts/alpha1-host-to-research.py --input alpha1-switch-results --out alpha1-switch-drafts

The script reads declared realizations from acceptance manifests. An
undeclared Bluetooth-bus UHID realization is **skipped**, never synthesized.
If one is added later, its BUS_BLUETOOTH metadata alone does not establish
radio discovery, pairing, or Bluetooth transport fidelity.

## Recorded evidence boundaries

Each JSON condition records exact virtualgamepad checkout and research
submodule-pin SHAs, dirty flag, kernel architecture/release, compiler/SDL
versions, the SDL probe source hash, association-owned hidraw/evdev basenames,
current bound driver, and separate SDL3 HIDAPI-requested and evdev-requested
probe results with actual backend and GUID where available. Backend selection
is observed, never inferred from the requested SDL environment toggle.
On uinput, a HIDAPI-enabled probe may still choose evdev; do not call this
native HIDAPI acceptance.

Switch Pro output callbacks record only outgoing 0x80 USB commands and
0x01 subcommand IDs (0x03, 0x40, 0x48 and 0x10 for an SPI read request).
They **do not** retain SPI contents, calibration, EEPROM content or an
independently observed 0x21 ACK. An outgoing subcommand is not proof
that the receiving consumer saw a reply. This ordering is a follow-up
lead, not a verified contention outcome.

Research surveys also propose Steam Input, Wine/Proton, SDL2, joycond
(Switch Pro), RetroArch and browser checks. None of those applications
is launched or claimed tested by this harness. Its automated consumer
matrix covers kernel driver binding, node association, and SDL3
HIDAPI/evdev only.

The batch runs N=1 and N=4 at 50 cycles for each of the four families
and each of linux.uinput and linux.uhid.usb: **16 timing JSON records**,
all with --sdl. It invokes the existing cargo xtask acceptance path.
Its raw output can contain transient paths and other host metadata;
the Bash script keeps that raw output in a private temporary directory,
sanitizes it to the selected folder, and removes the temporary data
on exit. Timing stage values and summaries are preserved; paths,
raw errors, CPU model and host labels are not. Operator privacy review
is still required before publishing any records.

Layout of the selected folder: matrix, timing, and research-drafts
subdirectories with one JSON file per condition/run. The converter emits
the current research experiment schema fields as kind virtual_host_run,
status blocked, empty sources and fixtures, and an explicit human-review
notice. The operator must reconcile the drafts against retained results
before opening a research PR; this PR commits no research records.

## Expected duration and restoration

The matrix runs two one-second SDL probes per selected condition, plus
compilation, node discovery and cleanup; expect **tens of seconds or more**.
An optional generic-driver condition adds probes where a UHID device
is eligible. The 16 timing batches cover **2,000 controller lifetimes**.
SDL time alone is a lower bound of roughly **3–4 minutes** at the existing
100 ms family profile; compilation, other SDL profiles and up to two-second
waits at failed stages can increase duration substantially.

The first preflight failure stops the batch before any controller creation.
An SDL mismatch is recorded as a failed observation rather than suppressed;
unexpected binding/restore or controller lifecycle failure stops subsequent
runs. Normal close releases the public-API controller and its virtual kernel
nodes. In rebind mode, restore the original HID driver **before** closing
the holder, then verify the original binding and disappearance of owned
nodes after close. Any separately granted temporary ACL must be revoked
by the administrator after the test session.

Pinned research references:
[experiment schema](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/schemas/experiment.schema.json),
[Sony contention](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/docs/sony-consumer-contention.md),
[Switch Pro contention](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/docs/switch-pro-consumer-contention.md),
[Xbox 360 contention](https://github.com/anderstvoss/controller-protocol-research/blob/77b56f69631f1a0da39bb0e3e5ecbbf8e83329d8/docs/xbox360-consumer-contention.md).
