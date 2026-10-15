import json
import subprocess
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def git(*args):
    return subprocess.check_output(['git', '-C', str(REPO), *args], text=True).strip()


class ResearchPin(unittest.TestCase):
    # Runs without the research checkout. Cargo fetches a Git dependency's
    # submodules unless .gitmodules sets `update = none`, so that marker keeps
    # Git consumers from ever cloning the research repository.
    def test_submodule_is_skipped_by_git_consumers(self):
        self.assertEqual(git('config', '--file', '.gitmodules', 'submodule.protocol-research.update'), 'none')
        self.assertEqual(git('config', '--file', '.gitmodules', 'submodule.protocol-research.url'),
                         'https://github.com/anderstvoss/controller-protocol-research.git')

    def test_pin_is_one_gitlink(self):
        self.assertRegex(git('ls-files', '--stage', '--', 'protocol-research'),
                         r'\A160000 [0-9a-f]{40} 0\tprotocol-research\Z')
        self.assertFalse((REPO / 'protocol-corpus.rev').exists())

    def test_generated_fixtures_match_pin(self):
        manifest = json.loads((REPO / 'tests/fixtures/protocol-research/manifest.json').read_text())
        self.assertEqual(manifest['corpus_commit'], git('ls-files', '--stage', '--', 'protocol-research').split()[1])


if __name__ == '__main__':
    unittest.main()
