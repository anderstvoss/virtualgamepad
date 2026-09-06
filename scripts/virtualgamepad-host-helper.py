#!/usr/bin/python3 -I
"""Installed root-owned development helper. Policy-bound device, module and validation-job operations."""
import fcntl
import json
import os
import re
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path

EXECUTABLE = Path('/usr/local/libexec/virtualgamepad-host-helper')
CONFIG = Path('/etc/virtualgamepad/host-helper.conf')
STATE = Path('/run/virtualgamepad-host-helper')
ENVIRONMENT = {'PATH': '/usr/sbin:/usr/bin', 'LANG': 'C', 'LC_ALL': 'C'}
ACTIONS = ('status', 'uhid-grant', 'uhid-restore', 'uinput-grant', 'uinput-restore')


def trusted(path, directory=False):
    """Never import or read privileged policy/state through writable ancestors."""
    path = Path(path)
    for index, ancestor in enumerate([path, *path.parents]):
        metadata = ancestor.lstat()
        is_dir = directory or index > 0
        valid_kind = stat.S_ISDIR(metadata.st_mode) if is_dir else stat.S_ISREG(metadata.st_mode)
        if not valid_kind or metadata.st_uid != 0 or metadata.st_mode & 0o022:
            raise ValueError('helper, policy and state require root-owned, non-writable, non-symlink paths')


def parse_uid(text):
    if not re.fullmatch(r'[0-9]{1,10}', text) or not 0 < int(text) < 2**32 - 1:
        raise ValueError('expected a non-root numeric UID')
    return int(text)


def parse_policy(text):
    policy = {'allow_module': [], 'allow_job': []}
    for line in text.splitlines():
        if not line or line.startswith('#'):
            continue
        key, separator, value = line.partition('=')
        if not separator:
            raise ValueError('invalid policy line')
        if key == 'allow_uid' and key not in policy:
            policy[key] = parse_uid(value)
        elif key in ('allow_module', 'allow_job') and re.fullmatch(r'[a-z][a-z0-9_-]{0,47}', value):
            if value in policy[key]:
                raise ValueError('duplicate policy entry')
            policy[key].append(value)
        else:
            raise ValueError('invalid policy key/value')
    if 'allow_uid' not in policy:
        raise ValueError('policy must authorize one UID')
    return policy


def authorize(policy, sudo_uid):
    uid = parse_uid(sudo_uid)
    if policy['allow_uid'] != uid:
        raise ValueError('invoking UID is not authorized by the installed policy')
    return uid


def parse_acl(text):
    entries = {}
    for line in text.splitlines():
        if not line:
            continue
        match = re.fullmatch(r'(user|group|mask|other):([0-9]*):([r-][w-][x-])', line)
        if match is None:
            raise ValueError('unexpected ACL format')
        kind, identity, permissions = match.groups()
        if (kind in ('mask', 'other') and identity) or (kind, identity) in entries:
            raise ValueError('invalid or duplicate ACL entry')
        entries[kind, identity] = permissions
    if not {('user', ''), ('group', ''), ('other', '')} <= entries.keys():
        raise ValueError('incomplete ACL')
    if any(identity for _, identity in entries) and ('mask', '') not in entries:
        raise ValueError('extended ACL has no mask')
    return entries


def acl_text(entries):
    return ''.join(f'{kind}:{identity}:{permissions}\n' for (kind, identity), permissions in sorted(entries.items()))


def grant_acl(original, uid):
    entries = parse_acl(original)
    old_mask = entries.get(('mask', ''), entries['group', ''])
    # Expanding the mask must not activate dormant permissions for another user
    # or group. Clamp those entries to their previous effective permissions.
    for key, permissions in list(entries.items()):
        if key[0] == 'group' or (key[0] == 'user' and key[1]):
            entries[key] = ''.join(p if m != '-' else '-' for p, m in zip(permissions, old_mask))
    existing = entries.get(('user', str(uid)), '---')
    entries['user', str(uid)] = 'rw' + existing[2]
    entries['mask', ''] = 'rw' + old_mask[2]
    return acl_text(entries)


