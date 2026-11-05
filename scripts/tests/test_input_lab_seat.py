"""Untrusted HID_PHYS validation for the opt-in root udev helper."""

import ast
import fnmatch
import re
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
LAB_ROOT = 'virtualgamepad/input-lab/'
PREFIX = LAB_ROOT + 'uhid/'
UINPUT_PREFIX = LAB_ROOT + 'uinput/'
VALID = {
    PREFIX + "steam-controller-2026/p2a-i0": "seat-vg-alpha-p2a",
    UINPUT_PREFIX + "steam-controller-2026/p2a-i0": "seat-vg-alpha-p2a",
    PREFIX + 'dualsense/p2a-i0': 'seat-vg-alpha-p2a',
    PREFIX + 'dualshock4/p3f-i10': 'seat-vg-alpha-p3f',
    PREFIX + 'dualsense/p7fffffff-i0': 'seat-vg-alpha-p7fffffff',
    PREFIX + 'dualshock4/p1-iffffffffffffffff': 'seat-vg-alpha-p1',
    UINPUT_PREFIX + 'dualsense/p2a-i0': 'seat-vg-alpha-p2a',
    UINPUT_PREFIX + 'dualshock4/p3f-i10': 'seat-vg-alpha-p3f',
    UINPUT_PREFIX + 'dualsense/p7fffffff-i0': 'seat-vg-alpha-p7fffffff',
    UINPUT_PREFIX + 'dualshock4/p1-iffffffffffffffff': 'seat-vg-alpha-p1',
}
INVALID = [
    '', 'seat0', 'virtualgamepad/uhid/dualsense/p2a-i0',
    *[LAB_ROOT + transport + '/dualsense/p2a-i0'
      for transport in ['usb', 'evdev', 'UHID', 'UINPUT', 'uInput', 'uhid/uinput']],
    UINPUT_PREFIX + 'DualSense/p2a-i0', UINPUT_PREFIX + 'dualsense/p2a-i0/extra',
    UINPUT_PREFIX + 'switch-pro/p2a-i0', UINPUT_PREFIX + 'dualsense/p2a-i0\n',
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
    def test_steam_label_is_bounded_and_event_grant_requires_validated_parent(self):
        for transport in ['uhid', 'uinput']:
            base = LAB_ROOT + transport + '/steam-controller-2026/'
            self.assertEqual(MODULE.seat_for_phys(base + 'p2a-i0'), 'seat-vg-alpha-p2a')
            for suffix in ['p0-i0', 'p02a-i0', 'p2a-i0/extra', 'p2a-i0\n',
                           'p7fffffff-iffffffffffffffff']:
                self.assertIsNone(MODULE.seat_for_phys(base + suffix))
        rules = HELPER.with_name('71-virtualgamepad-input-lab.rules').read_text()
        grants = [line for line in rules.splitlines()
                  if 'VG_INPUT_LAB_STEAM' in line and 'GROUP=' in line]
        self.assertEqual(len(grants), 1)
        self.assertIn('ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*"', grants[0])
        self.assertIn('ENV{VG_INPUT_LAB_STEAM}=="1"', grants[0])
        self.assertIn('ENV{HID_PHYS}=="virtualgamepad/input-lab/uhid/steam-controller-2026/p*-i*"', rules)
        self.assertEqual(rules.count('IMPORT{parent}="VG_INPUT_LAB_STEAM"'), 2)
        self.assertIn('GROUP="virtualgamepad-uhid", MODE="0660"', grants[0])

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
        for transport in ['uhid', 'uinput']:
            prefix = LAB_ROOT + transport + '/dualsense/p'
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

    @staticmethod
    def child_policy(rules, subsystem, kernel, parent, attributes=None, ancestor_attributes=None):
        """Model the used rule subset: ENV matching precedes parent imports.

        The installed-rule failure reproduced with udevadm test on the host:
        an ENV guard on the IMPORT line cannot see the imported property.
        """
        attributes = attributes or {}
        ancestor_attributes = ancestor_attributes or {}
        properties = {}
        tags = set()
        for line in rules.splitlines():
            if not line.startswith(f'SUBSYSTEM=="{subsystem}"'):
                continue
            pattern = re.search(r'KERNEL=="([^\"]+)"', line)
            if pattern and not fnmatch.fnmatchcase(kernel, pattern[1]):
                continue
            attrs = re.findall(r'(ATTRS?)\{([^}]+)\}=="([^\"]+)"', line)
            if any(not fnmatch.fnmatchcase((ancestor_attributes if kind == 'ATTRS' else attributes).get(key, ''), pattern)
                   for kind, key, pattern in attrs):
                continue
            matches = re.findall(r'ENV\{([^}]+)\}=="([^\"]+)"', line)
            if any(not fnmatch.fnmatchcase(properties.get(key, ''), pattern)
                   for key, pattern in matches):
                continue
            program_result = None
            if 'PROGRAM==' in line:
                program_result = MODULE.seat_for_phys(attributes.get('phys', ''))
                if program_result is None:
                    continue
            imports = re.findall(r'IMPORT\{parent\}="([^\"]+)"', line)
            if any(key not in parent for key in imports):
                continue
            for key in imports:
                properties[key] = parent[key]
            for key, value in re.findall(r'ENV\{([^}]+)\}="([^\"]*)"', line):
                if value == '%c':
                    properties[key] = program_result
                    continue
                properties[key] = re.sub(r'%E\{([^}]+)\}',
                                         lambda match: properties.get(match[1], ''), value)
            for key, value in re.findall(r'(GROUP|MODE)="([^\"]+)"', line):
                properties['@' + key] = value
            tags.update(re.findall(r'TAG\+="([^\"]+)"', line))
        return properties, tags

    def test_validated_parent_reaches_each_child_before_guard(self):
        rules = HELPER.with_name('71-virtualgamepad-input-lab.rules').read_text()
        seat = 'seat-vg-alpha-p2a'
        for subsystem, kernel in [('input', 'input1'), ('input', 'event1'),
                                  ('hidraw', 'hidraw1')]:
            with self.subTest(subsystem=subsystem, kernel=kernel):
                properties, tags = self.child_policy(rules, subsystem, kernel,
                                                     {'VG_INPUT_LAB_SEAT': seat})
                if subsystem == 'hidraw':
                    self.assertIn('uaccess', tags)
                else:
                    self.assertEqual(properties.get('ID_SEAT'), seat)
                if kernel.startswith('event'):
                    self.assertEqual(properties.get('LIBINPUT_IGNORE_DEVICE'), '1')
                for parent in [{}, {'VG_INPUT_LAB_SEAT': ''},
                               {'VG_INPUT_LAB_SEAT': 'seat0'},
                               {'VG_INPUT_LAB_SEAT': 'seat-vg-alpha-p'}]:
                    properties, tags = self.child_policy(rules, subsystem, kernel, parent)
                    self.assertNotIn('ID_SEAT', properties)
                    self.assertNotIn('LIBINPUT_IGNORE_DEVICE', properties)
                    self.assertNotIn('uaccess', tags)

    def test_uinput_validates_sysfs_phys_and_propagates_to_event(self):
        rules = HELPER.with_name('71-virtualgamepad-input-lab.rules').read_text()
        self.assertIn('ATTR{phys}==', rules)
        self.assertIn('%s{phys}', rules)
        for label, seat in VALID.items():
            if '/uinput/' not in label:
                continue
            with self.subTest(label=label):
                parent, _ = self.child_policy(rules, 'input', 'input1', {}, {'phys': label})
                self.assertEqual(parent.get('ID_SEAT'), seat)
                event, _ = self.child_policy(rules, 'input', 'event1', parent, ancestor_attributes={'phys': label})
                self.assertEqual(event.get('ID_SEAT'), seat)
                self.assertEqual(event.get('LIBINPUT_IGNORE_DEVICE'), '1')
                self.assertEqual(event.get('@GROUP'), 'virtualgamepad-uinput')
                self.assertEqual(event.get('@MODE'), '0660')
        for label in [*INVALID, '"' + next(iter(VALID)) + '"']:
            parent, _ = self.child_policy(rules, 'input', 'input1', {}, {'phys': label})
            event, tags = self.child_policy(rules, 'input', 'event1', parent, ancestor_attributes={'phys': label})
            self.assertNotIn('@GROUP', event)
            self.assertNotIn('@MODE', event)
            self.assertNotIn('ID_SEAT', event)
            self.assertNotIn('LIBINPUT_IGNORE_DEVICE', event)
            self.assertNotIn('uaccess', tags)

    def test_uinput_event_access_requires_both_validated_marker_and_uinput_parent(self):
        rules = HELPER.with_name('71-virtualgamepad-input-lab.rules').read_text()
        seat = 'seat-vg-alpha-p2a'
        label = UINPUT_PREFIX + 'dualsense/p2a-i0'
        for parent, phys in [({}, label), ({'VG_INPUT_LAB_SEAT': 'seat0'}, label),
                             ({'VG_INPUT_LAB_SEAT': seat}, PREFIX + 'dualsense/p2a-i0')]:
            properties, _ = self.child_policy(rules, 'input', 'event1', parent,
                                              ancestor_attributes={'phys': phys})
            self.assertNotIn('@GROUP', properties)
            self.assertNotIn('@MODE', properties)

    def test_same_rule_import_guard_regression_drops_valid_parent(self):
        seat = 'seat-vg-alpha-p2a'
        broken = ('SUBSYSTEM=="input", KERNEL=="event*", '
                  'IMPORT{parent}="VG_INPUT_LAB_SEAT", '
                  'ENV{VG_INPUT_LAB_SEAT}=="seat-vg-alpha-p?*", '
                  'ENV{ID_SEAT}="%E{VG_INPUT_LAB_SEAT}", '
                  'ENV{LIBINPUT_IGNORE_DEVICE}="1"')
        properties, tags = self.child_policy(broken, 'input', 'event1',
                                             {'VG_INPUT_LAB_SEAT': seat})
        self.assertEqual((properties, tags), ({}, set()))


if __name__ == '__main__':
    unittest.main()
