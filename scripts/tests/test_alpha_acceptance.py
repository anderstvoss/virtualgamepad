import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('acceptance', Path(__file__).parents[1] / 'run-alpha-acceptance.py')
tool = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tool)


class AcceptanceMatrix(unittest.TestCase):
    def test_build_receipt_rejects_changed_revision_or_dirty_source(self):
        with patch.object(tool.subprocess, 'check_output', side_effect=['new-revision\n']):
            with self.assertRaises(RuntimeError): tool.ensure_candidate(Path('/synthetic'), 'old-revision')
        with patch.object(tool.subprocess, 'check_output', side_effect=['revision\n', ' M source.rs\n']):
            with self.assertRaises(RuntimeError): tool.ensure_candidate(Path('/synthetic'), 'revision')
        with patch.object(tool.subprocess, 'check_output', side_effect=['revision\n', '']):
            tool.ensure_candidate(Path('/synthetic'), 'revision')

    def test_all_families_directions_duplex_ownerships_and_three_trials(self):
        rows = tool.matrix()
        self.assertEqual(len(rows), 72)
        self.assertEqual(len({tuple(row.values()) for row in rows}), 72)
        for family in tool.FAMILIES:
            cells = [r for r in rows if r['family'] == family]
            self.assertEqual(len(cells), 24)
            self.assertEqual({r['ownership'] for r in cells}, {None, *tool.OWNERSHIPS})
            self.assertEqual({r['trial'] for r in cells}, {1, 2, 3})


if __name__ == '__main__':
    unittest.main()
