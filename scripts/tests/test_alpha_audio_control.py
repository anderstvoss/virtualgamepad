import importlib.util
from pathlib import Path
import unittest
import json
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
    def test_outer_control_requests_owned_graph_thread_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); binary = root / 'control'; binary.touch()
            report = root / 'result.json'
            def run(command, timeout, **kwargs):
                self.assertEqual(kwargs, dict(quantum=256, diagnostics=root / 'result.json.trial-1.graph.json'))
                Path(command[3]).write_text(json.dumps(dict(accepted=False, missing=512)))
                return 1
            with patch.object(control.sys, 'argv', ['control', '--control', str(binary), '--report', str(report),
                         '--seconds', '3', '--trials', '1', '--topology', 'direct', '--quantum', '256']), \
                 patch.object(control.lab, 'run', side_effect=run), patch('builtins.print'):
                self.assertEqual(control.main(), 1)
            row = json.loads(report.read_text())['trials'][0]
            self.assertEqual(row['graph_diagnostics_file'], str(root / 'result.json.trial-1.graph.json'))
            self.assertEqual(row['missing'], 512)
            self.assertFalse(row['accepted'])

    def test_existing_control_report_is_preserved_before_any_graph_is_started(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); binary = root / 'control'; binary.touch()
            report = root / 'result.json'; report.write_text('historical synthetic failure')
            with patch.object(control.sys, 'argv', ['control', '--control', str(binary), '--report', str(report)]), \
                 patch.object(control.lab, 'run') as run, patch.object(control.sys, 'stderr'):
                with self.assertRaises(SystemExit) as error: control.main()
            self.assertEqual(error.exception.code, 2)
            run.assert_not_called()
            self.assertEqual(report.read_text(), 'historical synthetic failure')

    def direct_graph(self):
        graph = [dict(id=node, type='PipeWire:Interface:Node',
                      info=dict(props={'node.name': name}))
                 for node, name in [(10, 'alpha-independent-producer'),
                                    (20, 'alpha-independent-receiver')]]
        for node, role, first in [(10, 'out', 30), (20, 'in', 40)]:
            for offset, channel in enumerate(('FL', 'FR')):
                graph.append(dict(id=first+offset, type='PipeWire:Interface:Port',
                                  info=dict(props={'node.id': node, 'port.direction': role,
                                                   'audio.channel': channel})))
        return graph

    def test_direct_links_require_exact_owned_nodes_channels_and_directions(self):
        graph = self.direct_graph()
        self.assertEqual(control.direct_ports(graph), [(30, 40), (31, 41)])
        self.assertIsNone(control.direct_ports(graph[:-1]))
        wrong = self.direct_graph()
        wrong[-1]['info']['props']['node.id'] = 99
        self.assertIsNone(control.direct_ports(wrong))
        wrong[-1]['info']['props']['node.id'] = 20
        wrong[-1]['info']['props']['port.direction'] = 'out'
        self.assertIsNone(control.direct_ports(wrong))
        for duplicate in (graph[0], graph[-1]):
            with self.assertRaisesRegex(ValueError, 'ambiguous'):
                control.direct_ports(graph + [duplicate])

    def test_direct_port_readiness_timeout_terminates_and_reaps_control(self):
        child = Mock()
        child.poll.return_value = None
        with patch.object(control.subprocess, 'Popen', return_value=child), \
             patch.object(control.subprocess, 'run'), \
             patch.object(control.lab, 'decode_graph', return_value=[]), \
             patch.object(control.time, 'monotonic', side_effect=[0, 6]):
            with self.assertRaisesRegex(TimeoutError, 'ports'):
                control.run_direct(Path('/synthetic/control'), 2, None)
        child.terminate.assert_called_once()
        child.wait.assert_called_once_with(timeout=2)

    def test_direct_link_failure_kills_stalled_owned_child_and_preserves_error(self):
        child = Mock()
        child.poll.return_value = None
        child.wait.side_effect = [subprocess.TimeoutExpired('control', 2), 0]
        def commands(command, **kwargs):
            if command[0] == 'pw-link':
                raise subprocess.CalledProcessError(1, command)
        with patch.object(control.subprocess, 'Popen', return_value=child), \
             patch.object(control.subprocess, 'run', side_effect=commands), \
             patch.object(control.lab, 'decode_graph', return_value=self.direct_graph()):
            with self.assertRaises(subprocess.CalledProcessError):
                control.run_direct(Path('/synthetic/control'), 2, None)
        child.terminate.assert_called_once()
        child.kill.assert_called_once()
        self.assertEqual(child.wait.call_count, 2)

    def test_direct_trial_keeps_same_acceptance_and_does_not_spawn_loopback(self):
        with tempfile.TemporaryDirectory(prefix='virtualgamepad-pw-lab-') as directory:
            for missing in (0, 128):
                row = self.receipt(); row['missing'] = missing
                response = subprocess.CompletedProcess('fake', 0, stdout=json.dumps(row))
                with patch.dict(os.environ, PIPEWIRE_RUNTIME_DIR=directory,
                                XDG_RUNTIME_DIR=directory, PIPEWIRE_REMOTE='pipewire-0'), \
                     patch.object(control, 'run_direct', return_value=response), \
                     patch.object(control.subprocess, 'Popen') as spawn, \
                     patch('builtins.print') as output:
                    self.assertEqual(control.inside(Path('/synthetic/control'), 60,
                                                    topology='direct'), int(bool(missing)))
                    self.assertEqual(json.loads(output.call_args.args[0])['topology'], 'direct')
                    spawn.assert_not_called()

    def test_direct_control_retains_initiating_and_cleanup_failures(self):
        child = Mock()
        child.poll.return_value = None
        child.terminate.side_effect = OSError('synthetic cleanup failure')
        with patch.object(control.subprocess, 'Popen', return_value=child), \
             patch.object(control.subprocess, 'run', side_effect=RuntimeError('synthetic graph failure')):
            with self.assertRaisesRegex(RuntimeError, 'synthetic graph failure; cleanup failed: synthetic cleanup failure'):
                control.run_direct(Path('/synthetic/control'), 2, None)

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
                    partial_bytes=0, errors=0, ledger_overflow=0, out_of_order=0)
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
            ledger = Path(directory) / 'ledger.jsonl'
            subprocess.run([binary, '--self-test', str(ledger)], check=True, timeout=5)
            rows = [json.loads(line) for line in ledger.read_text().splitlines()]
            self.assertEqual(rows[0]['producer_begin'], 95999)
            self.assertEqual(rows[0]['measured_frames'], 1)
            self.assertEqual(rows[0]['queue_result'], -5)
            self.assertTrue(rows[0]['buffer_dequeued'])
            self.assertIsNone(rows[0]['header_sequence'])
            self.assertEqual(rows[1]['marker_counts'][:3], [1, 0, 2])
            self.assertEqual(sum(rows[1]['marker_counts']), 256)
            self.assertEqual(len(rows[1]['marker_counts']), 256)
            before = ledger.read_bytes()
            self.assertNotEqual(subprocess.run([binary, '--self-test', str(ledger)], timeout=5).returncode, 0)
            self.assertEqual(ledger.read_bytes(), before)

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
        for key in ('partial_bytes', 'invalid', 'errors', 'ledger_overflow', 'out_of_order'):
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
