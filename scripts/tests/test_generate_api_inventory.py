import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE = Path(__file__).resolve().parents[1] / 'generate-api-inventory.py'
SPEC = importlib.util.spec_from_file_location('inventory', MODULE)
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


class InventoryTests(unittest.TestCase):
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
