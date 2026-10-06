#!/usr/bin/env python3
"""Check Cargo package/example and Python script references in provider validation."""
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def validate(text, packages, root):
    names = {package['name']: package for package in packages}
    commands = 0
    for line in text.splitlines():
        if re.search(r'\bcargo (?:test|build|check)\b', line):
            commands += 1
            selected = re.findall(r'(?:-p|--package)\s+([\w-]+)', line)
            if not selected or any(name not in names for name in selected):
                raise ValueError(f'Unknown or missing workspace package: {line.strip()}')
            example = re.search(r'--example\s+([\w-]+)', line)
            if example and not any(any(target['name'] == example[1] and 'example' in target['kind']
                                       for target in names[name]['targets']) for name in selected):
                raise ValueError(f'Unknown example: {line.strip()}')
        for script in re.findall(r'\bpython3\s+(scripts/[\w-]+\.py)', line):
            commands += 1
            if not (root / script).is_file():
                raise ValueError(f'Missing validator: {script}')
    if commands == 0:
        raise ValueError('No maintained validation commands found')
    return commands


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--workflow', type=Path, default=ROOT / '.github/workflows/provider-tier-b.yml')
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'], cwd=ROOT))
    packages = [package for package in metadata['packages'] if package['id'] in metadata['workspace_members']]
    count = validate(args.workflow.read_text(), packages, ROOT)
    print(f'{count} provider validation command references match workspace metadata and files')


if __name__ == '__main__':
    main()
