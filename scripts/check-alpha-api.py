#!/usr/bin/env python3
"""Build and compare the ordinary root API with the reviewed alpha snapshot."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('inventory', ROOT/'scripts/generate-api-inventory.py')
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)
BASELINE = ROOT/'docs/architecture-overhaul/ALPHA_API_SNAPSHOT.json'


def snapshot(doc_root, manifest):
    links = inventory.parse((doc_root/'index.html').read_text()).links
    unknown_modules = [link for link in links if link.startswith('mod.') and link != 'mod.experimental.html']
    if unknown_modules:
        raise ValueError(f'Unreviewed ordinary root modules: {unknown_modules}')
    surface = inventory.public_surface(doc_root)
    # These rustdoc/compiler implementation details are not application traits.
    compiler_traits = ('Freeze', 'UnsafeUnpin', 'StructuralPartialEq')
    items = {name: {'declarations': declarations,
                    'traits': sorted(trait for trait in traits
                                     if not any(f' {kind} for ' in trait for kind in compiler_traits))}
             for name, (declarations, traits) in surface.items()}
    return {'schema': 1, 'scope': 'ordinary root; experimental and supporting SPI excluded',
            'features': sorted(manifest['features']),
            'default_features': manifest['features']['default'], 'items': items}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true', help='explicitly refresh the baseline for review')
    parser.add_argument('--doc-root', type=Path, help='inspect already built docs instead of invoking Cargo')
    args = parser.parse_args()
    manifest = tomllib.loads((ROOT/'Cargo.toml').read_text())
    views = {}
    if args.doc_root is not None:
        if args.write:
            parser.exit(2, 'Writing the baseline requires fresh default and all-feature builds.\n')
        views['all'] = snapshot(args.doc_root, manifest)
    else:
        # Isolate documentation from workspace/dev-dependency feature unification.
        target = ROOT/'target/alpha-api'
        env = dict(os.environ, CARGO_TARGET_DIR=str(target), RUSTDOCFLAGS='-D warnings')
        for view, feature_args in [('default', ['--no-default-features']), ('all', ['--all-features'])]:
            subprocess.run(['cargo', 'doc', '--locked', '-p', 'virtualgamepad', '--no-deps',
                            *feature_args], cwd=ROOT, env=env, check=True)
            views[view] = snapshot(target/'doc/virtualgamepad', manifest)
    current = {'schema': 2, 'views': views}
    if args.write:
        BASELINE.write_text(json.dumps(current, indent=2, sort_keys=True) + '\n')
    elif any(json.loads(BASELINE.read_text())['views'].get(view) != data for view, data in views.items()):
        parser.exit(1, 'Alpha root API changed. Review necessity and migration notes before refreshing the snapshot.\n')
    else:
        print(f'Alpha root API matches the reviewed snapshot ({len(views["all"]["items"])} items).')


if __name__ == '__main__':
    main()
