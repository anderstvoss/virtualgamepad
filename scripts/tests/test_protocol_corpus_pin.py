import json
import subprocess
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


class CorpusPin(unittest.TestCase):
    # Runs without the private corpus: Cargo fetches a Git dependency's
    # submodules, so the corpus must stay a pinned revision, not a submodule.
    def test_repository_has_no_corpus_submodule(self):
        self.assertFalse((REPO / '.gitmodules').exists())
        tracked = subprocess.check_output(['git', '-C', str(REPO), 'ls-files', '--stage', 'protocol-corpus'],
                                          text=True).strip()
        self.assertEqual(tracked, '')

    def test_pin_is_one_full_revision(self):
        self.assertRegex((REPO / 'protocol-corpus.rev').read_text(), r'\A[0-9a-f]{40}\n\Z')

    def test_generated_fixtures_match_pin(self):
        manifest = json.loads((REPO / 'tests/fixtures/protocol-corpus/manifest.json').read_text())
        self.assertEqual(manifest['corpus_commit'], (REPO / 'protocol-corpus.rev').read_text().strip())


if __name__ == '__main__':
    unittest.main()
