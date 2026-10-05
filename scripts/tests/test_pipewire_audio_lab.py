import importlib.util
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('pw_lab', Path(__file__).parents[1]/'run-pipewire-audio-lab.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class IsolatedLab(unittest.TestCase):
    def test_graph_diagnostic_requires_bounded_object_array(self):
        self.assertEqual(lab.decode_graph(b'[{"id": 1}]'), [{'id': 1}])
        for value in (b'{}', b'x' * (lab.DIAGNOSTIC_LIMIT + 1), b'invalid'):
            with self.assertRaises(ValueError):
                lab.decode_graph(value)

    def test_snapshot_preserves_absolute_test_deadline(self):
        test = Mock(args=['synthetic'], pid=123)
        test.wait.side_effect = [subprocess.TimeoutExpired('synthetic', 3), 101]
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / 'receipt.json'
            with patch.object(lab, 'snapshot', return_value={'graph': []}), \
                    patch.object(lab.time, 'monotonic', side_effect=[10, 15]):
                self.assertEqual(lab.wait_with_snapshot(test, [test], {}, 20, report), 101)
            self.assertEqual(test.wait.call_args_list[1].kwargs['timeout'], 15)
            self.assertTrue(report.exists())

    def test_snapshot_failure_and_occupied_receipt_do_not_claim_acceptance(self):
        test = Mock(args=['synthetic'])
        test.wait.side_effect = subprocess.TimeoutExpired('synthetic', 3)
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / 'receipt.json'
            report.write_text('original')
            with patch.object(lab, 'snapshot', return_value={'graph': []}), \
                    self.assertRaises(FileExistsError):
                lab.wait_with_snapshot(test, [test], {}, 20, report)
            self.assertEqual(report.read_text(), 'original')
            with patch.object(lab, 'snapshot', side_effect=ValueError('quota')), \
                    self.assertRaisesRegex(ValueError, 'quota'):
                lab.wait_with_snapshot(test, [test], {}, 20, report)

    def test_fast_failure_is_distinguished_from_running_graph_snapshot(self):
        test = Mock()
        test.wait.return_value = 101
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / 'receipt.json'
            with patch.object(lab, 'snapshot', return_value={'graph': []}):
                self.assertEqual(lab.wait_with_snapshot(test, [test], {}, 20, report), 101)
            self.assertIn('test_exit_before_snapshot', report.read_text())

    def test_child_environment_does_not_connect_to_desktop_or_load_user_state(self):
        source = dict(PIPEWIRE_REMOTE='desktop', PIPEWIRE_RUNTIME_DIR='/old',
                      PIPEWIRE_QUANTUM='1024/48000', WIREPLUMBER_CONFIG_DIR='/user',
                      PATH='/bin')
        env = lab.environment(Path('/synthetic/private'), source)
        self.assertEqual(env['PIPEWIRE_RUNTIME_DIR'], '/synthetic/private')
        self.assertEqual(env['XDG_STATE_HOME'], '/synthetic/private/state')
        self.assertEqual(env['PIPEWIRE_REMOTE'], 'pipewire-0')
        self.assertNotIn('WIREPLUMBER_CONFIG_DIR', env)
        self.assertNotIn('PIPEWIRE_QUANTUM', env)
        self.assertEqual(source['PIPEWIRE_REMOTE'], 'desktop')

    def test_invalid_quantum_opens_no_resources(self):
        with self.assertRaises(ValueError):
            lab.run(["synthetic-test"], 1, 1024)

    def test_refuses_privileged_audio_worker(self):
        with patch.object(os, 'geteuid', return_value=0), self.assertRaises(ValueError):
            lab.run(['synthetic-test'], 1)

    def test_timeout_cleanup_targets_only_owned_process_group(self):
        child = Mock(pid=123)
        child.poll.return_value = None
        child.wait.side_effect = [subprocess.TimeoutExpired('synthetic',3), 0]
        with patch.object(os, 'killpg') as kill:
            lab.stop(child)
        self.assertEqual(kill.call_args_list[0].args, (123, signal.SIGTERM))
        self.assertEqual(kill.call_args_list[1].args, (123, signal.SIGKILL))
        self.assertEqual(child.wait.call_count, 2)

    def test_already_reaped_child_is_never_signalled(self):
        child = Mock(pid=123)
        child.poll.return_value = 0
        with patch.object(os, 'killpg') as kill:
            lab.stop(child)
        kill.assert_not_called()


if __name__ == '__main__':
    unittest.main()
