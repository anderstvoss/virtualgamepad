"""No-device safety checks; live ownership belongs to the isolated input runner."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


@unittest.skipUnless(sys.platform == 'linux' and shutil.which('cc'), 'Linux C compiler required')
class NativeFeedbackProbe(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        cls.image = Path(cls.directory.name) / 'feedback'
        source = Path(__file__).parents[1] / 'alpha-evdev-feedback-probe.c'
        subprocess.run(['cc', '-std=gnu11', '-O2', '-Wall', '-Wextra', '-Werror',
                        str(source), '-o', str(cls.image)], check=True, capture_output=True)

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def test_invalid_arguments_never_request_a_device_operation(self):
        for arguments in ([], [''], ['bad'], ['-1'], ['1025'], ['999999999999999999999999999'], ['0', 'extra'], ['0', '--exit-active', 'extra']):
            with self.subTest(arguments=arguments):
                result = subprocess.run([str(self.image), *arguments], capture_output=True, timeout=2)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, b'')

    def test_non_input_descriptors_cannot_report_success(self):
        with tempfile.TemporaryFile() as regular:
            result = subprocess.run([str(self.image), str(regular.fileno())],
                                    pass_fds=(regular.fileno(),), capture_output=True, timeout=2)
            self.assertEqual(result.returncode, 2)
        descriptor = os.open('/dev/null', os.O_RDWR | os.O_CLOEXEC)
        try:
            result = subprocess.run([str(self.image), str(descriptor)], pass_fds=(descriptor,),
                                    capture_output=True, timeout=2)
            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stdout, b'')
        finally:
            os.close(descriptor)


if __name__ == '__main__':
    unittest.main()
