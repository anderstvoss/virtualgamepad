import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('scheduled_audio', Path(__file__).parents[1] / 'run-alpha-scheduled-audio-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class ScheduledAudioOwnership(unittest.TestCase):
    def test_transient_limits_and_privilege_drop_are_explicit_and_bounded(self):
        args = SimpleNamespace(uid=1001, control=Path('/approved/control'), rust_test=Path('/approved/rust'))
        command = lab.unit_command(args, 'owned.service', 'owned description', 1002, Path('/owned/runtime'))
        for expected in ('--property=LimitRTPRIO=88', '--property=LimitRTTIME=200ms',
                         '--property=RuntimeMaxSec=540', '--property=KillMode=control-group',
                         '--clear-groups', '--bounding-set=-all', '--ambient-caps=-all', '--no-new-privs'):
            self.assertIn(expected, command)
        self.assertNotIn('--property=CapabilityBoundingSet=CAP_SYS_NICE', command)
        self.assertIn('--reuid=1001', command)
        self.assertIn('--regid=1002', command)

    def test_restoration_is_idempotent_when_owned_unit_is_collected(self):
        with patch.object(lab, 'properties', return_value={'LoadState': 'not-found'}), patch.object(lab.subprocess, 'run') as command:
            lab.restore('owned.service', 'expected')
            lab.restore('owned.service', 'expected')
            command.assert_not_called()

    def test_changed_identity_never_stops_foreign_unit(self):
        with patch.object(lab, 'properties', return_value={'LoadState': 'loaded', 'Description': 'foreign'}), patch.object(lab.subprocess, 'run') as command:
            with self.assertRaisesRegex(RuntimeError, 'identity changed'):
                lab.restore('owned.service', 'expected')
            command.assert_not_called()

    def test_owned_active_unit_is_stopped_and_checked(self):
        with patch.object(lab, 'properties', side_effect=[{'LoadState': 'loaded', 'Description': 'expected'}, {'LoadState': 'not-found'}]), patch.object(lab.subprocess, 'run') as command:
            lab.restore('owned.service', 'expected')
            command.assert_called_once_with(['systemctl', 'stop', 'owned.service'], check=True, timeout=15)

    def test_incomplete_restoration_is_a_failure(self):
        state = {'LoadState': 'loaded', 'Description': 'expected', 'ActiveState': 'active'}
        with patch.object(lab, 'properties', return_value=state), patch.object(lab.subprocess, 'run'):
            with self.assertRaisesRegex(RuntimeError, 'remains active'):lab.restore('owned.service', 'expected')

    def test_workspace_removal_never_follows_foreign_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            owned = root / 'owned';owned.mkdir()
            foreign = root / 'foreign';foreign.mkdir()
            (foreign / 'keep').write_text('sanitized foreign resource')
            (owned / 'alias').symlink_to(foreign, target_is_directory=True)
            expected = lab.workspace_identity(owned)
            with patch.object(lab, 'workspace_identity', return_value=(-1,-1)):
                with self.assertRaisesRegex(RuntimeError, 'identity changed'):
                    lab.remove_workspace(owned,expected)
            lab.remove_workspace(owned,expected)
            lab.remove_workspace(owned,expected)
            self.assertEqual((foreign/'keep').read_text(),'sanitized foreign resource')

    def test_hash_mismatch_refuses_image_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            image = Path(directory) / 'image';image.write_bytes(b'fake binary')
            with patch.object(lab, 'trusted'):
                with self.assertRaisesRegex(RuntimeError, 'approved hash'):
                    lab.checked_image(image, hashlib.sha256(b'other fake image').hexdigest())

    def exercise_apply(self, failure=None, cleanup=None, occupied=False):
        args = SimpleNamespace(uid=1001, control=Path('/approved/control'), control_hash='a'*64,
                               rust_test=Path('/approved/rust'), rust_hash='b'*64)
        output = io.StringIO()
        self.addCleanup(lambda: None)
        workspace = tempfile.TemporaryDirectory()
        self.addCleanup(workspace.cleanup)
        with patch.object(lab.tempfile, 'mkdtemp', return_value=workspace.name), patch.object(lab.os, 'chown'), patch.object(lab.os, 'geteuid', return_value=0), \
             patch.object(lab.sys, 'flags', SimpleNamespace(isolated=1)), \
             patch.object(lab, 'checked_image'), patch.object(lab, 'trusted'), \
             patch.object(lab.pwd, 'getpwuid', return_value=SimpleNamespace(pw_uid=1001,pw_gid=1002)), \
             patch.object(lab, 'properties', return_value={'LoadState':'loaded' if occupied else 'not-found'}), \
             patch.object(lab.subprocess, 'run', side_effect=failure, return_value=SimpleNamespace(returncode=0)) as run, \
             patch.object(lab, 'restore', side_effect=cleanup) as restore, contextlib.redirect_stdout(output):
            if occupied:
                with self.assertRaisesRegex(RuntimeError, 'already exists'):lab.apply(args)
                run.assert_not_called();restore.assert_not_called()
                return
            result = lab.apply(args)
            restore.assert_called_once()
            self.assertEqual(run.call_args.kwargs['timeout'], 570)
            self.assertEqual(run.call_args.kwargs['env'], {'PATH':'/usr/sbin:/usr/bin:/sbin:/bin','LANG':'C'})
            self.assertEqual(run.call_args.kwargs['preexec_fn'], lab.output_limit)
        return result, output.getvalue()

    def test_occupied_unit_is_rejected_without_start_or_cleanup(self):self.exercise_apply(occupied=True)
    def test_partial_startup_error_still_restores(self):
        result, output = self.exercise_apply(OSError('synthetic startup error'))
        self.assertEqual(result,1);self.assertIn('synthetic startup error',output)
    def test_timeout_preserves_initiating_and_cleanup_errors(self):
        result, output = self.exercise_apply(subprocess.TimeoutExpired('owned unit',570), RuntimeError('synthetic cleanup error'))
        self.assertEqual(result,1);self.assertIn('timed out',output);self.assertIn('synthetic cleanup error',output)
    def test_success_restores(self):self.assertEqual(self.exercise_apply()[0],0)
    def test_interrupt_flows_through_restoration(self):
        result, output = self.exercise_apply(InterruptedError('synthetic interruption'))
        self.assertEqual(result,1);self.assertIn('synthetic interruption',output)
    def test_child_cannot_run_with_root_or_unbounded_priority(self):
        with patch.object(lab.os,'geteuid',return_value=0):
            with self.assertRaisesRegex(RuntimeError,'must not run as root'):lab.child('/fake/control','/fake/rust')
        with patch.object(lab.os,'geteuid',return_value=1001), patch.object(lab.resource,'getrlimit',return_value=(0,0)):
            with self.assertRaisesRegex(RuntimeError,'priority grant'):lab.child('/fake/control','/fake/rust')
    def test_output_quota_failure_still_restores(self):
        def oversized(*args, **kwargs):
            kwargs['stdout'].write(b'x' * (lab.LIMIT + 1))
            return SimpleNamespace(returncode=0)
        result, output = self.exercise_apply(oversized)
        self.assertEqual(result,1);self.assertIn('output quota',output)

    def test_default_plan_cannot_start_units_or_apply_privileged_changes(self):
        arguments = ['lab', '--uid', '1001', '--control', '/fake/control', '--control-hash', 'a'*64,
                     '--rust-test', '/fake/rust', '--rust-hash', 'b'*64]
        with patch.object(lab.sys, 'argv', arguments), patch.object(lab, 'apply') as apply, contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(lab.main(),0)
            apply.assert_not_called()

    def test_signal_and_output_limit_boundaries(self):
        with self.assertRaises(InterruptedError):lab.interrupted(15,None)
        with patch.object(lab.resource,'setrlimit') as setlimit:lab.output_limit()
        setlimit.assert_called_once_with(lab.resource.RLIMIT_FSIZE,(lab.LIMIT,lab.LIMIT))


if __name__ == '__main__':unittest.main()
