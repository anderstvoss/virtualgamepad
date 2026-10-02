#!/usr/bin/env python3
"""Compile root-only downstream consumers across features, then rebuild offline.

Uses cached dependencies and a disposable source-path consumer; does not create
controllers. Exact Git release consumption is a later release gate.
"""
import os
from itertools import combinations
from pathlib import Path
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def feature_combinations(names):
    return [','.join(group) for count in range(len(names) + 1)
            for group in combinations(sorted(names), count)]


def main():
    features = tomllib.loads((ROOT/'Cargo.toml').read_text())['features']
    names = sorted(set(features) - {'default'})
    with tempfile.TemporaryDirectory(prefix='vgpd-alpha-consumer-') as directory:
        consumer = Path(directory)
        # The repo path is discovered, not a tracked machine-specific assumption.
        quoted_path = str(ROOT).replace('\\', '\\\\').replace('"', '\\"')
        (consumer/'Cargo.toml').write_text(
            '[package]\nname = "alpha-root-consumer"\nversion = "0.0.0"\nedition = "2024"\n'
            '[workspace]\n[dependencies]\n'
            f'virtualgamepad = {{ path = "{quoted_path}", default-features = false }}\n'
            '[features]\n' + ''.join(
                f'{name} = ["virtualgamepad/{name}"]\n' for name in names))
        (consumer/'src').mkdir()
        (consumer/'src/lib.rs').write_text((ROOT/'tests/ui/root_audio_consumer.rs').read_text())
        env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT/'target/alpha-consumer'))
        common = ['--manifest-path', str(consumer/'Cargo.toml')]
        subprocess.run(['cargo', 'generate-lockfile', '--offline', *common], env=env, check=True)
        for selected in feature_combinations(names):
            command = ['cargo', 'check', '--offline', '--locked', '--all-targets', *common]
            if selected:
                command += ['--features', selected]
            subprocess.run(command, env=env, check=True)
        # Demonstrate a second cached build with the same lock and feature set.
        subprocess.run(['cargo', 'check', '--offline', '--locked', *common,
                        '--features', 'audio-pipewire,audio-usbip'], env=env, check=True)
        print('Root-only feature matrix and cached offline rebuild passed.')


if __name__ == '__main__':
    main()
