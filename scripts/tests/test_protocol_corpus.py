import importlib.util
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('corpus_check', REPO / 'scripts/check-protocol-corpus.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.PIPE, text=True).strip()


class CorpusWorkflow(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        git(self.root, 'init')
        self.corpus = self.root / 'protocol-corpus'
        shutil.copytree(REPO / 'protocol-corpus', self.corpus, ignore=shutil.ignore_patterns('.git', '__pycache__'))
        git(self.corpus, 'init')
        git(self.corpus, 'add', '.')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-m', 'fixture')
        revision = git(self.corpus, 'rev-parse', 'HEAD')
        git(self.root, 'update-index', '--add', '--cacheinfo', '160000,' + revision + ',protocol-corpus')

    def test_regeneration_and_stale_output(self):
        MODULE.check(self.root, write=True)
        MODULE.check(self.root)
        (self.root / 'tests/fixtures/protocol-corpus/ds-neutral.hex').write_text('00\n')
        with self.assertRaisesRegex(ValueError, 'stale'):
            MODULE.check(self.root)

    def test_missing_and_mismatched_submodule(self):
        (self.corpus / '.git').rename(self.root / 'saved-git')
        with self.assertRaisesRegex(ValueError, 'absent'):
            MODULE.check(self.root)
        (self.root / 'saved-git').rename(self.corpus / '.git')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '--allow-empty', '-m', 'unadopted')
        with self.assertRaisesRegex(ValueError, 'HEAD differs'):
            MODULE.check(self.root)

    def test_dirty_corpus(self):
        (self.corpus / 'untracked.txt').write_text('synthetic')
        with self.assertRaisesRegex(ValueError, 'uncommitted'):
            MODULE.check(self.root)


if __name__ == '__main__':
    unittest.main()
