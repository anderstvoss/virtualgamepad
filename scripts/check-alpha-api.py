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
BASELINE = ROOT/'docs/api/ALPHA_API_SNAPSHOT.json'
WII_BASELINE = ROOT/'docs/api/WII_REMOTE_API_SNAPSHOT.json'


def snapshot(doc_root, manifest):
    page = inventory.parse((doc_root/'index.html').read_text())
    modules = page.modules | {link for link in page.links if link.startswith('mod.')}
    unknown_modules = sorted(modules - {'experimental/index.html', 'mod.experimental.html'})
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


def wii_snapshot(doc_root):
    """Review the Wii supporting API separately from the ordinary root promise."""
    compiler_traits = ('Freeze', 'UnsafeUnpin', 'StructuralPartialEq')
    return {
        'schema': 1,
        'scope': 'gr-curated-controllers Wii supporting API; not ordinary root',
        'items': {
            name: {'declarations': declarations,
                   'traits': sorted(trait for trait in traits
                                    if not any(f' {kind} for ' in trait
                                               for kind in compiler_traits))}
            for name, (declarations, traits) in inventory.public_surface(doc_root).items()
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true', help='explicitly refresh the baseline for review')
    parser.add_argument('--doc-root', type=Path, help='inspect already built docs instead of invoking Cargo')
    parser.add_argument('--wii-doc-root', type=Path,
                        help='Wii supporting docs accompanying --doc-root; otherwise inferred')
    args = parser.parse_args()
    if args.wii_doc_root is not None and args.doc_root is None:
        parser.error('--wii-doc-root requires --doc-root')
    manifest = tomllib.loads((ROOT/'Cargo.toml').read_text())
    views = {}
    if args.doc_root is not None:
        if args.write:
            parser.exit(2, 'Writing the baseline requires fresh default and all-feature builds.\n')
        views['all'] = snapshot(args.doc_root, manifest)
        wii = wii_snapshot(args.wii_doc_root or
                           args.doc_root.parent/'gr_curated_controllers/wii_remote')
    else:
        # Isolate documentation from workspace/dev-dependency feature unification.
        target = ROOT/'target/alpha-api'
        env = dict(os.environ, CARGO_TARGET_DIR=str(target), RUSTDOCFLAGS='-D warnings')
        for view, feature_args in [('default', ['--no-default-features']), ('all', ['--all-features'])]:
            subprocess.run(['cargo', 'doc', '--locked', '-p', 'virtualgamepad', '--no-deps',
                            *feature_args], cwd=ROOT, env=env, check=True)
            views[view] = snapshot(target/'doc/virtualgamepad', manifest)
        subprocess.run(['cargo', 'doc', '--locked', '-p', 'gr-curated-controllers',
                        '--all-features', '--no-deps'], cwd=ROOT, env=env, check=True)
        wii = wii_snapshot(target/'doc/gr_curated_controllers/wii_remote')
    current = {'schema': 2, 'views': views}
    if args.write:
        BASELINE.write_text(json.dumps(current, indent=2, sort_keys=True) + '\n')
        WII_BASELINE.write_text(json.dumps(wii, indent=2, sort_keys=True) + '\n')
    elif any(json.loads(BASELINE.read_text())['views'].get(view) != data for view, data in views.items()):
        parser.exit(1, 'Alpha root API changed. Review necessity and migration notes before refreshing the snapshot.\n')
    elif not WII_BASELINE.exists() or json.loads(WII_BASELINE.read_text()) != wii:
        parser.exit(1, 'Wii supporting API changed. Review the imported-resource API before refreshing the snapshot.\n')
    else:
        print(f'Alpha root API matches the reviewed snapshot ({len(views["all"]["items"])} items).')
        print(f'Wii supporting API matches its separate reviewed snapshot ({len(wii["items"])} items).')


if __name__ == '__main__':
    main()
