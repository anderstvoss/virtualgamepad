#!/usr/bin/env python3
"""Collect one identity-checked SBOM for every Cargo workspace package."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess


def collect(packages, output):
    reports = []
    names = set()
    for package in packages:
        name = package['name']
        if name in names:
            raise ValueError(f'duplicate workspace package: {name}')
        names.add(name)
        source = Path(package['manifest_path']).parent / 'bom.json'
        document = json.loads(source.read_text())
        component = document.get('metadata', {}).get('component', {})
        if component.get('name') != name or component.get('version') != package['version']:
            raise ValueError(f'SBOM component identity mismatch: {name}')
        if package['dependencies'] and not document.get('components'):
            raise ValueError(f'SBOM dependencies missing: {name}')
        reports.append((source, output / f'{name}.bom.json'))
    if not reports:
        raise ValueError('no workspace packages')
    # Validate the whole set before producing an uploadable partial artifact.
    if output.exists() and any(output.iterdir()):
        raise ValueError('SBOM output must be empty to exclude stale reports')
    output.mkdir(parents=True, exist_ok=True)
    for source, target in reports:
        shutil.copyfile(source, target)
    return names


def verify(packages, directory):
    expected = {f"{package['name']}.bom.json": package for package in packages}
    if len(expected) != len(packages) or not expected:
        raise ValueError('duplicate or empty workspace package set')
    actual = {path.name for path in directory.iterdir()}
    if actual != set(expected):
        raise ValueError(f'Artifact package set mismatch: missing={set(expected)-actual}, extra={actual-set(expected)}')
    for filename, package in expected.items():
        document = json.loads((directory / filename).read_text())
        component = document.get('metadata', {}).get('component', {})
        if component.get('name') != package['name'] or component.get('version') != package['version']:
            raise ValueError(f'Artifact component identity mismatch: {filename}')
        if package['dependencies'] and not document.get('components'):
            raise ValueError(f'Artifact dependencies missing: {filename}')
    return set(expected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--output', type=Path)
    group.add_argument('--verify', type=Path, help='verify an extracted uploaded artifact')
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'], text=True))
    members = set(metadata['workspace_members'])
    packages = [p for p in metadata['packages'] if p['id'] in members]
    names = verify(packages, args.verify) if args.verify else collect(packages, args.output)
    print(f'Validated {len(names)} workspace SBOMs')


if __name__ == '__main__':
    main()
