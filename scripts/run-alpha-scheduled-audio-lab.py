#!/usr/bin/python3 -I
"""Qualify an owned private audio graph with temporary client scheduling limits.

Default is a plan. Apply requires immutable administrator-installed code/images.
No persistent scheduling policy, shared graph or service is changed. A transient
unit owns every descendant; its deadline and realtime CPU limit remain bounded.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import pwd
import resource
import re
import signal
import shutil
import stat
import subprocess
import sys
import tempfile
import uuid

sys.dont_write_bytecode = True
LIMIT = 1024 * 1024


def output_limit():
    resource.setrlimit(resource.RLIMIT_FSIZE, (LIMIT, LIMIT))


def interrupted(signum, frame):
    del frame
    raise InterruptedError(f'audio lab interrupted by signal {signum}')


def trusted(path):
    for item in (path, *path.parents):
        metadata = item.lstat()
        regular = stat.S_ISREG(metadata.st_mode) if item == path else stat.S_ISDIR(metadata.st_mode)
        if metadata.st_uid != 0 or metadata.st_mode & 0o022 or not regular:
            raise RuntimeError('audio lab images and parents must be immutable root-owned paths')


def checked_image(path, expected):
    trusted(path)
    if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise RuntimeError('audio lab image differs from approved hash')


def properties(unit):
    result = subprocess.run(['systemctl', 'show', unit, '-p', 'LoadState', '-p', 'Description',
                             '-p', 'ActiveState', '-p', 'ControlGroup'], check=True, capture_output=True, text=True, timeout=10)
    return dict(line.split('=', 1) for line in result.stdout.splitlines() if '=' in line)


def restore(unit, description):
    state = properties(unit)
    if state.get('LoadState') == 'not-found':
        return
    if state.get('Description') != description:
        raise RuntimeError('transient audio unit identity changed; refusing foreign cleanup')
    subprocess.run(['systemctl', 'stop', unit], check=True, timeout=15)
    state = properties(unit)
    if state.get('LoadState') != 'not-found' and state.get('ActiveState') not in ('inactive', 'failed'):
        raise RuntimeError('owned audio unit remains active')
    group = state.get('ControlGroup')
    if group:
        path = Path('/sys/fs/cgroup') / group.lstrip('/') / 'cgroup.procs'
        if path.exists() and path.read_text().strip():
            raise RuntimeError('owned audio cgroup still contains processes')


def workspace_identity(path):
    meta = path.lstat()
    return meta.st_dev, meta.st_ino


def remove_workspace(path, expected):
    if not path.exists() and not path.is_symlink():
        return
    if workspace_identity(path) != expected or not shutil.rmtree.avoids_symlink_attacks:
        raise RuntimeError('workspace identity changed or safe removal unavailable; retained')
    shutil.rmtree(path)


def unit_command(args, unit, description, gid, workspace):
    return ['systemd-run', '--wait', '--pipe', '--collect', '--unit=' + unit,
            '--setenv=TMPDIR=' + str(workspace),
            '--property=Description=' + description, '--property=NoNewPrivileges=yes',
            '--property=CapabilityBoundingSet=CAP_SETUID CAP_SETGID CAP_SETPCAP',
            '--property=KillMode=control-group', '--property=RuntimeMaxSec=540',
            '--property=LimitRTPRIO=88', '--property=LimitRTTIME=200ms',
            '--property=LimitFSIZE=' + str(LIMIT), '--', '/usr/bin/setpriv',
            '--reuid=' + str(args.uid), '--regid=' + str(gid), '--clear-groups',
            '--bounding-set=-all', '--inh-caps=-all', '--ambient-caps=-all', '--no-new-privs',
            '--', sys.executable, '-I', str(Path(__file__).absolute()), '--child',
            str(args.control), str(args.rust_test)]


def child(control, rust_test):
    if os.geteuid() == 0:
        raise RuntimeError('audio clients must not run as root')
    if resource.getrlimit(resource.RLIMIT_RTPRIO) != (88, 88):
        raise RuntimeError('expected temporary, bounded realtime priority grant')
    # This applies only to the test process and inherited test descendants.
    # PipeWire may promote its own data loops within the inherited priority cap.
    os.sched_setscheduler(0, os.SCHED_RR, os.sched_param(20))
    result = dict(scope='private scheduled graph controls; no native timing claim',
                  uid=os.getuid(), scheduler=os.sched_getscheduler(0),
                  priority=os.sched_getparam(0).sched_priority,
                  rtprio=resource.getrlimit(resource.RLIMIT_RTPRIO),
                  rttime=resource.getrlimit(resource.RLIMIT_RTTIME), rust=[])
    scripts = Path(__file__).absolute().parent
    with tempfile.TemporaryDirectory(prefix='virtualgamepad-scheduled-controls-') as directory:
        root = Path(directory)
        with (root / 'c.log').open('w') as log:
            completed = subprocess.run([sys.executable, '-I', str(scripts / 'run-alpha-audio-control.py'),
                                        '--control', control, '--seconds', '60', '--trials', '3',
                                        '--report', str(root / 'c.json')], stdout=log,
                                       stderr=subprocess.STDOUT, timeout=245, check=False)
        result['c_exit'] = completed.returncode
        result['c'] = json.loads((root / 'c.json').read_text())
        result['c_diagnostics'] = (root / 'c.log').read_text()
        environment = dict(os.environ, VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS='62')
        for trial in range(1, 4):
            with (root / 'rust.log').open('w') as log:
                completed = subprocess.run([sys.executable, '-I', str(scripts / 'run-pipewire-audio-lab.py'),
                                            '--timeout', '80', '--', rust_test, '--ignored', '--exact',
                                            'latency_graph_direct_control', '--nocapture'],
                                           env=environment, stdout=log, stderr=subprocess.STDOUT,
                                           timeout=85, check=False)
            result['rust'].append(dict(trial=trial, status=completed.returncode,
                                       diagnostics=(root / 'rust.log').read_text()))
    result['qualified'] = result['c']['qualified'] and all(row['status'] == 0 for row in result['rust'])
    print(json.dumps(result), flush=True)
    return 0 if result['qualified'] else 1


def apply(args):
    if os.geteuid() != 0 or not sys.flags.isolated:
        raise RuntimeError('apply requires an isolated administrator interpreter')
    checked_image(args.control, args.control_hash)
    checked_image(args.rust_test, args.rust_hash)
    for name in (Path(__file__).name, 'run-alpha-audio-control.py', 'run-pipewire-audio-lab.py'):
        trusted(Path(__file__).absolute().parent / name)
    account = pwd.getpwuid(args.uid)
    if account.pw_uid <= 0 or account.pw_gid == 0:
        raise RuntimeError('audio test account must be non-root')
    unit = 'virtualgamepad-alpha-audio-' + uuid.uuid4().hex + '.service'
    description = 'Owned alpha audio controls ' + unit
    if properties(unit).get('LoadState') != 'not-found':
        raise RuntimeError('transient unit already exists; no actions taken')
    initiating = None
    cleanup = []
    status = None
    workspace = None
    expected_workspace = None
    environment = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C'}
    handlers = {s: signal.signal(s, interrupted) for s in (signal.SIGTERM, signal.SIGINT)}
    try:
        previous_mask = signal.pthread_sigmask(signal.SIG_BLOCK, handlers)
        try:
            workspace = Path(tempfile.mkdtemp(prefix='virtualgamepad-alpha-audio-', dir='/run'))
            expected_workspace = workspace_identity(workspace)
        finally:
            signal.pthread_sigmask(signal.SIG_SETMASK, previous_mask)
        os.chown(workspace, account.pw_uid, account.pw_gid)
        with tempfile.TemporaryFile() as output:
            completed = subprocess.run(unit_command(args, unit, description, account.pw_gid, workspace),
                                       stdout=output, stderr=subprocess.STDOUT, stdin=subprocess.DEVNULL,
                                       env=environment, timeout=570, check=False, preexec_fn=output_limit)
            status = completed.returncode
            output.seek(0)
            diagnostic = output.read(LIMIT + 1)
            if len(diagnostic) > LIMIT:
                raise RuntimeError('audio receipt exceeds output quota')
            print(diagnostic.decode(errors='replace'), end='')
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        initiating = str(error)
    finally:
        for signum in handlers:
            signal.signal(signum, signal.SIG_IGN)
        try:
            restore(unit, description)
        except (OSError, RuntimeError, subprocess.SubprocessError) as error:
            cleanup.append(str(error))
        if workspace is not None and expected_workspace is not None and not cleanup:
            try:
                remove_workspace(workspace, expected_workspace)
            except (OSError, RuntimeError) as error:
                cleanup.append(str(error))
        for signum, handler in handlers.items():
            signal.signal(signum, handler)
    print(json.dumps(dict(unit=unit, status=status, initiating=initiating, cleanup=cleanup,
                          workspace_removed=workspace is None or not workspace.exists())), flush=True)
    return 0 if status == 0 and not initiating and not cleanup else 1


def main():
    if sys.argv[1:2] == ['--child']:
        if len(sys.argv) != 4:
            raise ValueError('fixed child requires two approved images')
        return child(*sys.argv[2:])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--uid', type=int, required=True)
    parser.add_argument('--control', type=Path, required=True)
    parser.add_argument('--control-hash', required=True)
    parser.add_argument('--rust-test', type=Path, required=True)
    parser.add_argument('--rust-hash', required=True)
    args = parser.parse_args()
    if args.uid <= 0 or not args.control.is_absolute() or not args.rust_test.is_absolute():
        parser.error('use a non-root UID and absolute approved images')
    if not all(re.fullmatch('[a-f0-9]{64}', value) for value in (args.control_hash, args.rust_hash)):
        parser.error('provide approved SHA256 hashes')
    if not args.apply:
        print(json.dumps(dict(actions=['temporary owned unit with bounded RT limits',
                                       'non-root six independent controls', 'owned cgroup restoration'],
                              uid=args.uid, priority_cap=88, client_priority=20, deadline_seconds=540)))
        return 0
    return apply(args)


if __name__ == '__main__':
    sys.exit(main())
