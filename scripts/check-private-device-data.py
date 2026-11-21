#!/usr/bin/env python3
"""Reject indexed private controller data by path, envelope and conservative shape.

Reads Git index blobs, not working-tree files. Diagnostics display names only;
neither content nor blob IDs are printed. No physical device data is required.
"""
from pathlib import Path
import subprocess
import sys

MAGIC = b'VGPWEEP\0'
DUMP_EXTENSIONS = ('.wii-eeprom', '.switch-spi', '.sony-feature', '.device-dump')
DATA_EXTENSIONS = ('.bin', '.dump', '.dat', '.raw', '.hex')
WII_LENGTHS = {0x1700, 0x10000}
SWITCH_LENGTHS = {0x8000, 0x10000, 0x80000}
SONY_LENGTHS = {16, 20, 37, 41, 49, 64}
DATA_HINTS = ('eeprom', 'spi', 'firmware', 'calibration', 'pairing', 'device-dump')
SONY_HINTS = ('dualsense', 'dualshock', 'ds4', 'sony', 'hid-feature')


def _looks_binary(content):
    if not content:
        return False
    sample = content[:1024]
    return b'\0' in sample or sum(
        byte < 9 or 13 < byte < 32 or byte > 126 for byte in sample
    ) * 5 >= len(sample)


def suspicious(name, content):
    """Conservative shape heuristics; no attempt to identify a physical unit."""
    lower = name.decode('utf-8', errors='replace').replace('\\', '/').lower()
    parts = lower.split('/')
    basename = parts[-1]
    if 'private' in parts or any('.private.' in segment for segment in parts):
        return True
    if any(basename.endswith(extension) for extension in DUMP_EXTENSIONS):
        return True
    if MAGIC in content:
        return True
    extension = '.' + basename.rsplit('.', 1)[-1] if '.' in basename else ''
    if extension not in DATA_EXTENSIONS:
        return False
    hint = any(word in lower for word in DATA_HINTS)
    sony = any(word in lower for word in SONY_HINTS)
    binary = _looks_binary(content)
    if binary and (hint or sony):
        return True
    # Raw whole-image blobs may be mislabeled generically; catch typical sizes.
    if binary and extension in ('.bin', '.dump', '.raw'):
        return len(content) in WII_LENGTHS | SWITCH_LENGTHS
    # Feature-only blobs have small, legitimate sizes; require a family clue.
    return sony and len(content) in SONY_LENGTHS


def violations(repo):
    entries = subprocess.check_output(
        ['git', 'ls-files', '--stage', '-z'], cwd=repo).split(b'\0')
    files = []
    for entry in filter(None, entries):
        metadata, name = entry.split(b'\t', 1)
        mode, oid, _stage = metadata.split()
        if mode != b'160000':
            files.append((name, oid))
    if not files:
        return []
    output = subprocess.run(
        ['git', 'cat-file', '--batch'], cwd=repo,
        input=b''.join(oid + b'\n' for _, oid in files),
        stdout=subprocess.PIPE, check=True).stdout
    offset = 0
    rejected = set()
    for name, _ in files:
        end = output.index(b'\n', offset)
        size = int(output[offset:end].rsplit(b' ', 1)[1])
        start = end + 1
        content = output[start:start + size]
        offset = start + size + 1
        if suspicious(name, content):
            rejected.add(name.decode('utf-8', errors='replace'))
    return sorted(rejected)


def main():
    rejected = violations(Path(__file__).resolve().parents[1])
    for name in rejected:
        print(f'Private device data must not be tracked: {name}', file=sys.stderr)
    return int(bool(rejected))


if __name__ == '__main__':
    sys.exit(main())
