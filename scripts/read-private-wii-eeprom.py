#!/usr/bin/env python3
"""Read only the documented Wii EEPROM window into private caller storage.

Raw mode requires owner-managed hid-generic binding and a temporary hidraw ACL. Sends
ONLY output report 0x17 with EEPROM flags zero; no register access, writes,
LED/status/report-mode commands or initialization. Stop on the first error.
Alternatively, accept exactly one bounded dump on stdin from the owner-run
serialized kernel debugfs reader (0x17 under the kernel command lock).
Range: merged wii-gap-wiibrew-host-eeprom-visible-range and
wii-gap-wiibrew-memory-request-response-reports research claims.
"""
import argparse
import os
from pathlib import Path
import re
import select
import stat
import struct
import sys
import time
import zlib

MAGIC = b'VGPWEEP\0'
EXTENT = 0x1700
REVISIONS = {'original': 0, 'early-plus': 1, 'tr': 2}


def request(address, count):
    if not 0 <= address < EXTENT or not 1 <= count <= 16 or address + count > EXTENT:
        raise ValueError('Read outside documented EEPROM window')
    return bytes([0x17, 0, 0, address >> 8, address & 255, 0, count])


def read_image(send, receive):
    image = bytearray()
    for address in range(0, EXTENT, 16):
        count = min(16, EXTENT - address)
        report = request(address, count)
        if send(report) != len(report):
            raise RuntimeError('Short read-request delivery; stopped')
        deadline = time.monotonic() + 5
        while True:
            reply = receive(max(0, deadline - time.monotonic()))
            if not reply:
                raise RuntimeError('Memory-read timeout/disconnect; stopped')
            if reply[0] != 0x21:
                if time.monotonic() >= deadline:
                    raise RuntimeError('Memory-read timeout; stopped')
                continue
            if len(reply) != 22 or reply[3] & 15:
                raise RuntimeError('Memory-read error/malformed response; stopped')
            actual = int.from_bytes(reply[4:6], 'big')
            if actual != address or (reply[3] >> 4) + 1 != count:
                raise RuntimeError('Unexpected memory-read response; stopped')
            image.extend(reply[6:6 + count])
            break
    return bytes(image)


def envelope(image, revision):
    if len(image) != EXTENT:
        raise ValueError('Incomplete EEPROM image')
    # Optional extension calibration blocks are absent: register reads are not
    # authorized here. Base Remote calibration remains inside the EEPROM bytes.
    content = MAGIC + struct.pack('<HBBIB', 1, 1, REVISIONS[revision], len(image), 0) + image
    return content + struct.pack('<I', zlib.crc32(content))


def private_directory():
    directory = Path.home() / '.local/share/virtualgamepad-private/wii-eeprom'
    directory = directory.resolve()
    if any((parent / '.git').exists() for parent in (directory, *directory.parents)):
        raise RuntimeError('Private storage must be outside every Git checkout')
    # Every directory created by this helper is 0700, even with a permissive umask.
    missing = []
    ancestor = directory
    while not ancestor.exists():
        missing.append(ancestor)
        ancestor = ancestor.parent
    for parent in reversed(missing):
        parent.mkdir(mode=0o700)
    for parent in [directory, directory.parent]:
        if stat.S_IMODE(parent.stat().st_mode) != 0o700 or parent.stat().st_uid != os.getuid():
            raise RuntimeError('Private storage must be owner-owned and mode 0700')
    return directory


def checked_device(node):
    node = Path(node)
    if node.parent != Path('/dev') or not re.fullmatch(r'hidraw[0-9]+', node.name):
        raise RuntimeError('Expected a hidraw device node')
    device = (Path('/sys/class/hidraw') / node.name / 'device').resolve(strict=True)
    properties = dict(line.split('=', 1) for line in (device / 'uevent').read_text().splitlines() if '=' in line)
    if properties.get('HID_ID') not in ('0005:0000057E:00000306', '0005:0000057E:00000330'):
        raise RuntimeError('Not a supported physical Bluetooth Wii Remote')
    if not properties.get('HID_NAME', '').startswith('Nintendo RVL-CNT-01') or 'virtualgamepad' in properties.get('HID_PHYS', '').lower():
        raise RuntimeError('Not a physical Nintendo Remote')
    if (device / 'driver').resolve().name != 'hid-generic':
        raise RuntimeError('Owner must bind hid-generic first to avoid kernel EEPROM-read races')
    return node


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--device')
    source.add_argument('--from-stdin', action='store_true', help='Owner-run kernel reader piped directly here; never echo raw data')
    parser.add_argument('--revision', required=True, choices=REVISIONS)
    parser.add_argument('--name', required=True, help='Caller label, never a device identifier')
    args = parser.parse_args()
    try:
        if os.geteuid() == 0:
            raise RuntimeError('Run as the ordinary user with a temporary device ACL')
        if not re.fullmatch(r'[A-Za-z0-9_-]+', args.name):
            raise RuntimeError('Choose a simple caller label')
        node = checked_device(args.device) if args.device else None
        destination = private_directory() / (args.name + '.wii-eeprom')
        if destination.exists():
            raise RuntimeError('Destination already exists; choose a new caller label')
        if args.from_stdin:
            image = sys.stdin.buffer.read(EXTENT + 1)
            if len(image) != EXTENT:
                raise RuntimeError('Incomplete or oversized kernel EEPROM dump')
        else:
            fd = os.open(node, os.O_RDWR | os.O_NONBLOCK | os.O_CLOEXEC)
            try:
                def receive(timeout):
                    ready, _, _ = select.select([fd], [], [], timeout)
                    return os.read(fd, 64) if ready else b''
                image = read_image(lambda report: os.write(fd, report), receive)
            finally:
                os.close(fd)
        # Complete success only: no partial dump is saved. Never replace a dump.
        fd = os.open(destination, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_CLOEXEC, 0o600)
        try:
            with os.fdopen(fd, 'wb') as output:
                output.write(envelope(image, args.revision))
                output.flush()
                os.fsync(output.fileno())
        except BaseException:
            destination.unlink()
            raise
        print(f'Private EEPROM read passed; {len(image)} bytes saved. No content or hashes printed.')
        return 0
    except (OSError, ValueError, RuntimeError):
        # Do not allow exceptions or paths to leak real content/identifiers.
        print('Private EEPROM read failed; stopped without probing other addresses. Check binding, ACL and connection locally.', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
