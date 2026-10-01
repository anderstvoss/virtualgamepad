#!/usr/bin/env python3
"""Compile root-only downstream consumers across features, then rebuild offline.

Uses cached dependencies and a disposable source-path consumer; does not create
controllers. Exact Git release consumption is a later release gate.
"""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    with tempfile.TemporaryDirectory(prefix='vgpd-alpha-consumer-') as directory:
        consumer = Path(directory)
        # The repo path is discovered, not a tracked machine-specific assumption.
        quoted_path = str(ROOT).replace('\\', '\\\\').replace('"', '\\"')
        (consumer/'Cargo.toml').write_text(
            '[package]\nname = "alpha-root-consumer"\nversion = "0.0.0"\nedition = "2024"\n'
            '[workspace]\n[dependencies]\n'
            f'virtualgamepad = {{ path = "{quoted_path}", default-features = false }}\n'
            '[features]\nexperimental = ["virtualgamepad/experimental"]\n'
            'audio-pipewire = ["virtualgamepad/audio-pipewire"]\n'
            'audio-usbip = ["virtualgamepad/audio-usbip"]\n')
        (consumer/'src').mkdir()
        (consumer/'src/lib.rs').write_text((ROOT/'tests/ui/root_audio_consumer.rs').read_text())
        env = dict(os.environ, CARGO_TARGET_DIR=str(ROOT/'target/alpha-consumer'))
        common = ['--manifest-path', str(consumer/'Cargo.toml')]
        subprocess.run(['cargo', 'generate-lockfile', '--offline', *common], env=env, check=True)
        for features in ['', 'experimental', 'audio-pipewire', 'audio-usbip', 'audio-pipewire,audio-usbip']:
            command = ['cargo', 'check', '--offline', '--locked', '--all-targets', *common]
            if features:
                command += ['--features', features]
            subprocess.run(command, env=env, check=True)
        # Demonstrate a second cached build with the same lock and feature set.
        subprocess.run(['cargo', 'check', '--offline', '--locked', *common,
                        '--features', 'audio-pipewire,audio-usbip'], env=env, check=True)
        print('Root-only feature matrix and cached offline rebuild passed.')


if __name__ == '__main__':
    main()
