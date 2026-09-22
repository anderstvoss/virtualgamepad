#!/usr/bin/env python3
"""Unprivileged ALSA duplex checks for the explicitly attached VHCI lab worker.

Resolve the emulated card through current USB ancestry on every trial. Read only
synthetic microphone data and retain only aggregate results, never recordings.
Successful playback submission does not establish worker-side loss acceptance.
"""
import argparse
import concurrent.futures
import json
from pathlib import Path
import re
import struct
import subprocess
import time

PROFILES = {
    'dualsense': ('Virtualgamepad DualSense emulated audio', 4, 2),
    'dualshock4': ('Virtualgamepad DS4 emulated audio', 2, 1),
    'xbox360': ('Virtualgamepad Xbox 360 HID emulated audio', 2, 1),
}


def active_device(status, port):
    for line in status.splitlines()[1:]:
        fields = line.split()
        if len(fields) != 7:
            raise ValueError('malformed VHCI inventory')
        if int(fields[1]) == port:
            if fields[:1] != ['hs'] or fields[2:4] != ['006', '003']:
                raise ValueError('selected port has no active high-speed lab session')
            if not re.fullmatch(r'[1-9][0-9]*-[1-9][0-9]*', fields[6]):
                raise ValueError('unexpected virtual root-port device ancestry')
            return int(fields[4],16), fields[6]
    raise ValueError('selected VHCI port not found')


def resolve_card(port, family, previous=None):
    ownership = active_device(Path('/sys/devices/platform/vhci_hcd.0/status').read_text(), port)
    if previous is not None and ownership != previous:
        raise ValueError('lab session changed between trials')
    device = (Path('/sys/bus/usb/devices') / ownership[1]).resolve(strict=True)
    if 'vhci_hcd.0' not in device.parts:
        raise ValueError('selected device does not belong to VHCI')
    if (device/'manufacturer').read_text().strip() != 'Virtualgamepad' or \
            (device/'product').read_text().strip() != PROFILES[family][0]:
        raise ValueError('selected device is not the requested compiled emulation')
    cards = [card for card in Path('/sys/class/sound').glob('card[0-9]*')
             if (card/'device').resolve().is_relative_to(device)]
    if len(cards) != 1:
        raise ValueError('expected exactly one associated ALSA card')
    return ownership, int(cards[0].name[4:])


def inspect_capture(data, channels):
    if len(data) % (channels*2):
        raise ValueError('partial captured PCM frame')
    total = valid = silence = gaps = 0
    previous = None
    first_unexpected = last_unexpected = None
    for frame in struct.iter_unpack('<'+'h'*channels,data):
        first = frame[0]
        matches = 100 <= first <= 196 and all(value-first == 100*c for c,value in enumerate(frame))
        total += 1
        if not matches:
            if first_unexpected is None:
                first_unexpected = total - 1
            last_unexpected = total - 1
        valid += matches
        silence += all(value == 0 for value in frame)
        if previous is not None and matches:
            gaps += first-100 != (previous-100+1)%97
        previous = first if matches else None
    return dict(frames=total, exact_pattern=valid, silence=silence, pattern_gaps=gaps,
                first_unexpected_frame=first_unexpected,last_unexpected_frame=last_unexpected)


def run_trial(card, family, seconds):
    _, channels, microphones = PROFILES[family]
    warmup_seconds = 2
    total_seconds = seconds + warmup_seconds
    data = struct.pack('<'+'h'*channels,*[101,-202,303,-404][:channels]) * (48000*total_seconds)
    common = ['-q','-D',f'hw:{card},0','-t','raw','-f','S16_LE','-r','48000','-c']
    started = time.monotonic()
    with concurrent.futures.ThreadPoolExecutor(2) as pool:
        playback = pool.submit(subprocess.run,['aplay',*common,str(channels)],input=data,
                               stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=total_seconds+8)
        capture = pool.submit(subprocess.run,['arecord',*common,str(microphones),'-d',str(total_seconds)],
                              stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=total_seconds+8)
        output, incoming = playback.result(), capture.result()
    warmup_bytes = 48000 * warmup_seconds * microphones * 2
    result = inspect_capture(incoming.stdout[warmup_bytes:],microphones)
    result['warmup'] = inspect_capture(incoming.stdout[:warmup_bytes],microphones)
    result['warmup_seconds'] = warmup_seconds
    result['captured_total_frames'] = len(incoming.stdout) // (microphones * 2)
    result.update(command_interval_seconds=time.monotonic()-started,
                  playback_status=output.returncode,capture_status=incoming.returncode,
                  playback_errors=output.stderr.decode(errors='replace'),
                  capture_errors=incoming.stderr.decode(errors='replace'))
    passed = (output.returncode == incoming.returncode == 0 and not output.stderr and not incoming.stderr
              and result['frames'] == result['exact_pattern'] == 48000*seconds
              and result['captured_total_frames'] == 48000*total_seconds
              and result['silence'] == result['pattern_gaps'] == 0)
    result['passed'] = passed
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port',type=int,required=True)
    parser.add_argument('--profile',choices=PROFILES,required=True)
    parser.add_argument('--seconds',type=int,default=60)
    parser.add_argument('--trials',type=int,default=3)
    args = parser.parse_args()
    if not 1 <= args.seconds <= 60 or not 1 <= args.trials <= 3 or args.port < 0:
        parser.error('seconds must be 1..60, trials 1..3, and port nonnegative')
    ownership = None
    for trial in range(args.trials):
        ownership, card = resolve_card(args.port,args.profile,ownership)
        print(json.dumps(dict(status='running',profile=args.profile,trial=trial,
                              duration_seconds=args.seconds)),flush=True)
        result = run_trial(card,args.profile,args.seconds)
        result.update(trial=trial,profile=args.profile)
        # Preserve both the PCM failure and the device-lifecycle observation.
        # A disconnected session must not replace the useful result with a traceback.
        try:
            resolve_card(args.port,args.profile,ownership)
            result['session_present_after_trial'] = True
        except (ValueError, OSError) as error:
            result['session_present_after_trial'] = False
            result['session_error'] = str(error)
            result['passed'] = False
        print(json.dumps(result,sort_keys=True),flush=True)
        if not result['passed']:
            return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
