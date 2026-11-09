#!/usr/bin/env python3
"""Reject staged/tracked Wii EEPROM envelopes and private dump extensions.

Inspect Git index blobs, never working-tree dump content. Diagnostics contain
only repository-relative names, never content, calibration values or hashes.
"""
from pathlib import Path
import subprocess
import sys

MAGIC = b'VGPWEEP\0'
DUMP_EXTENSION = '.wii-eeprom'


def violations(repo):
    entries = subprocess.check_output(
        ['git', 'ls-files', '--stage', '-z'], cwd=repo).split(b'\0')
    files = []
    for entry in filter(None, entries):
        metadata, name = entry.split(b'\t', 1)
        mode, oid, _stage = metadata.split()
        if mode != b'160000':  # Submodule contents have their own index.
            files.append((name, oid))
    result = subprocess.run(
        ['git', 'cat-file', '--batch'], cwd=repo,
        input=b''.join(oid + b'\n' for _, oid in files),
        stdout=subprocess.PIPE, check=True).stdout
    offset = 0
    rejected = set()
    for name, _oid in files:
        line_end = result.index(b'\n', offset)
        size = int(result[offset:line_end].rsplit(b' ', 1)[1])
        start = line_end + 1
        content = result[start:start + size]
        offset = start + size + 1
        if name.lower().endswith(DUMP_EXTENSION.encode()) or MAGIC in content:
            rejected.add(name.decode('utf-8', errors='replace'))
    return sorted(rejected)


def main():
    rejected = violations(Path(__file__).resolve().parents[1])
    for name in rejected:
        print(f'Private Wii EEPROM file must not be tracked: {name}', file=sys.stderr)
    return int(bool(rejected))


if __name__ == '__main__':
    sys.exit(main())
