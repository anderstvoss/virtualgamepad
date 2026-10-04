import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('collect_sbom', Path(__file__).resolve().parents[1] / 'collect-sbom.py')
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class CollectionTests(unittest.TestCase):
    def fixture(self, directory, name):
        path = Path(directory) / name
        path.mkdir()
        (path / 'bom.json').write_text(json.dumps({'metadata': {'component': {'name': name, 'version': '1'}}, 'components': [{}]}))
        return {'name': name, 'version': '1', 'manifest_path': str(path / 'Cargo.toml'), 'dependencies': [{}]}

    def test_root_and_members_are_required_and_identity_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.fixture(directory, 'virtualgamepad')
            member = self.fixture(directory, 'member')
            output = Path(directory) / 'out'
            self.assertEqual(module.collect([root, member], output), {'virtualgamepad', 'member'})
            self.assertTrue((output / 'virtualgamepad.bom.json').exists())
            self.assertEqual(module.verify([root, member], output), {'virtualgamepad.bom.json', 'member.bom.json'})
            (output / 'duplicate.bom.json').write_text('{}')
            with self.assertRaises(ValueError):
                module.verify([root, member], output)
            (output / 'duplicate.bom.json').unlink()
            with self.assertRaises(ValueError):
                module.collect([root, member], output)
            (Path(root['manifest_path']).parent / 'bom.json').unlink()
            with self.assertRaises(FileNotFoundError):
                module.collect([root, member], Path(directory) / 'missing')
            self.assertFalse((Path(directory) / 'missing').exists())

    def test_duplicate_foreign_and_empty_dependency_reports_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.fixture(directory, 'virtualgamepad')
            with self.assertRaises(ValueError):
                module.collect([root, root], Path(directory) / 'out')
            source = Path(root['manifest_path']).parent / 'bom.json'
            for document in [{'metadata': {'component': {'name': 'wrong', 'version': '1'}}},
                             {'metadata': {'component': {'name': 'virtualgamepad', 'version': '1'}}}]:
                source.write_text(json.dumps(document))
                with self.assertRaises(ValueError):
                    module.collect([root], Path(directory) / 'out')
