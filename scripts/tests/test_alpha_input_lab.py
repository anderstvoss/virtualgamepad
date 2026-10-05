import importlib.util
from pathlib import Path
import tempfile
import unittest
import contextlib
import hashlib
import io
import os
import subprocess
import sys
from types import SimpleNamespace
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('input_lab', Path(__file__).parents[1] / 'run-alpha-input-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class InputIsolation(unittest.TestCase):
    def test_rules_select_one_pid_and_never_change_unrelated_device_permissions(self):
        text = lab.rule(42)
        self.assertIn('ATTRS{phys}=="virtualgamepad/*/p2a-i*"', text)
        self.assertIn('ENV{ID_SEAT}="seat-vg-alpha-p2a"', text)
        self.assertIn('ENV{LIBINPUT_IGNORE_DEVICE}="1"', text)
        for forbidden in ('MODE=', 'OWNER=', 'GROUP=', 'RUN=', 'TAG='):
            self.assertNotIn(forbidden, text)
        for invalid in (0, 1, -1, True, '42', '42"*'):
            with self.assertRaises(ValueError):lab.rule(invalid)

    def test_inventory_excludes_foreign_pid_and_similar_numeric_prefix(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for number, physical in enumerate(('virtualgamepad/uinput/p2a-i0',
                    'virtualgamepad/uhid/dualshock4/p2a-i1/input1',
                    'virtualgamepad/uinput/p2aa-i0', 'foreign/p2a-i0')):
                device = root / ('event' + str(number)) / 'device'
                device.mkdir(parents=True)
                (device / 'phys').write_text(physical)
            self.assertEqual(lab.owned_inputs(42, root), ['event0', 'event1'])

    def test_compound_labels_keep_exact_process_and_full_creation_identity(self):
        token = '0123456789abcdef0123456789abcdef'
        self.assertTrue(lab.owned_physical(f'virtualgamepad/p2a/{token}/c0000', 42))
        self.assertTrue(lab.owned_physical(f'virtualgamepad/p2a/{token}/c0001', 42))
        for foreign in (f'virtualgamepad/p2aa/{token}/c0001',
                        f'virtualgamepad/p2a/{token[:16]}/c0001',
                        f'virtualgamepad/{token}/c0001'):
            self.assertFalse(lab.owned_physical(foreign, 42))
        self.assertIn('ATTRS{phys}=="virtualgamepad/p2a/*/c*"', lab.rule(42))

    def test_empty_sony_phys_uses_only_matching_hid_ancestor(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inventory = root / 'inventory';inventory.mkdir()
            hid_subsystem = root / 'hid';hid_subsystem.mkdir()
            for number, physical in enumerate(('virtualgamepad/uhid/dualsense/p2a-i0',
                    'virtualgamepad/uhid/dualsense/p2aa-i0', 'foreign/p2a-i0')):
                hid = root / ('controller' + str(number));hid.mkdir()
                (hid / 'subsystem').symlink_to(hid_subsystem)
                (hid / 'uevent').write_text('HID_ID=fake\nHID_PHYS=' + physical + '\n')
                device = hid / 'input' / 'input0';device.mkdir(parents=True)
                (device / 'phys').write_text('')
                event = device / ('event' + str(number));event.mkdir()
                (event / 'device').symlink_to(device)
                (inventory / event.name).symlink_to(event)
            self.assertEqual(lab.owned_inputs(42, inventory), ['event0'])
            text = lab.rule(42)
            self.assertIn('SUBSYSTEMS=="hid", ATTRS{uevent}=="*HID_PHYS=virtualgamepad/*/p2a-i*"', text)

    def test_restoration_requires_device_removal_and_keeps_changed_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'owned.rules';path.write_text('fake rule')
            expected = lab.identity(path)
            with self.assertRaisesRegex(RuntimeError, 'devices remain'):
                lab.restore_rule(path, expected, 42, lambda pid: ['event3'])
            self.assertTrue(path.exists())
            with patch.object(lab, 'identity', return_value=(-1, -1)):
                with self.assertRaisesRegex(RuntimeError, 'identity changed'):
                    lab.restore_rule(path, expected, 42, lambda pid: [])
            self.assertTrue(path.exists())
            lab.restore_rule(path, expected, 42, lambda pid: [])
            lab.restore_rule(path, expected, 42, lambda pid: [])
            self.assertFalse(path.exists())

    def test_dead_group_is_not_a_reason_to_skip_rule_restoration(self):
        with patch.object(lab.os, 'killpg', side_effect=ProcessLookupError):
            lab.terminate_group(42)
        with patch.object(lab.os, 'killpg', side_effect=PermissionError):
            with self.assertRaises(PermissionError):lab.terminate_group(42)

    def test_interrupt_raises_into_registered_restoration(self):
        with self.assertRaises(InterruptedError):lab.interrupted(15, None)

    def test_output_quota_is_inherited_before_test_exec(self):
        with patch.object(lab.resource, 'setrlimit') as limit:
            lab.output_limit()
        limit.assert_called_once_with(lab.resource.RLIMIT_FSIZE, (lab.LIMIT, lab.LIMIT))

    def exercise_run(self, failure=None, timeout=False):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory);image = root / 'test';image.write_bytes(b'fake executable')
            args = SimpleNamespace(command=[str(image)], command_hash=hashlib.sha256(image.read_bytes()).hexdigest(),
                                   uid=1001, input_gid=1003,
                                   env=[], timeout=1)
            process = Mock(pid=42);process.wait.return_value = 0
            with patch.object(lab, 'RULES', root), patch.object(lab, 'trusted'), \
                 patch.object(lab.os, 'geteuid', return_value=0), \
                 patch.object(lab.sys, 'flags', SimpleNamespace(isolated=1)), \
                 patch.object(lab.pwd, 'getpwuid', return_value=SimpleNamespace(pw_uid=1001, pw_gid=1002)), \
                 patch.object(lab, 'creation_group', return_value=args.input_gid), \
                 patch.object(lab.subprocess, 'Popen', return_value=process) as spawn, \
                 patch.object(lab.subprocess, 'run', side_effect=failure) as commands, \
                 patch.object(lab.os, 'waitid', return_value=None if timeout else Mock()), \
                 patch.object(lab.time, 'monotonic', side_effect=[0, 2, 2] if timeout else None, return_value=0), \
                 patch.object(lab.os, 'write') as release, patch.object(lab, 'owned_inputs', return_value=[]), \
                 patch.object(lab, 'terminate_group') as terminate, contextlib.redirect_stdout(io.StringIO()):
                result = lab.run(args)
            self.assertFalse(list(root.glob('*.rules')))
            terminate.assert_called_once_with(42)
            process.wait.assert_called_once_with(timeout=10)
            argv = spawn.call_args.args[0]
            self.assertIn('--reuid=1001', argv)
            self.assertIn('--bounding-set=-all', argv)
            self.assertIn('--no-new-privs', argv)
            self.assertEqual(spawn.call_args.kwargs['env'], {'PATH': '/usr/bin:/bin', 'LANG': 'C'})
            return result, release, commands

    def test_failed_preparation_never_releases_child_and_restores_rule(self):
        result, release, _ = self.exercise_run(RuntimeError('synthetic verification failure'))
        self.assertEqual(result, 1);release.assert_not_called()

    def test_success_retains_leader_until_rule_restoration(self):
        result, release, commands = self.exercise_run()
        self.assertEqual(result, 0);release.assert_called_once()
        self.assertEqual(commands.call_args_list[-1].args[0], ['udevadm', 'control', '--reload'])

    def test_timeout_terminates_owned_group_and_restores_rule(self):
        result, release, _ = self.exercise_run(timeout=True)
        self.assertEqual(result, 1);release.assert_called_once()

    @unittest.skipUnless(hasattr(os, 'geteuid') and os.geteuid() != 0, 'non-root Linux child seam')
    def test_real_child_waits_for_gate_then_execs_with_the_same_pid(self):
        read_fd, write_fd = os.pipe()
        process = subprocess.Popen([sys.executable, '-I', str(Path(lab.__file__).absolute()),
            '--child', str(read_fd), '{}', sys.executable, '-I', '-c',
            'import os;print(os.getpid(),os.environ["VIRTUALGAMEPAD_INPUT_LAB_SEAT"])'],
            pass_fds=(read_fd,), stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        os.close(read_fd)
        try:
            self.assertIsNone(process.poll())
            os.write(write_fd, b'G');os.close(write_fd);write_fd = -1
            output, error = process.communicate(timeout=5)
            self.assertEqual(process.returncode, 0, error.decode())
            self.assertEqual(output.decode().strip(), f'{process.pid} seat-vg-alpha-p{process.pid:x}')
        finally:
            if write_fd >= 0:os.close(write_fd)
            if process.poll() is None:process.kill();process.wait()

    @unittest.skipUnless(hasattr(os, 'geteuid') and os.geteuid() != 0, 'non-root Linux child seam')
    def test_gate_eof_rejects_command_without_execution(self):
        read_fd, write_fd = os.pipe()
        process = subprocess.Popen([sys.executable, '-I', str(Path(lab.__file__).absolute()),
            '--child', str(read_fd), '{}', sys.executable, '-I', '-c', 'print("must not execute")'],
            pass_fds=(read_fd,), stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        os.close(read_fd);os.close(write_fd)
        output, _ = process.communicate(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        self.assertEqual(output, b'')
