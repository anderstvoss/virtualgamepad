# Scoped host preparation and rollback

The user authorized this VM for validation provided other research is preserved.
No persistent enrollment, service enablement, driver rebinding, reboot, shared
service restart, or kernel replacement is part of this procedure. Host changes
requiring administration remain blocked because noninteractive sudo is unavailable.
Machine-specific inventory and rollback records are private, outside Git.

## First: read-only inventory

Run `python3 scripts/host-preflight.py all`. The command never opens device nodes,
connects to sockets, loads modules, changes ACLs, or creates resources. Review
existing ACLs (`getfacl`), matching kernel module vermagic (`modinfo -k`), loaded
modules, mounts, service state, UDC bindings, consumer settings, and reference
hardware ownership. A node existing is not proof that its kernel mechanism is
registered. Do not grant access to an unverified node.

Current survey: uinput is registered and accessible through an existing user ACL;
UHID has a root-only node but no visible misc registration. The installed UHID
module vermagic matches the running kernel. ConfigFS is mounted, but its gadget
subtree and dummy UDCs are absent. The broker units are inactive, and its config
and socket are absent. Preserve the existing ConfigFS mount and uinput ACL.

## Temporary UHID experiment

An administrator first records whether uhid is already loaded, then loads it only
if missing. Match the node's major/minor to `/sys/class/misc/uhid/dev` before
changing its ACL. Capture `getfacl -p /dev/uhid` after node registration, then grant
only the test identity read/write access with `setfacl -m u:<test-uid>:rw /dev/uhid`.
Use the real test UID, not root's UID from a sudo shell. No chmod 666 or input-group
enrollment is permitted.

Build the integration test as the ordinary user. Run the process-owned UHID test
and then the controlled B/P cases as that user. Check consumer ACLs separately;
only grant access to session-specific hidraw/event nodes after verifying their
sysfs identity. Do not use a wildcard node rule. Destroy controllers and close
all test processes before restoring the captured ACL with `setfacl --restore`.
If a node was replaced, revalidate identity before restoration; never restore a
saved ACL to an unrelated reused path. Removing an ACL does not revoke existing
open descriptors, which is why process shutdown precedes restoration.

Unload only a module loaded by this run, and only if unused and not adopted by
another workload. Otherwise leave it loaded and report that retained state.
Missing administrator access blocks this live step, not deterministic development.

## Optional gadget experiment

Prepare only missing libcomposite/HID-gadget/dummy_hcd mechanisms; built-in support
does not need modprobe. Reuse ConfigFS. Reserve one observed-unused dummy UDC with
other research users before starting; do not unbind occupied devices or change
module capacity parameters while shared work is running.

Install the reviewed broker binary and root-owned config containing `allow_uid`,
`instance`, and `allow_udc`. Install/apply the optional tmpfiles configuration for
its state directories. Start the socket for this experiment without enabling it
at boot. Record previous unit/config state and restore it afterward. Do not
replace an existing active broker or delete another instance's state.

The broker serializes cooperating instances through an exclusive lock. It only
allocates administrator-authorized UDCs and only recovers instance-journaled
resources with matching filesystem identity and UDC binding. It does not reserve
resources against unrelated software that ignores the lock: the experiment
requires exclusive use of the agreed UDC. Failed cleanup retains the journal;
ambiguous ownership stops recovery and requires operator inspection.

Compile ignored low-level tests without elevation. Run only the specific reviewed
test binary/case in an administrator-created transient service with empty
`CapabilityBoundingSet`, empty `AmbientCapabilities`, and
`ProtectKernelModules=true`, using the reserved test profile. Never run Cargo or
the entire workspace test suite as root. Validate creation, required startup
exchange, input/output, client death, restart, and cleanup. Also test the actual
socket-activated broker with `virtualgamepad-broker-no-capabilities.conf` as a
temporary drop-in. If an operation fails, record its syscall/path/error; do not
add capabilities automatically.

CAP_SYS_MODULE and ambient capabilities have been removed from the runtime unit.
CAP_SYS_ADMIN remains explicitly unproven pending this experiment; the empty-set
drop-in is not a validated deployment default. Gate G also needs a kernel/profile
answer for metadata and completion limitations; privilege changes do not solve it.

## Development consumers and later gates

SDL source is pinned to release 3.2.0, commit
`535d80badefc83c5c527ec5748f2a20d6a9310fe`. Development-only CMake 3.31.6 and Ninja
1.11.1.4 are installed in a private virtual environment. A private extraction of
libudev development headers supports the console input probe build; no system
package or loader configuration is changed. Record the build options and scope
PKG_CONFIG_PATH/LD_LIBRARY_PATH to the probe process. This is not a desktop SDL or
Steam acceptance result. The launcher builds in a fresh private directory and
cleans it on success or failure.

A Steam launcher is present, but interactive usability and reference-device
availability remain unverified. Do not launch/reconfigure Steam merely to inventory
it. Run its controlled comparison when a dedicated session and target are available.

Capture is opt-in per experiment: choose a reserved bus/controller, limit duration,
keep raw data private, sanitize fixture derivatives, and never grant persistent
all-bus usbmon access. Audio and Bluetooth preparation waits for F/H/L/M. Corpus
CI credentials are read-only and development-only; ordinary builds need neither
private-repository access nor a Python validator.
