import importlib.util
from pathlib import Path
import unittest
import shlex
import shutil
import subprocess
import tempfile
import os
from unittest.mock import Mock, patch

spec = importlib.util.spec_from_file_location('control', Path(__file__).parents[1] / 'run-alpha-audio-control.py')
control = importlib.util.module_from_spec(spec)
spec.loader.exec_module(control)


class MarkerReceipt(unittest.TestCase):
    def test_cli_helper_import_leaves_fresh_checkout_clean_without_local_excludes(self):
        for script in ('run-alpha-acceptance.py', 'run-alpha-audio-control.py'):
            with self.subTest(script=script), tempfile.TemporaryDirectory() as directory:
                root = Path(directory); scripts = root / 'scripts'; scripts.mkdir()
                shutil.copyfile(Path(__file__).parents[1] / script, scripts / script)
                (scripts / 'run-pipewire-audio-lab.py').write_text('# Sanitized helper fixture.\n')
                subprocess.run(['git', 'init', '-q', str(root)], check=True)
                subprocess.run(['git', '-C', str(root), 'add', '.'], check=True)
                subprocess.run(['git', '-C', str(root), '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-qm', 'fixture'], check=True)
                environment = dict(os.environ)
                environment.pop('PYTHONDONTWRITEBYTECODE', None)
                environment.pop('PYTHONPYCACHEPREFIX', None)
                subprocess.run([os.sys.executable, str(scripts / script), '--help'], env=environment, check=True, stdout=subprocess.DEVNULL, timeout=5)
                self.assertEqual(subprocess.check_output(['git', '-C', str(root), 'status', '--porcelain'], text=True), '')
                self.assertFalse((scripts / '__pycache__').exists())

    def test_symlink_to_an_existing_runtime_is_rejected_without_spawning(self):
        with tempfile.TemporaryDirectory() as directory:
            alias = Path(directory) / 'virtualgamepad-pw-lab-fake'
            alias.symlink_to(Path(directory), target_is_directory=True)
            with patch.dict(os.environ, PIPEWIRE_RUNTIME_DIR=str(alias), XDG_RUNTIME_DIR=str(alias), PIPEWIRE_REMOTE='pipewire-0'), \
                 patch.object(control.subprocess, 'Popen') as spawn:
                with self.assertRaises(RuntimeError): control.inside(Path('/synthetic/control'), 2)
                spawn.assert_not_called()

    def test_timed_out_control_terminates_and_reaps_owned_loopback(self):
        with tempfile.TemporaryDirectory(prefix='virtualgamepad-pw-lab-') as directory:
            loop = Mock()
            loop.poll.return_value = None
            name = 'alpha-independent-' + str(os.getpid())
            nodes = '[{"info":{"props":{"node.name":"' + name + '.sink"}}},{"info":{"props":{"node.name":"' + name + '.source"}}}]'
            with patch.dict(os.environ, PIPEWIRE_RUNTIME_DIR=directory, XDG_RUNTIME_DIR=directory, PIPEWIRE_REMOTE='pipewire-0'), \
                 patch.object(control.subprocess, 'Popen', return_value=loop), \
                 patch.object(control.subprocess, 'check_output', return_value=nodes), \
                 patch.object(control.subprocess, 'run', side_effect=subprocess.TimeoutExpired('fake-control', 20)):
                with self.assertRaises(subprocess.TimeoutExpired): control.inside(Path('/synthetic/control'), 2)
            loop.terminate.assert_called_once()
            loop.wait.assert_called_once_with(timeout=2)

    def receipt(self):
        row = dict(planned=2880000, generated=2880000, graph_submitted=2880000,
                    graph_received=2880000, missing=0, duplicate=0, invalid=0,
                    partial_bytes=0, errors=0, ledger_overflow=0)
        row.update(source_rate=48000, sink_rate=48000, source_channels=2, sink_channels=2)
        row['producer_elapsed_ns'] = 60_000_000_000
        return row

    def test_complete_graph_accounting(self):
        self.assertTrue(control.acceptance(self.receipt(), 60))

    def test_full_totals_from_fast_or_slow_graph_do_not_prove_measured_duration(self):
        for duration in (1_000_000_000, 58_000_000_000, 62_000_000_000, None):
            row = self.receipt()
            row['producer_elapsed_ns'] = duration
            self.assertFalse(control.acceptance(row, 60))

    @unittest.skipUnless(shutil.which('cc') and shutil.which('pkg-config'), 'C/PipeWire developer tools unavailable')
    def test_actual_c_marker_and_callback_phase_logic_excludes_startup_and_detects_errors(self):
        flags = subprocess.run(['pkg-config', '--cflags', '--libs', 'libpipewire-0.3'], capture_output=True, text=True)
        if flags.returncode:
            self.skipTest('PipeWire headers unavailable')
        with tempfile.TemporaryDirectory() as directory:
            binary = str(Path(directory) / 'control')
            subprocess.run(['cc', '-Wall', '-Wextra', '-Werror', str(Path(__file__).parents[1] / 'alpha-audio-control.c'),
                            '-o', binary, *shlex.split(flags.stdout)], check=True, timeout=30)
            subprocess.run([binary, '--self-test'], check=True, timeout=5)

    def test_callback_gap_diagnostics_never_excuse_marker_loss(self):
        row = self.receipt()
        row.update(source_max_process_gap_ns=1000000000, sink_max_process_gap_ns=1000000000)
        row.update(source_measured_clock=dict(first_ticks=0, last_ticks=2880000, rate_num=1, rate_denom=48000, changes=0),
                   sink_measured_clock=dict(first_ticks=0, last_ticks=60_000_000_000, rate_num=1, rate_denom=1_000_000_000, changes=0))
        row['missing'] = 128
        self.assertFalse(control.acceptance(row, 60))

    def test_stdin_or_generation_does_not_prove_graph_submission(self):
        row = self.receipt()
        row['graph_submitted'] -= 128
        self.assertFalse(control.acceptance(row, 60))

    def test_equal_totals_do_not_hide_loss_and_duplication(self):
        row = self.receipt()
        row.update(missing=128, duplicate=128)
        self.assertFalse(control.acceptance(row, 60))

    def test_partial_channel_corruption_incomplete_drain_and_missing_counters(self):
        for key in ('partial_bytes', 'invalid', 'errors', 'ledger_overflow'):
            row = self.receipt()
            row[key] = 1
            self.assertFalse(control.acceptance(row, 60))
        for key in ('planned', 'generated', 'graph_submitted', 'graph_received'):
            row = self.receipt()
            row[key] -= 1
            self.assertFalse(control.acceptance(row, 60))
        row = self.receipt()
        del row['missing']
        self.assertFalse(control.acceptance(row, 60))


if __name__ == '__main__':
    unittest.main()