class Host:
    def __init__(self, name='uhid'):
        if name not in ('uhid', 'uinput'):
            raise ValueError('unsupported creation device')
        self.name = name
        self.device = Path('/dev') / name
        self.registration = Path('/sys/class/misc') / name / 'dev'

    def run(self, arguments, data=None):
        options = {'input': data} if data is not None else {'stdin': subprocess.DEVNULL}
        result = subprocess.run(arguments, **options, text=True, capture_output=True,
                                env=ENVIRONMENT, cwd='/', timeout=30, check=True)
        return result.stdout

    def prepare(self):
        loaded = False
        if not self.registration.exists():
            self.run(['/usr/sbin/modprobe', self.name])
            loaded = True
        for _ in range(50):
            if self.registration.exists() and self.device.exists():
                return loaded
            time.sleep(0.1)
        raise ValueError('Creation-device registration/node did not appear; no ACL granted')

    def identity(self):
        metadata = self.device.lstat()
        expected = self.registration.read_text().strip()
        actual = f'{os.major(metadata.st_rdev)}:{os.minor(metadata.st_rdev)}'
        if not stat.S_ISCHR(metadata.st_mode) or metadata.st_uid != 0 or actual != expected:
            raise ValueError('Creation node does not match the root-owned registered kernel device')
        return dict(dev=metadata.st_dev, ino=metadata.st_ino, rdev=metadata.st_rdev,
                    owner=metadata.st_uid, group=metadata.st_gid)

    def acl(self):
        return acl_text(parse_acl(self.run(['/usr/bin/getfacl', '-cpnE', str(self.device)])))

    def set_acl(self, value):
        # Input is ACL entries only: no filenames, owner headers or restore paths.
        self.run(['/usr/bin/setfacl', '--no-mask', '--set-file=-', str(self.device)], value)


