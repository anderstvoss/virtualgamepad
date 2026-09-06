import importlib.util
import os
import stat
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('preflight', Path(__file__).resolve().parents[1] / 'host-preflight.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class Preflight(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def write(self, name, data):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(data)

    def test_denied_device_is_not_opened_and_identity_is_checked(self):
        self.write('dev/uhid', 'synthetic')
        self.write('sys/class/misc/uhid/dev', '10:239')
        original = Path.stat
        class Device:
            st_mode = stat.S_IFCHR | 0o600
            st_rdev = os.makedev(10, 239)
        with patch.object(Path, 'stat', lambda p, **kw: Device() if p == self.root / 'dev/uhid' else original(p, **kw)):
            result = MODULE.inspect(MODULE.TARGETS[0], self.root, lambda *_: False)
        self.assertEqual(result['checks'][0]['status'], 'denied')
        self.assertEqual((self.root / 'dev/uhid').read_text(), 'synthetic')
        self.assertEqual(MODULE.inspect(MODULE.TARGETS[0], self.root)['checks'][0]['status'], 'mismatch')

    def test_missing_mechanism(self):
        self.assertEqual(MODULE.inspect(MODULE.TARGETS[1], self.root)['checks'][0]['status'], 'missing')

    def test_denied_binding_does_not_report_available_udc(self):
        self.write('etc/virtualgamepad/broker.conf', 'allow_udc=dummy_udc.0\n')
        self.write('sys/kernel/config/usb_gadget/research/UDC', 'dummy_udc.0')
        (self.root / 'sys/class/udc/dummy_udc.0').mkdir(parents=True)
        original = Path.read_text
        def read(path, **kwargs):
            if path.name == 'UDC':
                raise PermissionError('synthetic denied binding')
            return original(path, **kwargs)
        with patch.object(Path, 'read_text', read):
            checks = MODULE.inspect(MODULE.TARGETS[2], self.root)['checks']
        self.assertIn('denied', [r['status'] for r in checks])
        self.assertNotIn('available', [r['status'] for r in checks])

    def test_occupied_and_unvalidated_resources_are_read_only(self):
        self.write('etc/virtualgamepad/broker.conf', 'allow_udc=dummy_udc.0\nallow_udc=dummy_udc.1\n')
        self.write('sys/kernel/config/usb_gadget/unrelated/UDC', 'dummy_udc.0')
        (self.root / 'sys/class/udc/dummy_udc.0').mkdir(parents=True)
        before = {str(p): p.read_bytes() for p in self.root.rglob('*') if p.is_file()}
        checks = MODULE.inspect(MODULE.TARGETS[2], self.root)['checks']
        self.assertIn('occupied', [r['status'] for r in checks])
        self.assertIn('missing', [r['status'] for r in checks])
        self.assertIn('unvalidated', [r['status'] for r in checks])
        self.assertEqual(before, {str(p): p.read_bytes() for p in self.root.rglob('*') if p.is_file()})


if __name__ == '__main__':
    unittest.main()
