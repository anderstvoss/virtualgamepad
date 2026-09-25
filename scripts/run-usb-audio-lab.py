#!/usr/bin/python3 -I
"""Explicit administrator-run, bounded VHCI feasibility probe; not a broker.

No persistent permissions, remote listener, physical routing or sysfs detach.
Closing the owned socket shuts down only this attachment, avoiding a detach-by-
port race. The stock kernel releases its port on USB/IP connection loss.
"""
import argparse
import ctypes
import os
from pathlib import Path
import pwd
import resource
import secrets
import selectors
import signal
import socket
import stat
import subprocess
import sys
import time

VHCI = Path('/sys/devices/platform/vhci_hcd.0')
PROFILES = ('dualsense', 'dualshock4', 'xbox360')


def available_port(text, selected):
    """Fail closed on malformed inventory; never displace an occupied port."""
    lines = text.splitlines()
    if not lines or lines[0].split() != ['hub', 'port', 'sta', 'spd', 'dev', 'sockfd', 'local_busid']:
        raise ValueError('unexpected VHCI status header')
    ports = {}
    for line in lines[1:]:
        fields = line.split()
        if len(fields) != 7 or fields[0] not in ('hs', 'ss'):
            raise ValueError('malformed VHCI status row')
        hub, port, state, speed, device, descriptor, bus = fields
        numbers = [int(port), int(state), int(speed), int(device, 16), int(descriptor)]
        if any(value < 0 for value in numbers) or numbers[0] in ports:
            raise ValueError('ambiguous VHCI status')
        ports[numbers[0]] = (hub, *numbers[1:], bus)
    if ports.get(selected) != ('hs', 4, 0, 0, 0, '0-0'):
        raise ValueError('the administrator-selected high-speed port is absent or occupied')
    return selected


def child_limits(parent):
    """Runs after subprocess drops groups/GID/UID, immediately before exec."""
    if os.getuid() == 0 or os.geteuid() == 0:
        raise RuntimeError('worker privilege drop failed')
    libc = ctypes.CDLL(None, use_errno=True)
    # No privilege gain from setuid executables; terminate on supervisor death.
    if libc.prctl(38, 1, 0, 0, 0) or libc.prctl(1, signal.SIGTERM, 0, 0, 0):
        raise OSError(ctypes.get_errno(), 'worker hardening failed')
    if os.getppid() != parent:
        raise RuntimeError('lab supervisor already exited')
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_NOFILE, (32, 32))
    resource.setrlimit(resource.RLIMIT_AS, (512*1024*1024, 512*1024*1024))


def wait_ready(process, timeout=5):
    # Bound both response size and duration; do not use an unbounded readline.
    with selectors.DefaultSelector() as selector:
        selector.register(process.stdout, selectors.EVENT_READ)
        deadline = time.monotonic() + timeout
        ready = bytearray()
        while time.monotonic() < deadline:
            if not selector.select(max(0, deadline-time.monotonic())):
                break
            part = os.read(process.stdout.fileno(), 1)
            if not part:
                raise RuntimeError('worker exited before readiness')
            ready.extend(part)
            if ready.endswith(b'\n'):
                if ready != b'READY\n':
                    raise RuntimeError('unexpected worker readiness response')
                return
            if len(ready) > 64:
                raise RuntimeError('oversized worker readiness response')
        raise TimeoutError('worker readiness deadline exceeded')


def supervise(process, seconds):
    with selectors.DefaultSelector() as selector:
        selector.register(process.stdout, selectors.EVENT_READ)
        deadline = time.monotonic() + seconds + 5
        remaining = 128*1024
        while time.monotonic() < deadline:
            if not selector.select(min(1, max(0, deadline-time.monotonic()))):
                if process.poll() is not None:
                    return process.returncode
                continue
            data = os.read(process.stdout.fileno(), 4096)
            if not data:
                return process.wait(timeout=2)
            remaining -= len(data)
            if remaining < 0:
                raise RuntimeError('worker exceeded diagnostic output quota')
            # Fixed worker counters only, no recordings or USB identity logs.
            sys.stdout.write(data.decode('utf-8', errors='replace'))
            sys.stdout.flush()
        raise TimeoutError('bounded lab session expired')


def run(args):
    if os.geteuid() != 0:
        raise RuntimeError('administrator privileges are required for VHCI attach')
    uid = int(os.environ.get('SUDO_UID', '0'))
    if uid <= 0:
        raise RuntimeError('invoke through sudo from the ordinary lab account')
    account = pwd.getpwuid(uid)
    worker = args.worker.resolve(strict=True)
    if not worker.is_file():
        raise ValueError('worker must be a built executable')
    available_port((VHCI / 'status').read_text(), args.port)
    # Open the fixed sysfs operation before spawning. Never accept a sysfs path.
    attach = os.open(VHCI / 'attach', os.O_WRONLY | os.O_CLOEXEC | os.O_NOFOLLOW)
    process = None
    parent = os.getpid()
    try:
        if not stat.S_ISREG(os.fstat(attach).st_mode):
            raise RuntimeError('unexpected VHCI attach node')
        kernel_socket, worker_socket = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        with kernel_socket, worker_socket:
            device = secrets.randbelow(0xfffffffe) + 1
            process = subprocess.Popen(
                [str(worker), args.profile, str(device), str(args.seconds)],
                stdin=worker_socket, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                cwd='/', env={'PATH': '/usr/bin:/bin', 'LANG': 'C'},
                user=uid, group=account.pw_gid, extra_groups=[], close_fds=True,
                preexec_fn=lambda: child_limits(parent),
            )
            worker_socket.close()
            try:
                wait_ready(process)
                # Recheck immediately before attach; the kernel also rejects
                # occupied ports atomically. Never detach to make room.
                available_port((VHCI / 'status').read_text(), args.port)
                request = f'{args.port} {kernel_socket.fileno()} {device} 3'.encode('ascii')
                if os.write(attach, request) != len(request):
                    raise RuntimeError('incomplete VHCI attach')
                print(f'Attached compiled {args.profile} lab profile on port {args.port}; '
                      f'worker runs as the invoking user for at most {args.seconds}s.', flush=True)
                status = supervise(process, args.seconds)
                if status:
                    raise RuntimeError(f'unprivileged worker exited with status {status}')
            finally:
                # This socket, unlike a reusable port number, proves ownership.
                # Shutdown makes VHCI retire only our device even if setup failed.
                try:
                    kernel_socket.shutdown(socket.SHUT_RDWR)
                except OSError as error:
                    print(f'Owned socket shutdown failed: {error}', file=sys.stderr)
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait(timeout=3)
                process.stdout.close()
    finally:
        os.close(attach)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', type=Path, required=True)
    parser.add_argument('--profile', choices=PROFILES, required=True)
    parser.add_argument('--port', type=int, required=True,
                        help='explicit administrator authorization for one unused high-speed port')
    parser.add_argument('--seconds', type=int, default=120)
    args = parser.parse_args()
    if not 1 <= args.seconds <= 240 or args.port < 0:
        parser.error('seconds must be 1..240 and port nonnegative')
    try:
        run(args)
    except (OSError, ValueError, RuntimeError, TimeoutError, subprocess.SubprocessError) as error:
        print(f'USB audio lab: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
