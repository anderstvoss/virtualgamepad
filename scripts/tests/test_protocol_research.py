import hashlib
import json
import importlib.util
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('research_check', REPO / 'scripts/check-protocol-research.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.PIPE, text=True).strip()


class ResearchWorkflow(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        git(self.root, 'init')
        self.corpus = self.root / 'protocol-research'
        shutil.copytree(REPO / 'protocol-research', self.corpus, ignore=shutil.ignore_patterns('.git', '__pycache__'))
        git(self.corpus, 'init')
        # Detached maintenance can recreate .git files during temporary cleanup.
        for repo in (self.root, self.corpus):
            git(repo, 'config', 'maintenance.auto', 'false')
            git(repo, 'config', 'gc.auto', '0')
        git(self.corpus, 'add', '.')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-m', 'fixture')
        self.pin(git(self.corpus, 'rev-parse', 'HEAD'))

        (self.root / '.gitmodules').write_text(
            '[submodule "protocol-research"]\n\tpath = protocol-research\n'
            '\turl = https://example.invalid/research.git\n\tupdate = none\n')

    def pin(self, revision):
        git(self.root, 'update-index', '--add', '--cacheinfo', '160000,' + revision + ',protocol-research')

    def test_fixture_commits_do_not_launch_background_maintenance(self):
        for repo in (self.root, self.corpus):
            result = subprocess.run(
                ['git', '-C', str(repo), '-c', 'user.name=Fixture',
                 '-c', 'user.email=fixture@example.invalid', 'commit', '--allow-empty',
                 '-m', 'maintenance boundary'],
                env={**os.environ, 'GIT_TRACE': '1'}, text=True, capture_output=True, check=True)
            self.assertNotRegex(result.stderr, r'(?:maintenance run|gc)\s+--auto')

    def test_regeneration_and_stale_output(self):
        MODULE.check(self.root, write=True)
        MODULE.check(self.root)
        (self.root / 'tests/fixtures/protocol-research/ds-neutral.hex').write_text('00\n')
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

    def test_wii_fixtures_are_adopted_with_fixed_source_shapes(self):
        MODULE.check(self.root, write=True)
        generated = self.root / 'tests/fixtures/protocol-research'
        manifest = json.loads((generated / 'manifest.json').read_text())
        self.assertEqual(manifest['generator_version'], 4)
        self.assertEqual(len(list(generated.glob('wii-c5-*.hex'))), 15)
        self.assertEqual(len(bytes.fromhex((generated / 'wii-c5-neutral-37.hex').read_text())), 22)
        path = self.corpus / 'fixtures/wii-c5-neutral-37.hex'
        path.write_text('00' * 23 + '\n')
        record_path = self.corpus / 'records/fixture/wii-c5-neutral-37.json'
        record = json.loads(record_path.read_text())
        record['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
        record_path.write_text(json.dumps(record))
        git(self.corpus, 'add', '.')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-m', 'wrong Wii shape with matching hash')
        self.pin(git(self.corpus, 'rev-parse', 'HEAD'))
        with self.assertRaisesRegex(ValueError, 'unexpected fixture size: wii-c5-neutral-37'):
            MODULE.check(self.root, write=True)

    def test_consumer_shape_is_checked_independently_of_hash(self):
        path = self.corpus / 'fixtures/wii-c5-evdev-golden.json'
        shape = json.loads(path.read_text())
        shape['controls'].pop()
        path.write_text(json.dumps(shape))
        record_path = self.corpus / 'records/fixture/wii-c5-evdev-golden.json'
        record = json.loads(record_path.read_text())
        record['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
        record_path.write_text(json.dumps(record))
        git(self.corpus, 'add', '.')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-m', 'wrong consumer shape with matching hash')
        self.pin(git(self.corpus, 'rev-parse', 'HEAD'))
        with self.assertRaisesRegex(ValueError, 'unexpected consumer shape'):
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

    def test_missing_pin_or_update_marker(self):
        git(self.root, 'update-index', '--force-remove', 'protocol-research')
        with self.assertRaisesRegex(ValueError, 'gitlink'):
            MODULE.check(self.root)
        self.pin(git(self.corpus, 'rev-parse', 'HEAD'))
        (self.root / '.gitmodules').write_text(
            '[submodule "protocol-research"]\n\tpath = protocol-research\n'
            '\turl = https://example.invalid/research.git\n')
        with self.assertRaisesRegex(ValueError, 'update = none'):
            MODULE.check(self.root)

    def test_recursive_clone_skips_research(self):
        # Git consumers clone this repository recursively (Cargo always does);
        # `update = none` keeps them from fetching the research submodule.
        MODULE.check(self.root, write=True)
        git(self.root, 'add', '.gitmodules', 'tests')
        git(self.root, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-m', 'synthetic superproject')
        clone = self.root / 'recursive-clone'
        git(self.root, 'clone', '--recurse-submodules', str(self.root), str(clone))
        self.assertEqual(list((clone / 'protocol-research').iterdir()), [])
        with self.assertRaisesRegex(ValueError, 'absent'):
            MODULE.check(clone)

    def test_acceptance_evidence_must_exist_at_its_declared_revision(self):
        previous = git(self.corpus, 'rev-parse', 'HEAD')
        (self.corpus / 'records/claim/later-evidence.json').write_text('{}')
        git(self.corpus, 'add', '.')
        git(self.corpus, '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid',
            'commit', '-m', 'later evidence')
        latest = git(self.corpus, 'rev-parse', 'HEAD')
        self.pin(latest)
        folder = self.root / 'acceptance'
        folder.mkdir()
        manifest = folder / 'virtualgamepad.fixture.toml'

        def declare(revision, evidence):
            manifest.write_text('research_revision = "' + revision + '"\n'
                                '[[functions]]\nid = "fixture"\n[functions.cells.local]\n'
                                'evidence = ["' + evidence + '"]\n')

        declare(previous, 'later-evidence')
        with self.assertRaisesRegex(ValueError, 'missing acceptance evidence.*later-evidence'):
            MODULE.check(self.root, write=True)
        declare(latest, 'later-evidence')
        MODULE.check(self.root, write=True)
        # An older revision remains valid when it contains its own evidence.
        declare(previous, 'wii-w5-kernel-motionplus-gyro-delivery')
        MODULE.check(self.root)
        declare(previous.upper(), 'wii-w5-kernel-motionplus-gyro-delivery')
        MODULE.check(self.root)
        declare(latest, 'later-evidence')
        git(self.corpus, 'checkout', previous)
        self.pin(previous)
        with self.assertRaisesRegex(ValueError, 'acceptance research revision'):
            MODULE.check(self.root)
        git(self.corpus, 'checkout', latest)
        self.pin(latest)
        declare('0' * 40, 'later-evidence')
        with self.assertRaisesRegex(ValueError, 'acceptance research revision'):
            MODULE.check(self.root)

    def test_dirty_corpus(self):
        (self.corpus / 'untracked.txt').write_text('synthetic')
        with self.assertRaisesRegex(ValueError, 'uncommitted'):
            MODULE.check(self.root)


if __name__ == '__main__':
    unittest.main()
