#!/usr/bin/env python3
"""Run a bounded headless test in a private, bounded-quantum PipeWire graph.

Requires PipeWire tools and WirePlumber's policy-only profile. No root access,
desktop setting changes, hardware monitors, or persistent service installation.
Example: python3 scripts/run-pipewire-audio-lab.py --timeout 240 -- cargo test ...
"""
import argparse
import os
from pathlib import Path
import signal
import stat
import subprocess
import tempfile
import time


def environment(root, inherited):
    env = inherited.copy()
    for name in ('PIPEWIRE_CONFIG_DIR', 'PIPEWIRE_CONFIG_PREFIX', 'PIPEWIRE_CONFIG_NAME',
                 'WIREPLUMBER_CONFIG_DIR', 'PIPEWIRE_QUANTUM', 'PIPEWIRE_LATENCY',
                 'PIPEWIRE_RATE'):
        env.pop(name, None)
    env.update(PIPEWIRE_RUNTIME_DIR=str(root), PIPEWIRE_REMOTE='pipewire-0',
               XDG_RUNTIME_DIR=str(root), XDG_CONFIG_HOME=str(root/'config'),
               XDG_STATE_HOME=str(root/'state'), XDG_CACHE_HOME=str(root/'cache'))
    return env


def stop(process):
    # Every child gets its own process group. Never signal a numeric group after
    # its leader has been reaped: that identifier could belong to a new process.
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        process.wait()


def run(command, timeout, quantum=512):
    if quantum not in (128, 256, 512):
        raise ValueError("lab quantum must be 128, 256 or 512 frames")
    if os.geteuid() == 0:
        raise ValueError('run the PipeWire lab as an ordinary user')
    with tempfile.TemporaryDirectory(prefix='virtualgamepad-pw-lab-') as directory:
        root = Path(directory)
        for name in ('config', 'state', 'cache'):
            (root/name).mkdir(mode=0o700)
        env = environment(root, os.environ)
        env['VIRTUALGAMEPAD_AUDIO_LAB_QUANTUM'] = str(quantum)
        processes = []
        try:
            daemon = subprocess.Popen(['pipewire'], env=env, start_new_session=True)
            processes.append(daemon)
            deadline = time.monotonic() + 5
            socket = root/'pipewire-0'
            while not socket.exists():
                if daemon.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError('private PipeWire daemon did not become ready')
                time.sleep(.02)
            if not stat.S_ISSOCK(socket.lstat().st_mode):
                raise RuntimeError('private PipeWire endpoint is not a socket')
            for key in ('clock.min-quantum', 'clock.quantum'):
                subprocess.run(['pw-metadata', '-n', 'settings', '0', key, str(quantum)],
                               env=env, check=True, timeout=5)
            policy = subprocess.Popen(['wireplumber', '-p', 'policy'], env=env,
                                      start_new_session=True)
            processes.append(policy)
            # The test's own bounded endpoint/link readiness handles policy startup.
            test = subprocess.Popen(command, env=env, start_new_session=True)
            processes.append(test)
            return test.wait(timeout=timeout)
        finally:
            for process in reversed(processes):
                stop(process)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--timeout', type=int, default=240)
    parser.add_argument('--quantum', type=int, choices=(128, 256, 512), default=512)
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command or not 1 <= args.timeout <= 900:
        parser.error('provide a test command and timeout of 1..900 seconds')
    return run(command, args.timeout, args.quantum)


if __name__ == '__main__':
    raise SystemExit(main())
