import importlib.util
from pathlib import Path
import unittest

MODULE = Path(__file__).resolve().parents[1]/'check-alpha-git-consumers.py'
SPEC = importlib.util.spec_from_file_location('git_consumers', MODULE)
consumer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(consumer)


class ExactRevisionTests(unittest.TestCase):
    def test_branch_tag_abbreviated_and_injected_revisions_are_rejected(self):
        for value in ['main', 'v0.1.0', 'abc1234', 'a'*39, 'g'*40, 'a'*40+'\n']:
            with self.assertRaises(ValueError):
                consumer.validate_revision(value)
        self.assertEqual(consumer.validate_revision('ab'*20), 'ab'*20)
