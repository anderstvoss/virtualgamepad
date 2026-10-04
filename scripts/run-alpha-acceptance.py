#!/usr/bin/env python3
"""Prebuild an exact clean candidate and qualify/run its audio acceptance matrix.

Use a new report directory outside the checkout. --prepare-only creates the
build receipt and external-tester source bundle. --native rejects virtual hosts.
Privileged, physical and manual GUI acceptance remain separate handoff tasks.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys

spec = importlib.util.spec_from_file_location('lab', Path(__file__).with_name('run-pipewire-audio-lab.py'))
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)

FAMILIES = ('dualsense', 'dualshock4', 'xbox360')
DIRECTIONS = ('latency_graph_source_to_library_samples', 'latency_graph_library_microphone',
              'latency_graph_native_playback', 'latency_graph_native_microphone')
OWNERSHIPS = ('samples-samples', 'samples-native', 'native-samples', 'native-native')


def matrix():
    return [dict(family=f, test=t, ownership=o, trial=i)
            for f in FAMILIES
            for t, o in [(t, None) for t in DIRECTIONS] + [('latency_graph_duplex', o) for o in OWNERSHIPS]
            for i in range(1, 4)]


def digest(path):
    with path.open('rb') as source:
        result = hashlib.sha256()
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            result.update(chunk)
        return result.hexdigest()


def ensure_candidate(root, revision):
    if (subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip() != revision or
            subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip()):
        raise RuntimeError('candidate changed during preparation; build evidence is invalid')


def private_test(command, logfile, timeout, env=None):
    inherited = os.environ.copy()
    try:
        os.environ.update(env or {})
        # The supervisor owns process groups and graph cleanup. The extra child
        # only captures output and propagates status; it performs no graph I/O.
        capture = [sys.executable, '-c',
                   'import subprocess,sys; f=open(sys.argv[1],"w"); r=subprocess.run(sys.argv[2:],stdout=f,stderr=subprocess.STDOUT); f.close(); sys.exit(r.returncode)',
                   str(logfile), *command]
        return lab.run(capture, timeout)
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        logfile.write_text(str(error) + '\n')
        return 1
    finally:
        os.environ.clear()
        os.environ.update(inherited)


def prepare(root, report):
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=root, text=True).strip():
        raise RuntimeError('candidate checkout must be clean, including untracked source')
    tree = subprocess.check_output(['git', 'rev-parse', 'HEAD^{tree}'], cwd=root, text=True).strip()
    command = ['cargo', 'build', '--locked', '--workspace', '--all-targets', '--all-features', '--message-format=json']
    with (report / 'build.jsonl').open('w') as stdout, (report / 'build.log').open('w') as stderr:
        subprocess.run(command, cwd=root, stdout=stdout, stderr=stderr, check=True)
    binaries = {}
    for line in (report / 'build.jsonl').read_text().splitlines():
        event = json.loads(line)
        if event.get('reason') == 'compiler-artifact' and event.get('executable'):
            name = event['target']['name']
            if name in ('pipewire_live', 'gr-privileged-broker', 'gr-audio-worker', 'gui_soak', 'usb_audio_probe'):
                # Do not select a binary's unit-test harness as the installed daemon.
                if event['profile']['test'] and name != 'pipewire_live':
                    continue
                binaries[name] = Path(event['executable'])
    if 'pipewire_live' not in binaries:
        raise RuntimeError('Cargo did not produce the PipeWire test binary')
    control = report / 'alpha-audio-control'
    flags = shlex.split(subprocess.check_output(['pkg-config', '--cflags', '--libs', 'libpipewire-0.3'], text=True))
    subprocess.run(['cc', '-Wall', '-Wextra', '-Werror', str(root / 'scripts/alpha-audio-control.c'),
                    '-o', str(control), *flags], check=True)
    binaries['independent-control'] = control
    subprocess.run([str(control), '--self-test'], check=True)
    ensure_candidate(root, revision)
    receipt = dict(revision=revision, tree=tree, lockfile_sha256=digest(root / 'Cargo.lock'),
                   compiler=subprocess.check_output(['rustc', '-vV'], text=True), build_command=command,
                   c_compiler=subprocess.check_output(['cc', '--version'], text=True).splitlines()[0],
                   c_flags=['-Wall', '-Wextra', '-Werror', *flags],
                   build_environment={k: os.environ[k] for k in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
                       'CARGO_BUILD_TARGET', 'RUSTUP_TOOLCHAIN') if k in os.environ},
                   binaries={name: dict(path=str(path), sha256=digest(path)) for name, path in binaries.items()})
    (report / 'build-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    subprocess.run(['git', 'archive', '--format=tar', '--output=' + str(report / 'candidate-source.tar'), revision],
                   cwd=root, check=True)
    return receipt, binaries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--report-dir', type=Path, required=True)
    parser.add_argument('--prepare-only', action='store_true')
    parser.add_argument('--native', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    report = args.report_dir.resolve()
    if report == root or root in report.parents:
        parser.error('report directory must be outside the checkout')
    report.mkdir(mode=0o700)  # Refuse accidental reuse/overwrite of evidence.
    receipt, binaries = prepare(root, report)
    cells = matrix()
    results = dict(revision=receipt['revision'], qualified=False, cells=cells)
    if args.prepare_only:
        results['status'] = 'prepared; acceptance unperformed'
        (report / 'acceptance.json').write_text(json.dumps(results, indent=2) + '\n')
        return 0
    virt = subprocess.run(['systemd-detect-virt'], capture_output=True, text=True)
    results['virtualization'] = dict(status=virt.returncode, result=virt.stdout.strip())
    if args.native and not (virt.returncode == 1 and virt.stdout.strip() == 'none'):
        results['status'] = 'blocked: native-host identity not established'
        (report / 'acceptance.json').write_text(json.dumps(results, indent=2) + '\n')
        return 1
    command = [sys.executable, str(root / 'scripts/run-alpha-audio-control.py'), '--control',
               str(binaries['independent-control']), '--report', str(report / 'independent.json')]
    c_status = subprocess.run(command, cwd=root).returncode
    direct = []
    for trial in range(1, 4):
        direct.append(private_test([str(binaries['pipewire_live']), 'latency_graph_direct_control',
                                   '--ignored', '--exact', '--nocapture'], report / f'direct-{trial}.log', 90,
                                  dict(VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS='62')))
    results['controls'] = dict(independent_status=c_status, direct_statuses=direct)
    results['qualified'] = c_status == 0 and direct == [0, 0, 0]
    if results['qualified']:
        for index, cell in enumerate(cells):
            env = dict(VIRTUALGAMEPAD_AUDIO_FAMILY=cell['family'], VIRTUALGAMEPAD_AUDIO_TRIAL_SECONDS='62')
            if cell['ownership'] is not None:
                env['VIRTUALGAMEPAD_AUDIO_OWNERSHIP'] = cell['ownership']
            cell['status'] = private_test([str(binaries['pipewire_live']), cell['test'], '--ignored', '--exact',
                                          '--nocapture'], report / f'cell-{index + 1}.log', 90, env)
        results['slow_consumers'] = {family: private_test([str(binaries['pipewire_live']), 'soak_' + family,
            '--ignored', '--exact', '--nocapture'], report / ('slow-' + family + '.log'), 240,
            dict(VIRTUALGAMEPAD_AUDIO_FAMILY=family)) for family in FAMILIES}
    else:
        for cell in cells:
            cell['status'] = 'blocked: graph controls failed'
    results['status'] = 'passed' if (results['qualified'] and all(c['status'] == 0 for c in cells)
        and all(s == 0 for s in results['slow_consumers'].values())) else 'failed or blocked'
    (report / 'acceptance.json').write_text(json.dumps(results, indent=2) + '\n')
    return 0 if results['status'] == 'passed' else 1


if __name__ == '__main__':
    sys.exit(main())
