import hashlib
import json
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
        self.pin(git(self.corpus, 'rev-parse', 'HEAD'))

    def pin(self, revision):
        (self.root / 'protocol-corpus.rev').write_text(revision + '\n')

    def test_regeneration_and_stale_output(self):
        MODULE.check(self.root, write=True)
        MODULE.check(self.root)
        (self.root / 'tests/fixtures/protocol-corpus/ds-neutral.hex').write_text('00\n')
        with self.assertRaisesRegex(ValueError, 'stale'):
            MODULE.check(self.root)

    def test_reference_fixture_size_is_checked_independently_of_hash(self):
        path = self.corpus / 'fixtures/sony-contact-pair.hex'
        path.write_text('00' * 9 + '\n')
        record_path = self.corpus / 'records/fixture/sony-contact-pair.json'
        record = json.loads(record_path.read_text())
        record['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
        record_path.write_text(json.dumps(record))
        git(self.corpus, 'add', '.')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-m', 'wrong shape with valid hash')
        self.pin(git(self.corpus, 'rev-parse', 'HEAD'))
        with self.assertRaisesRegex(ValueError, 'unexpected fixture size: sony-contact-pair'):
            MODULE.check(self.root, write=True)

    def test_missing_and_mismatched_checkout(self):
        (self.corpus / '.git').rename(self.root / 'saved-git')
        with self.assertRaisesRegex(ValueError, 'absent'):
            MODULE.check(self.root)
        (self.root / 'saved-git').rename(self.corpus / '.git')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '--allow-empty', '-m', 'unadopted')
        with self.assertRaisesRegex(ValueError, 'HEAD differs'):
            MODULE.check(self.root)

    def test_remote_rejects_unpublished_revision(self):
        remote = self.root / 'remote.git'
        git(self.root, 'init', '--bare', str(remote))
        git(self.corpus, 'remote', 'add', 'origin', str(remote))
        git(self.corpus, 'push', 'origin', 'HEAD:main')
        MODULE.check(self.root, write=True, verify_remote=True)
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '--allow-empty', '-m', 'unpublished')
        self.pin(git(self.corpus, 'rev-parse', 'HEAD'))
        with self.assertRaises(subprocess.CalledProcessError):
            MODULE.check(self.root, write=True, verify_remote=True)

    def test_missing_or_malformed_pin(self):
        (self.root / 'protocol-corpus.rev').unlink()
        with self.assertRaisesRegex(ValueError, 'missing'):
            MODULE.check(self.root)
        for bad in ('', 'a1789d6', 'A' * 40, 'g' * 40, ('0' * 40 + '\n') * 2):
            (self.root / 'protocol-corpus.rev').write_text(bad)
            with self.assertRaisesRegex(ValueError, 'full lowercase commit SHA'):
                MODULE.check(self.root)

    def test_clone_needs_no_corpus_access(self):
        # Git consumers clone this repository recursively (Cargo always does);
        # the corpus must not be a submodule they are forced to fetch.
        MODULE.check(self.root, write=True)
        (self.root / '.gitignore').write_text('/protocol-corpus/\n')
        git(self.root, 'add', '.gitignore', 'protocol-corpus.rev', 'tests')
        git(self.root, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-m', 'synthetic superproject')
        self.assertEqual(git(self.root, 'ls-files', '--stage', 'protocol-corpus'), '')
        clone = self.root / 'recursive-clone'
        git(self.root, 'clone', '--recurse-submodules', str(self.root), str(clone))
        self.assertFalse((clone / 'protocol-corpus').exists())
        with self.assertRaisesRegex(ValueError, 'absent'):
            MODULE.check(clone)

    def test_dirty_corpus(self):
        (self.corpus / 'untracked.txt').write_text('synthetic')
        with self.assertRaisesRegex(ValueError, 'uncommitted'):
            MODULE.check(self.root)


if __name__ == '__main__':
    unittest.main()
