# UHID access after reboot: research review and resolution plan

**Current status: complete.** After PR108, the maintainer confirmed that post-reboot
UHID creation and cleanup work with the installed boot policy. This removes the
reboot issue from active blockers. That confirmation is separate from the original
agent observations below; this update did not perform another reboot.
See the [active pre-alpha ledger](PRE_ALPHA_STATUS.md).

## Historical diagnosis before maintainer reboot acceptance


Status: GUI recovery confirmed by the user; persistent boot configuration
installed and verified. Reboot acceptance remains pending.
Reviewed against `e0fb72196bf8d45c23b9b11a7dc5113125f06099` (post-107 main).
This document records the initial review, subsequent live diagnosis and remaining
boot acceptance. It does not claim a completed reboot fix.

## Findings

The research correctly separates persistent host authorization from controller
protocol resource persistence. Its initial temporary-ACL hypothesis was not needed
to explain the reproduced GUI failure: loading the missing UHID module alone
restored access through existing host policy. Reboot acceptance is still pending.

| Evidence | What it establishes | What it does not establish |
| --- | --- | --- |
| `scripts/virtualgamepad-host-helper.py`: `Host.prepare`, `Journal`, and `execute` | Grant prepares the selected device, settles udev, records identity and ACL, and grants a temporary lease under `/run`. | That the affected login had such a lease before the reported reboot. |
| `scripts/install-host-helper.py`: bootstrap and tmpfiles policy | Installation recreates the runtime directory; it does not install persistent creation-device permissions or boot module loading. | Whether other host policy supplies either prerequisite. |
| `udev/70-virtualgamepad-uhid.rules` | The repository supplies an opt-in `virtualgamepad-uhid`, mode `0660` policy. | That the rule or group is installed or the effective login is enrolled. |
| `crates/gr-provider-linux-uhid/src/lib.rs`: preflight, open, close | The provider opens a fresh node and owns its session; host setup stays outside it. | Successful creation on a newly booted host. |
| `scripts/host-preflight.py`: `inspect` and `main` | Inventory checks registration, character-device identity and effective access without opening the node. | Successful `open`, controller creation, or consumer access. |

