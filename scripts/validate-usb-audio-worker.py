#!/usr/bin/env python3
"""Unprivileged process-level USB/IP test; no kernel attach or physical devices."""
import argparse
from pathlib import Path
import socket
import struct
import subprocess


def exact(peer, size):
    data = bytearray()
    while len(data) < size:
        part = peer.recv(size-len(data))
        if not part:
            raise RuntimeError('worker closed before completion')
        data.extend(part)
    return bytes(data)


def transfer(peer, sequence, endpoint, incoming, length, setup=bytes(8), payload=b'', packets=0):
    header = struct.pack('>10I', 1, sequence, 0x10001, incoming, endpoint, 0, length, 0, packets, 1)
    descriptors = b''
    if packets:
        size = length // packets
        descriptors = b''.join(struct.pack('>4I', i*size, size, 0, 0) for i in range(packets))
    peer.sendall(header + setup + payload + descriptors)
    response = exact(peer, 48)
    command, returned, device, direction, endpoint, status, actual, _, count, _ = struct.unpack('>10I', response[:40])
    assert (command, returned, device, direction, endpoint, count) == (3, sequence, 0, 0, 0, packets)
    data = exact(peer, actual) if incoming else b''
    iso = exact(peer, count*16)
    if status:
        assert actual == 0
    return status, actual, data, iso


def trial(worker, family, channels, microphones):
    peer, child = socket.socketpair()
    process = None
    with peer, child:
        peer.settimeout(2)
        process = subprocess.Popen([str(worker), family, str(0x10001), '1'],
                                   stdin=child, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        child.close()
        try:
            assert process.stdout.readline(65) == b'READY\n'
            result = transfer(peer,1,0,1,18,bytes([0x80,6,0,1,0,0,18,0]))
            assert result[:2] == (0,18) and result[2][:2] == bytes([18,1])
            for sequence, setup in [
                (2, [0,9,1,0,0,0,0,0]),
                (3, [1,11,1,0,1,0,0,0]),
                (4, [1,11,1,0,2,0,0,0]),
            ]:
                assert transfer(peer,sequence,0,0,0,bytes(setup))[:2] == (0,0)
            # Real compiled input/feature logic, not an echo personality.
            status, actual, data, _ = transfer(peer,5,3,1,64)
            assert status == 0 and actual > 0
            if family != 'xbox360':
                report_id = 9 if family == 'dualsense' else 0x12
                status, actual, data, _ = transfer(peer,6,0,1,64,bytes([0xa1,1,report_id,3,3,0,64,0]))
                assert status == 0 and actual > 0 and data[0] == report_id
            # Descriptor-valid setup whose controller payload is rejected must
            # return EPIPE after receipt of the payload, never a premature ACK.
            report_id = 2 if family == 'dualsense' else 5
            if family == 'xbox360':
                report_id = 0
            status, actual, _, _ = transfer(peer,7,0,0,2,
                bytes([0x21,9,report_id,3,3,0,2,0]),bytes([report_id,0]))
            assert status == (1 << 32)-32 and actual == 0
            payload = struct.pack('<'+'h'*channels, *range(1,channels+1))*48*8
            status, actual, _, iso = transfer(peer,8,1,0,len(payload),payload=payload,packets=8)
            assert status == 0 and actual == len(payload)
            assert all(struct.unpack_from('>4I',iso,i*16)[2:] == (48*channels*2,0) for i in range(8))
            status, actual, data, iso = transfer(peer,9,2,1,49*microphones*2*32,packets=32)
            assert status == 0 and actual == 48*microphones*2*32
            frames = list(struct.iter_unpack('<'+'h'*microphones,data))
            assert all(100 <= frame[0] <= 196 for frame in frames)
            assert all(frame[c]-frame[0] == 100*c for frame in frames for c in range(microphones))
            assert all((b[0]-100) == ((a[0]-100+1)%97) for a,b in zip(frames,frames[1:]))
            assert all(struct.unpack_from('>4I',iso,i*16)[2:] == (48*microphones*2,0) for i in range(32))
            stdout, stderr = process.communicate(timeout=3)
            if process.returncode:
                raise RuntimeError(f'{family}: {stderr.decode(errors="replace")}')
            print(f'{family}: enumeration, personality reports, exact SET stall, playback, capture pattern and timed shutdown passed')
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=2)
            process.stdout.close()
            process.stderr.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', type=Path, required=True)
    args = parser.parse_args()
    worker = args.worker.resolve(strict=True)
    for family, channels, microphones in [('dualsense',4,2), ('dualshock4',2,1), ('xbox360',2,1)]:
        trial(worker, family, channels, microphones)


if __name__ == '__main__':
    main()
