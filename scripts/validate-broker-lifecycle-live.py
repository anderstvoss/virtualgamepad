#!/usr/bin/python3 -I
"""Bounded non-root lifecycle checks for a staged, isolated USB audio broker.

Only synthetic sessions are opened. A successful open is verified through its
compiled serial, VHCI status, ALSA ancestry and pre-attachment exclusion policy.
No shared profile/default is modified. Worker/broker death injection belongs to
an authoritative root supervisor, not this ordinary client.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import socket
import select
import subprocess
import struct
import time

spec = importlib.util.spec_from_file_location('broker_audio', Path(__file__).with_name('validate-broker-audio-live.py'))
audio = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audio)


def fd_count():
    return len(list(Path('/proc/self/fd').iterdir()))


def wait_detached(port, ownership, timeout=5):
    deadline = time.monotonic() + timeout
    while True:
        text = Path('/sys/devices/platform/vhci_hcd.0/status').read_text()
        try:
            current = audio.live.active_device(text, port)
        except ValueError:
            # Only the exact free state establishes release; malformed or newly
            # occupied inventories are not evidence that the owned session closed.
            rows = [line.split() for line in text.splitlines()[1:]]
            selected = [row for row in rows if len(row) == 7 and row[1].isdigit() and int(row[1]) == port]
            if len(selected) == 1 and selected[0][0] == 'hs' and selected[0][2:6] == ['004', '000', '00000000', '000000'] and selected[0][6] == '0-0':
                return
        else:
            if current != ownership:
                raise RuntimeError('VHCI identity changed during cleanup; no foreign resource touched')
        if time.monotonic() >= deadline:
            raise TimeoutError('owned attachment did not disappear')
        time.sleep(.02)


def attachment_snapshot(port, peer):
    """Bounded failure evidence; never consume the broker's pending reply."""
    result = {}
    try:
        rows = Path('/sys/devices/platform/vhci_hcd.0/status').read_text().splitlines()[1:]
        result['vhci_rows'] = [row for row in rows if len(row.split()) == 7 and
                               row.split()[1].isdigit() and int(row.split()[1]) == port]
    except OSError as error:
        result['vhci_unavailable'] = str(error)
    try:
        if select.select([peer], [], [], 0)[0]:
            data = peer.recv(259 + 4, socket.MSG_PEEK | socket.MSG_DONTWAIT)
            result['broker_reply_hex'] = data.hex()
            result['broker_eof'] = not data
        else:
            result['broker_readable'] = False
    except OSError as error:
        result['broker_unavailable'] = str(error)
    return result


class Session:
    def __init__(self, profile, instance, port):
        audio.live.compiled_serial(instance, 1)  # Validate before resource creation.
        self.closed = False
        self.broker, self.generation, self.device, self.bus, self.tag, self.channels = audio.opened(profile)
        self.port = port
        self.ownership = (self.device, self.bus)
        try:
            subprocess.run(['udevadm', 'settle', '--timeout=3'], check=True, timeout=4)
            deadline = time.monotonic() + 3
            while True:
                try:
                    ownership, card = audio.live.resolve_card(port, profile, self.ownership)
                    if ownership != self.ownership:
                        raise RuntimeError('broker handoff and VHCI identity differ')
                    self.isolation = audio.reserve_direct_alsa(card, self.bus, instance, self.generation)
                    self.diagnostics = audio.worker_diagnostics(self.channels[0], self.generation)
                    break
                except FileNotFoundError:
                    if time.monotonic() >= deadline: raise
                    time.sleep(.02)
        except BaseException as initiating:
            evidence = attachment_snapshot(port, self.broker)
            try: self.close()
            except BaseException as cleanup:
                raise RuntimeError(json.dumps(dict(initiating=str(initiating),
                    attachment=evidence, cleanup=str(cleanup)))) from initiating
            raise RuntimeError(json.dumps(dict(initiating=str(initiating),
                attachment=evidence, cleanup=[]))) from initiating

    def close(self, abandon=False):
        if self.closed:
            return
        errors = []
        try:
            if not abandon:
                # Keep the required channels alive until the broker owns close.
                # Closing them first can turn normal cleanup into worker death.
                audio.message(self.broker, 2, 2, struct.pack('<Q', self.generation))
                if audio.reply(self.broker) != (2, 0x80, struct.pack('<Q', self.generation)):
                    raise ValueError('cleanup acknowledgement differs from the owned generation')
                if self.broker.recv(1) != b'':
                    raise ValueError('single-use broker connection did not terminate')
        except BaseException as error:
            errors.append(str(error))
        finally:
            for channel in self.channels:
                try: channel.close()
                except OSError as error: errors.append(str(error))
            self.broker.close()
            self.closed = True
        try: wait_detached(self.port, self.ownership)
        except BaseException as error: errors.append(str(error))
        if errors:
            raise RuntimeError(json.dumps(dict(cleanup=errors)))