The kernel documents one open per created device and destruction on file close.
That supports retaining the existing process-owned session model; it offers no
reason to add reboot recovery to a live session. See the
[Linux UHID API documentation](https://docs.kernel.org/hid/uhid.html).

### Read-only review snapshot

The review environment reported the following on 2026-09-10; these are sanitized
observations, not a before/after reboot record:

- `creation-device: missing` and `consumer-access: unvalidated` from inventory.
- `/dev/uhid` existed as a character device with mode `0600`, but
  `/sys/class/misc/uhid/dev` and `/sys/module/uhid` were absent.
- An ordinary-user `O_RDWR | O_NONBLOCK` open failed with `EACCES`.
- The dedicated group and named repository rule were absent. A different local
  UHID rule selected the broad `input` group and mode `0660`.
- No explicit `uhid` entry was found in `/etc/modules-load.d` or
  `/usr/lib/modules-load.d`. Other boot mechanisms were not exhaustively audited.

This is an unresolved registration/authorization state. An existing node alone
cannot establish a valid kernel registration. The difference between the local
rule and observed mode also needs investigation. Do not conclude that a group
change alone fixes this environment, or that module loading is necessarily the
missing boot mechanism. Confirm the affected host and its device/sysfs visibility
before attempting provisioning. No host configuration or ACL was changed.

### Corrections to the handoff's acceptance procedure

- Inventory's `missing` combines absent node and absent registration. Record
  these separately; the review snapshot demonstrates why this matters.
- Inventory exits 1 even when creation access is `ready`, because
  `consumer-access` remains `unvalidated`. Judge the named JSON check, not exit
  zero. Consumer validation remains a separate gate.
- Inventory currently accepts `linux.uhid.usb`, not `linux.uhid.bluetooth`, even
  though the provider supports both exact targets. Use USB inventory for the
  shared creation node and retain exact-target provider tests for both buses.
- `open_node` maps every non-`NotFound` error to `AccessDenied`. That label alone
  cannot prove an ACL problem; preserve the underlying OS error during diagnosis.
- Before/after records must live in private storage known to survive reboot.
  `/tmp` may be cleared at boot and is unsuitable as the only evidence copy.
- A competing administrator rule is a decision to resolve before installation,
  not something an installer should overwrite or defeat through rule ordering.

## Confirmed GUI failure and current-session repair

The reported GUI screenshot explicitly selected HID / UHID and showed
`linux.uhid.usb cannot access device node /dev/uhid`. Its displayed live DualSense
was an earlier Evdev controller, not a successfully created UHID controller.
The failure therefore was not merely the GUI's default target selection.

Read-only checks reproduced a root-only node, no visible UHID registration or
loaded module, and no active helper leases. Module metadata confirmed modular
UHID support. Loading only `uhid` through the installed helper and settling udev
produced this transition:

| Check | Before module load | After module load |
| --- | --- | --- |
| Kernel misc registration | absent | present, matches the character node |
| Node policy | root-only `0600` | existing administrator group rule, `0660` |
| Named-user ACL lease | inactive | inactive; no grant performed |
| Creation inventory | missing | ready |
| Ordinary-user creation | open denied | live provider and curated DualSense tests passed |

This isolates missing registration as the immediate trigger: the module's device
registration allowed existing udev policy to apply. No group enrollment, ACL
grant, rule replacement or broad permission change was performed. The existing
broad-group policy remains an administrator-owned follow-up; it is not the
repository's recommended installation policy.

The new ignored demo test calls the actual `App::create`, starts its real service
workers, waits for kernel input binding, creates two controllers, removes one,
creates again and shuts down the application. It passed as the ordinary user
after module loading, with all test-owned devices removed. This verifies the
GUI creation/worker path, not a manual rendered-window interaction or SDL/Steam
compatibility. The minimal provider and curated startup/cleanup tests also passed.

The user subsequently confirmed that the change appears to resolve the GUI
failure. This supplies user-observed recovery in addition to the automated live
test evidence; it is not a report of successful post-reboot validation.

The GUI now retains the underlying creation error and adds registration-aware
setup guidance, with deterministic regressions for missing registration, a retry
after registration, unavailable inspection and unrelated provider failures.
Providers still do no privileged setup.

`modules-load.d/virtualgamepad-uhid.conf` supplies the narrow boot opt-in, with
installation and rollback guidance in the deployment document. The installed
helper is authorized to load modules but not install boot policy. After the
administrator installation step was provided, a follow-up inspection confirmed
that `/etc/modules-load.d/virtualgamepad-uhid.conf` is root-owned with mode `0644`
and matches the reviewed repository file byte-for-byte. Reboot acceptance below remains necessary; no
reboot was performed during this repair.

## Resolution sequence

### 0. Compare the live harness and demo before choosing a host fix

Ordinary workspace tests skip the ignored live UHID tests. The provider's
deterministic tests use fake I/O; their success says nothing about `/dev/uhid`
access. The minimal ignored provider test opens the real provider with a small
synthetic descriptor, sends input and closes. It does not exercise a curated
controller's complete kernel startup.

The curated live tests call the same `create_dualsense`/`create_dualshock4`
functions as `demo/src/gui.rs::App::create`, with explicit UHID selection.
Both reach the same `LinuxUhidProvider`, which checks access, opens a fresh node
and sends `UHID_CREATE2`. Neither the tests nor GUI grants access or loads UHID;
the documented helper grant is a separate preparation step. The live tests poll
the controller in their test loop; the GUI starts a service worker after creation
and closes the controller if that worker fails.

The demo defaults to **Evdev on every launch** (`App::default`), whereas the live
UHID tests explicitly select UHID. Confirm the GUI's selected target and actual
error path before attributing a post-restart failure to UHID permissions.

On the same boot, revision, ordinary-user login and host preparation state, run
the exact ignored test `dualsense_services_kernel_startup_and_removes_its_device`
in `dualsense_uhid_live`, then launch the demo from that same login and explicitly
select DualSense and HID / UHID. Keep these runs sequential and make no grants
or restores between them. Verify the test actually ran rather than being ignored.
Record whether the GUI reports `Creation failed`, reports a later provider
failure, or creates successfully but is absent from the consumer.

- If both fail opening the node, continue with host registration/access diagnosis.
- If the live curated test passes and the GUI fails, first compare target,
  executable revision, effective groups and launch confinement. Then investigate
  GUI creation/worker lifecycle with a deterministic regression for the actual
  failing sequence, rather than changing host permissions speculatively.
- If both create but only consumer discovery fails, investigate consumer access,
  discovery and protocol servicing separately from creation-device authorization.

### 1. Establish the failing boot state

On the affected Linux host, arrange a controlled reboot with its operator. Before
any helper grant, module load, or permission repair, record the boot identity,
effective UID/groups, dedicated group membership, node type/owner/group/mode/ACL,
kernel registration and major/minor match, module state, applicable udev rules,
and boot module policy. Keep identifying details and raw records private.

Run `python3 scripts/host-preflight.py linux.uhid.usb`. Classify using the
creation-device row and a minimal ordinary-user open where identity is valid:

| State | Next action |
| --- | --- |
| Registration or node absent | Determine kernel support and host/sysfs visibility. If modular support exists, an administrator can test loading only `uhid`, wait for udev and recheck identity. |
| Identity mismatch | Stop grants; resolve the stale, substituted or incorrectly exposed node first. |
| Valid node, access denied | Inspect effective groups, ACLs and competing rules; distinguish missing enrollment from a stale login. |
| Inventory ready, open/create fails | Capture the actual OS error and smallest provider failure before investigating runtime behavior. |

Once identity is valid, a controlled temporary grant/restore experiment can test
the ACL hypothesis. Capture the baseline after udev settlement, grant only via
the scoped helper, run the ordinary-user creation test, then restore the lease
and verify idempotent cleanup. If registration was missing, establish and record
registration first: `uhid-grant` itself can load the module, so a successful grant
alone does not isolate permissions as the cause. Do not use reboot for cleanup.

### 2. Prove the narrow persistent policy manually

Resolve competing UHID rules with the administrator before applying policy.
Choose the dedicated `virtualgamepad-uhid` system group and the reviewed repository
rule; explicitly enroll only the intended trusted user. Record pre-existing
files and membership so rollback can preserve administrator-owned state.

Reload udev rules, trigger only `/sys/class/misc/uhid` after it exists, and settle.
Verify kernel identity, resulting group/mode and ACL, then use a fresh login to
verify effective membership and access. Checking access as the root installer
does not validate the intended user's session. Do not enroll into `input` or
make the node world-writable.

Add an explicit `/etc/modules-load.d/virtualgamepad-uhid.conf` containing only
`uhid` only if fresh-boot evidence shows modular UHID stays unregistered until
explicitly loaded. Built-in or already automatically registered UHID needs no
such file. A missing sysfs view alone is insufficient evidence.

Rollback removes only policy and membership introduced by this intervention,
after checking for subsequent administrator changes. Reapply the retained policy
to the selected device and verify it. Group removal requires new login sessions
to take effect; do not unload a module while any user may still depend on it.

### 3. Decide the repository implementation from that evidence

Prefer documenting the proven installation and improving inventory diagnostics
first. The existing rule may be sufficient; a new privileged installer is not
required to resolve a single host's configuration.

The next diagnostic change should distinguish missing node from registration,
explain temporary versus persistent access for UHID denial, and cover both
exact UHID inventory targets. Preserve the read-only behavior and existing
statuses unless a separately reviewed compatibility change is justified.
Keep broad public error/API redesign out of this fix.

If repeatable supported provisioning is warranted after manual proof, add a
separate administrator-run installer with this bounded contract:

- Require explicit UHID selection and an explicitly resolved local user; never
  infer the target identity from a root process's effective access.
- Validate all destinations, existing group and policies before mutation. Reuse
  matching state, reject symlinks/unsafe destinations and conflicting files,
  and install only the reviewed rule bytes. Report competing policy for review.
- Enroll only the dedicated group. Load no modules by default; any boot policy
  is a separate explicit option supported by the recorded evidence.
- Trigger only the selected registered UHID device and settle before checking
  results. Report absent registration or a required fresh login accurately.
- Make repeats idempotent and report partial failures with exact recovery steps.
  Roll back only changes owned by this installation; retain unrelated policy.

The runtime provider remains unprivileged. The development helper journal
remains boot-scoped, and controller resource snapshots stay unrelated to host
permissions. No fd resurrection, generic sudo grant, or additional provider-side
privilege is planned.

## Validation and closure

For a diagnostic change, extend `scripts/tests/test_host_preflight.py` with
synthetic node-only and registration-only states, mismatch, denied, ready,
repeated inspection without mutation, and both exact UHID targets. Assert the
consumer row and CLI exit semantics so `ready` is not mistaken for full validation.

If an installer is introduced, test through a fake host/command runner: first
install, unchanged repeat, group reuse/enrollment and stale-login reporting,
exact rule bytes, conflicting rules/files and unsafe destinations, missing
registration, opt-in module policy, selected-device trigger/settle, failure at
each mutation boundary, recovery and idempotent removal. Assert no broad group,
world-write grant, arbitrary module, or temporary-journal persistence occurs.
Retain helper identity-change, restoration, repeated polling and cleanup tests,
plus existing provider protocol acknowledgements and lifecycle regressions.

Actual reboot acceptance remains mandatory for closing the access issue:

1. With temporary leases restored and the selected durable policy applied,
   establish ordinary-user creation and cleanup in a fresh login.
2. Reboot the affected host. Before any grant or manual module load, confirm
   valid registration/node identity, the intended group/mode/ACL, effective
   membership and `creation-device: ready`.
3. Run `cargo test -p gr-provider-linux-uhid
   creates_and_destroys_a_process_owned_hid_device -- --ignored --nocapture`
   as the ordinary user. Verify the named test ran and passed, then repeat it
   to exercise fresh creation after cleanup.
4. Reboot again and repeat steps 2–3 without manual repair. Preserve sanitized
   results from both boots, with the exact policy and any module-load requirement.

Run the repository's format, workspace check, clippy, workspace test, Python
tooling test and secret-scan gates for the implementation. Deterministic tests
cannot replace reboot evidence; a plan PR or a successful temporary grant does
not close the reported bug.

The implemented resolution is the explicit boot configuration and installation
guidance, plus improved GUI diagnostics; a new privileged installer is unnecessary
for this fix. Remaining acceptance is creation and cleanup after reboot without
manual preparation. Review of the pre-existing broad-group rule remains a
separate administrator follow-up; this repair did not replace that policy.
