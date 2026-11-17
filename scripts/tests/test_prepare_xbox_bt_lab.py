import importlib.util
from pathlib import Path
import unittest


SPEC = importlib.util.spec_from_file_location(
    'prepare_xbox_bt_lab', Path(__file__).resolve().parents[1] / 'prepare-xbox-bt-lab.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class XboxBtLabRules(unittest.TestCase):
    def test_guard_uses_exact_virtual_identity_and_validated_label(self):
        vendor = 'ENV{ID_XPADNEO_SUPPORTED}="1"\n'
        wrapped = MODULE.wrap(vendor)
        self.assertIn('ENV{HID_ID}=="0005:0000045E:00000B13"', wrapped)
        self.assertIn('virtualgamepad/input-lab/uhid/xbox-series/p*-i*', wrapped)
        self.assertIn('PROGRAM=="/usr/lib/udev/virtualgamepad-input-lab-seat.py', wrapped)
        self.assertLess(wrapped.index('GOTO='), wrapped.index(vendor))
        self.assertEqual(wrapped.split(MODULE.GUARD, 1)[1],
                         vendor + f'LABEL="{MODULE.LABEL}"\n')

    def test_rejects_unexpected_vendor_and_double_wrapping(self):
        for vendor in ('', 'unrelated driver', MODULE.wrap('ID_XPADNEO_SUPPORTED\n')):
            with self.subTest(vendor=vendor), self.assertRaises(ValueError):
                MODULE.wrap(vendor)
