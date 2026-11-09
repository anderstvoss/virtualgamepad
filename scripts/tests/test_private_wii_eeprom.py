"""Synthetic-only index guard regressions; never read physical EEPROM data."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'check-private-wii-eeprom.py'
SPEC = importlib.util.spec_from_file_location('private_wii_eeprom', SCRIPT)
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)


class PrivateEepromGuardTests(unittest.TestCase):
    def test_index_rejects_magic_and_extension_even_when_worktree_was_replaced(self):
        with tempfile.TemporaryDirectory() as directory:
            repo = Path(directory)
            subprocess.run(['git', 'init', '-q', directory], check=True)
            for name, content in {
                'ordinary.bin': b'synthetic only',
                'hidden.bin': b'prefix' + guard.MAGIC + b'synthetic only',
                'synthetic.WII-EEPROM': b'synthetic only',
            }.items():
                (repo / name).write_bytes(content)
            subprocess.run(['git', 'add', '.'], cwd=repo, check=True)
            (repo / 'hidden.bin').write_bytes(b'replaced worktree')
            self.assertEqual(guard.violations(repo), ['hidden.bin', 'synthetic.WII-EEPROM'])

    def test_current_repository_index_contains_no_private_envelopes(self):
        self.assertEqual(guard.violations(SCRIPT.parents[1]), [])

    def test_dump_extension_is_ignored(self):
        result = subprocess.run(
            ['git', 'check-ignore', '--no-index', '--stdin'], cwd=SCRIPT.parents[1],
            input='synthetic.wii-eeprom\n', text=True, capture_output=True, check=False)
        self.assertEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
