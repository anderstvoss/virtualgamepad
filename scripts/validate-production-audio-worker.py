#!/usr/bin/env python3
"""Exercise the installed-worker entry point without root or a kernel attachment."""
import argparse
import fcntl
import importlib.util
import os
from pathlib import Path
import signal
import socket
import struct
import time

spec = importlib.util.spec_from_file_location('usb_probe', Path(__file__).with_name('validate-usb-audio-worker.py'))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def message(peer, tag, payload):
    peer.sendall(struct.pack('<IHB', len(payload)+3, 1, tag)+payload)


def receive(peer):
    size, = struct.unpack('<I', probe.exact(peer, 4))
    assert 3 <= size <= 259
    body = probe.exact(peer, size)
    assert body[:2] == b'\x01\x00'
    return body[2], body[3:]


def wait(pid, timeout):
    deadline = time.monotonic()+timeout
    while time.monotonic() < deadline:
        found, status = os.waitpid(pid, os.WNOHANG)
        if found:
            return os.waitstatus_to_exitcode(status)
        time.sleep(.01)
    raise TimeoutError('worker did not exit')


def trial(worker, family, tag, channels, microphones):
    pairs = [socket.socketpair() for _ in range(4)]
    pid = os.fork()
    if pid == 0:
        try:
            # Duplicate away from all fixed ABI slots before remapping any slot.
            copies = [fcntl.fcntl(child.fileno(), fcntl.F_DUPFD_CLOEXEC, 10) for _, child in pairs]
            for source, destination in zip(copies, (0, 3, 4, 5)):
                os.dup2(source, destination, inheritable=True)
            os.execv(str(worker), [str(worker), family, str(0x10001), '7', '020102030405'])
        except BaseException:
            os._exit(125)
    reaped = False
    for _, child in pairs:
        child.close()
    usb, control, playback, microphone = [parent for parent, _ in pairs]
    for peer in (usb, control, playback, microphone):
        peer.settimeout(3)
    generation = struct.pack('<Q', 7)
    try:
        assert receive(control) == (0, b'\x01'+generation)
        # Real USB enumeration and controller-owned request completion.
        assert probe.transfer(usb,1,0,1,18,bytes([0x80,6,0,1,0,0,18,0]))[:2] == (0,18)
        for sequence, setup in [(2,[0,9,1,0,0,0,0,0]),(3,[1,11,1,0,1,0,0,0]),(4,[1,11,1,0,2,0,0,0])]:
            assert probe.transfer(usb,sequence,0,0,0,bytes(setup))[:2] == (0,0)
        assert probe.transfer(usb,5,3,1,64)[0] == 0
        # Feed caller microphone IPC, then verify those exact frames on USB.
        expected = bytearray()
        for block in range(8):
            samples = [(100+(block*128+frame)%97)+channel*100 for frame in range(128) for channel in range(microphones)]
            pcm = struct.pack('<'+'h'*len(samples), *samples)
            expected.extend(pcm)
            header = b'VGPA'+bytes([1,0,tag,1])+struct.pack('<QQIIQ',7,block*128,128,0,block)
            microphone.sendall(header+pcm)
        # Bounded warm-up permits the separate PCM thread to prime its queue.
        time.sleep(.03)
        status, actual, captured, _ = probe.transfer(usb,6,2,1,49*microphones*2*8,packets=8)
        assert status == 0 and actual == 48*microphones*2*8
        assert captured == expected[:actual]
        # Host USB playback must traverse the independent outbound PCM channel.
        pcm = struct.pack('<'+'h'*channels, *range(1,channels+1))*48*8
        assert probe.transfer(usb,7,1,0,len(pcm),payload=pcm,packets=8)[:2] == (0,len(pcm))
        received = bytearray()
        position = 0
        while len(received) < len(pcm):
            header = probe.exact(playback,40)
            assert header[:8] == b'VGPA'+bytes([1,0,tag,0])
            gen, first, count, flags, _ = struct.unpack('<QQIIQ',header[8:])
            assert gen == 7 and first == position and flags == 0 and 0 < count <= 128
            received.extend(probe.exact(playback,count*channels*2))
            position += count
        assert received == pcm
        message(control,3,generation)
        operation, counters = receive(control)
        assert operation == 3 and len(counters) == 72
        message(control,4,generation)
        assert receive(control) == (4,generation)
        status = wait(pid,3)
        reaped = True
        assert status == 0, status
        assert playback.recv(1) == b''
        print(f'{family}: production worker enumeration, bidirectional PCM IPC, diagnostics and closure passed')
    finally:
        if not reaped:
            found, _ = os.waitpid(pid,os.WNOHANG)
            if not found:
                os.kill(pid,signal.SIGTERM)
                os.waitpid(pid,0)
        for parent, _ in pairs:
            parent.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker',type=Path,required=True)
    args = parser.parse_args()
    worker = args.worker.resolve(strict=True)
    for family, tag, channels, microphones in [('dualsense',1,4,2),('dualshock4',2,2,1),('xbox360',3,2,1)]:
        trial(worker,family,tag,channels,microphones)


if __name__ == '__main__':
    main()
