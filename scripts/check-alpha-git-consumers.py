#!/usr/bin/env python3
"""Validate root-only consumers against an exact published Git revision."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('consumers', ROOT/'scripts/check-root-consumers.py')
consumers = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(consumers)


def validate_revision(value):
    if not re.fullmatch(r'[0-9a-f]{40}', value):
        raise ValueError('Use an exact 40-character Git commit, not a branch or tag')
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', required=True, type=validate_revision)
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    manifest = tomllib.loads((ROOT/'Cargo.toml').read_text())
    url = manifest['workspace']['package']['repository']
    names = sorted(set(manifest['features'])-{'default'})
    with tempfile.TemporaryDirectory(prefix='alpha-git-consumer-') as directory:
        root = Path(directory)
        (root/'src').mkdir()
        (root/'Cargo.toml').write_text(
            '[package]\nname="alpha-git-consumer"\nversion="0.0.0"\nedition="2024"\n'
            '[workspace]\n[dependencies]\nvirtualgamepad = { git = '+json.dumps(url)+
            ', rev = '+json.dumps(args.revision)+', default-features = false }\n[features]\n'+
            ''.join(f'{name}=["virtualgamepad/{name}"]\n' for name in names))
        source = (ROOT/'tests/ui/root_audio_consumer.rs').read_text()
        (root/'src/lib.rs').write_text(source)
        env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT/'target/alpha-git-consumer'))
        common = ['--manifest-path', str(root/'Cargo.toml')]
        subprocess.run(['cargo', 'generate-lockfile', *common], env=env, check=True)
        lock = (root/'Cargo.lock').read_bytes()
        if args.revision not in lock.decode():
            raise RuntimeError('Resolved lock does not contain the exact requested Git revision')
        for selected in consumers.feature_combinations(names):
            command = ['cargo', 'check', '--locked', '--all-targets', *common]
            if selected:
                command += ['--features', selected]
            subprocess.run(command, env=env, check=True)
        subprocess.run(['cargo', 'check', '--locked', '--offline', *common,
                        '--features', ','.join(names)], env=env, check=True)
        if (root/'Cargo.lock').read_bytes() != lock:
            raise RuntimeError('Consumer lock changed during locked validation')
        lock_file = args.report.with_suffix('.lock')
        lock_file.write_bytes(lock)
        args.report.write_text(json.dumps({
            'revision': args.revision, 'repository': url,
            'consumer_sha256': hashlib.sha256(source.encode()).hexdigest(),
            'lock_sha256': hashlib.sha256(lock).hexdigest(),
            'lock_file': lock_file.name,
            'feature_combinations': consumers.feature_combinations(names),
            'compiler': subprocess.check_output(['rustc', '--version'], text=True).strip(),
            'cached_offline_rebuild': 'passed', 'scope': 'compile-only; no host I/O or hardware acceptance',
        }, indent=2)+'\n')


if __name__ == '__main__':
    main()
