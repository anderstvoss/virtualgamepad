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
import socket
import struct
import threading
import subprocess
import sys
import tempfile
import time

STATE = Path('/run/virtualgamepad-state')
VHCI = Path('/sys/devices/platform/vhci_hcd.0/status')
STAGING_PARENT = Path('/var/lib')
OUTPUT_LIMIT = 1024 * 1024
SOCKET_TRIGGER_INTERVAL = 2
ORIGINAL = ('virtualgamepad-broker.socket', 'virtualgamepad-broker.service')
RULES = Path('/run/udev/rules.d')
USB = Path('/sys/bus/usb/devices')


def trusted(path, directory=False):
    meta = path.lstat()
    if (meta.st_uid != 0 or meta.st_mode & 0o022 or
            not (stat.S_ISDIR(meta.st_mode) if directory else stat.S_ISREG(meta.st_mode))):
        raise RuntimeError('expected root-owned, non-symlink, non-writable lab path')


def require_executable_staging():
    for parent in (STAGING_PARENT, *STAGING_PARENT.parents):
        trusted(parent, directory=True)
    if os.statvfs(STAGING_PARENT).f_flag & os.ST_NOEXEC:
        raise RuntimeError('candidate staging filesystem is noexec; refusing maintenance')


def allocate_staging_directory():
    require_executable_staging()
    return Path(tempfile.mkdtemp(prefix='virtualgamepad-alpha-', dir=STAGING_PARENT))


def broker_config(client_uid, instance, ports, worker_uid, worker_gid):
    # Build lines explicitly: adjacent string literals before .join() would
    # otherwise turn the configuration prefix into the port separator.
    audio_isolation_rule(instance)
    lines = [f'allow_uid={client_uid}', f'instance={instance}']
    lines.extend(f'allow_vhci_port={port}' for port in ports)
    lines.extend((f'worker_uid={worker_uid}', f'worker_gid={worker_gid}'))
    return '\n'.join(lines) + '\n'


def instance_name(path):
    # tempfile's suffix alphabet includes underscores; broker instances don't.
    name = path.name.replace('_', '-')
    if not re.fullmatch(r'[a-z0-9-]{1,32}', name):
        raise RuntimeError('staging name cannot represent a broker instance')
    return name


def audio_isolation_rule(instance):
    """Build a serial AND VHCI ancestry rule, never a shared profile mutation.

    Parent keys on one udev rule must match the same ancestor. USB serial and
    platform VHCI ancestry therefore require separate stages. The first rule
    sets a private property, and the second requires both it and the platform.
    Installing this rule must precede the first candidate attachment.
    """
    if not re.fullmatch(r'[a-z0-9-]{1,32}', instance):
        raise RuntimeError('invalid audio isolation instance')
    return (
        'SUBSYSTEM=="sound", KERNEL=="card[0-9]*", '
        f'ATTRS{{serial}}=="vg-{instance}-*", '
        f'ENV{{VG_ALPHA_AUDIO_INSTANCE}}="{instance}"\n'
        'SUBSYSTEM=="sound", KERNEL=="card[0-9]*", '
        f'ENV{{VG_ALPHA_AUDIO_INSTANCE}}=="{instance}", '
        'SUBSYSTEMS=="platform", KERNELS=="vhci_hcd.0", ENV{ACP_IGNORE}="1"\n'
    )


def remaining_audio_devices(instance, root=USB):
    """A serial is an exclusion selector, never authority to remove a device."""
    audio_isolation_rule(instance)  # Validate before constructing any selector.
    found = []
    for entry in root.iterdir():
        try:
            serial = (entry / 'serial').read_text().strip()
        except FileNotFoundError:
            continue  # Most USB interfaces have no serial attribute.
        if serial.startswith(f'vg-{instance}-'):
            resolved = entry.resolve(strict=True)
            if 'vhci_hcd.0' not in resolved.parts:
                raise RuntimeError('audio serial has unexpected ancestry; refusing restoration')
            found.append(entry.name)
    return sorted(found)


