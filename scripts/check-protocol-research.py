#!/usr/bin/env python3
"""Verify the pinned protocol-research submodule and reproducible checked-in test artifacts."""
import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

GENERATOR_VERSION = 4
SUBMODULE = 'protocol-research'


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], text=True, stderr=subprocess.PIPE).strip()


def read_pin(root):
    # The research repository is a submodule marked `update = none` in
    # .gitmodules, so Cargo and `git clone --recurse-submodules` skip it and
    # Git consumers of this repository never fetch it. The gitlink in the
    # index is the pin.
    try:
        update = git(root, 'config', '--file', '.gitmodules', 'submodule.' + SUBMODULE + '.update')
    except subprocess.CalledProcessError:
        update = ''
    if update != 'none':
        raise ValueError('.gitmodules must mark ' + SUBMODULE + ' with update = none')
    entry = git(root, 'ls-files', '--stage', '--', SUBMODULE)
    match = re.fullmatch(r'160000 ([0-9a-f]{40}) 0\t' + SUBMODULE, entry)
    if not match:
        raise ValueError(SUBMODULE + ' must be a submodule gitlink')
    return match.group(1)


def check_acceptance_evidence(root, research, pin):
    """Resolve evidence at each manifest's revision, not just current corpus HEAD."""
    inventories = {}
    for path in sorted((root / 'acceptance').glob('*.toml')):
        manifest = tomllib.loads(path.read_text())
        revision = manifest['research_revision']
        if revision not in inventories:
            try:
                if not re.fullmatch(r'[0-9a-fA-F]{40}', revision):
                    raise ValueError('expected a full commit hash')
                git(research, 'merge-base', '--is-ancestor', revision, pin)
                tree = git(research, 'ls-tree', '-r', '--name-only', revision, 'records')
            except (ValueError, subprocess.CalledProcessError) as error:
                raise ValueError('acceptance research revision unavailable or outside pin: '
                                 + path.name + ': ' + revision) from error
            inventories[revision] = {Path(name).stem for name in tree.splitlines()
                                     if name.endswith('.json')}
        references = {record for function in manifest['functions']
                      for cell in function['cells'].values() for record in cell['evidence']}
        missing = references - inventories[revision]
        if missing:
            raise ValueError('missing acceptance evidence at declared revision: '
                             + path.name + ': ' + ', '.join(sorted(missing)))