def exercise(profile, instance, port, cycles=2):
    generations = []
    receipts = []
    for abandon in [False] * cycles + [True]:
        before = fd_count()
        session = Session(profile, instance, port)
        generations.append(session.generation)
        try:
            receipts.append(dict(profile=profile, generation=session.generation,
                                 serial=session.isolation['serial'], abandoned=abandon,
                                 worker_diagnostics=session.diagnostics))
        finally:
            session.close(abandon=abandon)
        session.close()  # Local idempotence; wire connections are single-use.
        if fd_count() != before:
            raise RuntimeError('ordinary client descriptor count did not return to baseline')
    if len(set(generations)) != len(generations):
        raise RuntimeError('recreated sessions reused an identity')
    return receipts


def exit_before_handoff(instance, port):
    audio.live.compiled_serial(instance, 1)
    peer = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        peer.settimeout(2); peer.connect('/run/virtualgamepad/broker.sock')
        _, uid, _ = struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        if uid != 0: raise RuntimeError('broker is not root')
        audio.message(peer, 2, 1, bytes([1,2,1,2,3,4,5]))
        peer.shutdown(socket.SHUT_RDWR)
    finally: peer.close()
    # The server may still be constructing a session. The supervisor waits for
    # bounded attachment/journal cleanup before recreating the positive session.
    print(json.dumps(dict(event='client-disconnected-before-handoff', port=port)), flush=True)