class AudioIsolation:
    """Own one temporary rule; register identities even after partial startup."""
    def __init__(self, instance, run, directory=RULES, inventory=remaining_audio_devices):
        self.instance = instance
        self.run = run
        self.directory = directory
        self.path = directory / f'99-{instance}-audio.rules'
        self.inventory = inventory
        self.directory_identity = None
        self.parent_identity = None
        self.rule_identity = None
        self.rule_digest = None
        self.reload_pending = False

    def prepare(self):
        data = audio_isolation_rule(self.instance).encode()
        if self.inventory(self.instance):
            raise RuntimeError('audio isolation requires no existing session devices')
        for parent in self.directory.parents:
            trusted(parent, directory=True)
        if self.directory.exists() or self.directory.is_symlink():
            trusted(self.directory, directory=True)
        else:
            self.directory.mkdir(mode=0o755)
            self.directory_identity = identity(self.directory)
        self.parent_identity = identity(self.directory)
        fd = os.open(self.path, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o644)
        self.rule_identity = identity(self.path)
        self.reload_pending = True
        try:
            with os.fdopen(fd, 'wb') as output:
                output.write(data)
                output.flush()
                os.fsync(output.fileno())
        finally:
            # A failed write is still ours, with its observed partial contents.
            self.rule_digest = hashlib.sha256(self.path.read_bytes()).hexdigest()
        self.run(['udevadm', 'verify', str(self.path)])
        self.run(['udevadm', 'control', '--reload-rules'])
        # Do not trigger existing devices. New attachments receive the rule.

    def restore(self, timeout=5):
        if (self.parent_identity is not None and
                (self.rule_identity is not None or self.reload_pending) and
                identity(self.directory) != self.parent_identity):
            raise RuntimeError('audio isolation directory changed; refusing restoration')
        if self.rule_identity is not None:
            deadline = time.monotonic() + timeout
            while self.inventory(self.instance):
                if time.monotonic() >= deadline:
                    raise RuntimeError('owned audio devices remain; isolation rule retained')
                time.sleep(.05)
            if (identity(self.path) != self.rule_identity or
                    hashlib.sha256(self.path.read_bytes()).hexdigest() != self.rule_digest):
                raise RuntimeError('audio isolation rule changed; refusing removal')
            remove_owned(self.path, self.rule_identity)
            self.rule_identity = None
        if self.reload_pending:
            self.run(['udevadm', 'control', '--reload-rules'])
            self.reload_pending = False
        if self.directory_identity is not None:
            remove_owned(self.directory, self.directory_identity, directory=True)
            self.directory_identity = None


PHASES = ('rejection', 'provider-lifecycle', 'provider-client-exit', 'provider-client-before-handoff', 'provider-worker-death', 'provider-broker-death', 'provider-siblings-admission', 'usb-functional')


def selected_ports(first, additional=()):
    ports = (first, *additional)
    if (not 1 <= len(ports) <= 4 or len(set(ports)) != len(ports) or
            any(type(port) is not int or not 0 <= port <= 65535 for port in ports) or
            ports != tuple(sorted(ports))):
        raise ValueError('invalid explicit port allowlist')
    return ports


def phase_command(phase, images, instance, port, additional=()):
    """Only predefined ordinary-client probes; never a privileged shell hook."""
    if phase not in PHASES:
        raise ValueError('unknown predefined provider phase')
    audio_isolation_rule(instance)
    if type(port) is not int or not 0 <= port <= 65535:
        raise ValueError('invalid authorized phase port')
    ports = selected_ports(port, additional)
    if phase == 'provider-siblings-admission':
        if len(ports) != 4: raise ValueError('sibling/admission phase requires four explicitly authorized ports')
        return ['/usr/bin/python3','-I',str(images/'validate-broker-lifecycle-live.py'),
                '--instance',instance,'--port',str(port),'--scenario','siblings-admission',
                '--ports',*[str(value) for value in ports]]
    if phase == 'rejection':
        return ['/usr/bin/python3', '-I', str(images / 'validate-broker-rejection-live.py')]
    if phase == 'usb-functional':
        return ['/usr/bin/python3', '-I', str(images / 'validate-broker-audio-live.py'),
                '--instance', instance, '--profile', 'all', '--seconds', '3']
    return ['/usr/bin/python3', '-I', str(images / 'validate-broker-lifecycle-live.py'),
            '--instance', instance, '--port', str(port), '--scenario',
            {'provider-client-exit': 'client-exit', 'provider-client-before-handoff': 'client-before-handoff', 'provider-worker-death': 'worker-death', 'provider-broker-death': 'broker-death'}.get(phase, 'normal'), '--ports', *[str(value) for value in ports]]


def journal_port(data, generation, device, allowed):
    for port in allowed:
        if data == f'1 {generation} {device} {port}\n'.encode(): return port
    raise RuntimeError('fault request differs from root-owned journal or authorized ports')


def installed_executable(properties):
    value = properties.get('ExecStart', '').strip()
    match = re.fullmatch(r'\{ path=(/[^;\n]+) ; argv\[\]=[^\n]* \}', value)
    if not match or value.count('{ path=') != 1:
        raise RuntimeError('installed executable provenance is unavailable or ambiguous')
    return Path(match[1])


def identity(path):
    info = path.lstat()
    return info.st_dev, info.st_ino


def rewrite_owned_record(path, expected, current, replacement):
    """Change only a held lab record for a predefined hostile-journal trial."""
    if len(current) > 256 or len(replacement) > 256:
        raise ValueError('bounded journal fixture required')
    descriptor = os.open(path, os.O_RDWR | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, 'r+b') as output:
        info = os.fstat(output.fileno())
        if ((info.st_dev, info.st_ino) != expected or not stat.S_ISREG(info.st_mode) or
                output.read(257) != current):
            raise RuntimeError('owned journal changed; refusing fixture mutation')
        output.seek(0); output.truncate(); output.write(replacement); output.flush()
        os.fsync(output.fileno())
    if identity(path) != expected or path.read_bytes() != replacement:
        raise RuntimeError('owned journal changed during fixture mutation')


def fingerprint(path):
    for parent in path.parents:
        trusted(parent, directory=True)
    if not path.exists() and not path.is_symlink():
        return dict(absent=True)
    trusted(path)
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor, 'rb') as source:
        metadata = os.fstat(source.fileno())
        digest = hashlib.sha256()
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(chunk)
    return dict(device=metadata.st_dev, inode=metadata.st_ino, sha256=digest.hexdigest())


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


