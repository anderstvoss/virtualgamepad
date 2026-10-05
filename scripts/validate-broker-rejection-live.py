#!/usr/bin/env python3
"""Bounded non-root rejection probes for the isolated candidate broker lab only.

The maintenance runner supplies the private socket mapping and dropped identity.
No successful controller construction or installed-provider recovery is claimed.
"""
import argparse
import array
import json
import os
import socket
import struct
import time

REASON = (b'privileged host session failed: dummy_hcd HID is unavailable: Linux f_hid exposes '
          b'GET_REPORT IDs without report type/request length or an explicit negative reply; '
          b'complete controller request semantics cannot be represented')


def frame(tag, body=b''):
    return struct.pack('<IHB', len(body) + 3, 1, tag) + body


def exact(peer, count):
    result = bytearray()
    while len(result) < count:
        data, ancillary, flags, _ = peer.recvmsg(count - len(result), socket.CMSG_SPACE(256 * 4),
                                               socket.MSG_CMSG_CLOEXEC)
        unexpected = bool(ancillary)
        for level, kind, payload in ancillary:
            if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
                descriptors = array.array('i')
                descriptors.frombytes(payload[:len(payload) // descriptors.itemsize * descriptors.itemsize])
                for descriptor in descriptors:
                    os.close(descriptor)
        if unexpected or flags & socket.MSG_CTRUNC or not data:
            raise RuntimeError('unexpected descriptors, truncation or premature EOF')
        result.extend(data)
    return bytes(result)


def expect_error(peer, reason):
    length, = struct.unpack('<I', exact(peer, 4))
    if not 3 <= length <= 259:
        raise RuntimeError('unbounded response length')
    if exact(peer, length) != struct.pack('<HB', 1, 0x81) + reason:
        raise RuntimeError('incorrect rejection response')


def connect():
    peer = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    try:
        peer.settimeout(3)
        peer.connect('/run/virtualgamepad/broker.sock')
        if struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))[1] != 0:
            raise RuntimeError('candidate broker must have root peer identity')
        return peer
    except BaseException:
        peer.close()
        raise


def closed(peer):
    try:
        if peer.recv(1):
            raise RuntimeError('malformed framing was not terminated')
    except ConnectionResetError:
        pass


def probe(unauthorized=False):
    if os.geteuid() == 0 or os.getegid() == 0 or os.getgroups():
        raise RuntimeError('probe requires an unprivileged identity without supplementary groups')
    status = dict(line.split(':', 1) for line in open('/proc/self/status') if ':' in line)
    if (any(int(status[key].strip(), 16) for key in ('CapEff', 'CapPrm', 'CapAmb', 'CapBnd')) or
            status['NoNewPrivs'].strip() != '1'):
        raise RuntimeError('probe capabilities or privilege policy differ')
    if unauthorized:
        with connect() as peer:
            peer.sendall(frame(1, bytes((1, 1))))
            expect_error(peer, f'peer {os.geteuid()} is not authorized'.encode())
        return dict(scope='daemon peer authorization', uid=os.geteuid(), checks=1)
    checks = 0
    for _ in range(2):
        for family in range(1, 5):
            with connect() as peer:
                peer.sendall(frame(1, bytes((1, family))))
                expect_error(peer, REASON)
                # A second request proves progress after the first rejection.
                peer.sendall(frame(1, bytes((1, family))))
                expect_error(peer, REASON)
                checks += 2
    for tag, body in ((255, b''), (1, b''), (1, b'\1'), (1, b'\1\5'), (1, b'\2\1'),
                      (2, b''), (3, b''), (4, b''), (5, b'')):
        with connect() as peer:
            peer.sendall(frame(tag, body))
            expect_error(peer, b'malformed broker request')
        checks += 1
    for payload in (struct.pack('<I', 260), struct.pack('<I', 2), b'\x05', frame(1, b'\1\1')[:-1]):
        with connect() as peer:
            peer.sendall(payload)
            if len(payload) > 1:
                peer.shutdown(socket.SHUT_WR)
            # The single-byte case must hit the absolute partial-frame deadline.
            closed(peer)
        checks += 1
    reader, writer = os.pipe2(os.O_NONBLOCK | os.O_CLOEXEC)
    try:
        with connect() as peer:
            peer.sendmsg([frame(1, b'\1\1')], [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                                                array.array('i', [writer]))])
            os.close(writer); writer = None
            closed(peer)
        deadline = time.monotonic() + 2
        while True:
            try:
                if os.read(reader, 1) != b'':
                    raise RuntimeError('unexpected descriptor use')
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    raise RuntimeError('broker retained unexpected descriptor') from None
                time.sleep(.01)
        checks += 1
    finally:
        os.close(reader)
        if writer is not None:
            os.close(writer)
    return dict(scope='rejection, framing, progress and unexpected-FD ownership only',
                uid=os.geteuid(), checks=checks)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--unauthorized', action='store_true')
    print(json.dumps(probe(parser.parse_args().unauthorized)))