class Journal:
    def __init__(self, name='uhid'):
        if name not in ('uhid', 'uinput'):
            raise ValueError('unsupported lease name')
        self.path = STATE / (name + '.json')

    def load(self):
        try:
            trusted(self.path)
        except FileNotFoundError:
            return None
        if self.path.stat().st_size > 16384:
            raise ValueError('oversized lease journal')
        return json.loads(self.path.read_text())

    def save(self, value):
        encoded = json.dumps(value, sort_keys=True)
        if len(encoded.encode('utf-8')) > 16384:
            raise ValueError('lease is too large to recover; no journal or ACL change allowed')
        descriptor, name = tempfile.mkstemp(prefix='lease-', dir=STATE)
        temporary = Path(name)
        try:
            with os.fdopen(descriptor, 'w') as stream:
                stream.write(encoded)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, self.path)
            directory = os.open(STATE, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
        finally:
            temporary.unlink(missing_ok=True)

    def clear(self):
        self.path.unlink()


def validate_lease(lease, uid):
    if not isinstance(lease, dict) or lease.get('version') != 1 or lease.get('uid') != uid:
        raise ValueError('lease ownership/version mismatch')
    if lease.get('phase') not in ('preparing', 'prepared', 'granted'):
        raise ValueError('invalid lease phase')
    if lease['phase'] != 'preparing':
        if not isinstance(lease.get('identity'), dict):
            raise ValueError('lease identity missing')
        if grant_acl(lease['original'], uid) != lease['expected']:
            raise ValueError('lease ACL mismatch')


def execute(action, uid, host, journal):
    if action not in ACTIONS:
        raise ValueError('unknown action')
    lease = journal.load()
    if lease is not None:
        validate_lease(lease, uid)
    if action == 'status':
        return dict(active=lease is not None, phase=lease['phase'] if lease else None)
    if action.endswith('-grant'):
        if lease is not None:
            if lease['phase'] == 'granted' and host.identity() == lease['identity'] and host.acl() == lease['expected']:
                return dict(active=True, result='already granted')
            raise ValueError('an incomplete or changed lease needs restoration/operator review')
        lease = dict(version=1, uid=uid, phase='preparing')
        journal.save(lease)
        # Keep a journal even if module loading or registration fails. Do not
        # unload modules automatically: another researcher may now use them.
        lease['module_load_requested'] = host.prepare()
        lease.update(identity=host.identity(), original=host.acl(), phase='prepared')
        lease['expected'] = grant_acl(lease['original'], uid)
        journal.save(lease)
        if host.identity() != lease['identity'] or host.acl() != lease['original']:
            raise ValueError('device or ACL changed during preparation; no grant attempted')
        host.set_acl(lease['expected'])
        if host.identity() != lease['identity'] or host.acl() != lease['expected']:
            raise ValueError('grant verification failed; journal retained')
        lease['phase'] = 'granted'
        journal.save(lease)
        return dict(active=True, result='temporary creation-device access granted', module_load_requested=lease['module_load_requested'])
    if lease is None:
        return dict(active=False, result='already restored')
    if lease['phase'] != 'preparing':
        if host.identity() != lease['identity']:
            raise ValueError('device identity changed; refusing restoration onto a reused node')
        current = host.acl()
        if current not in (lease['original'], lease['expected']):
            raise ValueError('ACL changed outside this lease; refusing to overwrite other research')
        if current != lease['original']:
            host.set_acl(lease['original'])
        if host.identity() != lease['identity'] or host.acl() != lease['original']:
            raise ValueError('restoration verification failed; journal retained')
    journal.clear()
    return dict(active=False, result='ACL restored; any loaded module is retained for administrator review')


def parse_action(arguments):
    if len(arguments) == 1 and arguments[0] in ACTIONS:
        return arguments[0], None
    if len(arguments) == 2 and arguments[0] in ('module-load', 'run-job') and re.fullmatch(r'[a-z][a-z0-9_-]{0,47}', arguments[1]):
        return arguments[0], arguments[1]
    raise ValueError('expected status, {uhid|uinput}-{grant|restore}, module-load NAME, or run-job NAME')


def run_operation(action, name, policy, host):
    if action == 'module-load':
        if name not in policy['allow_module']:
            raise ValueError('module is not administrator-approved')
        host.run(['/usr/sbin/modprobe', name])
        return dict(result='module load requested; module retained for administrator review', module=name)
    if action != 'run-job' or name not in policy['allow_job']:
        raise ValueError('job is not administrator-approved')
    path = CONFIG.parent / 'host-jobs' / (name + '.json')
    trusted(path)
    if path.stat().st_size > 16384:
        raise ValueError('oversized job definition')
    job = json.loads(path.read_text())
    if set(job) != {'argv'} or not isinstance(job['argv'], list) or not job['argv']:
        raise ValueError('job requires a fixed argv list')
    if not all(isinstance(arg, str) and '\x00' not in arg for arg in job['argv']):
        raise ValueError('invalid job arguments')
    executable = Path(job['argv'][0])
    if not executable.is_absolute():
        raise ValueError('job executable must be absolute and administrator-owned')
    trusted(executable)
    # Only administrator-authored arguments; never append caller input. Jobs
    # must use root-owned inputs as documented. No shell, stdin or user env.
    output = host.run(job['argv'])
    return dict(result='job completed', job=name, output=output)


def main():
    action, name = parse_action(sys.argv[1:])
    if os.geteuid() != 0 or not sys.flags.isolated or Path(__file__) != EXECUTABLE:
        raise ValueError('use the installed root-owned helper through sudo, not the checkout source')
    trusted(EXECUTABLE)
    trusted(CONFIG)
    policy = parse_policy(CONFIG.read_text())
    uid = authorize(policy, os.environ.get('SUDO_UID', ''))
    trusted(STATE, directory=True)
    descriptor = os.open(STATE / 'lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if name is not None:
            result = run_operation(action, name, policy, Host())
        elif action == 'status':
            result = {device: execute(action, uid, Host(device), Journal(device)) for device in ('uhid', 'uinput')}
        else:
            device = action.split('-')[0]
            if action.endswith('-grant') and device not in policy['allow_module']:
                raise ValueError('creation mechanism is not administrator-approved')
            result = execute(action, uid, Host(device), Journal(device))
        print(json.dumps(result, sort_keys=True))
    finally:
        os.close(descriptor)


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        sys.exit('Host helper refused: ' + str(error))