def worker_death(profile, instance, port, fault_socket, broker_death=False):
    before = fd_count()
    session = Session(profile, instance, port)
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as supervisor:
            supervisor.settimeout(10); supervisor.connect(str(fault_socket))
            _, uid, _ = struct.unpack('3i', supervisor.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
            if uid != 0: raise RuntimeError('fault supervisor is not root')
            supervisor.sendall(json.dumps(dict(generation=session.generation, device=session.device)).encode()+b'\n')
            if supervisor.recv(1) != (b'B' if broker_death else b'K'): raise RuntimeError('fault supervisor did not confirm injection')
        session.broker.settimeout(5)
        if broker_death:
            error = b'owned broker terminated; connection closed'
            try:
                if session.broker.recv(1)!=b'': raise RuntimeError('dead broker connection remained open')
            except ConnectionResetError: pass
        else:
            version, tag, error = audio.reply(session.broker)
            if (version, tag) != (2, 0x81) or not error.startswith(b'audio worker exited: '):
                raise RuntimeError('worker death did not produce the exact terminal error')
            if session.broker.recv(1) != b'': raise RuntimeError('worker-death connection remained open')
        for channel in session.channels:
            channel.settimeout(5)
            if channel.recv(1) != b'': raise RuntimeError('worker-death channel remained open')
    finally:
        session.close(abandon=True)
    if fd_count() != before: raise RuntimeError('worker-death descriptor ownership did not return to baseline')
    return dict(profile=profile, generation=session.generation, terminal_error=error.decode(), cleanup=True)


def capacity_rejection():
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        peer.settimeout(5); peer.connect('/run/virtualgamepad/broker.sock')
        _, uid, _ = struct.unpack('3i',peer.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
        if uid != 0: raise RuntimeError('broker is not root')
        audio.message(peer,2,1,bytes([1,2,1,2,3,4,5]))
        if audio.reply(peer) != (2,0x81,b'broker resource limit reached'):
            raise RuntimeError('per-peer admission did not produce exact rejection')
        if peer.recv(1) != b'': raise RuntimeError('rejected admission connection remained open')


def siblings_and_admission(instance, ports):
    if len(ports) != 4 or len(set(ports)) != 4 or any(not 0 <= port <= 65535 for port in ports):
        raise ValueError('four distinct explicitly authorized ports required')
    before = fd_count(); sessions=[]; cleanup=[]; initiating=None; receipts=[]
    try:
        for profile,port in zip(['dualsense','dualshock4','xbox360','dualsense'],ports):
            sessions.append(Session(profile,instance,port))
        capacity_rejection()
        removed = sessions[1].generation
        sessions[1].close()
        for index,session in enumerate(sessions):
            if index != 1: audio.worker_diagnostics(session.channels[0],session.generation)
        sessions[1] = Session('dualshock4',instance,ports[1])
        if sessions[1].generation == removed: raise RuntimeError('capacity recovery reused an identity')
        for session in sessions:
            receipts.append(dict(generation=session.generation,serial=session.isolation['serial'],
                                 diagnostics=audio.worker_diagnostics(session.channels[0],session.generation)))
    except BaseException as error: initiating=str(error)
    finally:
        for session in reversed(sessions):
            try: session.close(); session.close()
            except BaseException as error: cleanup.append(str(error))
    if fd_count()!=before: cleanup.append('sibling descriptors did not return to baseline')
    if initiating is not None or cleanup: raise RuntimeError(json.dumps(dict(initiating=initiating,cleanup=cleanup)))
    return receipts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--instance', required=True)
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--scenario', choices=['normal', 'client-exit', 'client-before-handoff', 'worker-death','broker-death','startup-rejection','siblings-admission'], default='normal')
    parser.add_argument('--fault-socket', type=Path)
    parser.add_argument('--ports', type=int, nargs='+')
    args = parser.parse_args()
    if os.geteuid() == 0: parser.error('client must run without root privileges')
    if not 0 <= args.port <= 65535: parser.error('invalid authorized port')
    audio.live.compiled_serial(args.instance, 1)
    if args.scenario == 'startup-rejection':
        with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as peer:
            peer.settimeout(5); peer.connect('/run/virtualgamepad/broker.sock')
            try:
                if peer.recv(1)!=b'': raise RuntimeError('pending journal unexpectedly admitted a client')
            except ConnectionResetError: pass
        print(json.dumps(dict(status='passed',scenario='pending-startup-rejected')),flush=True)
        return
    if args.scenario == 'siblings-admission':
        if args.ports is None: parser.error('four authorized ports required')
        print(json.dumps(dict(status='passed',scenario='siblings-admission',sessions=siblings_and_admission(args.instance,args.ports))),flush=True)
        return
    if args.scenario == 'client-before-handoff':
        exit_before_handoff(args.instance, args.port)
        return
    if args.scenario == 'client-exit':
        session = Session('dualsense', args.instance, args.port)
        print(json.dumps(dict(event='verified-client-exit', generation=session.generation)), flush=True)
        os._exit(0)  # Deliberately bypass application cleanup after handoff.
    if args.scenario in ('worker-death','broker-death'):
        if args.fault_socket is None or not args.fault_socket.is_absolute(): parser.error('immutable supervisor socket is required')
        receipts = [worker_death(profile, args.instance, args.port, args.fault_socket, args.scenario=='broker-death') for profile in audio.live.PROFILES]
        print(json.dumps(dict(status='passed', scenario=args.scenario, sessions=receipts)), flush=True)
        return
    receipts = []
    for profile in audio.live.PROFILES:
        receipts.extend(exercise(profile, args.instance, args.port))
    print(json.dumps(dict(status='passed', scenario='normal', sessions=receipts)), flush=True)


if __name__ == '__main__':
    main()
