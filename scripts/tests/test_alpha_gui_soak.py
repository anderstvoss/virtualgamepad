import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('gui_soak', Path(__file__).parents[1] / 'run-alpha-gui-soak.py')
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


class GuiSoak(unittest.TestCase):
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
