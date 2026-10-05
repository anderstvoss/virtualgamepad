#!/usr/bin/env python3
"""Run independent C continuity controls in fresh private PipeWire graphs.

Build alpha-audio-control.c before invoking this script. Reports belong outside
tracked source. This checks continuity, not native timing or product acceptance.
"""
import argparse
import importlib.util
import json
import os
import stat
from pathlib import Path
import subprocess
import sys
import time

# A clean candidate must not become dirty merely by loading its lab helper.
sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location('private_lab', Path(__file__).with_name('run-pipewire-audio-lab.py'))
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)


def acceptance(result, seconds):
    expected = seconds * 48000
    elapsed = result.get('producer_elapsed_ns')
    # Match the existing steady-state soak's 1% rate envelope. Complete marker
    # totals from a fast/freewheeling graph do not establish a sustained trial.
    if type(elapsed) is not int or not seconds * 990_000_000 <= elapsed <= seconds * 1_010_000_000:
        return False
    for key in ('planned', 'generated', 'graph_submitted', 'graph_received'):
        if type(result.get(key)) is not int or result[key] != expected:
            return False
    return all(type(result.get(key)) is int and result[key] == 0
               for key in ('missing', 'duplicate', 'invalid', 'partial_bytes', 'errors')) and all(
                   result.get(key) == value for key, value in
                   (('source_rate', 48000), ('sink_rate', 48000), ('source_channels', 2), ('sink_channels', 2)))


def inside(control, seconds):
    root = Path(os.environ.get('PIPEWIRE_RUNTIME_DIR', '/nonexistent'))
    metadata = root.lstat()
    if (not root.name.startswith('virtualgamepad-pw-lab-') or
            not stat.S_ISDIR(metadata.st_mode) or metadata.st_mode & 0o077 or
            os.environ.get('PIPEWIRE_REMOTE') != 'pipewire-0' or
            os.environ.get('XDG_RUNTIME_DIR') != str(root) or
            metadata.st_uid != os.getuid()):
        raise RuntimeError('independent control requires the owned private lab')
    name = 'alpha-independent-' + str(os.getpid())
    loop = subprocess.Popen(['pw-loopback', '-n', name, '-c', '2', '-m', '[ FL FR ]',
        '-i', f'{{ node.name = "{name}.sink" media.class = "Audio/Sink" node.virtual = true node.autoconnect = false }}',
        '-o', f'{{ node.name = "{name}.source" media.class = "Audio/Source" node.virtual = true node.autoconnect = false }}'],
        start_new_session=False)
    try:
        deadline = time.monotonic() + 5
        while True:
            nodes = json.loads(subprocess.check_output(['pw-dump'], timeout=2))
            names = {n.get('info', {}).get('props', {}).get('node.name') for n in nodes}
            if {name + '.sink', name + '.source'} <= names:
                break
            if loop.poll() is not None or time.monotonic() >= deadline:
                raise TimeoutError('owned loopback nodes did not become ready')
            time.sleep(.02)
        result = subprocess.run([str(control), str(seconds), name + '.sink', name + '.source'],
                                capture_output=True, text=True, timeout=seconds + 18)
        # One fixed-size JSON receipt, never arbitrary recordings or a stdin count.
        if len(result.stdout) > 16384:
            raise RuntimeError('oversized control receipt')
        receipt = json.loads(result.stdout)
        receipt['exit_status'] = result.returncode
        receipt['accepted'] = result.returncode == 0 and acceptance(receipt, seconds)
        print(json.dumps(receipt), flush=True)
        return 0 if receipt['accepted'] else 1
    finally:
        # Keep this child in the outer supervisor's process group so a killed
        # test cannot orphan a separate loopback group. Normal exit reaps it.
        if loop.poll() is None:
            loop.terminate()
            try:
                loop.wait(timeout=2)
            except subprocess.TimeoutExpired:
                loop.kill()
                loop.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--control', type=Path, required=True)
    parser.add_argument('--seconds', type=int, choices=range(1, 61), default=60)
    parser.add_argument('--trials', type=int, choices=range(1, 4), default=3)
    parser.add_argument('--report', type=Path)
    parser.add_argument('--inside-lab', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    control = args.control.resolve(strict=True)
    if args.inside_lab:
        return inside(control, args.seconds)
    if args.report is None:
        parser.error('--report is required outside the private child')
    rows = []
    for trial in range(args.trials):
        # Capture each child receipt through a temporary file in the report's
        # directory, keeping graph/process teardown in the existing supervisor.
        import tempfile
        with tempfile.TemporaryDirectory(prefix='alpha-control-report-') as directory:
            receipt = Path(directory) / 'receipt.json'
            child = [sys.executable, str(Path(__file__).resolve()), '--control', str(control),
                     '--seconds', str(args.seconds), '--inside-lab']
            command = [sys.executable, '-c',
                       'import subprocess,sys; f=open(sys.argv[1],"w"); r=subprocess.run(sys.argv[2:],stdout=f); f.close(); sys.exit(r.returncode)',
                       str(receipt), *child]
            try:
                status = lab.run(command, args.seconds + 30)
                row = json.loads(receipt.read_text())
                row['supervisor_status'] = status
            except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
                row = dict(accepted=False, error=str(error))
            row['trial'] = trial + 1
            rows.append(row)
            print(json.dumps(row), flush=True)
    qualified = args.seconds == 60 and len(rows) == 3 and all(r.get('accepted') for r in rows)
    args.report.write_text(json.dumps(dict(qualified=qualified, seconds=args.seconds, trials=rows), indent=2) + '\n')
    return 0 if all(r.get('accepted') for r in rows) else 1


if __name__ == '__main__':
    sys.exit(main())
