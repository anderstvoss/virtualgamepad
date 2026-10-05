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
            try: self.close()
            except BaseException as cleanup:
                raise RuntimeError(json.dumps(dict(initiating=str(initiating), cleanup=str(cleanup)))) from initiating
            raise

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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--instance', required=True)
    parser.add_argument('--port', type=int, required=True)
    parser.add_argument('--scenario', choices=['normal', 'client-exit'], default='normal')
    args = parser.parse_args()
    if os.geteuid() == 0: parser.error('client must run without root privileges')
    if not 0 <= args.port <= 65535: parser.error('invalid authorized port')
    audio.live.compiled_serial(args.instance, 1)
    if args.scenario == 'client-exit':
        session = Session('dualsense', args.instance, args.port)
        print(json.dumps(dict(event='verified-client-exit', generation=session.generation)), flush=True)
        os._exit(0)  # Deliberately bypass application cleanup after handoff.
    receipts = []
    for profile in audio.live.PROFILES:
        receipts.extend(exercise(profile, args.instance, args.port))
    print(json.dumps(dict(status='passed', scenario='normal', sessions=receipts)), flush=True)


if __name__ == '__main__':
    main()
