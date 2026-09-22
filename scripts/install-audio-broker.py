#!/usr/bin/python3 -I
"""Explicit administrator installation of the stock-Linux audio broker."""
import argparse
import grp
import os
from pathlib import Path
import pwd
import stat
import subprocess
import tempfile

ENV = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C'}
WORKER = 'virtualgamepad-audio'
BASE = Path('/usr/libexec/virtualgamepad')


def trusted_directory(path):
    for item in reversed([path, *path.parents]):
        if not item.exists():
            item.mkdir(mode=0o755)
        m = item.lstat()
        if not stat.S_ISDIR(m.st_mode) or m.st_uid != 0 or m.st_mode & 0o022:
            raise ValueError('installation requires root-owned non-writable directory parents')


def install(path, data, mode, replace=False):
    trusted_directory(path.parent)
    if path.exists() or path.is_symlink():
        m = path.lstat()
        if not stat.S_ISREG(m.st_mode) or m.st_uid != 0 or stat.S_IMODE(m.st_mode) != mode:
            raise ValueError(f'unsafe existing installation: {path}')
        if path.read_bytes() == data:
            return
        if not replace:
            raise ValueError(f'existing administrator configuration differs: {path}')
    fd, name = tempfile.mkstemp(prefix='.audio-install-', dir=path.parent)
    tmp = Path(name)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data)
            os.fchmod(stream.fileno(), mode)
            stream.flush()
            os.fsync(stream.fileno())
        if replace:
            os.replace(tmp, path)
        else:
            os.link(tmp, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        tmp.unlink(missing_ok=True)


def configuration(uid, worker_uid, worker_gid, ports):
    if any(not isinstance(v, int) or not 0 < v < 2**32-1 for v in (uid, worker_uid, worker_gid)) or uid == worker_uid:
        raise ValueError('client and worker require distinct non-root identities')
    if not ports or len(set(ports)) != len(ports) or any(not 0 <= p <= 65535 for p in ports):
        raise ValueError('provide distinct VHCI ports in range 0..65535')
    return (f'allow_uid={uid}\ninstance=default\nworker_uid={worker_uid}\nworker_gid={worker_gid}\n'
            + ''.join(f'allow_vhci_port={p}\n' for p in ports)).encode()


def service():
    return b'''[Unit]
Description=VirtualGamepad stock-Linux USB audio broker
Requires=virtualgamepad-broker.socket
After=virtualgamepad-broker.socket
[Service]
Type=exec
ExecStart=/usr/libexec/virtualgamepad/virtualgamepad-broker --socket-activation --config /etc/virtualgamepad/broker.conf
User=root
Group=root
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict
ReadWritePaths=/run/virtualgamepad /run/virtualgamepad-state /sys/devices/platform/vhci_hcd.0/attach
RestrictAddressFamilies=AF_UNIX
IPAddressDeny=any
CapabilityBoundingSet=CAP_SYS_ADMIN CAP_SETUID CAP_SETGID
AmbientCapabilities=CAP_SYS_ADMIN CAP_SETUID CAP_SETGID
ProtectKernelModules=true
SystemCallArchitectures=native
TasksMax=128
LimitNOFILE=1024
Restart=no
'''


def socket_unit(group):
    if not group or any(c not in 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-' for c in group):
        raise ValueError('unsupported socket group name')
    return f'''[Unit]
Description=VirtualGamepad authorized audio broker socket
[Socket]
ListenStream=/run/virtualgamepad/broker.sock
SocketUser=root
SocketGroup={group}
SocketMode=0660
DirectoryMode=0711
RemoveOnStop=true
[Install]
WantedBy=sockets.target
'''.encode()


def run(args):
    subprocess.run(args, check=True, env=ENV, cwd='/', stdin=subprocess.DEVNULL)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--port', type=int, action='append', required=True)
    parser.add_argument('--binaries', type=Path, default=Path(__file__).resolve().parents[1] / 'target/debug')
    args = parser.parse_args()
    # Validate everything possible before changing the host.
    configuration(1, 2, 2, args.port)
    binaries = {}
    for source, dest in [('gr-privileged-broker', 'virtualgamepad-broker'), ('gr-audio-worker', 'gr-audio-worker')]:
        content = (args.binaries / source).read_bytes()
        if not content.startswith(b'\x7fELF'):
            raise ValueError('build the broker and worker ELF executables first')
        binaries[BASE / dest] = content
    if not args.apply:
        print('Plan: install fixed broker/worker, dedicated unprivileged account, root policy/state, and socket-activated service.')
        print('Allowlisted VHCI ports:', ', '.join(map(str, args.port)))
        print('No modules, sound permissions, physical routing, or sudo rules will be changed.')
        return
    if os.geteuid() != 0 or not __import__('sys').flags.isolated:
        raise ValueError('apply requires sudo /usr/bin/python3 -I')
    uid = int(os.environ.get('SUDO_UID', '0'))
    account = pwd.getpwuid(uid)
    if uid == 0 or account.pw_gid == 0:
        raise ValueError('invoke from an ordinary user account')
    # Never stop an existing broker or invalidate active attachments automatically.
    active = subprocess.run(['/usr/bin/systemctl', 'is-active', '--quiet', 'virtualgamepad-broker.service'], env=ENV, cwd='/', stdin=subprocess.DEVNULL)
    if active.returncode == 0:
        raise ValueError('broker is active; close sessions and stop it before explicit installation')
    try:
        worker = pwd.getpwnam(WORKER)
    except KeyError:
        run(['/usr/sbin/useradd', '--system', '--user-group', '--no-create-home', '--home-dir', '/nonexistent', '--shell', '/usr/sbin/nologin', WORKER])
        worker = pwd.getpwnam(WORKER)
    if worker.pw_shell not in ('/usr/sbin/nologin', '/sbin/nologin') or worker.pw_dir != '/nonexistent':
        raise ValueError('existing worker account requires administrator review')
    config = configuration(uid, worker.pw_uid, worker.pw_gid, args.port)
    # Policy is never overwritten. Code updates are explicit through this installer.
    install(Path('/etc/virtualgamepad/broker.conf'), config, 0o600)
    for destination, content in binaries.items():
        install(destination, content, 0o755, replace=True)
    service_path = Path('/etc/systemd/system/virtualgamepad-broker.service')
    previous = service().replace(b'AmbientCapabilities=CAP_SYS_ADMIN CAP_SETUID CAP_SETGID', b'AmbientCapabilities=')
    known_previous = service_path.is_file() and not service_path.is_symlink() and service_path.read_bytes() == previous
    install(service_path, service(), 0o644, replace=known_previous)
    install(Path('/etc/systemd/system/virtualgamepad-broker.socket'), socket_unit(grp.getgrgid(account.pw_gid).gr_name), 0o644)
    tmpfiles = b'd /run/virtualgamepad-state 0700 root root -\nd /run/virtualgamepad-state/default 0700 root root -\nd /run/virtualgamepad-state/default.audio 0700 root root -\n'
    install(Path('/etc/tmpfiles.d/virtualgamepad-broker.conf'), tmpfiles, 0o644)
    run(['/usr/bin/systemd-tmpfiles', '--create', '/etc/tmpfiles.d/virtualgamepad-broker.conf'])
    run(['/usr/bin/systemctl', 'daemon-reload'])
    run(['/usr/bin/systemctl', 'enable', '--now', 'virtualgamepad-broker.socket'])
    print('Installed. Socket activation is ready; no controller was created.')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        raise SystemExit('Audio broker installation refused: ' + str(error)) from error
