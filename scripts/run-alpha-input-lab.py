#!/usr/bin/python3 -I
"""Run a hash-pinned non-root input test behind temporary, process-owned udev rules.

Default is a dry run. Install a reviewed root-owned copy before --apply. Rules
apply only to the gated child's creation labels; no existing devices are triggered
or persistent configuration changed. Child identity is retained until restoration.
"""
import argparse
import hashlib
import grp
import json
import os
from pathlib import Path
import pwd
import re
import resource
import signal
import stat
import subprocess
import sys
import tempfile
import time

RULES = Path('/run/udev/rules.d')
INPUT = Path('/sys/class/input')
LIMIT = 1024 * 1024


def output_limit():
    resource.setrlimit(resource.RLIMIT_FSIZE, (LIMIT, LIMIT))


def terminate_group(pid):
    try:
        os.killpg(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass  # A held zombie leader may have no live group members.


def labels(pid):
    if type(pid) is not int or pid <= 1:
        raise ValueError('invalid owned process identity')
    return f'/p{pid:x}-i', f'seat-vg-alpha-p{pid:x}'


def rule(pid):
    prefix, seat = labels(pid)
    # Sony's HID driver leaves input phys empty. Match its HID parent's
    # complete uevent line instead; uinput retains its own physical label.
    settings = ('ENV{ID_SEAT}="' + seat + '", ENV{LIBINPUT_IGNORE_DEVICE}="1"\n')
    return ('SUBSYSTEM=="input", ATTRS{phys}=="virtualgamepad/*' + prefix +
            '*", ' + settings +
            'SUBSYSTEM=="input", ATTRS{phys}=="virtualgamepad/p' + f'{pid:x}' +
            '/*/c*", ' + settings +
            'SUBSYSTEM=="input", SUBSYSTEMS=="hid", ATTRS{uevent}=="*HID_PHYS=virtualgamepad/*' +
            prefix + '*", ' + settings)


def owned_physical(physical, pid):
    prefix, _ = labels(pid)
    return (physical.startswith('virtualgamepad/') and prefix in physical or
            re.fullmatch(r'virtualgamepad/p' + f'{pid:x}' + r'/[a-f0-9]{32}/c[a-f0-9]{4}', physical) is not None)


def input_is_owned(entry, pid):
    try:
        if owned_physical((entry / 'device/phys').read_text().strip(), pid):
            return True
    except FileNotFoundError:
        pass
    # Inspect the actual sysfs ancestry, not global HID inventory: each event
    # must descend from the matching HID resource. No vendor-only inference.
    for ancestor in entry.resolve().parents:
        try:
            if (ancestor / 'subsystem').resolve().name != 'hid':
                continue
            properties = (ancestor / 'uevent').read_text().splitlines()
        except FileNotFoundError:
            continue
        if any(line.startswith('HID_PHYS=') and
               owned_physical(line[len('HID_PHYS='):], pid) for line in properties):
            return True
    return False


def owned_inputs(pid, root=INPUT):
    labels(pid)  # Validate even when inventory is empty.
    return sorted(entry.name for entry in root.iterdir()
                  if re.fullmatch(r'event[0-9]+', entry.name) and input_is_owned(entry, pid))


def trusted(path, directory=False):
    meta = path.lstat()
    if (meta.st_uid != 0 or meta.st_mode & 0o022 or not
            (stat.S_ISDIR(meta.st_mode) if directory else stat.S_ISREG(meta.st_mode))):
        raise RuntimeError('lab code and command must be immutable root-owned paths')


def identity(path):
    info = path.lstat()
    return info.st_dev, info.st_ino


def creation_group():
    group = Path('/dev/uinput').stat().st_gid
    # ACL-based creation access can leave uinput in group root after boot.
    # Root is never granted to the test client; its read group is then the
    # configured Linux input group. The test still opens creation as its UID.
    return group if group != 0 else grp.getgrnam('input').gr_gid


def restore_rule(path, expected, pid, inventory=owned_inputs):
    if inventory(pid):
        raise RuntimeError('owned input devices remain; isolation rule retained')
    if not path.exists() and not path.is_symlink():
        return
    if identity(path) != expected:
        raise RuntimeError('isolation rule identity changed; refusing removal')
    path.unlink()


def child(arguments):
    descriptor, client_environment, *command = arguments
    if os.geteuid() == 0 or not command:
        raise RuntimeError('test must execute without root privileges')
    descriptor = int(descriptor)
    try:
        if os.read(descriptor, 1) != b'G':
            raise RuntimeError('isolation preparation did not release the test')
    finally:
        os.close(descriptor)
    _, seat = labels(os.getpid())
    environment = os.environ.copy()
    environment.update(json.loads(client_environment))
    environment['VIRTUALGAMEPAD_INPUT_LAB_SEAT'] = seat
    os.execvpe(command[0], command, environment)


def interrupted(signum, frame):
    del frame
    raise InterruptedError(f'input lab interrupted by signal {signum}')


def run(args):
    if os.geteuid() != 0 or not sys.flags.isolated:
        raise RuntimeError('apply requires an isolated administrator interpreter')
    for image in (Path(__file__).absolute(), Path(args.command[0])):
        trusted(image)
        for parent in image.parents:
            trusted(parent, True)
    trusted(Path('/usr/bin/setpriv'))
    if hashlib.sha256(Path(args.command[0]).read_bytes()).hexdigest() != args.command_hash:
        raise RuntimeError('command image differs from approved hash')
    account = pwd.getpwuid(args.uid)
    if account.pw_uid <= 0 or account.pw_gid == 0:
        raise RuntimeError('test account must be non-root')
    if args.input_gid <= 0 or args.input_gid != creation_group():
        raise RuntimeError('input group must match the prepared creation/input read group')
    for parent in RULES.parents:
        trusted(parent, True)
    read_fd, write_fd = os.pipe()
    process = None
    path = None
    expected = None
    initiating = None
    cleanup = []
    status = None
    created_directory = None
    handlers = {s: signal.signal(s, interrupted) for s in (signal.SIGTERM, signal.SIGINT)}
    try:
        if not RULES.exists():
            RULES.mkdir(mode=0o755)
            created_directory = identity(RULES)
        trusted(RULES, True)
        with tempfile.TemporaryFile() as output:
            command = ['/usr/bin/setpriv', '--reuid=' + str(account.pw_uid),
                       '--regid=' + str(account.pw_gid), '--groups=' + str(args.input_gid),
                       '--bounding-set=-all', '--inh-caps=-all', '--ambient-caps=-all',
                       '--no-new-privs', '--', sys.executable, '-I', str(Path(__file__).absolute()),
                       '--child', str(read_fd), json.dumps(dict(args.env)), *args.command]
            process = subprocess.Popen(command, pass_fds=(read_fd,), start_new_session=True,
                                       stdout=output, stderr=subprocess.STDOUT,
                                       preexec_fn=output_limit,
                                       env={'PATH': '/usr/bin:/bin', 'LANG': 'C'})
            os.close(read_fd); read_fd = -1
            path = RULES / f'71-virtualgamepad-alpha-p{process.pid:x}.rules'
            descriptor = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o644)
            expected = identity(path)
            with os.fdopen(descriptor, 'w') as destination:
                destination.write(rule(process.pid))
            subprocess.run(['udevadm', 'verify', str(path)], check=True, timeout=10)
            subprocess.run(['udevadm', 'control', '--reload'], check=True, timeout=10)
            os.write(write_fd, b'G'); os.close(write_fd); write_fd = -1
            deadline = time.monotonic() + args.timeout
            while os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is None:
                if output.tell() > LIMIT:
                    raise RuntimeError('test exceeded output quota')
                if time.monotonic() >= deadline:
                    raise TimeoutError('input test exceeded deadline')
                time.sleep(.05)
            output.seek(0)
            diagnostic = output.read(LIMIT + 1)
            if len(diagnostic) > LIMIT:
                raise RuntimeError('test exceeded output quota')
            print(diagnostic.decode(errors='replace'), end='')
    except BaseException as error:
        initiating = str(error)
    finally:
        for signum in handlers:
            signal.signal(signum, signal.SIG_IGN)
        for descriptor in (read_fd, write_fd):
            if descriptor >= 0:
                os.close(descriptor)
        if process is not None:
            # WNOWAIT keeps the leader PID reserved through group/device cleanup.
            try:
                terminate_group(process.pid)
                deadline = time.monotonic() + 5
                while owned_inputs(process.pid) and time.monotonic() < deadline:
                    time.sleep(.05)
                if expected is not None:
                    restore_rule(path, expected, process.pid)
                    subprocess.run(['udevadm', 'control', '--reload'], check=True, timeout=10)
            except (OSError, RuntimeError, subprocess.SubprocessError) as error:
                cleanup.append(str(error))
            try:
                status = process.wait(timeout=10)
            except subprocess.TimeoutExpired as error:
                cleanup.append(str(error))
        if created_directory is not None:
            try:
                if identity(RULES) != created_directory or any(RULES.iterdir()):
                    raise RuntimeError('new rule directory changed or is nonempty; retained')
                RULES.rmdir()
            except (OSError, RuntimeError) as error:
                cleanup.append(str(error))
        for signum, handler in handlers.items():
            signal.signal(signum, handler)
    result = dict(status=status, initiating=initiating, cleanup=cleanup,
                  rule_removed=path is None or not path.exists())
    print(json.dumps(result), flush=True)
    return 0 if status == 0 and not initiating and not cleanup else 1


