#!/usr/bin/env python3
"""Generate the working root inventory from public rustdoc pages (no extra packages).

Build with cargo doc --locked -p virtualgamepad --no-deps --all-features first.
This is a reviewed candidate inventory, not a frozen compatibility snapshot.
"""
import argparse
from html.parser import HTMLParser
from pathlib import Path


class PublicPage(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = set()
        self.declarations = []
        self.traits = []
        self.source = None
        self.capture = None
        self.depth = 0
        self.parts = []

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        href = attributes.get('href', '')
        if (tag == 'a' and attributes.get('class') == 'src' and self.source is None
                and href.startswith('../src/')):
            self.source = href
        if tag == 'a' and href.endswith('.html') and '/' not in href:
            if href.startswith(('struct.', 'enum.', 'trait.', 'type.', 'fn.', 'constant.', 'static.', 'macro.', 'union.', 'mod.')):
                self.links.add(href)
        if self.capture and tag not in ('br', 'wbr', 'img', 'hr', 'input', 'meta', 'link'):
            self.depth += 1
        elif (tag == 'pre' and attributes.get('class') == 'rust item-decl') or (
                tag in ('h3', 'h4') and 'code-header' in attributes.get('class', '').split()):
            self.capture = tag
            self.depth = 1
            self.parts = []

    def handle_endtag(self, tag):
        if self.capture:
            self.depth -= 1
            if self.depth == 0:
                declaration = ' '.join(''.join(self.parts).split())
                if declaration.startswith(('pub ', '#[')):
                    self.declarations.append(declaration)
                elif self.capture == 'h3' and ' for ' in declaration:
                    # Concrete and auto traits are compatibility-relevant. Generic
                    # blanket impls belong to the toolchain, not this root type.
                    if declaration.split(' for ', 1)[1].split('where', 1)[0].strip() not in ('T', 'U'):
                        self.traits.append(declaration)
                self.capture = None

    def handle_data(self, data):
        if self.capture:
            self.parts.append(data)


def parse(page):
    parser = PublicPage()
    parser.feed(page)
    return parser


def public_surface(root):
    """Compare API declarations/traits, excluding target-specific source URLs."""
    result = {}
    for link in sorted(parse((root / 'index.html').read_text()).links):
        if not link.startswith('mod.'):
            page = parse((root / link).read_text())
            if not page.declarations:
                raise ValueError(f'No public declaration found: {link}')
            result[link] = (page.declarations, page.traits)
    if not result:
        raise ValueError('No root API items found')
    return result


def render(root):
    links = parse((root / 'index.html').read_text()).links
    output = [
        '# Working root API inventory', '',
        'Baseline: merged PRs #122–125, #127 and #128 (`ec4c2ff`) plus the post-GUI audit tree.',
        'Generated from Linux all-feature rustdoc. Supporting crates and',
        '`experimental` are outside the alpha application compatibility promise.',
        'This is the post-GUI working inventory; no hard freeze is declared.',
        'Ordinary-root declarations are available with every feature set on all',
        'compiling platforms. Provider creation is separately gated by Linux,',
        'features and host prerequisites; see [the audit](ALPHA_ROOT_API_AUDIT.md).', '',
        '## Review decisions', '',
        '| Group | Decision / rationale | Availability / future stress |',
        '| --- | --- | --- |',
        '| Native controller state, controls and output | Keep: package-owned native semantics | Four current families; future families retain independent state |',
        '| Creation, identity, service and lifecycle | Keep; cleanup errors supplement typed initiating cause | All platforms compile; unsupported providers reject; no fallback/manager |',
        '| Topology and surfaces | Change: private containers plus accessors; remove pixel sizing | Read-only UI introspection; construction SPI is not root |',
        '| Realization/component metadata | Change: exact primary realization and typed kinds | Explicit associated audio and composite USB; no role parsing |',
        '| PCM read, format, ownership and errors | Change: root-owned read, typed argument/ownership errors; remove matching | API types always visible; backends require Linux opt-in features |',
        '| Application audio health and pacing | Keep/change: opaque diagnostics and optional consumption progress | Borrowed direction-wide access; future groups remain additive |',
        '| Graph timing and bridge telemetry | Move to experimental instrumentation | Unstable qualification tooling; not end-to-end latency |',
        '| Old session/factory SPI | Remove unused traits; retain distinct manifest requirements | No ordinary-root construction contract |', '',
        'Public fields appear in declarations below. Inherent public methods follow',
        'each item. Re-exported types are expanded by rustdoc; their original owner',
        'remains the supporting controller/contract crate, not a new plugin contract.',
        'All listed ordinary-root items are retained with the group rationale above;',
        'the change/remove migration is in [migration notes](../ALPHA_API_MIGRATION.md).', '',
        'Concrete and auto trait implementations follow each declaration. Compiler',
        'blanket implementations and inherited trait method bodies are omitted.',
        'This rustdoc inventory is an audit aid, not a complete semver checker.', '',
        '## Exact current public declarations', '',
    ]
    for link in sorted(links):
        if link.startswith('mod.'):
            output += ['`experimental`: opt-in unstable research/SPI module; excluded from ordinary-root inventory.', '']
            continue
        page = parse((root / link).read_text())
        declarations = page.declarations
        if not declarations:
            raise ValueError(f'No public declaration found: {link}')
        name = link.split('.', 1)[1][:-5]
        source = (page.source or 'source unavailable').split('#', 1)[0]
        output += [f'### `{name}`', '', f'Owner/source: `{source}`', '', '```rust', *dict.fromkeys(declarations), '```', '']
        if page.traits:
            output += ['Trait implementations:', '', '```rust', *dict.fromkeys(page.traits), '```', '']
    if not links:
        raise ValueError('No root API items found')
    return '\n'.join(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--doc-root', type=Path, default=Path('target/doc/virtualgamepad'))
    parser.add_argument('--output', type=Path, default=Path('docs/architecture-overhaul/API_INVENTORY.md'))
    parser.add_argument('--check', action='store_true', help='fail if the saved inventory differs; do not write')
    parser.add_argument('--compare-doc-root', type=Path,
                        help='compare ordinary declarations/traits with another rustdoc root; do not write')
    arguments = parser.parse_args()
    rendered = render(arguments.doc_root)
    if arguments.compare_doc_root:
        if public_surface(arguments.doc_root) != public_surface(arguments.compare_doc_root):
            parser.exit(1, 'Ordinary root declarations or traits differ between rustdoc roots.\n')
    elif arguments.check:
        if arguments.output.read_text() != rendered:
            parser.exit(1, 'Root API inventory differs; regenerate and review the change.\n')
    else:
        arguments.output.write_text(rendered)


if __name__ == '__main__':
    main()