def check(root, write=False, verify_remote=False):
    root = Path(root).resolve()
    research = root / SUBMODULE
    revision = read_pin(root)
    if not (research / '.git').exists():
        raise ValueError('research checkout absent: run git submodule update --init --checkout ' + SUBMODULE)
    if git(research, 'rev-parse', 'HEAD') != revision:
        raise ValueError('research HEAD differs from the pinned gitlink; git add ' + SUBMODULE
                         + ' to adopt it or git submodule update --checkout to restore the pin')
    if git(research, 'status', '--porcelain'):
        raise ValueError('research checkout has uncommitted changes')
    if verify_remote:
        subprocess.run(['git', '-C', str(research), 'fetch', 'origin', 'main'], check=True)
        subprocess.run(['git', '-C', str(research), 'merge-base', '--is-ancestor', revision, 'FETCH_HEAD'], check=True)
    check_acceptance_evidence(root, research, revision)
    outputs = {}
    inputs = {}
    for name, expected_size in [('ds-neutral', 64), ('ds-cross', 64),
                                ('ds4-active-layout', 64), ('sony-contact-pair', 8),
                                ('wii-c5-neutral-30', 3), ('wii-c5-neutral-31', 6),
                                ('wii-c5-neutral-32', 11), ('wii-c5-neutral-33', 18),
                                ('wii-c5-neutral-34', 22), ('wii-c5-neutral-35', 22),
                                ('wii-c5-neutral-36', 22), ('wii-c5-neutral-37', 22),
                                ('wii-c5-neutral-3d', 22), ('wii-c5-neutral-3e', 22),
                                ('wii-c5-neutral-3f', 22), ('wii-c5-status-low', 7),
                                ('wii-c5-write-invalid-address', 5),
                                ('wii-c5-read-invalid-address', 22),
                                ('wii-c5-functional-descriptor', 191),
                                ('wii-c5-evdev-golden', None),
                                ('steam-c6-neutral-42', 54),
                                ('steam-c6-neutral-45', 46),
                                ('steam-c6-neutral-47', 46),
                                ('steam-c6-buttons-all-42', 54),
                                ('steam-c6-stick-trigger-max-42', 54),
                                ('steam-c6-stick-trigger-min-42', 54),
                                ('steam-c6-touch-corners-pressure-42', 54),
                                ('steam-c6-imu-extremes-42', 54),
                                ('steam-c6-full-edge-45', 46),
                                ('steam-c6-timestamp-touch-47', 46),
                                ('steam-c6-battery-status-43', 15),
                                ('steam-c6-output-rumble-80', 10),
                                ('steam-c6-output-pulse-81', 8),
                                ('steam-c6-output-command-82', 4),
                                ('steam-c6-output-lfo-83', 10),
                                ('steam-c6-output-sweep-84', 9),
                                ('steam-c6-output-script-85', 4),
                                ('steam-c6-output-rumble-80-fields', 10),
                                ('steam-c6-output-pulse-81-fields', 8),
                                ('steam-c6-output-command-82-fields', 4),
                                ('steam-c6-output-lfo-83-fields', 10),
                                ('steam-c6-output-sweep-84-fields', 9),
                                ('steam-c6-output-script-85-fields', 4),
                                ('steam-c6-feature-serial-get', 64),
                                ('steam-c6-feature-attrs-get', 64),
                                ('steam-c6-feature-lizard-enable-map', 64),
                                ('steam-c6-feature-lizard-enable-defaults', 64),
                                ('steam-c6-feature-lizard-disable-clear', 64),
                                ('steam-c6-feature-lizard-disable-kernel', 64),
                                ('steam-c6-feature-lizard-disable-sdl', 64),
                                ('steam-c6-feature-imu-on', 64),
                                ('steam-c6-feature-imu-off', 64),
                                ('steam-c6-reply-serial-empty', 64),
                                ('steam-c6-reply-attributes-empty', 64),
                                ('steam-c6-reply-attributes-short', 3)]:
        record_path = 'records/fixture/' + name + '.json'
        record_bytes = (research / record_path).read_bytes()
        record = json.loads(record_bytes)
        path = (research / record['path']).resolve()
        if not path.is_relative_to(research) or record['kind'] != 'synthetic':
            raise ValueError('expected a research-local synthetic fixture')
        data = path.read_bytes()
        if hashlib.sha256(data).hexdigest() != record['sha256']:
            raise ValueError('fixture hash mismatch: ' + name)
        extension = '.hex'
        if expected_size is None:
            extension = '.json'
            shape = json.loads(data)
            counts = [sum(c['source_index'] == i for c in shape['controls']) for i in range(6)]
            if not shape['manifest']['synthetic'] or counts != [12, 3, 8, 7, 21, 3]:
                raise ValueError('unexpected consumer shape: ' + name)
        elif len(bytes.fromhex(data.decode().strip())) != expected_size:
            raise ValueError('unexpected fixture size: ' + name)
        inputs[record_path] = hashlib.sha256(record_bytes).hexdigest()
        inputs[record['path']] = record['sha256']
        outputs[name + extension] = data
    outputs['manifest.json'] = (json.dumps(dict(corpus_commit=revision, schema_version=1,
        generator_version=GENERATOR_VERSION, inputs=inputs), indent=2, sort_keys=True) + '\n').encode()
    target = root / 'tests/fixtures/protocol-research'
    if write:
        target.mkdir(parents=True, exist_ok=True)
        for name, data in outputs.items():
            (target / name).write_bytes(data)
    for name, data in outputs.items():
        if not (target / name).exists() or (target / name).read_bytes() != data:
            raise ValueError('stale generated fixture: ' + name + '; run scripts/check-protocol-research.py --write')
    return revision


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    parser.add_argument('--verify-remote', action='store_true')
    args = parser.parse_args()
    try:
        print('Protocol research verified:', check(Path(__file__).resolve().parents[1], args.write, args.verify_remote))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit('Protocol research validation failed: ' + str(error))