def require_no_clients(text, pid, journald_pid=None):
    rows = [line.split() for line in text.splitlines()]
    owner = re.compile(r'pid=' + str(pid) + r',fd=(\d+)\)')
    for row in rows:
        fds = {int(value) for value in owner.findall(' '.join(row))}
        if not fds:
            continue
        # Stdio may be a systemd journal stream. Exempt it only when the
        # configured journal peer and the reciprocal socket identity are proven.
        if (journald_pid and fds <= {1, 2} and len(row) >= 9 and
                row[:2] == ['u_str', 'ESTAB']):
            peers = [peer for peer in rows if len(peer) >= 9 and
                     peer[:2] == ['u_str', 'ESTAB'] and
                     peer[4] == '/run/systemd/journal/stdout' and
                     peer[5] == row[7] and peer[7] == row[5] and
                     re.search(r'pid=' + str(journald_pid) + r',fd=\d+\)', ' '.join(peer))]
            if len(peers) == 1:
                continue
        raise RuntimeError('installed broker has connected clients or unverified sockets')


def process_children(pid, proc_root=Path('/proc')):
    """Linux children are per-thread; inspecting only the leader misses workers."""
    root = proc_root / str(pid)
    def started():
        # comm can contain spaces/parentheses; field 22 follows the closing comm.
        fields = (root / 'stat').read_text().rsplit(')', 1)[1].split()
        return int(fields[19])
    try:
        before = started()
        tasks = set((root / 'task').iterdir())
        if root / 'task' / str(pid) not in tasks:
            raise ValueError('process leader is absent')
        children = set()
        for task in tasks:
            if not task.name.isdigit():
                raise ValueError('invalid task identity')
            for value in (task / 'children').read_text().split():
                if not value.isdigit() or int(value) <= 0:
                    raise ValueError('invalid child identity')
                children.add(int(value))
        if before != started() or tasks != set((root / 'task').iterdir()):
            raise ValueError('process identity or task inventory changed')
        return sorted(children)
    except (OSError, ValueError, IndexError) as error:
        raise RuntimeError('process child ownership could not be verified') from error


def worker_snapshot(pid, proc=Path('/proc')):
    root = proc / str(pid)
    status = dict(line.split(':', 1) for line in (root / 'status').read_text().splitlines() if ':' in line)
    fields = (root / 'stat').read_text().rsplit(')', 1)[1].split()
    executable = (root / 'exe').stat()
    arguments = (root / 'cmdline').read_bytes()
    if len(arguments) > 4096: raise RuntimeError('oversized worker launch identity')
    return dict(start=int(fields[19]), parent=int(fields[1]), uids=status['Uid'].split(),
                caps=int(status['CapEff'].strip(), 16), nnp=status['NoNewPrivs'].strip(),
                groups=status['Groups'].split(), image=(executable.st_dev, executable.st_ino),
                cgroups=(root / 'cgroup').read_text().splitlines(), argv=arguments.split(b'\0')[:-1])


def validate_worker(snapshot, parent, uid, image, cgroup, instance, generation, device):
    argv = snapshot['argv']
    if (snapshot['parent'] != parent or snapshot['uids'] != [str(uid)]*4 or
            snapshot['caps'] != 0 or snapshot['nnp'] != '1' or snapshot['groups'] or
            snapshot['image'] != image or snapshot['cgroups'] != ['0::' + cgroup] or
            len(argv) != 9 or argv[1] not in (b'dualsense', b'dualshock4', b'xbox360') or
            argv[2:4] != [str(device).encode(), str(generation).encode()] or
            argv[5] != instance.encode()):
        raise RuntimeError('worker process identity is not the owned session')


