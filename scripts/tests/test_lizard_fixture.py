import importlib.util
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('lizard_research_check', REPO / 'scripts/check-protocol-research.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class DesktopFixtureShapes(unittest.TestCase):
    def test_complete_mouse_and_keyboard_records_remain_independent(self):
        MODULE.validate_desktop_fixture(b'400100000000\n410000280000000000\n')
        for packet in [b'40\n', b'4100002800000000\n', b'420000000000\n', b'400000000000410000280000000000\n']:
            with self.subTest(packet=packet), self.assertRaisesRegex(ValueError, 'shape'):
                MODULE.validate_desktop_fixture(packet)

    def test_reserved_and_key_fields_cannot_smuggle_unreviewed_data(self):
        for packet in [b'400400000000\n', b'410001280000000000\n', b'410000ff0000000000\n']:
            with self.subTest(packet=packet), self.assertRaises(ValueError):
                MODULE.validate_desktop_fixture(packet)
