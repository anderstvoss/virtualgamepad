#!/usr/bin/python3 -I
"""Reversible, administrator-run candidate broker maintenance lab.

Default: print a plan only. --apply requires a root-owned copy of this reviewed
script, exact approved binary hashes, an idle installed broker, a free VHCI port,
and three distinct existing non-root identities. No services are enabled and no
installed binary/configuration is replaced. Client commands run without root.
Kernel/module/device provisioning is a separate administrator prerequisite.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import re
import signal
import stat
import subprocess
import sys
import tempfile
import time

STATE = Path('/run/virtualgamepad-state')
VHCI = Path('/sys/devices/platform/vhci_hcd.0/status')
ORIGINAL = ('virtualgamepad-broker.socket', 'virtualgamepad-broker.service')


def trusted(path, directory=False):
    meta = path.lstat()
    if (meta.st_uid != 0 or meta.st_mode & 0o022 or
            not (stat.S_ISDIR(meta.st_mode) if directory else stat.S_ISREG(meta.st_mode))):
        raise RuntimeError('expected root-owned, non-symlink, non-writable lab path')


def identity(path):
    info = path.lstat()
    return info.st_dev, info.st_ino


def remove_owned(path, expected, directory=False):
    if not path.exists() and not path.is_symlink():
        return
    if identity(path) != expected:
        raise RuntimeError('owned path identity changed; refusing cleanup')
    path.rmdir() if directory else path.unlink()


def free_port(text, selected):
    lines = text.splitlines()
    if not lines or lines[0].split() != ['hub', 'port', 'sta', 'spd', 'dev', 'sockfd', 'local_busid']:
        raise RuntimeError('unknown VHCI status header')
    ports = {}
    for line in lines[1:]:
        fields = line.split()
        if len(fields) != 7 or fields[0] not in ('hs', 'ss'):
            raise RuntimeError('malformed VHCI inventory')
        hub, port, state, speed, device, descriptor, bus = fields
        numbers = (int(port), int(state), int(speed), int(device, 16), int(descriptor))
        if any(n < 0 for n in numbers) or numbers[0] in ports:
            raise RuntimeError('ambiguous VHCI inventory')
        ports[numbers[0]] = (hub, *numbers[1:], bus)
    if ports.get(selected) != ('hs', 4, 0, 0, 0, '0-0'):
        raise RuntimeError('selected VHCI port is absent or occupied')


def maintenance(backend):
    saved = backend.snapshot()
    backend.preflight(saved)
    initiating = None
    cleanup = []
    touched = False
    try:
        touched = True  # Register restoration before a possibly partial stop.
        backend.stop_original_socket()
        backend.assert_idle(saved)
        backend.stop_original_service()
        backend.prepare()
        backend.start_candidate()
        backend.execute()
    except BaseException as error:
        initiating = error
    finally:
        for operation in (backend.stop_candidate, backend.cleanup):
            try:
                operation()
            except BaseException as error:
                cleanup.append(str(error))
        if touched:
            try:
                backend.restore(saved)
            except BaseException as error:
                cleanup.append(str(error))
    if initiating is not None or cleanup:
        raise RuntimeError(json.dumps(dict(initiating=None if initiating is None else str(initiating),
                                          cleanup=cleanup)))


class Host:
    def __init__(self, args):
        self.args = args
        self.owned = []
        self.units = []
        self.root = None
        self.client_attempted = False
        self.events = []

    def run(self, argv, timeout=15):
        result = subprocess.run(argv, check=True, capture_output=True, text=True, timeout=timeout,
                                env={'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C'})
        self.events.append(dict(command=argv, status=result.returncode))
        return result.stdout

    def snapshot(self):
        saved = {}
        for unit in ORIGINAL:
            output = self.run(['systemctl', 'show', unit, '-p', 'ActiveState', '-p', 'MainPID'])
            properties = dict(line.split('=', 1) for line in output.splitlines())
            if properties['ActiveState'] not in ('active', 'inactive'):
                raise RuntimeError('original service is not in a stable state')
            saved[unit] = properties
        return saved

    def assert_idle(self, saved):
        # Socket activation may have changed the PID since the restoration
        # snapshot. Inspect the current process, never an obsolete identity.
        del saved
        current = self.snapshot()
        pid = int(current[ORIGINAL[1]]['MainPID'])
        sockets = self.run(['ss', '-H', '-xnp', 'state', 'connected'])
        if pid and re.search(r'pid=' + str(pid) + r'[,)]', sockets):
            raise RuntimeError('installed broker has connected clients')
        if pid:
            listeners = self.run(['ss', '-H', '-xlnp'])
            owned = [line.split() for line in listeners.splitlines()
                     if re.search(r'pid=' + str(pid) + r'[,)]', line)]
            if len(owned) != 1 or len(owned[0]) < 4 or owned[0][2] != '0':
                raise RuntimeError('broker listener ownership/backlog is not verified idle')
            # A worker or unobserved child is unexplained ownership, not idle.
            children = Path(f'/proc/{pid}/task/{pid}/children').read_text().strip()
            if children:
                raise RuntimeError('installed broker has child processes')
        for child in STATE.iterdir():
            if child.name == 'gadget.lock':
                continue
            trusted(child, directory=True)
            if any(child.iterdir()):
                raise RuntimeError('existing ownership journals are not empty')
        gadget = Path('/sys/kernel/config/usb_gadget')
        if gadget.exists() and any((p / 'UDC').read_text().strip() for p in gadget.iterdir()):
            raise RuntimeError('existing bound gadgets require operator ownership review')

    def preflight(self, saved):
        trusted(STATE, directory=True)
        self.assert_idle(saved)
        free_port(VHCI.read_text(), self.args.port)
        for uid in (self.args.client_uid, self.args.worker_uid, self.args.unauthorized_uid):
            if uid <= 0:
                raise RuntimeError('all test identities must be non-root')
            if pwd.getpwuid(uid).pw_gid == 0:
                raise RuntimeError('test identities must not have a root primary group')
        if len({self.args.client_uid, self.args.worker_uid, self.args.unauthorized_uid}) != 3:
            raise RuntimeError('client, worker and unauthorized identities must be distinct')

    def stop_original_socket(self):
        self.run(['systemctl', 'stop', ORIGINAL[0]])

    def stop_original_service(self):
        self.assert_idle(None)
        self.run(['systemctl', 'stop', ORIGINAL[1]])

    def remember(self, path, directory=False):
        self.owned.append((path, identity(path), directory))

    def create_dir(self, path):
        path.mkdir(mode=0o755)
        self.remember(path, True)

    def write(self, path, data, mode=0o600):
        descriptor = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, mode)
        self.remember(path)
        with os.fdopen(descriptor, 'wb') as output:
            output.write(data)
            output.flush()
            os.fsync(output.fileno())

    def prepare(self):
        self.root = Path(tempfile.mkdtemp(prefix='virtualgamepad-alpha-', dir='/run'))
        self.remember(self.root, True)
        self.root.chmod(0o755)
        self.instance = self.root.name
        self.create_dir(self.root / 'bin')
        self.create_dir(self.root / 'socket')
        self.create_dir(STATE / self.instance)
        self.create_dir(STATE / (self.instance + '.audio'))
        for source, expected, name in ((self.args.broker, self.args.broker_hash, 'gr-privileged-broker'),
                                      (self.args.worker, self.args.worker_hash, 'gr-audio-worker')):
            fd = os.open(source, os.O_RDONLY | os.O_NOFOLLOW)
            with os.fdopen(fd, 'rb') as image:
                if not stat.S_ISREG(os.fstat(image.fileno()).st_mode):
                    raise RuntimeError('candidate image must be a regular file')
                data = image.read(128 * 1024 * 1024 + 1)
            if len(data) > 128 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != expected:
                raise RuntimeError('candidate image differs from approved hash')
            self.write(self.root / 'bin' / name, data, 0o755)
        worker = pwd.getpwuid(self.args.worker_uid)
        config = (f'allow_uid={self.args.client_uid}\ninstance={self.instance}\n'
                  f'allow_vhci_port={self.args.port}\nworker_uid={worker.pw_uid}\nworker_gid={worker.pw_gid}\n')
        self.write(self.root / 'broker.conf', config.encode())
        self.service = self.instance + '.service'
        self.socket = self.instance + '.socket'
        socket_text = (f'[Socket]\nListenStream={self.root}/socket/broker.sock\nSocketMode=0660\n'
                       f'SocketGroup={pwd.getpwuid(self.args.client_uid).pw_gid}\nRemoveOnStop=yes\nService={self.service}\n')
        service_text = (f'[Service]\nExecStart={self.root}/bin/gr-privileged-broker --socket-activation --config {self.root}/broker.conf\n'
            f'PrivateMounts=yes\nBindReadOnlyPaths={self.root}/bin:/usr/libexec/virtualgamepad\n'
            'ProtectSystem=strict\nProtectHome=yes\nPrivateTmp=yes\nNoNewPrivileges=yes\n'
            'CapabilityBoundingSet=CAP_SYS_ADMIN CAP_SETUID CAP_SETGID\nRestrictAddressFamilies=AF_UNIX\n'
            'ProtectKernelModules=yes\nKillMode=control-group\nTimeoutStopSec=10\n'
            f'ReadWritePaths={STATE} /sys/devices/platform/vhci_hcd.0/attach\n')
        for name, text in ((self.service, service_text), (self.socket, socket_text)):
            self.write(Path('/run/systemd/system') / name, text.encode())
            self.units.append(name)
        self.run(['systemctl', 'daemon-reload'])

    def start_candidate(self):
        free_port(VHCI.read_text(), self.args.port)
        self.run(['systemctl', 'start', self.socket])

    def execute(self):
        account = pwd.getpwuid(self.args.client_uid)
        self.client_attempted = True
        self.run(['systemd-run', '--wait', '--pipe', '--collect', '--unit=' + self.instance + '-client',
                  '--uid=' + str(account.pw_uid), '--gid=' + str(account.pw_gid),
                  '--property=SupplementaryGroups=', '--property=NoNewPrivileges=yes',
                  '--property=PrivateMounts=yes', '--property=KillMode=control-group',
                  '--property=RuntimeMaxSec=' + str(self.args.timeout),
                  '--property=BindPaths=' + str(self.root / 'socket') + ':/run/virtualgamepad',
                  '--', *self.args.command], timeout=self.args.timeout + 20)

    def stop_candidate(self):
        errors = []
        clients = [self.instance + '-client.service'] if self.client_attempted else []
        for unit in clients + list(reversed(self.units)):
            try:
                state = self.run(['systemctl', 'show', unit, '-p', 'LoadState', '--value']).strip()
                if state != 'not-found':
                    self.run(['systemctl', 'stop', unit])
            except subprocess.SubprocessError as error:
                errors.append(str(error))
        if errors:
            raise RuntimeError('; '.join(errors))

    def cleanup(self):
        if not self.owned:
            return
        for unit in self.units:
            state = self.run(['systemctl', 'show', unit, '-p', 'ActiveState', '--value']).strip()
            if state not in ('inactive', 'failed'):
                raise RuntimeError('candidate unit is not stopped; evidence retained')
        deadline = time.monotonic() + 5
        while True:
            try:
                free_port(VHCI.read_text(), self.args.port)
                break
            except RuntimeError:
                if time.monotonic() >= deadline:
                    raise RuntimeError('port cleanup unverified; owned evidence retained')
                time.sleep(.05)
        for path, expected, directory in reversed(self.owned):
            remove_owned(path, expected, directory)
        self.run(['systemctl', 'daemon-reload'])

    def restore(self, saved):
        errors = []
        for unit in reversed(ORIGINAL):
            try:
                self.run(['systemctl', 'start' if saved[unit]['ActiveState'] == 'active' else 'stop', unit])
            except subprocess.SubprocessError as error:
                errors.append(str(error))
        if errors:
            raise RuntimeError('; '.join(errors))
        if {k: v['ActiveState'] for k, v in self.snapshot().items()} != {k: v['ActiveState'] for k, v in saved.items()}:
            raise RuntimeError('original service state was not restored')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--broker', type=Path)
    parser.add_argument('--worker', type=Path)
    parser.add_argument('--broker-hash')
    parser.add_argument('--worker-hash')
    parser.add_argument('--revision')
    parser.add_argument('--client-uid', type=int)
    parser.add_argument('--worker-uid', type=int)
    parser.add_argument('--unauthorized-uid', type=int)
    parser.add_argument('--port', type=int)
    parser.add_argument('--timeout', type=int, default=300)
    parser.add_argument('--report', type=Path)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.apply:
        print(json.dumps(dict(apply=False, phases=['preflight', 'prepare', 'execute', 'restore'],
              requires=['administrator-run root-owned script', 'approved candidate hashes',
                        'idle original service and empty journals', 'free authorized VHCI port',
                        'three non-root identities', 'absolute unprivileged test command'],
              replaces_installed_files=False, bypasses_global_lock=False), indent=2))
        return 0
    if os.geteuid() != 0:
        parser.error('--apply requires administrator privileges')
    script = Path(__file__).absolute()
    trusted(script)
    for parent in script.parents:
        trusted(parent, directory=True)
    if args.command and args.command[0] == '--':
        args.command = args.command[1:]
    if (any(getattr(args, name) is None for name in ('broker', 'worker', 'broker_hash', 'worker_hash',
            'revision', 'client_uid', 'worker_uid', 'unauthorized_uid', 'port', 'report')) or
            not args.command or not Path(args.command[0]).is_absolute() or
            not 1 <= args.timeout <= 7260 or not 0 <= args.port <= 65535 or
            not re.fullmatch('[0-9a-f]{40}', args.revision) or
            not all(re.fullmatch('[0-9a-f]{64}', h) for h in (args.broker_hash, args.worker_hash))):
        parser.error('complete approved candidate/identity/port/report arguments are required')
    for parent in args.report.absolute().parents:
        trusted(parent, directory=True)
    host = Host(args)
    receipt = dict(revision=args.revision, broker_hash=args.broker_hash, worker_hash=args.worker_hash)
    # Reserve output before any maintenance: never overwrite a user-controlled path.
    fd = os.open(args.report, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
    status = 0
    def interrupted(signum, frame):
        del frame
        raise InterruptedError('lab interrupted by signal ' + str(signum))
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        maintenance(host)
        receipt['status'] = 'passed'
    except BaseException as error:
        receipt.update(status='failed', error=str(error))
        status = 1
    finally:
        receipt['commands'] = host.events
        with os.fdopen(fd, 'w') as output:
            json.dump(receipt, output, indent=2)
            output.write('\n')
    return status


if __name__ == '__main__':
    sys.exit(main())
