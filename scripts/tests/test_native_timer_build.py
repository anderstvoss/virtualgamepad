import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('timer_build', Path(__file__).parents[1] / 'build-alpha-native-timer.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class TimerBuildTests(unittest.TestCase):
    def test_default_does_not_build_or_mutate(self):
        with patch.object(module.argparse.ArgumentParser, 'parse_args', return_value=type('Args', (), {'apply':False})()), patch.object(module, 'build') as build, contextlib.redirect_stdout(io.StringIO()) as output:
            module.main()
        build.assert_not_called()
        self.assertFalse(json.loads(output.getvalue())['live_execution'])

    def test_root_or_dirty_candidate_rejects_before_creation(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / 'new'
            with patch.object(module.os, 'geteuid', return_value=0):
                with self.assertRaisesRegex(ValueError, 'ordinary-user'):
                    module.build(output, Path('.'))
            with patch.object(module.os, 'geteuid', return_value=1000), patch.object(module.subprocess, 'check_output', return_value=' M file'):
                with self.assertRaisesRegex(ValueError, 'clean committed'):
                    module.build(output, Path('.'))
            self.assertFalse(output.exists())

    def test_prebuild_hash_receipt_and_exclusive_directory(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / 'new'
            answers = ['', 'a'*40, 'b'*40, 'a'*40, '', 'fake cc version']
            def compile_image(command, **kwargs):
                self.assertEqual(kwargs, dict(check=True, timeout=60))
                self.assertEqual(command[1:6], ['-O2', '-Wall', '-Wextra', '-Werror', '-pthread'])
                Path(command[-1]).write_bytes(b'fake built diagnostic')
            repo = Path(__file__).resolve().parents[2]
            with patch.object(module.os, 'geteuid', return_value=1000), patch.object(module.shutil, 'which', return_value='/usr/bin/cc'), patch.object(module.subprocess, 'check_output', side_effect=answers), patch.object(module.subprocess, 'run', side_effect=compile_image):
                receipt = module.build(output, repo)
            self.assertEqual(receipt['revision'], 'a'*40)
            self.assertEqual(len(receipt['source_sha256']), 64)
            self.assertEqual(len(receipt['binary_sha256']), 64)
            self.assertEqual((output/'native-timer').stat().st_mode & 0o777, 0o500)
            self.assertEqual(json.loads((output/'build-receipt.json').read_text()), receipt)
            with patch.object(module.os, 'geteuid', return_value=1000), patch.object(module.shutil, 'which', return_value='/usr/bin/cc'), patch.object(module.subprocess, 'check_output', side_effect=['','a'*40,'b'*40]):
                with self.assertRaises(FileExistsError):module.build(output, repo)


if __name__ == '__main__':
    unittest.main()