class PinnedWorker:
    """A pidfd capability, acquired only after matching root-owned evidence."""
    def __init__(self, pid, verify):
        before = worker_snapshot(pid)
        verify(before)
        self.descriptor = os.pidfd_open(pid)
        try:
            after = worker_snapshot(pid)
            verify(after)
            if before != after: raise RuntimeError('worker identity changed during reservation')
        except BaseException:
            os.close(self.descriptor)
            self.descriptor = None
            raise

    def kill(self):
        if self.descriptor is None: raise RuntimeError('owned process handle is closed')
        try: signal.pidfd_send_signal(self.descriptor, signal.SIGKILL)
        except ProcessLookupError: pass  # The held capability cannot select a replacement PID.

    def close(self):
        if self.descriptor is not None:
            os.close(self.descriptor)
            self.descriptor = None


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
        self.unit_images = {}
        self.root = None
        self.client_attempted = False
        self.client_units = []
        self.client_receipts = []
        self.original_images = {}
        self.events = []
        self.audio_isolation = None

    def run(self, argv, timeout=15):
        # Regular-file spooling plus inherited/unit quotas prevent a verbose
        # validator from exhausting the privileged supervisor's memory or disk.
        with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
            result = subprocess.run(['/usr/bin/prlimit', '--fsize='+str(OUTPUT_LIMIT), '--', *argv],
                                    stdout=stdout, stderr=stderr, timeout=timeout,
                                    env={'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C'})
            stdout.seek(0); stderr.seek(0)
            output = stdout.read(OUTPUT_LIMIT + 1)
            diagnostic = stderr.read(OUTPUT_LIMIT + 1)
        event = dict(command=argv, status=result.returncode)
        self.events.append(event)
        if len(output) > OUTPUT_LIMIT or len(diagnostic) > OUTPUT_LIMIT:
            raise RuntimeError('lab command exceeded output quota')
        if result.returncode:
            event.update(stdout=output.decode('utf-8', errors='replace'),
                         stderr=diagnostic.decode('utf-8', errors='replace'))
            raise subprocess.CalledProcessError(result.returncode, argv, output, diagnostic)
        return output.decode('utf-8', errors='strict')

    def snapshot(self):
        saved = {}
        for unit in ORIGINAL:
            output = self.run(['systemctl', 'show', unit, '-p', 'ActiveState', '-p', 'MainPID', '-p', 'FragmentPath', '-p', 'ExecStart'])
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
        if pid:
            logging = dict(line.split('=', 1) for line in self.run(
                ['systemctl', 'show', ORIGINAL[1], '-p', 'StandardOutput', '-p', 'StandardError']).splitlines())
            journald_pid = None
            if logging.get('StandardOutput') == 'journal' and logging.get('StandardError') in ('journal', 'inherit'):
                journald_pid = int(self.run(['systemctl', 'show', 'systemd-journald.service', '-p', 'MainPID', '--value']).strip())
            require_no_clients(sockets, pid, journald_pid)
            listeners = self.run(['ss', '-H', '-xlnp'])
            owned = [line.split() for line in listeners.splitlines()
                     if re.search(r'pid=' + str(pid) + r'[,)]', line)]
            if len(owned) != 1 or len(owned[0]) < 4 or owned[0][2] != '0':
                raise RuntimeError('broker listener ownership/backlog is not verified idle')
            # A worker or unobserved child is unexplained ownership, not idle.
            children = process_children(pid)
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
        require_executable_staging()
        trusted(STATE, directory=True)
        trusted(Path('/usr/bin/setpriv'))
        trusted(Path('/usr/bin/prlimit'))
        self.assert_idle(saved)
        # The original installation is restoration evidence, never candidate
        # provenance. Preserve exact images/configuration rather than replacing them.
        for path in (installed_executable(saved[ORIGINAL[1]]),
                     Path('/usr/libexec/virtualgamepad/gr-privileged-broker'),
                     Path('/usr/libexec/virtualgamepad/gr-audio-worker'),
                     Path('/etc/virtualgamepad/broker.conf')):
            self.original_images[str(path)] = fingerprint(path)
        for properties in saved.values():
            path = Path(properties['FragmentPath'])
            if not path.is_absolute():
                raise RuntimeError('original unit provenance is unavailable')
            self.original_images[str(path)] = fingerprint(path)
        self.events.append(dict(original_installation=self.original_images.copy()))
        for port in selected_ports(self.args.port, self.args.additional_port):
            free_port(VHCI.read_text(), port)
        for uid in (self.args.client_uid, self.args.worker_uid, self.args.unauthorized_uid):
            if uid <= 0:
                raise RuntimeError('all test identities must be non-root')
            if pwd.getpwuid(uid).pw_gid == 0:
                raise RuntimeError('test identities must not have a root primary group')
        if len({self.args.client_uid, self.args.worker_uid, self.args.unauthorized_uid}) != 3:
            raise RuntimeError('client, worker and unauthorized identities must be distinct')

    def stop_original_socket(self):
        self.run(['systemctl', 'stop', ORIGINAL[0]])
        if Path('/run/virtualgamepad/broker.sock').exists():
            raise RuntimeError('original socket remains reachable; refusing service stop')

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
        self.root = allocate_staging_directory()
        self.remember(self.root, True)
        self.root.chmod(0o755)
        self.instance = instance_name(self.root)
        if self.args.isolate_audio:
            self.audio_isolation = AudioIsolation(self.instance, self.run)
            self.audio_isolation.prepare()
        self.create_dir(self.root / 'bin')
        self.create_dir(self.root / 'socket')
        self.create_dir(STATE / self.instance)
        self.create_dir(STATE / (self.instance + '.audio'))
        for source, expected, name in ((self.args.broker, self.args.broker_hash, 'gr-privileged-broker'),
                                      (self.args.worker, self.args.worker_hash, 'gr-audio-worker')):
            fd = os.open(source, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
            with os.fdopen(fd, 'rb') as image:
                if not stat.S_ISREG(os.fstat(image.fileno()).st_mode):
                    raise RuntimeError('candidate image must be a regular file')
                data = image.read(128 * 1024 * 1024 + 1)
            if len(data) > 128 * 1024 * 1024 or hashlib.sha256(data).hexdigest() != expected:
                raise RuntimeError('candidate image differs from approved hash')
            self.write(self.root / 'bin' / name, data, 0o755)
        worker = pwd.getpwuid(self.args.worker_uid)
        config = broker_config(self.args.client_uid, self.instance,
                               selected_ports(self.args.port, self.args.additional_port),
                               worker.pw_uid, worker.pw_gid)
        self.write(self.root / 'broker.conf', config.encode())
        self.service = self.instance + '.service'
        self.socket = self.instance + '.socket'
        mode = '0666' if self.args.unauthorized_probe else '0660'
        socket_text = (f'[Socket]\nListenStream={self.root}/socket/broker.sock\nSocketMode={mode}\n'
                       f'SocketGroup={pwd.getpwuid(self.args.client_uid).pw_gid}\nRemoveOnStop=yes\nService={self.service}\n'
                       f'TriggerLimitIntervalSec={SOCKET_TRIGGER_INTERVAL}\nTriggerLimitBurst=20\n')
        service_text = (f'[Service]\nExecStart={self.root}/bin/gr-privileged-broker --socket-activation --config {self.root}/broker.conf\n'
            f'PrivateMounts=yes\nBindReadOnlyPaths={self.root}/bin:/usr/libexec/virtualgamepad\n'
            'ProtectSystem=strict\nProtectHome=yes\nPrivateTmp=yes\nNoNewPrivileges=yes\n'
            'CapabilityBoundingSet=CAP_SYS_ADMIN CAP_SETUID CAP_SETGID\nRestrictAddressFamilies=AF_UNIX\n'
            'ProtectKernelModules=yes\nKillMode=control-group\nTimeoutStopSec=10\nMemoryHigh=768M\nMemoryMax=1G\nMemorySwapMax=0\n'
            f'ReadWritePaths={STATE} /sys/devices/platform/vhci_hcd.0/attach\n')
        for name, text in ((self.service, service_text), (self.socket, socket_text)):
            self.write(Path('/run/systemd/system') / name, text.encode())
            self.units.append(name)
            self.unit_images[name] = fingerprint(Path("/run/systemd/system") / name)
        self.run(['systemctl', 'daemon-reload'])

    def start_candidate(self):
        free_port(VHCI.read_text(), self.args.port)
        self.run(['systemctl', 'start', self.socket])

    def execute(self):
        if self.args.phase:
            command = phase_command(self.args.phase, self.args.probe_directory, self.instance, self.args.port, self.args.additional_port)
            if self.args.phase in ('provider-worker-death','provider-broker-death'):
                self.worker_death(command)
            else:
                self.run_client(self.args.client_uid, command, 'client')
            if self.args.phase in ('provider-client-exit', 'provider-client-before-handoff'):
                deadline = time.monotonic() + 5
                while True:
                    try:
                        free_port(VHCI.read_text(), self.args.port)
                        if any((STATE / (self.instance+'.audio')).iterdir()):
                            raise RuntimeError('owned journal cleanup remains pending')
                        # A pre-handoff close can race construction. Confirm the
                        # broker has no worker children, not just a momentary free port.
                        parent = int(self.run(['systemctl','show',self.service,'-p','MainPID','--value']).strip())
                        if parent and process_children(parent): raise RuntimeError('owned construction is still active')
                        break
                    except RuntimeError:
                        if time.monotonic() >= deadline: raise
                        time.sleep(.02)
                command = phase_command('provider-lifecycle', self.args.probe_directory, self.instance, self.args.port, self.args.additional_port)
                self.run_client(self.args.client_uid, command, 'after-client-exit')
        else:
            self.run_client(self.args.client_uid, self.args.command, 'client')
        if self.args.unauthorized_probe:
            self.run_client(self.args.unauthorized_uid,
                            ['/usr/bin/python3', '-I', str(self.args.unauthorized_probe), '--unauthorized'],
                            'unauthorized')
        if self.args.restart_empty:
            self.restart_empty_candidate()

    def worker_death(self, command):
        path = self.root / 'fault.sock'
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        listener.bind(str(path)); self.remember(path)
        os.chown(path, 0, pwd.getpwuid(self.args.client_uid).pw_gid)
        path.chmod(0o660); listener.listen(1); listener.settimeout(.2)
        cancelled = threading.Event()
        failures = []
        def supervise():
            try:
                for _ in range(3):
                    deadline = time.monotonic() + 20
                    while True:
                        if cancelled.is_set(): return
                        try: peer, _ = listener.accept(); break
                        except socket.timeout:
                            if time.monotonic() >= deadline: raise TimeoutError('owned fault client did not become ready')
                    with peer:
                        peer.settimeout(5)
                        client_pid, uid, _ = struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
                        client_group = self.run(['systemctl', 'show', self.instance+'-client.service', '-p', 'ControlGroup', '--value']).strip()
                        if uid != self.args.client_uid or (Path('/proc')/str(client_pid)/'cgroup').read_text().splitlines() != ['0::'+client_group]:
                            raise RuntimeError('fault request did not originate in the owned client unit')
                        data = bytearray()
                        while not data.endswith(b'\n'):
                            chunk = peer.recv(128)
                            if not chunk: raise EOFError('fault readiness ended')
                            data.extend(chunk)
                            if len(data)>256: raise ValueError('oversized fault readiness')
                        request = json.loads(data)
                        if (set(request) != {'generation', 'device'} or
                                type(request['generation']) is not int or not 0 < request['generation'] < 2**64 or
                                type(request['device']) is not int or not 0 < request['device'] < 2**32):
                            raise ValueError('invalid fault readiness identity')
                        generation, device = request['generation'], request['device']
                        record = STATE / (self.instance+'.audio') / f'{generation:016x}'
                        trusted(record)
                        record_identity = identity(record)
                        owned_port = journal_port(record.read_bytes(), generation, device,
                                                  selected_ports(self.args.port, self.args.additional_port))
                        parent = int(self.run(['systemctl', 'show', self.service, '-p', 'MainPID', '--value']).strip())
                        group = self.run(['systemctl', 'show', self.service, '-p', 'ControlGroup', '--value']).strip()
                        image = identity(self.root/'bin/gr-audio-worker')
                        held = []
                        try:
                            for pid in process_children(parent):
                                try:
                                    verify = lambda snapshot: validate_worker(snapshot, parent, self.args.worker_uid,
                                        image, group, self.instance, generation, device)
                                    held.append(PinnedWorker(pid, verify))
                                except RuntimeError: continue
                            if len(held) != 1: raise RuntimeError('owned session worker is absent or ambiguous')
                            if self.args.phase == 'provider-broker-death':
                                self.broker_death(parent, group, held[0], record, record_identity, generation, device, owned_port)
                                peer.sendall(b'B')
                            else:
                                held[0].kill()
                                self.events.append(dict(injected_worker_death=dict(generation=generation, device=device, pidfd=True)))
                                peer.sendall(b'K')
                        finally:
                            for worker in held: worker.close()
            except BaseException as error: failures.append(str(error))
        worker = threading.Thread(target=supervise)
        worker.start()
        initiating = None
        try:
            self.run_client(self.args.client_uid, [*command, '--fault-socket', str(path)], 'client')
        except BaseException as error: initiating = str(error)
        finally:
            cancelled.set(); worker.join(timeout=10); listener.close()
        if worker.is_alive(): failures.append('owned fault supervisor did not terminate')
        if initiating is not None or failures:
            raise RuntimeError(json.dumps(dict(initiating=initiating, supervisor=failures)))
        self.run_client(self.args.client_uid,
                        phase_command('provider-lifecycle', self.args.probe_directory, self.instance, self.args.port, self.args.additional_port),
                        'after-worker-death')

    def broker_death(self, parent, group, worker, record, expected, generation, device, port=None):
        import select
        image = identity(self.root/'bin/gr-privileged-broker')
        def verify(snapshot):
            if (snapshot['uids']!=['0']*4 or snapshot['image']!=image or
                    snapshot['cgroups']!=['0::'+group] or snapshot['nnp']!='1' or
                    snapshot['argv'][1:]!=[b'--socket-activation',b'--config',str(self.root/'broker.conf').encode()]):
                raise RuntimeError('broker process identity is not the staged owned unit')
        broker = PinnedWorker(parent,verify)
        self.owned.append((record, expected, False))  # Register before injected failure.
        data = f'1 {generation} {device} {self.args.port if port is None else port}\n'.encode()
        try:
            broker.kill()
            if not select.select([worker.descriptor],[],[],5)[0]:
                raise RuntimeError('owned worker survived broker death')
        finally: broker.close()
        self.run(['systemctl','stop',self.socket])
        self.run(['systemctl','stop',self.service])
        deadline=time.monotonic()+5
        while True:
            try:
                for port in selected_ports(self.args.port,self.args.additional_port): free_port(VHCI.read_text(),port)
                break
            except RuntimeError:
                if time.monotonic()>=deadline: raise RuntimeError('owned attachment survived broker death')
                time.sleep(.02)
        if identity(record)!=expected or record.read_bytes()!=data:
            raise RuntimeError('pending journal identity changed; operator restoration refused')
        self.reject_pending_startup(record, expected, data, 'pending-restart-' + str(generation))
        rejected = []
        for name, payload in [('truncated', b'1 '), ('malformed', b'not-a-valid-audio-record\n')]:
            rewrite_owned_record(record, expected, data, payload)
            self.reject_pending_startup(record, expected, payload, name + '-restart-' + str(generation))
            rewrite_owned_record(record, expected, payload, data)
            rejected.append(name)
        self.reject_replaced_journal(record, expected, data, generation)
        rejected.append('identity-replacement')
        # The root supervisor retained pidfds and exact inode/content while the
        # failure occurred. Reusable record fields alone never authorize cleanup.
        remove_owned(record,expected)
        self.restart_after_rejection()
        self.events.append(dict(broker_death_recovery=dict(generation=generation,device=device,
            worker_exit_verified=True,attachment_removed=True,pending_restart_rejected=True,
            journal_cleared_by_held_identity=True, hostile_journal_rejections=rejected)))

    def reject_replaced_journal(self, record, expected, data, generation):
        # Both identities are test-owned and registered before restart. The
        # original capability must never authorize clearing the replacement.
        if identity(record) != expected or record.read_bytes() != data:
            raise RuntimeError('original journal changed before replacement trial')
        # Preserve the inode with rename on the journal filesystem; /var/lib
        # staging can be a different filesystem from /run. Keep the backup
        # outside either provider's pending-record directory.
        backup = record.parent.parent / ('.' + self.instance + '-held-journal-' + str(generation))
        if backup.exists() or backup.is_symlink():
            raise RuntimeError('owned journal backup path occupied')
        self.owned.append((backup, expected, False))
        record.rename(backup)
        descriptor = os.open(record, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        info = os.fstat(descriptor)
        replacement = (info.st_dev, info.st_ino)
        self.owned.append((record, replacement, False))
        with os.fdopen(descriptor, 'wb') as output:
            output.write(data); output.flush(); os.fsync(output.fileno())
        try:
            self.reject_pending_startup(record, expected, data, 'identity-restart-' + str(generation))
        except RuntimeError as error:
            state = self.run(['systemctl', 'show', self.service, '-p', 'ActiveState', '--value']).strip()
            if ('journal preserved' not in str(error) or state != 'failed' or
                    identity(record) != replacement or record.read_bytes() != data):
                raise
        else:
            raise RuntimeError('replacement journal was accepted under the old identity')
        self.run(['systemctl', 'stop', self.socket]); self.run(['systemctl', 'stop', self.service])
        try:
            remove_owned(record, expected)
        except RuntimeError:
            if identity(record) != replacement or record.read_bytes() != data:
                raise RuntimeError('replacement record was altered by old-identity cleanup')
        else:
            raise RuntimeError('old identity unexpectedly cleared replacement journal')
        # Only the independently held identity of our injected fixture permits
        # its removal. Restore the original inode before ordinary lab recovery.
        remove_owned(record, replacement)
        if identity(backup) != expected or backup.read_bytes() != data:
            raise RuntimeError('original held journal changed; restoration refused')
        backup.rename(record)

    def restart_after_rejection(self):
        # reset-failed clears service start limits, but socket trigger rate
        # accounting survives stop/start. Respect the configured lab window;
        # never disable limits or alter installed units.
        # Stopped units may already be unloaded by systemd. Querying the
        # service loads its owned definition; only a retained failed state needs
        # reset. Failed units cannot be collected before that reset.
        state = self.run(['systemctl', 'show', self.service, '-p', 'ActiveState', '--value']).strip()
        if state == 'failed':
            self.run(['systemctl', 'reset-failed', self.service])
        elif state != 'inactive':
            raise RuntimeError('candidate is not stopped before rejection recovery')
        time.sleep(SOCKET_TRIGGER_INTERVAL + .1)
        self.start_candidate()

    def reject_pending_startup(self, record, expected, data, suffix):
        self.restart_after_rejection()
        self.run_client(self.args.client_uid,
            ['/usr/bin/python3','-I',str(self.args.probe_directory/'validate-broker-lifecycle-live.py'),
             '--instance',self.instance,'--port',str(self.args.port),'--scenario','startup-rejection'],
            suffix)
        state=self.run(['systemctl','show',self.service,'-p','ActiveState','--value']).strip()
        result = self.run(['systemctl', 'show', self.service, '-p', 'Result', '--value']).strip()
        status = self.run(['systemctl', 'show', self.service, '-p', 'ExecMainStatus', '--value']).strip()
        if state != 'failed' or result != 'exit-code' or status != '1':
            raise RuntimeError('candidate startup did not fail from broker execution')
        if identity(record)!=expected or record.read_bytes()!=data:
            raise RuntimeError('pending restart was not rejected with journal preserved')
        self.run(['systemctl','stop',self.socket]); self.run(['systemctl','stop',self.service])
        for port in selected_ports(self.args.port,self.args.additional_port): free_port(VHCI.read_text(),port)
        if int(self.run(['systemctl','show',self.service,'-p','MainPID','--value']).strip())!=0:
            raise RuntimeError('candidate broker remains alive; journal not cleared')

    def restart_empty_candidate(self):
        # Only an empty lab may restart: this does not test stale attachment
        # recovery and must never interrupt an unexplained worker or lease.
        for directory in (STATE / self.instance, STATE / (self.instance + '.audio')):
            recorded = next(item[1] for item in self.owned if item[0] == directory)
            if identity(directory) != recorded or any(directory.iterdir()):
                raise RuntimeError('candidate journal is changed or nonempty; refusing restart')
        free_port(VHCI.read_text(), self.args.port)
        old = int(self.run(['systemctl', 'show', self.service, '-p', 'MainPID', '--value']).strip())
        if old <= 0 or process_children(old):
            raise RuntimeError('candidate process is absent or has unexplained children')
        self.run(['systemctl', 'stop', self.socket])
        self.run(['systemctl', 'stop', self.service])
        if self.run(['systemctl', 'show', self.service, '-p', 'ActiveState', '--value']).strip() != 'inactive':
            raise RuntimeError('candidate did not stop before restart')
        self.start_candidate()
        self.run_client(self.args.client_uid, self.args.command, 'reconnected')
        new = int(self.run(['systemctl', 'show', self.service, '-p', 'MainPID', '--value']).strip())
        if new <= 0 or new == old:
            raise RuntimeError('candidate restart identity was not renewed')
        self.events.append(dict(empty_restart=dict(previous_pid=old, current_pid=new)))

    def run_client(self, uid, command, suffix):
        account = pwd.getpwuid(uid)
        self.client_attempted = True
        unit = self.instance + '-' + suffix + '.service'
        self.client_units.append(unit)  # Register before a partial systemd-run failure.
        output = self.run(['systemd-run', '--wait', '--pipe', '--collect', '--unit=' + unit,
                  '--setenv=XDG_RUNTIME_DIR=/run/user/' + str(account.pw_uid),
                  '--property=NoNewPrivileges=yes', '--property=MemoryHigh=384M',
                  '--property=MemoryMax=512M', '--property=MemorySwapMax=0',
                  '--property=CapabilityBoundingSet=CAP_SETUID CAP_SETGID CAP_SETPCAP',
                  '--property=PrivateMounts=yes', '--property=KillMode=control-group',
                  '--property=RuntimeMaxSec=' + str(self.args.timeout),
                  '--property=LimitFSIZE=' + str(OUTPUT_LIMIT),
                  '--property=BindPaths=' + str(self.root / 'socket') + ':/run/virtualgamepad',
                  '--', '/usr/bin/setpriv', '--reuid=' + str(account.pw_uid),
                  '--regid=' + str(account.pw_gid), '--clear-groups', '--bounding-set=-all',
                  '--inh-caps=-all', '--ambient-caps=-all', '--no-new-privs', '--',
                  *command], timeout=self.args.timeout + 20)
        self.client_receipts.append(dict(uid=uid, unit=unit, stdout=output))

    def stop_candidate(self):
        errors = []
        clients = list(reversed(self.client_units))
        for unit in clients + list(reversed(self.units)):
            try:
                state = self.run(['systemctl', 'show', unit, '-p', 'LoadState', '--value']).strip()
                if state != 'not-found':
                    if unit in self.units:
                        path = Path('/run/systemd/system') / unit
                        fragment = self.run(['systemctl', 'show', unit, '-p', 'FragmentPath', '--value']).strip()
                        if fragment != str(path) or fingerprint(path) != self.unit_images[unit]:
                            raise RuntimeError('candidate unit identity changed; refusing to stop it')
                    self.run(['systemctl', 'stop', unit])
            except (subprocess.SubprocessError, RuntimeError) as error:
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
                for port in selected_ports(self.args.port, self.args.additional_port):
                    free_port(VHCI.read_text(), port)
                break
            except RuntimeError:
                if time.monotonic() >= deadline:
                    raise RuntimeError('port cleanup unverified; owned evidence retained')
                time.sleep(.05)
        if self.audio_isolation is not None:
            self.audio_isolation.restore()
        for path, expected, directory in reversed(self.owned):
            remove_owned(path, expected, directory)
        self.run(['systemctl', 'daemon-reload'])

    def restore(self, saved):
        for path, expected in self.original_images.items():
            if fingerprint(Path(path)) != expected:
                raise RuntimeError('original installation changed; operator restoration required')
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
    parser.add_argument('--unauthorized-probe', type=Path,
                        help='root-owned probe; opens only the temporary lab socket to other UIDs')
    parser.add_argument('--isolate-audio', action='store_true',
                        help='install serial/VHCI ACP_IGNORE before candidate attachment')
    parser.add_argument('--restart-empty', action='store_true',
                        help='repeat client after restarting only an empty owned candidate')
    parser.add_argument('--phase', choices=PHASES)
    parser.add_argument('--probe-directory', type=Path,
                        help='immutable administrator-installed phase probe directory')
    parser.add_argument('--port', type=int)
    parser.add_argument('--additional-port', type=int, action='append', default=[], help='explicit unused additional port, at most three')
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
    if args.phase:
        if (args.command or args.probe_directory is None or
                not args.probe_directory.is_absolute() or args.restart_empty):
            parser.error('named phase requires probe directory, no arbitrary command or empty-restart option')
        if args.phase != 'rejection' and not args.isolate_audio:
            parser.error('positive phases require pre-attachment audio isolation')
        trusted(args.probe_directory, directory=True)
        for parent in args.probe_directory.parents:
            trusted(parent, directory=True)
        for name in ('validate-broker-rejection-live.py', 'validate-broker-lifecycle-live.py',
                     'validate-broker-audio-live.py', 'validate-usb-audio-live.py'):
            trusted(args.probe_directory / name)
    if args.command and args.command[0] == '--':
        args.command = args.command[1:]
    if (any(getattr(args, name) is None for name in ('broker', 'worker', 'broker_hash', 'worker_hash',
            'revision', 'client_uid', 'worker_uid', 'unauthorized_uid', 'port', 'report')) or
            (not args.phase and (not args.command or not Path(args.command[0]).is_absolute())) or
            not 1 <= args.timeout <= 7260 or not 0 <= args.port <= 65535 or
            not re.fullmatch('[0-9a-f]{40}', args.revision) or
            not all(re.fullmatch('[0-9a-f]{64}', h) for h in (args.broker_hash, args.worker_hash))):
        parser.error('complete approved candidate/identity/port/report arguments are required')
    try: selected_ports(args.port, args.additional_port)
    except ValueError as error: parser.error(str(error))
    for parent in args.report.absolute().parents:
        trusted(parent, directory=True)
    if args.unauthorized_probe:
        if not args.unauthorized_probe.is_absolute():
            parser.error('unauthorized probe must be an absolute root-owned path')
        trusted(args.unauthorized_probe)
        for parent in args.unauthorized_probe.parents:
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
        receipt['clients'] = host.client_receipts
        with os.fdopen(fd, 'w') as output:
            json.dump(receipt, output, indent=2)
            output.write('\n')
    return status


if __name__ == '__main__':
    sys.exit(main())
