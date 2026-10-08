#!/usr/bin/python3 -I
"""Disposable non-root Steam/FEX namespace. Default invocation is a plan.

Prove isolation with a synthetic sentinel before any Steam bootstrap. Bind a
private directory over the actual passwd home, clear environment, provide private
proc/dev/tmp/run and retain only public system/runtime assets. No existing Steam
state is read/copied. --apply proves isolation; --bootstrap additionally launches
public Steam/FEX assets with an owned display and bounded user unit. Neither
proves controller recognition or replaces interactive acceptance.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pwd
import secrets
import shutil
import signal
import struct
import stat
import zlib
import time
import threading
import subprocess
import tempfile

SENTINEL = '''import json, os, pathlib, pwd, sys
home = pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)
assert os.getuid() != 0, "consumer must remain non-root"
assert home == pathlib.Path(os.environ["HOME"])
assert not (home / sys.argv[1]).exists(), "real home sentinel leaked"
assert (home / "private-home-marker").read_text() == sys.argv[2]
assert not pathlib.Path("/run/host").exists()
status = pathlib.Path("/proc/self/status").read_text()
assert next(line for line in status.splitlines() if line.startswith("CapEff:")).split()[1] == "0000000000000000"
assert not pathlib.Path("/proc/1/root").resolve().joinpath("home", home.name, sys.argv[1]).exists()
print(json.dumps(dict(scope="namespace-isolation-only", uid=os.getuid(), real_home_hidden=True,
    passwd_home_hidden=True, capabilities_dropped=True, private_proc=True, private_dev=True)))
'''


def memory_sample(meminfo=Path('/proc/meminfo'), pressure=Path('/proc/pressure/memory')):
    """Read bounded public resource counters; missing evidence fails closed."""
    fields = {}
    for line in meminfo.read_text().splitlines():
        key, _, value = line.partition(':')
        if key in ('MemTotal', 'MemAvailable'):
            parts = value.split()
            if len(parts) != 2 or parts[1] != 'kB': raise RuntimeError('unknown memory counter format')
            fields[key] = int(parts[0]) * 1024
    if fields.keys() != {'MemTotal', 'MemAvailable'} or not 0 <= fields['MemAvailable'] <= fields['MemTotal']:
        raise RuntimeError('memory availability cannot be verified')
    rows = {row.split()[0]: dict(item.split('=', 1) for item in row.split()[1:])
            for row in pressure.read_text().splitlines()}
    full = float(rows['full']['avg10'])
    if not 0 <= full <= 100: raise RuntimeError('invalid memory pressure')
    return dict(total_bytes=fields['MemTotal'], available_bytes=fields['MemAvailable'], full_avg10=full)


def memory_budget(sample):
    # Bound the isolated consumer to half the detected physical RAM.
    maximum = sample['total_bytes'] // (2 * 1024**2) * 1024**2
    return dict(max_bytes=maximum, high_bytes=maximum * 3 // 4)


def require_memory(sample, starting=False):
    reserve = max(2 * 1024**3, sample['total_bytes'] * 15 // 100)
    required = reserve + (memory_budget(sample)['max_bytes'] if starting else 0)
    if sample['available_bytes'] < required or sample['full_avg10'] >= 5:
        raise RuntimeError('memory safety reserve or pressure threshold exceeded; owned trial stopped')



def bootstrap_unit_command(unit, client, seconds, budget):
    return ['systemd-run', '--user', '--wait', '--pipe', '--collect',
        '--unit=' + unit, '--property=KillMode=control-group', '--property=TimeoutStopSec=5',
        '--property=MemoryAccounting=yes', '--property=MemoryHigh=' + str(budget['high_bytes']),
        '--property=MemoryMax=' + str(budget['max_bytes']), '--property=MemorySwapMax=0',
        '--property=RuntimeMaxSec=' + str(seconds), '--', *client]


def termination_reason(code, output):
    if 'Finished with result: oom-kill' in output:
        return 'oom-kill'
    return 'normal-exit' if code == 0 else 'nonzero-exit'


def command(workspace, home, uid, gid, sentinel, marker, profile=None):
    if (type(uid) is not int or type(gid) is not int or uid <= 0 or gid <= 0 or
            not home.is_absolute() or home in (Path('/'), Path('/home')) or
            not workspace.is_absolute() or not sentinel.startswith('.vg-alpha-sentinel-') or
            '/' in sentinel):
        raise ValueError('invalid owned namespace identity')
    args = ['bwrap', '--unshare-user', '--uid', str(uid), '--gid', str(gid),
            '--unshare-pid', '--unshare-ipc', '--unshare-uts', '--die-with-parent',
            '--new-session', '--cap-drop', 'ALL', '--clearenv']
    for path in ['/usr', '/etc', '/snap']:
        args += ['--ro-bind', path, path]
    for path in ['/lib', '/lib64', '/bin', '/sbin']:
        if Path(path).exists(): args += ['--ro-bind', path, path]
    args += ['--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp', '--dir', '/run',
             '--dir', f'/run/user/{uid}', '--bind', str(profile if profile is not None else workspace / 'home'), str(home),
             '--ro-bind', str(workspace / 'sentinel.py'), '/sentinel.py',
             '--setenv', 'HOME', str(home), '--setenv', 'PATH', '/usr/bin:/bin',
             '--setenv', 'LANG', 'C', '--setenv', 'XDG_RUNTIME_DIR', f'/run/user/{uid}',
             '--chdir', str(home), '--', '/usr/bin/python3', '-I', '/sentinel.py', sentinel, marker]
    return args


def remove_sentinel(path, expected):
    stat = path.lstat()
    if (stat.st_dev, stat.st_ino) != expected:
        raise RuntimeError('sentinel identity changed; refusing removal')
    path.unlink()


BOOTSTRAP = r'''set -eu
/usr/bin/python3 -I /sentinel.py "$1" "$2"
mkdir -p "$SNAP_USER_COMMON" "$SNAP_USER_DATA"
/usr/bin/python3 -I /extract-rootfs.py "$SNAP/x86_rootfs.tar.gz" "$SNAP_USER_COMMON"
cp -r "$SNAP/fex_config" "$SNAP_USER_COMMON/fex_config"
export FEX_ROOTFS="$SNAP_USER_COMMON/x86_rootfs"
export FEX_APP_CONFIG_LOCATION="$SNAP_USER_COMMON/fex_config"
export FEX_SERVERSOCKETPATH="$SNAP_USER_COMMON/FEXServer.Socket"
exec /usr/bin/dbus-run-session -- "$SNAP/usr/bin/FEXBash" -c "$SNAP/usr/bin/steam -no-cef-sandbox"
'''


def bootstrap_command(workspace, home, uid, gid, sentinel, marker, display, seconds, profile=None):
    if type(display) is not int or not 200 <= display <= 299 or type(seconds) is not int or not 30 <= seconds <= 900:
        raise ValueError('invalid owned display or bounded deadline')
    args = command(workspace, home, uid, gid, sentinel, marker, profile)
    temporary = args.index("--tmpfs")
    args[temporary:temporary+2] = ["--bind", str(workspace / "tmp"), "/tmp"]
    boundary = args.index('--')
    snap = '/snap/steam/current'
    settings = ['--ro-bind', '/sys', '/sys', '--dir', '/tmp/.X11-unix',
                '--ro-bind', f'/tmp/.X11-unix/X{display}', f'/tmp/.X11-unix/X{display}',
                '--ro-bind', str(workspace / 'auth'), '/auth',
                '--ro-bind', str(workspace / 'bootstrap.sh'), '/bootstrap.sh',
                '--ro-bind', str(Path(__file__).with_name('extract-alpha-steam-rootfs.py')), '/extract-rootfs.py']
    environment = dict(DISPLAY=f':{display}', XAUTHORITY='/auth', LIBGL_ALWAYS_SOFTWARE='1',
                       SNAP=snap, SNAP_USER_COMMON=str(home / 'common'), SNAP_USER_DATA=str(home / 'data'),
                       PATH='/usr/bin:/bin')
    for key, value in environment.items(): settings += ['--setenv', key, value]
    return args[:boundary] + settings + ['--', '/bin/bash', '/bootstrap.sh', sentinel, marker]


def capture_display(workspace, display, path):
    environment = dict(os.environ, DISPLAY=f':{display}', XAUTHORITY=str(workspace/'auth'))
    data = subprocess.check_output(['xwd', '-root', '-silent'], env=environment, timeout=5)
    header = struct.unpack('>25I', data[:100])
    width, height = header[4:6]; stride = header[12]; step = header[11]//8
    offset = header[0]+header[19]*12
    if (width*height > 1280*900 or step not in (3,4) or offset+height*stride > len(data)):
        raise RuntimeError('owned screenshot exceeds known display geometry')
    pixels = bytearray()
    for y in range(height):
        pixels.append(0)
        row = data[offset+y*stride:offset+(y+1)*stride]
        for x in range(width):
            b,g,r = row[x*step:x*step+3]; pixels.extend((r,g,b))
    def chunk(kind, body):
        return struct.pack('>I',len(body))+kind+body+struct.pack('>I',zlib.crc32(kind+body))
    png = (b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>2I5B',width,height,8,2,0,0,0))+
           chunk(b'IDAT',zlib.compress(pixels))+chunk(b'IEND',b''))
    with path.open('xb') as output: output.write(png)


def display_command(server, display, auth, visible=False):
    if not server.is_absolute() or not server.is_file():
        raise RuntimeError('an explicit available display server binary is required')
    if type(display) is not int or not 200 <= display < 300:
        raise ValueError('invalid owned display number')
    command = [str(server), f':{display}', '-screen']
    if visible:
        if not os.environ.get('DISPLAY'):
            raise RuntimeError('visible nested display requires a host display')
        command += ['1280x900', '-title', 'Virtualgamepad alpha: disposable Steam session']
    else:
        command += ['0', '1280x900x24']
    return command + ['-nolisten', 'tcp', '-auth', str(auth), '-noreset']


def run_bootstrap(workspace, home, account, name, marker, xvfb, seconds, captures=None, visible=False, profile=None):
    if not xvfb.is_absolute() or not xvfb.is_file():
        raise RuntimeError('an explicit available Xvfb binary is required')
    display = next((number for number in range(200, 300)
                    if not Path(f'/tmp/.X11-unix/X{number}').exists() and
                    not Path(f'/tmp/.X{number}-lock').exists()), None)
    if display is None: raise RuntimeError('no unused owned display slot')
    unit = 'virtualgamepad-steam-' + secrets.token_hex(8) + '.service'
    auth = workspace / 'auth'
    auth.touch(mode=0o600, exist_ok=False)
    (workspace / 'bootstrap.sh').write_text(BOOTSTRAP)
    (workspace / 'tmp').mkdir(mode=0o700)
    subprocess.run(['xauth', '-f', str(auth), 'add', f':{display}',
                    'MIT-MAGIC-COOKIE-1', secrets.token_hex(16)], check=True, timeout=5)
    daemon = None
    launcher = None
    reader = None
    errors = []
    initiating = None
    code = None
    overflow = threading.Event()
    samples = []
    require_memory(memory_sample())
    limit = 1024 * 1024
    log = workspace / 'bootstrap.log'
    def drain():
        written = 0
        with log.open('wb') as output:
            while chunk := launcher.stdout.read(8192):
                allowed = min(len(chunk), max(0, limit - written))
                output.write(chunk[:allowed])
                written += allowed
                if allowed < len(chunk): overflow.set()
    budget = memory_budget(memory_sample())
    try:
        with (workspace / 'display.log').open('wb') as output:
            daemon = subprocess.Popen(display_command(xvfb, display, auth, visible),
                stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        deadline = time.monotonic() + 5
        while not Path(f'/tmp/.X11-unix/X{display}').exists():
            if daemon.poll() is not None or time.monotonic() >= deadline:
                raise RuntimeError('owned display startup failed')
            time.sleep(.02)
        client = bootstrap_command(workspace, home, account.pw_uid, account.pw_gid, name, marker, display, seconds, profile)
        require_memory(memory_sample(), starting=True)
        launcher = subprocess.Popen(bootstrap_unit_command(unit, client, seconds, budget),
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        reader = threading.Thread(target=drain)
        reader.start()
        deadline = time.monotonic() + seconds + 15
        next_sample = 0
        started = time.monotonic()
        next_capture = 30
        while launcher.poll() is None:
            if time.monotonic() >= next_sample:
                sample = memory_sample()
                samples.append(sample)
                require_memory(sample)
                next_sample = time.monotonic() + 1
            if captures is not None and time.monotonic()-started >= next_capture:
                capture_display(workspace, display, captures / f'display-{next_capture}.png')
                next_capture += 30
            if overflow.is_set(): raise RuntimeError('bounded bootstrap output exceeded')
            if time.monotonic() >= deadline: raise TimeoutError('owned bootstrap deadline expired')
            time.sleep(.1)
        code = launcher.returncode
        if code: raise RuntimeError(f'Steam bootstrap exited {code}; inspect sanitized receipt')
    except BaseException as error:
        initiating = str(error)
    finally:
        # This unpredictable unit name was registered before launch; no foreign
        # service is stopped. Stop even if systemd-run failed part way through.
        try:
            completed = subprocess.run(['systemctl', '--user', 'stop', unit],
                                       capture_output=True, timeout=12)
            if completed.returncode and b'not loaded' not in completed.stderr:
                errors.append('owned Steam unit restoration failed')
        except Exception as error: errors.append(str(error))
        if launcher is not None:
            try: launcher.wait(timeout=5)
            except subprocess.TimeoutExpired:
                launcher.terminate()
                try: launcher.wait(timeout=5)
                except subprocess.TimeoutExpired: launcher.kill(); launcher.wait(timeout=5)
        if reader is not None:
            reader.join(timeout=5)
            if reader.is_alive(): errors.append('owned output reader did not terminate')
        if daemon is not None and daemon.poll() is None:
            os.killpg(daemon.pid, signal.SIGTERM)
            try: daemon.wait(timeout=5)
            except subprocess.TimeoutExpired: os.killpg(daemon.pid, signal.SIGKILL); daemon.wait(timeout=5)
        if Path(f'/tmp/.X11-unix/X{display}').exists(): errors.append('owned display socket remains')
    return dict(scope='Steam bootstrap only; no controller acceptance',
                unit=unit, exit=code, initiating=initiating, cleanup=errors,
                memory_samples=samples, memory_high_bytes=budget['high_bytes'], memory_max_bytes=budget['max_bytes'],
                termination=termination_reason(code, log.read_text(errors='replace') if log.exists() else ''),
                log=log.read_text(errors='replace') if log.exists() else '',
                accepted=False, existing_profile_credentials_copied=False)


def require_disk(filesystem, available):
    if filesystem in ('tmpfs', 'ramfs') or available < 4*1024**3:
        raise RuntimeError('Steam bootstrap requires disk-backed disposable state with 4 GiB free')


def workspace_parent(selected=None):
    parent = selected or Path.home() / '.local/state/virtualgamepad-alpha-review'
    parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    info = parent.lstat()
    if parent.is_symlink() or info.st_uid != os.getuid() or info.st_mode & 0o022:
        raise RuntimeError('disposable workspace parent must be owned and private')
    filesystem = subprocess.check_output(['findmnt', '-n', '-o', 'FSTYPE', '-T', str(parent)],
                                         text=True, timeout=5).strip()
    if not filesystem or '\n' in filesystem: raise RuntimeError('disposable filesystem is ambiguous')
    require_disk(filesystem, shutil.disk_usage(parent).free)
    return parent


def profile_home(workspace, selected, real_home):
    if selected is None:
        path = workspace / 'home'
        path.mkdir(mode=0o700)
        return path, False
    if (not selected.is_absolute() or selected.resolve() != selected or
            selected == real_home or selected in real_home.parents):
        raise RuntimeError('persistent profile must be an explicit separate non-symlink path')
    parent = selected.parent.lstat()
    if parent.st_uid != os.getuid() or parent.st_mode & 0o022:
        raise RuntimeError('persistent profile parent must be owned and not writable by others')
    marker = selected / '.virtualgamepad-alpha-profile'
    expected = dict(version=1, scope='isolated-steam-lab', uid=os.getuid())
    reused = selected.exists()
    if not reused:
        selected.mkdir(mode=0o700)
        with marker.open('x') as output: output.write(json.dumps(expected) + '\n')
    info = selected.lstat()
    if not selected.is_dir() or selected.is_symlink() or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise RuntimeError('persistent profile must remain an owned private directory')
    fd = os.open(marker, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    info = os.fstat(fd)
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_nlink != 1 or info.st_size > 512:
        os.close(fd)
        raise RuntimeError('invalid test profile identity marker')
    with os.fdopen(fd) as source:
        if json.loads(source.read(513)) != expected or source.read(1):
            raise RuntimeError('existing directory is not an identified isolated test profile')
    return selected, reused


def lock_profile(profile):
    descriptor = os.open(profile / '.virtualgamepad-alpha-lock',
                         os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_nlink != 1:
            raise RuntimeError('invalid test profile lock')
        fcntl.flock(descriptor, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BaseException:
        os.close(descriptor)
        raise RuntimeError('isolated test profile is already in use or cannot be locked')
    return descriptor


def run(bootstrap=False, xvfb=None, seconds=120, work_directory=None, captures=None, visible=False, profile_directory=None):
    if os.geteuid() == 0:
        raise RuntimeError('Steam lab must run as an ordinary user')
    if shutil.which('bwrap') is None:
        raise RuntimeError('bubblewrap is unavailable; no bootstrap attempted')
    account = pwd.getpwuid(os.getuid())
    home = Path(account.pw_dir)
    marker = secrets.token_hex(16)
    name = '.vg-alpha-sentinel-' + marker
    canary = home / name
    fd = os.open(canary, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
    info = os.fstat(fd)
    expected = (info.st_dev, info.st_ino)
    initiating = None
    cleanup = []
    receipt = None
    profile_lock = None
    try:
        with os.fdopen(fd, 'w') as output: output.write('synthetic review sentinel')
        parent = workspace_parent(work_directory) if bootstrap else None
        with tempfile.TemporaryDirectory(prefix='virtualgamepad-alpha-steam-', dir=parent) as directory:
            workspace = Path(directory)
            profile, reused = profile_home(workspace, profile_directory, home)
            profile_lock = lock_profile(profile)
            with tempfile.NamedTemporaryFile(mode='w', dir=profile, delete=False) as output:
                output.write(marker)
                marker_path = Path(output.name)
            marker_path.replace(profile / 'private-home-marker')
            (workspace / 'sentinel.py').write_text(SENTINEL)
            completed = subprocess.run(command(workspace, home, account.pw_uid, account.pw_gid, name, marker, profile),
                                       capture_output=True, text=True, timeout=20, env={'PATH':'/usr/bin:/bin'})
            if completed.returncode:
                raise RuntimeError('namespace sentinel failed: ' + completed.stderr[:4096])
            if len(completed.stdout) > 4096:
                raise RuntimeError('oversized namespace receipt')
            receipt = json.loads(completed.stdout)
            receipt.update(test_profile_retained=profile_directory is not None, test_profile_reused=reused)
            if bootstrap:
                if captures is not None: captures.mkdir(mode=0o700, exist_ok=False)
                result = run_bootstrap(workspace, home, account, name, marker, xvfb, seconds, captures, visible, profile)
                receipt["bootstrap"] = result
                if result["initiating"] or result["cleanup"]:
                    raise RuntimeError("isolated Steam bootstrap did not complete")
    except Exception as error:
        initiating = str(error)
    finally:
        if profile_lock is not None: os.close(profile_lock)
        try: remove_sentinel(canary, expected)
        except Exception as error: cleanup.append(str(error))
    result = dict(status='passed' if initiating is None and not cleanup else 'failed',
                  evidence=receipt, initiating=initiating, cleanup=cleanup,
                  steam_launched=bootstrap and receipt is not None and "bootstrap" in receipt, existing_profile_credentials_copied=False)
    print(json.dumps(result), flush=True)
    return 0 if result['status'] == 'passed' else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--bootstrap', action='store_true', help='bounded isolated Steam/FEX bootstrap, not consumer acceptance')
    display = parser.add_mutually_exclusive_group()
    display.add_argument('--xvfb', type=Path, help='ordinary-user hidden display server binary')
    display.add_argument('--xephyr', type=Path, help='visible owned nested display for direct disposable login')
    parser.add_argument('--work-directory', type=Path, help='owned disk-backed parent for disposable bootstrap state')
    parser.add_argument('--profile-directory', type=Path, help='opt-in private test profile retained across runs; never the normal account profile')
    parser.add_argument('--captures', type=Path, help='exclusive external directory for owned-display snapshots')
    parser.add_argument('--seconds', type=int, choices=range(30, 901), default=120)
    args = parser.parse_args()
    if not args.apply:
        print(json.dumps(dict(apply=False, phases=['private namespace', 'synthetic isolation sentinel'],
                              steam_launched=False, copies_existing_profile=False)))
        return 0
    server = args.xephyr if args.xephyr is not None else args.xvfb
    if args.bootstrap and server is None: parser.error('--bootstrap requires --xvfb or --xephyr')
    return run(args.bootstrap, server, args.seconds, args.work_directory, args.captures, args.xephyr is not None, args.profile_directory)


if __name__ == '__main__':
    raise SystemExit(main())
