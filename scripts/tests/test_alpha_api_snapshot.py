import importlib.util
from contextlib import redirect_stderr, redirect_stdout
import io
import json
from pathlib import Path
import sys
import tempfile
import subprocess
import unittest
from unittest.mock import patch

MODULE = Path(__file__).resolve().parents[1]/'check-alpha-api.py'
SPEC = importlib.util.spec_from_file_location('alpha_api', MODULE)
api = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(api)


def render_fixture(source, docs):
    result = subprocess.run(
        ['rustdoc', '--edition=2024', '--crate-name', 'snapshot_fixture',
         str(source), '-o', str(docs)], capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(
            f'rustdoc fixture failed ({result.returncode}):\n'
            f'{result.stdout}\n{result.stderr}')


class SnapshotTests(unittest.TestCase):
    def test_steam_native_state_snapshot_tracks_fields_variants_methods_and_traits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(FileNotFoundError):
                api.steam_snapshot(root)
            (root/'index.html').write_text(
                '<a href="struct.State.html">State</a><a href="enum.Button.html">Button</a>')
            state = root/'struct.State.html'
            button = root/'enum.Button.html'
            declaration = '<pre class="rust item-decl">pub struct State { pub pressure: u16 }</pre>'
            buttons = '<pre class="rust item-decl">pub enum Button { Menu, View }</pre>'
            state.write_text(declaration)
            button.write_text(buttons)
            baseline = api.steam_snapshot(root)
            self.assertEqual(baseline['scope'],
                             'gr-curated-controllers Steam Controller 2026 supporting API; not ordinary root')
            for content in [declaration.replace('u16', 'i16'),
                            declaration + '<h4 class="code-header">pub fn neutralize(&amp;mut self)</h4>',
                            declaration + '<h3 class="code-header">impl !Sync for State</h3>']:
                state.write_text(content)
                self.assertNotEqual(api.steam_snapshot(root), baseline)
            state.write_text(declaration + '<h3 class="code-header">impl Freeze for State</h3>')
            self.assertEqual(api.steam_snapshot(root), baseline)
            button.write_text(buttons.replace('View', 'View, Extra'))
            self.assertNotEqual(api.steam_snapshot(root), baseline)

    def test_wii_import_api_is_checked_separately_and_requires_existing_docs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(FileNotFoundError):
                api.wii_snapshot(root)
            (root/'index.html').write_text('<a href="struct.Calibration.html">Calibration</a>')
            page = root/'struct.Calibration.html'
            declaration = '<pre class="rust item-decl">pub struct Calibration;</pre>'
            page.write_text(declaration)
            before = api.wii_snapshot(root)
            page.write_text(declaration + '<h4 class="code-header">pub fn import()</h4>')
            self.assertNotEqual(api.wii_snapshot(root), before)
            self.assertEqual(before['scope'],
                             'gr-curated-controllers Wii supporting API; not ordinary root')

    def test_rustdoc_failure_preserves_compiler_diagnostic(self):
        failure = subprocess.CompletedProcess(
            [], 1, stdout='fixture output', stderr='compiler unavailable')
        with patch.object(subprocess, 'run', return_value=failure):
            with self.assertRaisesRegex(
                    RuntimeError, r'(?s)failed \(1\).*fixture output.*compiler unavailable'):
                render_fixture(Path('fake.rs'), Path('fake-docs'))

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
                render_fixture(source, docs)
                doc_root = docs/'snapshot_fixture'
                if ordinary:
                    with self.assertRaisesRegex(ValueError, 'ordinary/index.html'):
                        api.snapshot(doc_root, manifest)
                else:
                    self.assertEqual(set(api.snapshot(doc_root, manifest)['items']),
                                     {'struct.Handle.html'})


class SteamSnapshotCliTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.docs = self.root/'doc/virtualgamepad'
        self.wii = self.root/'doc/gr_curated_controllers/wii_remote'
        self.steam = self.root/'doc/gr_curated_controllers/steam_controller_2026'
        for path in [self.docs, self.wii, self.steam]:
            path.mkdir(parents=True)
            (path/'index.html').write_text('<a href="struct.State.html">State</a>')
            (path/'struct.State.html').write_text(
                '<pre class="rust item-decl">pub struct State { pub value: u16 }</pre>')
        (self.root/'Cargo.toml').write_text('[features]\ndefault = []\nexperimental = []\n')
        manifest = {'features': {'default': [], 'experimental': []}}
        self.alpha_baseline = self.root/'alpha.json'
        self.wii_baseline = self.root/'wii.json'
        self.steam_baseline = self.root/'steam.json'
        self.alpha_baseline.write_text(json.dumps({
            'schema': 2, 'views': {'all': api.snapshot(self.docs, manifest)}}))
        self.wii_baseline.write_text(json.dumps(api.wii_snapshot(self.wii)))
        self.steam_baseline.write_text(json.dumps(api.steam_snapshot(self.steam)))
        self.original_baselines = [path.read_bytes() for path in
                                   [self.alpha_baseline, self.wii_baseline, self.steam_baseline]]

    def run_check(self, *extra):
        with patch.multiple(api, ROOT=self.root, BASELINE=self.alpha_baseline,
                            WII_BASELINE=self.wii_baseline, STEAM_BASELINE=self.steam_baseline), \
                patch.object(sys, 'argv', ['check-alpha-api.py', '--doc-root', str(self.docs), *extra]), \
                redirect_stdout(io.StringIO()) as output:
            api.main()
            return output.getvalue()

    def test_cli_checks_steam_drift_even_when_root_and_wii_are_unchanged(self):
        self.assertIn('Steam supporting API matches', self.run_check())
        page = self.steam/'struct.State.html'
        page.write_text(page.read_text().replace('u16', 'i16'))
        with redirect_stderr(io.StringIO()) as error, self.assertRaises(SystemExit) as raised:
            self.run_check()
        self.assertEqual(raised.exception.code, 1)
        self.assertIn('Steam supporting API changed', error.getvalue())
        self.assertEqual([path.read_bytes() for path in
                          [self.alpha_baseline, self.wii_baseline, self.steam_baseline]],
                         self.original_baselines)

    def test_cli_missing_steam_baseline_fails_instead_of_accepting_unreviewed_api(self):
        self.steam_baseline.unlink()
        with redirect_stderr(io.StringIO()) as error, self.assertRaises(SystemExit) as raised:
            self.run_check()
        self.assertEqual(raised.exception.code, 1)
        self.assertIn('Steam supporting API changed', error.getvalue())

    def test_cli_explicit_steam_docs_override_inferred_path(self):
        other = self.root/'explicit-steam-docs'
        other.mkdir()
        for path in self.steam.iterdir():
            (other/path.name).write_bytes(path.read_bytes())
        (self.steam/'struct.State.html').write_text(
            '<pre class="rust item-decl">pub struct State { pub changed: i32 }</pre>')
        self.assertIn('Steam supporting API matches', self.run_check('--steam-doc-root', str(other)))
        with self.assertRaises(FileNotFoundError):
            self.run_check('--steam-doc-root', str(self.root/'absent'))

    def test_cli_steam_doc_override_requires_root_docs(self):
        with patch.object(sys, 'argv', ['check-alpha-api.py', '--steam-doc-root', 'fake']), \
                redirect_stderr(io.StringIO()) as error, self.assertRaises(SystemExit) as raised:
            api.main()
        self.assertEqual(raised.exception.code, 2)
        self.assertIn('--steam-doc-root requires --doc-root', error.getvalue())
