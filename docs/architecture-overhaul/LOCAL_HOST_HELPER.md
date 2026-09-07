# Local development host helper

This is optional development delegation, not a product installation requirement.
It avoids both general passwordless sudo and cross-terminal sudo authentication
caching. It authorizes only one installed root-owned helper for one numeric UID.
Other commands retain the host's existing sudo policy.

## One-time installation

Review `scripts/virtualgamepad-host-helper.py` and
`scripts/install-host-helper.py`, then run this from the checkout as the intended
development account:

```bash
sudo /usr/bin/python3 -I scripts/install-host-helper.py
```

Enter the password in your terminal, never in chat. The installer copies the
helper to `/usr/local/libexec/virtualgamepad-host-helper`, writes root-owned policy
under `/etc/virtualgamepad`, creates private mode-0700 runtime state, and installs
one UID-specific sudoers rule. It validates the rule with visudo before publishing
it last. Existing differing files are not overwritten. Installation does not load
modules, grant device access, enable services, or change group membership.

The installer itself and the checkout's Python scripts are **not** passwordless
sudo commands. The installed helper starts Python in isolated mode, excluding
user import paths; it uses a fixed subprocess environment and no shell or caller
stdin. Policy, executable, job files, state and their ancestors must be root-owned,
non-symlink, and not writable by other users. See [Python isolated mode](https://docs.python.org/3/using/cmdline.html#cmdoption-I).

Verify installation:

```bash
sudo -n /usr/local/libexec/virtualgamepad-host-helper status
```

Run normal inventory as yourself: `python3 scripts/host-preflight.py all`. Running
inventory through sudo observes root access and still does not prepare anything.

## Updating installed code

After reviewing a helper fix, run:

```bash
sudo /usr/bin/python3 -I scripts/install-host-helper.py --update
```

This explicitly replaces only the installed executable, under the helper lock.
It preserves administrator policy, jobs, sudoers and active journals, and rejects
an unexpected account or unsafe ownership/modes. Updating privileged code still
requires administrator authentication; the delegated helper cannot update itself.

## Delegated operations

| Operation | Authority and safeguards |
| --- | --- |
| `status` | Inspect this helper's active creation-device leases without creating a device. |
| `uhid-grant` / `uhid-restore` | Prepare only UHID, verify its kernel registration/node identity, grant the installed policy UID temporary read/write access, and restore the recorded ACL. |
| `uinput-grant` / `uinput-restore` | The same lifecycle for uinput, with a separate lease. Reuse existing access when sufficient rather than calling grant unnecessarily. |
| `uhid-recover-udev` | Recover the documented first-registration group race only: require the original device, a group-only change, and the exact saved grant; remove that grant and reapply existing udev policy only to UHID. |
| `module-load NAME` | Load exactly an administrator-approved module name, without parameters. The initial list is uhid, uinput, libcomposite, usb_f_hid, dummy_hcd, and usbmon. No module is loaded by installation. |
| `run-job NAME` | Execute only a separately approved, root-owned job manifest with fixed arguments and a trusted executable. There are no jobs enabled by default. |

Every operation verifies SUDO_UID against installed policy, validates its complete
argument shape, and holds an exclusive lock. Invalid callers, unknown actions,
unapproved names, and extra arguments are rejected. A passwordless sudo rule for
the helper does not authorize Python, a shell, arbitrary paths, module parameters,
a package manager, or an installer.

## Device lease and restoration

The helper waits for udev to settle before capturing the creation-device baseline.
A settle timeout leaves permissions untouched. It journals the original ACL and filesystem/device identity before an ACL
write. It preserves other principals' effective permissions when expanding the
mask. Repeat grants do not replace the original baseline; status is read-only.
Interrupted grants can be restored from the journal. Oversized journals are
rejected before mutation so every accepted ACL baseline remains recoverable. Replacement nodes, changed
owners/groups, externally changed ACLs, and malformed journals require operator
review instead of overwriting another researcher's changes. The explicit
`uhid-recover-udev` exception handles only the observed first-registration race
with a simple original ACL and a journaled module-load request. Recovery has its
own persistent phase; failures retain the journal. It does not edit udev rules,
change groups, or trigger unrelated devices. Existing host policy may itself grant
creation access; restoring that policy does not revoke pre-existing authority.

After installation the agent can run:

```bash
sudo -n /usr/local/libexec/virtualgamepad-host-helper uhid-grant
# Run the unprivileged UHID tests and controlled acceptance session.
sudo -n /usr/local/libexec/virtualgamepad-host-helper uhid-restore
```

Close test controllers and processes before restoration: removing an ACL does not
revoke an already-open descriptor. Direct creation access is trusted-user access,
not a restriction to our controller profiles. Any processes under the authorized
UID share that authority. Device replacement by other administrator activity must
not run concurrently with the experiment.

The helper intentionally never unloads modules. Another workload may have adopted
them; loaded modules are retained for administrator review and reported as such.
Runtime lease state survives process/service restarts but not VM reboot. Boot
recreates the state directory through tmpfiles; the device nodes/temporary ACLs
also belong to that boot. Do not reboot as an experiment cleanup mechanism.

## Broader experiments without helper-code updates

Administrator policy is `/etc/virtualgamepad/host-helper.conf`. To permit another
module, add an `allow_module=NAME` entry after reviewing its impact on the shared
VM. Bluetooth module preparation remains opt-in for the relevant experiment.

For a recurring privileged experiment, install a root-owned job executable and a
root-owned JSON manifest under `/etc/virtualgamepad/host-jobs/NAME.json`, then add
`allow_job=NAME` to policy. For example, a deliberately approved broker startup
job can use this fixed manifest after its config and UDC ownership are prepared:

```json
{"argv":["/usr/bin/systemctl","start","virtualgamepad-broker.socket"]}
```

No caller-supplied arguments are appended. Jobs run from `/` with a fixed
environment, closed stdin and a 30-second subprocess timeout. Job definitions
must use root-owned scripts, libraries, config and executable inputs; root-owning
an interpreter alone does not make a user-writable script safe. Jobs should be
noninteractive and implement their own failure/descendant cleanup; the helper
is not a process supervisor or rollback engine for arbitrary job side effects.
Reserve resources before approving a job that touches services, captures traffic,
or creates gadgets. Do not approve generic shells, unrestricted copy/install
operations, or writable checkout test binaries.

Adding an approved job/module is a configuration change rather than a helper
rewrite; repeated runs need no password or configuration changes. Installing or
replacing privileged executable code still requires administrator review. This
keeps the helper broadly useful without implicitly granting unrestricted root.
Gadget startup, capture and future audio/Bluetooth operations are not performed
merely because the helper can represent an approved job.

## Revocation

Restore active leases first. Then an administrator removes the helper's rule with
`sudo visudo -f /etc/sudoers.d/virtualgamepad-host-helper` (delete the UID rule).
No other sudoers file needs changing. Root-owned binaries/configuration may remain
inert for inspection; no service was enabled. If a prior global timestamp override
was installed separately, remove that separate override and invalidate its cache
with `sudo -K` as well. The installer does not modify unrelated sudo configuration.

## Validation status

Synthetic tests cover caller/argument rejection, approved module dispatch, ACL mask
preservation, a real ACL encoding round trip on a private synthetic file, interrupted
grant/journal writes, node/ACL changes, repeated polling, idempotent restoration,
fixed job arguments, untrusted paths, sudoers syntax and refusal to overwrite
existing installation files, udev settlement before baseline capture, narrow race
recovery and policy-preserving executable updates.

Installed status succeeded, but the first live UHID grant exposed a udev ordering
race: the saved group preceded existing udev policy. The helper refused both grant
verification and normal restoration and retained its journal. The temporary ACL
and newly loaded UHID module remain until the updated helper performs recovery.
Live recovery and controller creation remain pending; this is not a UHID or B/P
acceptance pass. See [the host experiment](experiments/EXP-0005-host-access.md).
