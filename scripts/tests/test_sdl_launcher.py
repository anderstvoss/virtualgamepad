import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / 'run-sdl3-gamepad-probe.sh'


class SdlLauncher(unittest.TestCase):
    def test_private_build_cleanup_on_success_and_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / 'tools'
            tools.mkdir()
            temporary = root / 'temporary'
            temporary.mkdir()
            sentinel = temporary / 'virtualgamepad-sdl3-gamepad-probe'
            sentinel.write_text('unrelated research')
            pkg = tools / 'pkg-config'
            pkg.write_text('#!/bin/sh\nexit 0\n')
            pkg.chmod(0o700)
            cc = tools / 'cc'
            cc.write_text('''#!/bin/sh
[ "$FAIL_COMPILE" = 1 ] && exit 2
while [ "$#" -gt 0 ]; do
  if [ "$1" = -o ]; then
    shift
    printf '#!/bin/sh\\nexit %s\\n' "${PROBE_EXIT:-0}" > "$1"
    chmod 700 "$1"
    exit 0
  fi
  shift
done
exit 3
''')
            cc.chmod(0o700)
            for fail, probe, expected in [('0', '0', 0), ('1', '0', 2), ('0', '7', 7)]:
                env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ['PATH'],
                           TMPDIR=str(temporary), FAIL_COMPILE=fail, PROBE_EXIT=probe)
                result = subprocess.run(['bash', str(SCRIPT)], env=env, capture_output=True)
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertEqual(list(temporary.iterdir()), [sentinel])
                self.assertEqual(sentinel.read_text(), 'unrelated research')


if __name__ == '__main__':
    unittest.main()
