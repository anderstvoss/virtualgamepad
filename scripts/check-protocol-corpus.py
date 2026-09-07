#!/usr/bin/env python3
"""Verify a pinned corpus and reproducible checked-in test artifacts."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

GENERATOR_VERSION = 1


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True, stderr=subprocess.PIPE).strip()


def check(root, write=False, verify_remote=False):
    root = Path(root).resolve()
    corpus = root / 'protocol-corpus'
    if not (corpus / '.git').exists():
        raise ValueError('corpus absent: run git submodule update --init --recursive with repository access')
    entry = git(root, 'ls-files', '--stage', 'protocol-corpus').split()
    if len(entry) != 4 or entry[0] != '160000' or entry[2] != '0':
        raise ValueError('protocol-corpus must have one staged gitlink')
    revision = entry[1]
    if git(corpus, 'rev-parse', 'HEAD') != revision:
        raise ValueError('corpus HEAD differs from the gitlink; adopt or restore the intended revision')
    if git(corpus, 'status', '--porcelain'):
        raise ValueError('corpus has uncommitted changes')
    if verify_remote:
        subprocess.run(['git', '-C', str(corpus), 'fetch', 'origin', 'main'], check=True)
        subprocess.run(['git', '-C', str(corpus), 'merge-base', '--is-ancestor', revision, 'FETCH_HEAD'], check=True)
    outputs = {}
    inputs = {}
    for name in ['ds-neutral', 'ds-cross']:
        record_path = 'records/fixture/' + name + '.json'
        record_bytes = (corpus / record_path).read_bytes()
        record = json.loads(record_bytes)
        path = (corpus / record['path']).resolve()
        if not path.is_relative_to(corpus) or record['kind'] != 'synthetic':
            raise ValueError('expected a corpus-local synthetic fixture')
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != record['sha256']:
            raise ValueError('fixture hash mismatch: ' + name)
        if len(bytes.fromhex(data.decode().strip())) != 64:
            raise ValueError('unexpected DualSense input fixture size')
        inputs[record_path] = hashlib.sha256(record_bytes).hexdigest()
        inputs[record['path']] = record['sha256']
        outputs[name + '.hex'] = data
    outputs['manifest.json'] = (json.dumps(dict(corpus_commit=revision, schema_version=1,
        generator_version=GENERATOR_VERSION, inputs=inputs), indent=2, sort_keys=True) + '\n').encode()
    target = root / 'tests/fixtures/protocol-corpus'
    if write:
        target.mkdir(parents=True, exist_ok=True)
        for name, data in outputs.items():
            (target / name).write_bytes(data)
    for name, data in outputs.items():
        if not (target / name).exists() or (target / name).read_bytes() != data:
            raise ValueError('stale generated fixture: ' + name + '; run scripts/check-protocol-corpus.py --write')
    return revision


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    parser.add_argument('--verify-remote', action='store_true')
    args = parser.parse_args()
    try:
        print('Corpus verified:', check(Path(__file__).resolve().parents[1], args.write, args.verify_remote))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit('Corpus validation failed: ' + str(error))
