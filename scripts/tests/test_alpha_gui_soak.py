import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('gui_soak', Path(__file__).parents[1] / 'run-alpha-gui-soak.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class GuiSoak(unittest.TestCase):
    def cycle(self, seconds):
        return f'gui_soak seconds={seconds} rss="VmRSS:\\t  123456 kB" fds=17 threads=23'

    def test_elapsed_time_and_zero_exit_cannot_replace_completed_workload(self):
        for text in ('', 'gui_soak completed seconds=7200',
                     self.cycle(60), self.cycle(60) + '\ngui_soak completed seconds=7200'):
            with self.subTest(text=text), self.assertRaisesRegex(ValueError, 'workload'):
                lab.workload_receipt(text, 7200)

    def test_complete_cycles_and_final_boundary_are_accounted_separately(self):
        cycles = '\n'.join(self.cycle(value) for value in range(60, 7200, 60))
        row = lab.workload_receipt(cycles + '\ngui_soak completed seconds=7200', 7200)
        self.assertEqual(row, dict(cycles=119, first_cycle_seconds=60,
                                   last_cycle_seconds=7140, completed_seconds=7200))
        self.assertEqual(lab.workload_receipt(self.cycle(60) +
                         '\ngui_soak completed seconds=60', 60)['cycles'], 1)

    def test_partial_duplicate_regressing_or_malformed_workload_is_rejected(self):
        valid = self.cycle(60) + '\ngui_soak completed seconds=60'
        for text in (valid.replace('fds=17', 'fds=0'), valid.replace('threads=23', 'threads=0'),
                     valid.replace('completed seconds=60', 'completed seconds=59'),
                     valid + '\ngui_soak completed seconds=59', valid + '\n' + self.cycle(120),
                     self.cycle(60) + '\n' + valid,
                     self.cycle(120) + '\n' + valid,
                     valid.replace('threads=23', 'threads=23-extra')):
            with self.subTest(text=text), self.assertRaises(ValueError):
                lab.workload_receipt(text, 60)

    def test_pending_gui_close_may_repeat_without_counting_another_cycle(self):
        text = self.cycle(60) + '\ngui_soak completed seconds=60\ngui_soak completed seconds=60'
        self.assertEqual(lab.workload_receipt(text, 60)['cycles'], 1)

    def test_workload_file_is_bounded_and_retains_malformed_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'gui.log'
            valid = self.cycle(60) + '\ngui_soak completed seconds=60'
            path.write_text(valid)
            self.assertEqual(lab.workload_file(path, 60)['cycles'], 1)
            for data in (b'x' * (lab.WORKLOAD_LOG_LIMIT + 1), b'\xff'):
                path.write_bytes(data)
                with self.assertRaises((ValueError, UnicodeError)):
                    lab.workload_file(path, 60)
                self.assertEqual(path.read_bytes(), data)

    def test_owned_unit_has_memory_bound_group_cleanup_and_runtime_deadline(self):
        command = lab.unit_command('virtualgamepad-gui-soak-synthetic.service', Path('/synthetic/runner'), ['--apply'])
        for property in ('MemoryHigh=512M', 'MemoryMax=1G', 'MemorySwapMax=0', 'KillMode=control-group', 'RuntimeMaxSec=7260'):
            self.assertIn('--property=' + property, command)
        self.assertIn('--inside-unit', command)
        with self.assertRaises(ValueError): lab.unit_command('foreign.service', Path('/synthetic/runner'), [])

    def test_cleanup_inventory_only_matches_exact_owned_hid_labels(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, physical in [('owned', 'virtualgamepad/dualsense/p2a-i9'),
                                   ('foreign', 'virtualgamepad/dualsense/p2b-i9'),
                                   ('unrelated', 'other/p2a-i9')]:
                (root / name).mkdir()
                (root / name / 'phys').write_text(physical)
            self.assertEqual(lab.owned_hid(42, root), ['owned'])
