#!/usr/bin/python3 -I
"""Memory-bounded neutral GUI creation/removal soak in an owned display.

The default is a plan. Receipts persist in an exclusive caller-selected directory.
No audio/touch is injected. Successful completion supplies long-run evidence only;
keyboard/consumer acceptance still needs its own tests and resource interpretation.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import secrets
import signal
import shutil
import subprocess
import sys
import time

spec = importlib.util.spec_from_file_location('steam_lab', Path(__file__).with_name('run-alpha-steam-lab.py'))
resources = importlib.util.module_from_spec(spec)
spec.loader.exec_module(resources)


def owned_hid(pid, root=Path('/sys/bus/hid/devices')):
    token = f'/p{pid:x}-i'
    found = []
    for entry in root.iterdir():
        try:
            physical = (entry / 'phys').read_text().strip()
            if physical.startswith('virtualgamepad/') and token in physical: found.append(entry.name)
        except FileNotFoundError: pass
    return sorted(found)


def process_sample(pid):
    root = Path('/proc') / str(pid)
    status = dict(line.split(':', 1) for line in (root / 'status').read_text().splitlines() if ':' in line)
    tasks = list((root / 'task').iterdir())
    children = set()
    for task in tasks:
        try: children.update((task / 'children').read_text().split())
        except FileNotFoundError: pass
    return dict(rss_kib=int(status['VmRSS'].split()[0]), descriptors=len(list((root / 'fd').iterdir())),
                threads=len(tasks), children=sorted(children))


def unit_command(unit, script, arguments):
    if not unit.startswith('virtualgamepad-gui-soak-') or not unit.endswith('.service'):
        raise ValueError('invalid owned unit')
    return ['systemd-run', '--user', '--wait', '--pipe', '--collect', '--unit=' + unit,
            '--property=MemoryAccounting=yes', '--property=MemoryHigh=512M', '--property=MemoryMax=1G',
            '--property=MemorySwapMax=0', '--property=KillMode=control-group', '--property=TimeoutStopSec=10',
            '--property=RuntimeMaxSec=7260', '--', sys.executable, '-I', str(script), *arguments, '--inside-unit']


def trial(args):
    root = args.report.resolve()
    root.mkdir(mode=0o700, parents=False, exist_ok=False)
    binary = args.binary.resolve(strict=True)
    with binary.open("rb") as source: digest = hashlib.file_digest(source, "sha256").hexdigest()
    frozen = root / 'gui-soak'
    shutil.copyfile(binary, frozen)
    frozen.chmod(0o700)
    display = next((n for n in range(300, 400) if not Path(f'/tmp/.X11-unix/X{n}').exists()
                    and not Path(f'/tmp/.X{n}-lock').exists()), None)
    if display is None: raise RuntimeError('no unused owned display')
    auth = root / 'auth'; auth.touch(mode=0o600, exist_ok=False)
    for directory in ('home', 'config', 'state', 'cache'): (root / directory).mkdir(mode=0o700)
    children = []
    samples = []
    failure = None
    status = None
    elapsed = 0
    cleanup = []
    receipt = dict(revision=args.revision, binary_sha256=digest, duration_seconds=args.seconds,
                   audio_enabled=False, touch_injected=False, memory_max_bytes=1024**3,
                   host_boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip())
    environment = dict(PATH='/usr/bin:/bin', LANG='C', DISPLAY=f':{display}', XAUTHORITY=str(auth),
        LIBGL_ALWAYS_SOFTWARE='1', HOME=str(root / 'home'), XDG_CONFIG_HOME=str(root / 'config'),
        XDG_STATE_HOME=str(root / 'state'), XDG_CACHE_HOME=str(root / 'cache'),
        XDG_RUNTIME_DIR=f'/run/user/{os.getuid()}')
    try:
        resources.require_memory(resources.memory_sample())
        subprocess.run(['xauth', '-f', str(auth), 'add', f':{display}', 'MIT-MAGIC-COOKIE-1',
                        secrets.token_hex(16)], check=True, timeout=5)
        with (root / 'display.log').open('wb') as output:
            children.append(subprocess.Popen([str(args.xvfb.resolve(strict=True)), f':{display}',
                '-screen', '0', '1280x900x24', '-nolisten', 'tcp', '-auth', str(auth), '-noreset'],
                env=environment, stdout=output, stderr=subprocess.STDOUT, start_new_session=True))
        deadline = time.monotonic() + 5
        while not Path(f'/tmp/.X11-unix/X{display}').exists():
            if children[0].poll() is not None or time.monotonic() >= deadline: raise RuntimeError('owned display failed')
            time.sleep(.02)
        with (root / 'gui.log').open('wb') as output:
            app = subprocess.Popen([str(frozen), str(args.seconds)], env=environment,
                                   stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
        children.append(app)
        started = time.monotonic()
        receipt.update(app_pid=app.pid, display_pid=children[0].pid, display=f':{display}')
        (root / 'started.json').write_text(json.dumps(receipt, indent=2))
        next_sample = 0
        with (root / 'samples.jsonl').open('x') as output:
            while app.poll() is None:
                elapsed = time.monotonic() - started
                if elapsed > args.seconds + 45: raise TimeoutError('owned GUI deadline exceeded')
                memory = resources.memory_sample()
                resources.require_memory(memory)
                if elapsed >= next_sample:
                    try:
                        sample = dict(seconds=round(elapsed, 2), host_memory=memory, **process_sample(app.pid))
                        samples.append(sample)
                        output.write(json.dumps(sample) + '\n'); output.flush()
                        print(json.dumps(sample), flush=True)
                        next_sample = elapsed + 30
                    except FileNotFoundError: pass
                time.sleep(1)
        elapsed = time.monotonic() - started
        status = app.returncode
        if status != 0 or elapsed < args.seconds: raise RuntimeError('GUI did not complete its measured duration')
    except BaseException as error:
        failure = str(error)
    finally:
        for process in reversed(children):
            try:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try: process.wait(timeout=10)
                    except subprocess.TimeoutExpired: os.killpg(process.pid, signal.SIGKILL); process.wait(timeout=5)
            except Exception as error: cleanup.append(str(error))
        auth.unlink(missing_ok=True)
        remaining = owned_hid(children[1].pid) if len(children) > 1 else []
        if remaining: cleanup.append('owned HID nodes remain: ' + ','.join(remaining))
        if Path(f'/tmp/.X11-unix/X{display}').exists(): cleanup.append('owned display socket remains')
        receipt.update(exit=status, error=failure, cleanup=cleanup, samples=samples, elapsed_seconds=elapsed,
                       completed=failure is None and not cleanup)
        (root / 'result.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(dict(completed=receipt['completed'], cleanup=cleanup, error=failure)), flush=True)
    return 0 if receipt['completed'] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--xvfb', type=Path)
    parser.add_argument('--revision')
    parser.add_argument('--report', type=Path)
    parser.add_argument('--seconds', type=int, choices=range(60, 7201), default=7200)
    parser.add_argument('--inside-unit', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if not args.apply:
        print(json.dumps(dict(apply=False, phases=['memory preflight', 'owned display', 'neutral soak', 'owned cleanup'],
                              audio_enabled=False, memory_max_bytes=1024**3)))
        return 0
    if os.geteuid() == 0 or any(getattr(args, key) is None for key in ('binary', 'xvfb', 'revision', 'report')):
        parser.error('ordinary identity, binary, display, revision and exclusive report are required')
    if len(args.revision) != 40 or any(c not in '0123456789abcdef' for c in args.revision): parser.error('full revision required')
    resources.require_memory(resources.memory_sample())
    if args.inside_unit:
        def interrupted(signum, frame):
            del frame
            raise InterruptedError("owned GUI interrupted by signal " + str(signum))
        signal.signal(signal.SIGTERM, interrupted)
        signal.signal(signal.SIGINT, interrupted)
        return trial(args)
    unit = 'virtualgamepad-gui-soak-' + secrets.token_hex(8) + '.service'
    try:
        arguments = ["--apply", "--binary", str(args.binary.resolve(strict=True)),
                     "--xvfb", str(args.xvfb.resolve(strict=True)), "--revision", args.revision,
                     "--report", str(args.report.resolve()), "--seconds", str(args.seconds)]
        return subprocess.run(unit_command(unit, Path(__file__).resolve(), arguments),
                              timeout=args.seconds+55).returncode
    finally:
        completed = subprocess.run(['systemctl', '--user', 'stop', unit], capture_output=True, timeout=15)
        if completed.returncode and b'not loaded' not in completed.stderr:
            raise RuntimeError('owned GUI unit restoration failed')


if __name__ == '__main__':
    raise SystemExit(main())
