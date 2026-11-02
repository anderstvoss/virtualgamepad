"""Untrusted HID_PHYS validation for the opt-in root udev helper."""

import ast
import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

HELPER = Path(__file__).resolve().parents[2] / 'udev/virtualgamepad-input-lab-seat.py'
SPEC = importlib.util.spec_from_file_location('input_lab_seat', HELPER)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
PREFIX = 'virtualgamepad/input-lab/uhid/'
VALID = {
    PREFIX + 'dualsense/p2a-i0': 'seat-vg-alpha-p2a',
    PREFIX + 'dualshock4/p3f-i10': 'seat-vg-alpha-p3f',
    PREFIX + 'dualsense/p7fffffff-i0': 'seat-vg-alpha-p7fffffff',
    PREFIX + 'dualshock4/p1-iffffffffffffffff': 'seat-vg-alpha-p1',
}
INVALID = [
    '', 'seat0', 'virtualgamepad/uhid/dualsense/p2a-i0',
    PREFIX + 'switch-pro/p2a-i0', PREFIX + 'dualsense-edge/p2a-i0',
    PREFIX + 'dualshock4/../dualsense/p2a-i0',
    PREFIX + 'dualsense/p0-i0', PREFIX + 'dualsense/p00-i0',
    PREFIX + 'dualsense/p02a-i0', PREFIX + 'dualsense/p-1-i0',
    PREFIX + 'dualsense/p+42-i0', PREFIX + 'dualsense/p0x2a-i0',
    PREFIX + 'dualsense/p2A-i0', PREFIX + 'dualsense/p80000000-i0',
    PREFIX + 'dualsense/pffffffff-i0', PREFIX + 'dualsense/p100000000-i0',
    PREFIX + 'dualsense/p2a-i00', PREFIX + 'dualsense/p2a-i',
    PREFIX + 'dualsense/p2a-i10000000000000000',
    PREFIX + 'dualsense/p2a-i0/child',
    PREFIX + 'dualsense/p2a-i0\n', PREFIX + 'dualsense/p2a-i0\r\n',
    PREFIX + 'dualsense/p2a-i0\x00', PREFIX + 'dualsense/p２a-i0',
    PREFIX + 'dualsense/p2a-i0;printf injected',
    PREFIX + 'dualsense/p2a-i0$(printf injected)',
    PREFIX + 'dualsense/p2a-i0`printf injected`',
    PREFIX + 'dualsense/p2a-i0 --option',
    PREFIX + 'dualsense/p2a-i0"', PREFIX + "dualsense/p2a-i0'",
    ' ' + PREFIX + 'dualsense/p2a-i0',
    PREFIX + 'dualsense/p2a-i0 ',
    PREFIX + 'dualsense/p2a-i0' + 'a' * 4096,
]


class InputLabSeatTests(unittest.TestCase):
    def test_accepts_only_canonical_sony_numeric_identities(self):
        for label, seat in VALID.items():
            with self.subTest(label=label):
                self.assertEqual(MODULE.seat_for_phys(label), seat)
                with patch.object(sys, 'argv', ['helper', label]), \
                        patch('sys.stdout', new_callable=io.StringIO) as output:
                    self.assertEqual(MODULE.main(), 0)
                    self.assertEqual(output.getvalue(), seat + '\n')

    def test_rejects_malformed_and_injection_inputs_without_output(self):
        for label in INVALID:
            with self.subTest(label=label):
                self.assertIsNone(MODULE.seat_for_phys(label))
                with patch.object(sys, 'argv', ['helper', label]), \
                        patch('sys.stdout', new_callable=io.StringIO) as output, \
                        patch('sys.stderr', new_callable=io.StringIO) as errors:
                    self.assertNotEqual(MODULE.main(), 0)
                    self.assertEqual((output.getvalue(), errors.getvalue()), ('', ''))

    def test_phys_length_bound_including_exact_boundary(self):
        label = PREFIX + 'dualsense/p2a-i' + 'f' * 16
        self.assertLessEqual(len(label), 63)
        # For this fixed family and instance, the PID fills the remaining space.
        prefix = PREFIX + 'dualsense/p'
        digits = 63 - len(prefix) - len('-i') - 16
        boundary = prefix + '1' * digits + '-i' + 'f' * 16
        self.assertEqual(len(boundary), 63)
        self.assertIsNotNone(MODULE.seat_for_phys(boundary))
        self.assertIsNone(MODULE.seat_for_phys(prefix + '1' * (digits + 1) + '-i' + 'f' * 16))

    def test_wrong_argument_count_is_silent_failure(self):
        for argv in [['helper'], ['helper', next(iter(VALID)), 'extra']]:
            with self.subTest(argv=argv), patch.object(sys, 'argv', argv), \
                    patch('sys.stdout', new_callable=io.StringIO) as output, \
                    patch('sys.stderr', new_callable=io.StringIO) as errors:
                self.assertNotEqual(MODULE.main(), 0)
                self.assertEqual((output.getvalue(), errors.getvalue()), ('', ''))

    def test_cli_contract_no_shell_and_silent_rejection(self):
        for args, status, expected in [
            ([next(iter(VALID))], 0, next(iter(VALID.values())) + '\n'),
            ([], 1, ''),
            ([PREFIX + 'dualsense/p2a-i0;printf injected'], 1, ''),
            ([PREFIX + 'dualsense/p2a-i0\n'], 1, ''),
            (['x' * 4096], 1, ''),
        ]:
            with self.subTest(args=args):
                result = subprocess.run([sys.executable, '-I', str(HELPER), *args],
                                        capture_output=True, text=True, check=False, timeout=5)
                self.assertEqual((result.returncode, result.stdout, result.stderr),
                                 (status, expected, ''))

    def test_helper_has_only_validation_imports_and_no_external_io(self):
        source = HELPER.read_text()
        self.assertTrue(source.startswith('#!/usr/bin/python3 -I\n'))
        tree = ast.parse(source)
        imports = {node.module if isinstance(node, ast.ImportFrom) else alias.name
                   for node in ast.walk(tree) if isinstance(node, (ast.Import, ast.ImportFrom))
                   for alias in node.names}
        self.assertEqual(imports, {'__future__', 're', 'sys'})
        with patch('builtins.open', side_effect=AssertionError('unexpected file access')), \
                patch('socket.socket', side_effect=AssertionError('unexpected network access')), \
                patch('subprocess.Popen', side_effect=AssertionError('unexpected command')), \
                patch('os.system', side_effect=AssertionError('unexpected shell')):
            for label in [*VALID, *INVALID]:
                MODULE.seat_for_phys(label)

    def test_all_child_rules_require_helper_validated_parent_property(self):
        rules = HELPER.with_name('71-virtualgamepad-input-lab.rules').read_text()
        children = [line for line in rules.splitlines()
                    if line.startswith(('SUBSYSTEM=="input"', 'SUBSYSTEM=="hidraw"'))]
        self.assertEqual(len(children), 3)
        for line in children:
            self.assertIn('IMPORT{parent}="VG_INPUT_LAB_SEAT"', line)
            self.assertIn('ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*"', line)


if __name__ == '__main__':
    unittest.main()
