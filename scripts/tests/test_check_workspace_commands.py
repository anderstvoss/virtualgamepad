import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('workspace_commands', Path(__file__).resolve().parents[1] / 'check-workspace-commands.py')
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class CommandTests(unittest.TestCase):
    def test_obsolete_package_missing_example_and_missing_validator_fail(self):
        packages = [{'name': 'gr-usbip', 'targets': [{'name': 'usb_audio_probe', 'kind': ['example']}]}]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertEqual(module.validate('cargo build --locked -p gr-usbip --example usb_audio_probe', packages, root), 1)
            for line in ('cargo test -p gr-cli', 'cargo build -p gr-usbip --example removed',
                         'python3 scripts/missing.py', 'echo no validation'):
                with self.assertRaises(ValueError):
                    module.validate(line, packages, root)
