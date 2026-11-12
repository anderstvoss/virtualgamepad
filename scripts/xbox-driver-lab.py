#!/usr/bin/python3 -I
"""Manage fixed Xbox Series lab drivers and switch the wired reference."""

import argparse
import fcntl
import os
from pathlib import Path
import pwd
import socket
import stat
import struct
import subprocess
import tempfile


USB = Path('/sys/bus/usb')
HID = Path('/sys/bus/hid')
MODULE_SYSFS = Path('/sys/module')
SOCKET = '/run/virtualgamepad-xbox-driver-lab.sock'
LOCK = '/var/lib/virtualgamepad-xbox-driver-lab/driver.lock'
UNIT = 'virtualgamepad-xbox-driver-lab'
HELPER = Path('/usr/local/libexec/virtualgamepad/xbox-driver-lab.py')
ENV = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C'}
MODULES = {'xpad': ('xpad',), 'xone-wired': ('xone_wired', 'xone_gip_gamepad', 'xone_gip_headset')}
DRIVERS = {'xpad': 'xpad', 'xone': 'xone-wired'}
REQUESTS = ('status', 'load-xpadneo', 'unload-xpadneo',
            *(f'{action}{name}' for action in ('', 'load-', 'unload-') for name in DRIVERS))


def run(command, *args):
    result = subprocess.run((command, *args), env=ENV, capture_output=True, text=True,
                            check=False, timeout=15)
    if result.returncode:
        raise ValueError(result.stderr.strip() or f'{command} exited with {result.returncode}')


def read(path):
    try:
        return path.read_text().strip()
    except FileNotFoundError:
        return None


def driver(interface):
    link = interface / 'driver'
    return link.readlink().name if link.is_symlink() else None


def target():
    matches = []
    for device in (USB / 'devices').iterdir():
        if (read(device / 'idVendor'), read(device / 'idProduct')) == ('045e', '0b12'):
            config = read(device / 'bConfigurationValue')
            devnum = read(device / 'devnum')
            interface = USB / 'devices' / f'{device.name}:{config}.0'
            if devnum and read(interface / 'bInterfaceNumber') == '00':
                matches.append((interface, devnum))
    if len(matches) != 1:
        raise ValueError('connect exactly one wired Xbox Series controller (045e:0b12)')
    return matches[0]


def binding(reference, name, action):
    if target() != reference:
        raise ValueError('the USB reference changed during the request')
    interface, _ = reference
    (USB / 'drivers' / name / action).write_text(interface.name)


def load(name):
    for module in MODULES[name]:
        run('modprobe', module)


def switch(reference, requested):
    interface, _ = reference
    previous = driver(interface)
    if previous is not None and previous not in MODULES:
        raise ValueError(f'reference is owned by an unexpected driver: {previous}')
    load(requested)
    if target() != reference:
        raise ValueError('the USB reference changed during module load')
    current = driver(interface)
    if current == requested:
        return
    if current != previous:
        raise ValueError('the USB driver changed during module load')
    if previous:
        binding(reference, previous, 'unbind')
    try:
        binding(reference, requested, 'bind')
        if driver(interface) != requested:
            raise ValueError(f'{requested} did not bind the reference')
    except (OSError, ValueError) as error:
        if previous and driver(interface) is None:
            try:
                binding(reference, previous, 'bind')
            except (OSError, ValueError) as rollback:
                raise ValueError(f'{error}; restoring {previous} failed: {rollback}') from error
        raise


def unload(reference, name):
    interface, _ = reference
    owner = interface.resolve(strict=True).parent
    module = MODULES[name][0]
    if not (MODULE_SYSFS / module).exists():
        return
    directory = USB / 'drivers' / name
    if any(entry.is_symlink() and ':' in entry.name and entry.resolve().parent != owner
           for entry in directory.iterdir()):
        raise ValueError(f'{name} also owns another USB device; use per-device switching')
    if target() != reference:
        raise ValueError('the USB reference changed before module unload')
    run('rmmod', module)


def xpadneo_reference(entry):
    original, separator, instance = entry.name.partition('.')
    if (original != '0005:045E:0B13' or not separator or not instance
            or any(char not in '0123456789ABCDEF' for char in instance)):
        return False
    identities = {'HID_ID=0005:0000045E:00000B13', 'HID_ID=0005:0000045E:0000028E'}
    return bool(identities.intersection((read(entry / 'uevent') or '').splitlines()))


def xpadneo(request):
    directory = HID / 'drivers' / 'xpadneo'
    bound = []
    if directory.exists():
        bound = [entry for entry in directory.iterdir()
                 if entry.is_symlink() and entry.name.count(':') == 2]
    if request == 'load-xpadneo':
        run('modprobe', 'hid_xpadneo')
    elif (MODULE_SYSFS / 'hid_xpadneo').exists():
        if len(bound) > 1 or any(not xpadneo_reference(entry) for entry in bound):
            raise ValueError('xpadneo owns additional or unsupported HID devices; disconnect them before unload')
        run('rmmod', 'hid_xpadneo')
    loaded = (MODULE_SYSFS / 'hid_xpadneo').exists()
    return f'xpadneo: {"loaded" if loaded else "unloaded"}'


