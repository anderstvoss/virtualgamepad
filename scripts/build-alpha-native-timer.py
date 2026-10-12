#!/usr/bin/python3
"""Prebuild the timer diagnostic before live trials; default reports actions only."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess


def build(output, repo):
    if os.geteuid() == 0:
        raise ValueError('ordinary-user prebuild required')
    def git(*args):
        return subprocess.check_output(['git', *args], cwd=repo, text=True).strip()
    if git('status', '--porcelain'):
        raise ValueError('clean committed candidate required')
    revision, tree = git('rev-parse', 'HEAD'), git('rev-parse', 'HEAD^{tree}')
    spec = importlib.util.spec_from_file_location('timer_source', repo / 'scripts/validate-broker-audio-live.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    compiler = shutil.which('cc')
    if compiler is None:
        raise ValueError('C compiler unavailable')
    output.mkdir(mode=0o700)
    source = output / 'native-timer.c'
    image = output / 'native-timer'
    source.write_text(module.NATIVE_TIMER_SOURCE)
    command = [compiler, '-O2', '-Wall', '-Wextra', '-Werror', '-pthread', str(source), '-o', str(image)]
    # No execution, scheduling changes or privileged state during preparation.
    subprocess.run(command, check=True, timeout=60)
    image.chmod(0o500)
    if git('rev-parse', 'HEAD') != revision or git('status', '--porcelain'):
        raise ValueError('candidate changed during prebuild')
    receipt = dict(revision=revision, tree=tree, command=command,
                   compiler=subprocess.check_output([compiler, '--version'], text=True, timeout=10),
                   source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
                   binary_sha256=hashlib.sha256(image.read_bytes()).hexdigest(),
                   scope='timer diagnostic only; no audio or native-host qualification')
    (output / 'build-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if not args.apply:
        print(json.dumps(dict(apply=False, actions=['compile diagnostic', 'record source and binary hashes'],
                              live_execution=False, privileged_changes=False)))
        return
    if args.output is None:
        parser.error('--output required')
    print(json.dumps(build(args.output, Path(__file__).resolve().parents[1])))


if __name__ == '__main__':
    main()