def main():
    if sys.argv[1:2] == ['--child']:
        child(sys.argv[2:])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--uid', type=int, required=True)
    parser.add_argument('--input-gid', type=int, required=True)
    parser.add_argument('--command-hash', required=True)
    parser.add_argument('--timeout', type=int, default=180)
    parser.add_argument('--env', action='append', default=[])
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command[:1] == ['--']:
        args.command = args.command[1:]
    if not args.command or not Path(args.command[0]).is_absolute() or not 1 <= args.timeout <= 900:
        parser.error('provide an absolute command and bounded timeout')
    if not re.fullmatch('[a-f0-9]{64}', args.command_hash):
        parser.error('provide the approved SHA256')
    args.env = [value.split('=', 1) for value in args.env]
    if any(len(value) != 2 or value[0] not in ('VIRTUALGAMEPAD_SDL_PROBE', 'LD_LIBRARY_PATH',
                                             'DISPLAY', 'XAUTHORITY') for value in args.env):
        parser.error('only explicit test display and probe environment is accepted')
    if not args.apply:
        print(json.dumps(dict(command=args.command, uid=args.uid, input_gid=args.input_gid,
                              actions=['gate non-root child', 'verify/reload owned PID rule',
                                       'run bounded test', 'reap owned devices and restore rule']), indent=2))
        return 0
    return run(args)


if __name__ == '__main__':
    sys.exit(main())
