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
        self.source = None
        self.capture = None
        self.depth = 0
        self.parts = []

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        href = attributes.get('href', '')
        if tag == 'a' and attributes.get('class') == 'src' and self.source is None:
            self.source = href
        if tag == 'a' and href.endswith('.html') and '/' not in href:
            if href.startswith(('struct.', 'enum.', 'trait.', 'type.', 'fn.', 'constant.', 'mod.')):
                self.links.add(href)
        if self.capture and tag not in ('br', 'wbr', 'img', 'hr', 'input', 'meta', 'link'):
            self.depth += 1
        elif (tag == 'pre' and attributes.get('class') == 'rust item-decl') or (
                tag == 'h4' and 'code-header' in attributes.get('class', '').split()):
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
                self.capture = None

    def handle_data(self, data):
        if self.capture:
            self.parts.append(data)


def parse(page):
    parser = PublicPage()
    parser.feed(page)
    return parser


def render(root):
    links = parse((root / 'index.html').read_text()).links
    output = [
        '# Working root API inventory', '',
        'Baseline: PR #121 (`f8a10d0`) plus the current refinement working tree.',
        'Generated from Linux all-feature rustdoc. Supporting crates and',
        '`experimental` are outside the alpha application compatibility promise.',
        'This is the working inventory; regenerate after GUI feedback before freeze.', '',
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
    if not links:
        raise ValueError('No root API items found')
    return '\n'.join(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--doc-root', type=Path, default=Path('target/doc/virtualgamepad'))
    parser.add_argument('--output', type=Path, default=Path('docs/architecture-overhaul/API_INVENTORY.md'))
    arguments = parser.parse_args()
    arguments.output.write_text(render(arguments.doc_root))


if __name__ == '__main__':
    main()
