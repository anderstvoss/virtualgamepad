#!/usr/bin/env python3
"""Verify a pinned corpus and reproducible checked-in test artifacts."""
import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

GENERATOR_VERSION = 2
PIN_FILE = 'protocol-corpus.rev'


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True, stderr=subprocess.PIPE).strip()


def read_pin(root):
    # The corpus is pinned by a plain revision file, not a submodule, so Git
    # consumers of this repository never need access to the private corpus.
    try:
        revision = (root / PIN_FILE).read_text().strip()
    except FileNotFoundError:
        raise ValueError(PIN_FILE + ' is missing') from None
    if not re.fullmatch('[0-9a-f]{40}', revision):
        raise ValueError(PIN_FILE + ' must contain one full lowercase commit SHA')
    return revision


def check(root, write=False, verify_remote=False):
    root = Path(root).resolve()
    corpus = root / 'protocol-corpus'
    revision = read_pin(root)
    if not (corpus / '.git').exists():
        raise ValueError('corpus absent: clone the private corpus into protocol-corpus/ and check out '
                         'the revision in protocol-corpus.rev')
    if git(corpus, 'rev-parse', 'HEAD') != revision:
        raise ValueError('corpus HEAD differs from protocol-corpus.rev; adopt or restore the intended revision')
    if git(corpus, 'status', '--porcelain'):
        raise ValueError('corpus has uncommitted changes')
    if verify_remote:
        subprocess.run(['git', '-C', str(corpus), 'fetch', 'origin', 'main'], check=True)
        subprocess.run(['git', '-C', str(corpus), 'merge-base', '--is-ancestor', revision, 'FETCH_HEAD'], check=True)
    outputs = {}
    inputs = {}
    for name, expected_size in [('ds-neutral', 64), ('ds-cross', 64),
                                ('ds4-active-layout', 64), ('sony-contact-pair', 8)]:
        record_path = 'records/fixture/' + name + '.json'
        record_bytes = (corpus / record_path).read_bytes()
        record = json.loads(record_bytes)
        path = (corpus / record['path']).resolve()
        if not path.is_relative_to(corpus) or record['kind'] != 'synthetic':
            raise ValueError('expected a corpus-local synthetic fixture')
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != record['sha256']:
            raise ValueError('fixture hash mismatch: ' + name)
        if len(bytes.fromhex(data.decode().strip())) != expected_size:
            raise ValueError('unexpected fixture size: ' + name)
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
