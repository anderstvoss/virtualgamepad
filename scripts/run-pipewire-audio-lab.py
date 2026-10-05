#!/usr/bin/env python3
"""Run a bounded headless test in a private, bounded-quantum PipeWire graph.

Requires PipeWire tools and WirePlumber's policy-only profile. No root access,
desktop setting changes, hardware monitors, or persistent service installation.
Example: python3 scripts/run-pipewire-audio-lab.py --timeout 240 -- cargo test ...
"""
import argparse
import json
import os
from pathlib import Path
import resource
import signal
import stat
import subprocess
import tempfile
import time

DIAGNOSTIC_LIMIT = 1024 * 1024


def diagnostic_limit():
    resource.setrlimit(resource.RLIMIT_FSIZE, (DIAGNOSTIC_LIMIT, DIAGNOSTIC_LIMIT))


def decode_graph(output):
    if len(output) > DIAGNOSTIC_LIMIT:
        raise ValueError('private graph diagnostic exceeds its quota')
    graph = json.loads(output)
    if not isinstance(graph, list) or any(not isinstance(item, dict) for item in graph):
        raise ValueError('private graph diagnostic must be an object array')
    return graph


def thread_policy(pid):
    """Read only the unreaped, owned child's threads; never act on these IDs."""
    rows = []
    try:
        threads = (Path('/proc') / str(pid) / 'task').iterdir()
        for thread in threads:
            try:
                tid = int(thread.name)
                rows.append(dict(tid=tid, name=(thread / 'comm').read_text().strip(),
                                 policy=os.sched_getscheduler(tid),
                                 priority=os.sched_getparam(tid).sched_priority))
            except (FileNotFoundError, ProcessLookupError):
                continue
    except FileNotFoundError:
        pass
    return rows


def snapshot(processes, env):
    with tempfile.TemporaryFile() as output:
        subprocess.run(['pw-dump'], env=env, check=True, timeout=3, stdout=output,
                       stderr=subprocess.DEVNULL, preexec_fn=diagnostic_limit)
        output.seek(0)
        graph = decode_graph(output.read(DIAGNOSTIC_LIMIT + 1))
    return dict(graph=graph, processes=[dict(pid=p.pid, threads=thread_policy(p.pid))
                for p in processes if p.poll() is None])


def wait_with_snapshot(test, processes, env, timeout, report):
    started = time.monotonic()
    try:
        status = test.wait(timeout=min(3, timeout))
    except subprocess.TimeoutExpired:
        # A graph snapshot is evidence, not a replacement for measured markers.
        receipt = snapshot(processes, env)
        with report.open('x') as output:
            json.dump(receipt, output, indent=2)
        remaining = timeout - (time.monotonic() - started)
        if remaining <= 0:
            raise subprocess.TimeoutExpired(test.args, timeout)
        return test.wait(timeout=remaining)
    # Fast failures still retain a graph receipt, without pretending it was
    # taken during a measured phase. Never overwrite an earlier receipt.
    receipt = snapshot(processes, env)
    receipt['test_exit_before_snapshot'] = status
    with report.open('x') as output:
        json.dump(receipt, output, indent=2)
    return status


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


def run(command, timeout, quantum=512, diagnostics=None):
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
            if diagnostics is not None:
                return wait_with_snapshot(test, processes, env, timeout, diagnostics)
            return test.wait(timeout=timeout)
        finally:
            for process in reversed(processes):
                stop(process)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--timeout', type=int, default=240)
    parser.add_argument('--quantum', type=int, choices=(128, 256, 512), default=512)
    parser.add_argument('--diagnostics', type=Path,
                        help='new external JSON path for a bounded private graph/thread snapshot')
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ['--'] else args.command
    if not command or not 1 <= args.timeout <= 900:
        parser.error('provide a test command and timeout of 1..900 seconds')
    return run(command, args.timeout, args.quantum, args.diagnostics)


if __name__ == '__main__':
    raise SystemExit(main())
