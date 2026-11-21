"""Synthetic-only regressions for the Git index device-data guard."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'check-private-device-data.py'
SPEC = importlib.util.spec_from_file_location('device_data_guard', SCRIPT)
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)


class PrivateDeviceDataTests(unittest.TestCase):
    def test_paths_and_legacy_wii_envelope_are_blocked(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(['git', 'init', '-q', directory], check=True)
            fixtures = {
                'private/opaque.txt': b'synthetic-only',
                'synthetic.private.bin': b'synthetic-only',
                'legacy.wii-eeprom': b'synthetic-only',
                'nested/legacy.bin': guard.MAGIC + b'synthetic-only',
                'ordinary.txt': b'synthetic-only',
            }
            for name, data in fixtures.items():
                destination = repo / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
            subprocess.run(['git', 'add', '-f', '.'], cwd=repo, check=True)
            (repo / 'nested/legacy.bin').write_bytes(b'worktree no longer matches index')
            self.assertEqual(
                guard.violations(repo),
                ['legacy.wii-eeprom', 'nested/legacy.bin',
                 'private/opaque.txt', 'synthetic.private.bin'],
            )

    def test_synthetic_shaped_blobs_are_rejected_without_physical_data(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(['git', 'init', '-q', directory], check=True)
            sample = bytes(range(256))
            fixtures = {
                'generic.bin': sample * 23,
                'switch-spi.bin': sample,
                'sony-feature.bin': sample[:41],
                'ordinary.bin': b'ordinary printable fixture',
                'notes.md': 'Synthetic feature descriptions only.\\n'.encode(),
            }
            for name, data in fixtures.items():
                (repo / name).write_bytes(data)
            subprocess.run(['git', 'add', '.'], cwd=repo, check=True)
            self.assertEqual(
                guard.violations(repo),
                ['generic.bin', 'sony-feature.bin', 'switch-spi.bin'],
            )

    def test_ignore_rules_and_existing_index(self):
        root = SCRIPT.parents[1]
        self.assertEqual(guard.violations(root), [])
        check = subprocess.run(
            ['git', 'check-ignore', '--no-index', '--stdin'], cwd=root,
            input='private/example.txt\\nsynthetic.private.bin\\n',
            text=True, capture_output=True, check=False)
        self.assertEqual(check.returncode, 0)
        self.assertEqual(len(check.stdout.strip().splitlines()), 2)


if __name__ == '__main__':
    unittest.main()