def handle(request):
    if request not in REQUESTS:
        raise ValueError('unknown request')
    if request in ('load-xpadneo', 'unload-xpadneo'):
        return xpadneo(request)
    reference = target()
    if request != 'status':
        action, _, alias = request.rpartition('-')
        name = DRIVERS[alias]
        if action == 'load':
            load(name)
        elif action == 'unload':
            unload(reference, name)
        else:
            switch(reference, name)
    interface, _ = target()
    return f'{interface.name}: {driver(interface) or "unbound"}'


def serve(uid):
    with socket.socket(fileno=0) as conn:
        conn.settimeout(5)
        _, peer, _ = struct.unpack('iII', conn.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        try:
            if peer != uid:
                raise ValueError('unauthorized local user')
            with conn.makefile('rb') as request:
                line = request.readline(32)
            if not line.endswith(b'\n'):
                raise ValueError('missing request terminator')
            with open(LOCK, 'a', encoding='utf-8') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                answer = 'OK ' + handle(line[:-1].decode('ascii'))
        except (OSError, UnicodeError, ValueError, subprocess.TimeoutExpired) as error:
            answer = f'ERROR {error}'
        conn.sendall((answer + '\n').encode())


def trusted_parent(path):
    for directory in reversed(path.parents):
        directory.mkdir(mode=0o755, exist_ok=True)
        info = directory.lstat()
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
            raise ValueError(f'unsafe installation directory: {directory}')


def unit_files(uid):
    units = Path('/etc/systemd/system')
    socket_unit = f'''[Unit]
Description=VirtualGamepad Xbox Series driver lab socket
[Socket]
ListenStream={SOCKET}
SocketUser={uid}
SocketGroup=root
SocketMode=0600
Accept=yes
RemoveOnStop=yes
[Install]
WantedBy=sockets.target
'''
    service_unit = f'''[Unit]
Description=VirtualGamepad Xbox Series driver lab request
[Service]
ExecStart=/usr/bin/python3 -I {HELPER} serve {uid}
StandardInput=socket
StandardOutput=socket
StandardError=journal
NoNewPrivileges=yes
ProtectHome=yes
ProtectSystem=strict
PrivateTmp=yes
RestrictAddressFamilies=AF_UNIX
CapabilityBoundingSet=CAP_SYS_MODULE
StateDirectory={UNIT}
StateDirectoryMode=0700
ReadWritePaths=/sys/bus/usb/drivers
RuntimeMaxSec=75
'''
    return (
        (units / f'{UNIT}@.service', service_unit.encode()),
        (units / f'{UNIT}.socket', socket_unit.encode()),
    )


def install_file(path, data, replace):
    trusted_parent(path)
    try:
        info = path.lstat()
    except FileNotFoundError:
        info = None
    if info:
        if not stat.S_ISREG(info.st_mode) or info.st_uid != 0 or info.st_mode & 0o022:
            raise ValueError(f'unsafe existing administrator installation: {path}')
        if path.read_bytes() == data:
            return
        if not replace:
            raise ValueError(f'existing administrator installation differs: {path}; use --replace to update')
    with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as destination:
        temporary = Path(destination.name)
        try:
            destination.write(data)
            destination.flush()
            os.fchmod(destination.fileno(), 0o644)
            os.fsync(destination.fileno())
            if info:
                os.replace(temporary, path)
            else:
                os.link(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)


def install(user, replace=False):
    if os.geteuid() != 0:
        raise ValueError('installation requires root in an administrator terminal')
    uid = pwd.getpwnam(user).pw_uid
    if uid == 0:
        raise ValueError('choose a non-root lab operator')
    for path, data in ((HELPER, Path(__file__).read_bytes()), *unit_files(uid)):
        install_file(path, data, replace)
    run('systemctl', 'daemon-reload')
    run('systemctl', 'enable', '--now', f'{UNIT}.socket')


def client(request):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as conn:
        conn.settimeout(75)
        conn.connect(SOCKET)
        conn.sendall((request + '\n').encode('ascii'))
        with conn.makefile(encoding='utf-8') as response:
            answer = response.read(8192).strip()
    if not answer.startswith('OK '):
        raise ValueError(answer or 'driver lab service closed the connection')
    print(answer[3:])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=(*REQUESTS, 'install', 'serve'))
    parser.add_argument('argument', nargs='?')
    parser.add_argument('--replace', action='store_true', help='update trusted root-owned installation files')
    args = parser.parse_args()
    try:
        if args.action == 'install' and args.argument:
            install(args.argument, args.replace)
        elif args.action == 'serve' and args.argument and not args.replace and os.geteuid() == 0:
            serve(int(args.argument))
        elif args.action in REQUESTS and args.argument is None and not args.replace:
            client(args.action)
        else:
            parser.error('install needs a username; client actions take no arguments')
    except (OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
        parser.exit(1, f'{error}\n')


if __name__ == '__main__':
    main()
