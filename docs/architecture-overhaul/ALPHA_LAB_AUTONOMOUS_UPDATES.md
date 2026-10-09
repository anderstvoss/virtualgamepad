# Revocable autonomous alpha-lab updates

An administrator can bootstrap `scripts/package-alpha-lab-updater.py` once.
This explicitly delegates replacement of root-running lab code to the configured
ordinary account. Payload hashes establish captured-byte consistency, not an
independent approval of that code. Other processes sharing the account have the
same capability; this is not a Codex-process-exclusive identity boundary.

The generated root-owned updater accepts only `update` and `revoke`. Updates read
one fixed mailbox. They preserve the bootstrap's client/worker/unauthorized UIDs,
four authorized VHCI ports, eight named phases, fixed helper template and fixed
installation destinations. They never accept commands, executable paths,
environment overrides or identities on the command line. Adding phases or
changing the pinned installer/template requires a new administrator bootstrap.

Generate a clean committed candidate with `package-alpha-provider-lab.py`, then
place its packet in the configured mailbox. Invoke:

```sh
sudo -n -k /usr/local/libexec/virtualgamepad-codex-update update
```

The updater captures bounded regular files through a directory descriptor,
rejects symlinks/FIFOs, validates metadata and hashes, and stages captured files
under root ownership. It executes the bootstrap-pinned installer implementation,
never the packet's `install.py`. The existing helper/policy transaction retains
rollback, global ownership locking, service-idle checks and restoration. A held
active-stage lock refuses updates and revocation during trials. Existing receipts
and old immutable stages remain available. No trial or service action runs during
installation.

After finishing the review, revoke autonomous updates:

```sh
sudo -n -k /usr/local/libexec/virtualgamepad-codex-update revoke
```

The generated `revoke.sh` performs the same action. Revocation removes only the
identity-matching updater sudo policy. It preserves the existing lab's three
command categories, journal reader, installed candidate, receipts and unrelated
sudo policies. Subsequent updates fail. Repeated administrator invocation of
`revoke` reports already revoked. Changed policy/helper identity refuses removal.
A failed sudo-policy validation restores the exact updater policy.

If interrupted after candidate installation but before recording the new helper
hash, the updater conservatively refuses another update. An administrator must
inspect the immutable stages and restore the recorded identity; it must not
silently accept a changed helper. Failed bootstrap removes only files it created,
reporting initiating and cleanup failures independently.

Regression coverage: `scripts/tests/test_alpha_lab_updater.py` covers fixed scope,
helper-template integrity, bounded capture, occupied run locks, policy identity,
repeat revocation, rollback and root-owned capture handoff. Existing provider
package tests cover the candidate installer transaction. Live bootstrap/update/
revocation acceptance requires administrator provisioning and remains separate
from deterministic tests.
