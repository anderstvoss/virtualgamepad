import importlib.util
from pathlib import Path
import tempfile
import subprocess
import unittest

MODULE = Path(__file__).resolve().parents[1]/'check-alpha-api.py'
SPEC = importlib.util.spec_from_file_location('alpha_api', MODULE)
api = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(api)


class SnapshotTests(unittest.TestCase):
    def test_fields_methods_traits_and_features_change_the_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'index.html').write_text('<a href="struct.Handle.html">Handle</a>')
            page = root/'struct.Handle.html'
            initial = '<pre class="rust item-decl">pub struct Handle;</pre>'
            manifest = {'features': {'default': [], 'experimental': []}}
            page.write_text(initial)
            baseline = api.snapshot(root, manifest)
            for suffix in ['<h4 class="code-header">pub fn close(&amp;mut self)</h4>',
                           '<h3 class="code-header">impl !Sync for Handle</h3>']:
                page.write_text(initial+suffix)
                self.assertNotEqual(api.snapshot(root, manifest), baseline)
            page.write_text('<pre class="rust item-decl">pub struct Handle { pub id: u64 }</pre>')
            self.assertNotEqual(api.snapshot(root, manifest), baseline)
            page.write_text(initial+'<h3 class="code-header">impl Freeze for Handle</h3>')
            self.assertEqual(api.snapshot(root, manifest), baseline)
            manifest['features']['default'] = ['experimental']
            self.assertNotEqual(api.snapshot(root, manifest), baseline)

    def test_new_root_modules_fail_closed_but_experimental_is_excluded(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            index = root/'index.html'
            index.write_text('<a href="struct.Handle.html">Handle</a><a href="mod.experimental.html">SPI</a>')
            (root/'struct.Handle.html').write_text('<pre class="rust item-decl">pub struct Handle;</pre>')
            api.snapshot(root, {'features': {'default': []}})
            index.write_text(index.read_text()+'<a href="mod.extra.html">Extra</a>')
            with self.assertRaisesRegex(ValueError, 'Unreviewed'):
                api.snapshot(root, {'features': {'default': []}})

    def test_real_rustdoc_modules_fail_closed_but_experimental_is_excluded(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root/'lib.rs'
            manifest = {'features': {'default': [], 'experimental': []}}
            for ordinary in [False, True]:
                source.write_text('pub struct Handle; pub mod experimental { pub struct Spi; }'
                                  + (' pub mod ordinary { pub struct Added; }' if ordinary else ''))
                docs = root/str(ordinary)
                subprocess.run(['rustdoc', '--edition=2024', '--crate-name', 'snapshot_fixture',
                                str(source), '-o', str(docs)], check=True, capture_output=True)
                doc_root = docs/'snapshot_fixture'
                if ordinary:
                    with self.assertRaisesRegex(ValueError, 'ordinary/index.html'):
                        api.snapshot(doc_root, manifest)
                else:
                    self.assertEqual(set(api.snapshot(doc_root, manifest)['items']),
                                     {'struct.Handle.html'})
