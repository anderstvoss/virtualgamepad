import importlib.util
import tempfile
import subprocess
import sys
import unittest
from pathlib import Path

MODULE = Path(__file__).resolve().parents[1] / 'generate-api-inventory.py'
SPEC = importlib.util.spec_from_file_location('inventory', MODULE)
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


class InventoryTests(unittest.TestCase):
    def test_root_module_links_exclude_parent_external_and_nested_pages(self):
        page = inventory.parse('<a href="experimental/index.html">SPI</a>'
                               '<a href="ordinary/index.html">Ordinary</a>'
                               '<a href="../index.html">Parent</a>'
                               '<a href="./index.html">Self</a>'
                               '<a href="https://example.invalid/index.html">External</a>'
                               '<a href="ordinary/nested/index.html">Nested</a>')
        self.assertEqual(page.modules, {'experimental/index.html', 'ordinary/index.html'})

    def test_public_declarations_include_methods_and_skip_trait_implementation_details(self):
        page = inventory.parse('<pre class="rust item-decl"><code>pub struct Read { pub frames: usize }</code></pre>'
                               '<h4 class="code-header">pub fn frames(&amp;self) -&gt; usize</h4>'
                               '<h4 class="code-header">fn clone(&amp;self) -&gt; Self</h4>')
        self.assertEqual(page.declarations, ['pub struct Read { pub frames: usize }', 'pub fn frames(&self) -> usize'])

    def test_empty_docs_fail_instead_of_generating_a_false_complete_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'index.html').write_text('empty')
            with self.assertRaises(ValueError):
                inventory.render(root)

    def test_root_items_and_experimental_boundary_are_distinct(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'index.html').write_text('<a href="struct.Read.html">Read</a><a href="mod.experimental.html">SPI</a>')
            (root/'struct.Read.html').write_text('<pre class="rust item-decl">pub struct Read;</pre>')
            text = inventory.render(root)
            self.assertIn('pub struct Read;', text)
            self.assertIn('excluded from ordinary-root inventory', text)

    def test_attributes_and_void_tags_do_not_hide_public_fields_or_methods(self):
        page = inventory.parse('<pre class="rust item-decl"><code><div>#[non_exhaustive]</div>pub enum Access { Samples }</code></pre>'
                               '<h4 class="code-header">pub fn endpoint&lt;T&gt;(&amp;self)<wbr> -&gt; T</h4>')
        self.assertEqual(len(page.declarations), 2)
        self.assertIn('pub enum Access', page.declarations[0])
        self.assertEqual(page.declarations[1], 'pub fn endpoint<T>(&self) -> T')

    def test_concrete_and_auto_traits_are_retained_without_blanket_impls(self):
        page = inventory.parse('<h3 class="code-header">impl Clone for TouchSlot</h3>'
                               '<h3 class="code-header">impl !Sync for ControllerAudio</h3>'
                               '<h3 class="code-header">impl&lt;T&gt; From&lt;T&gt; for T</h3>'
                               '<h3 class="code-header">impl&lt;T&gt; Any for Twhere T: Static</h3>')
        self.assertEqual(page.traits, ['impl Clone for TouchSlot', 'impl !Sync for ControllerAudio'])

    def test_check_mode_detects_drift_without_modifying_saved_inventory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'index.html').write_text('<a href="struct.Read.html">Read</a>')
            (root/'struct.Read.html').write_text('<pre class="rust item-decl">pub struct Read;</pre>')
            output = root/'inventory.md'
            output.write_text(inventory.render(root))
            command = [sys.executable, str(MODULE), '--doc-root', str(root), '--output', str(output), '--check']
            self.assertEqual(subprocess.run(command, capture_output=True).returncode, 0)
            (root/'struct.Read.html').write_text('<pre class="rust item-decl">pub struct Read { pub frames: usize }</pre>')
            before = output.read_bytes()
            self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
            self.assertEqual(output.read_bytes(), before)

    def test_owner_does_not_fall_back_to_an_unrelated_std_trait_source(self):
        page = inventory.parse('<a class="src" href="https://doc.rust-lang.org/src/core/clone.rs">Source</a>'
                               '<a class="src" href="../src/controllers/state.rs.html">Source</a>')
        self.assertEqual(page.source, '../src/controllers/state.rs.html')

    def test_platform_comparison_ignores_owner_urls_but_detects_field_or_trait_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            other = root/'other'
            other.mkdir()
            for path in (root, other):
                (path/'index.html').write_text('<a href="struct.Read.html">Read</a>')
                (path/'struct.Read.html').write_text('<pre class="rust item-decl">pub struct Read;</pre>')
            (other/'struct.Read.html').write_text('<a class="src" href="../src/read.rs.html">Source</a>'
                                                '<pre class="rust item-decl">pub struct Read;</pre>')
            self.assertEqual(inventory.public_surface(root), inventory.public_surface(other))
            (other/'struct.Read.html').write_text('<pre class="rust item-decl">pub struct Read;</pre>'
                                                '<h3 class="code-header">impl Send for Read</h3>')
            self.assertNotEqual(inventory.public_surface(root), inventory.public_surface(other))
