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
import resource
import tempfile
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
    if not cards:
        raise FileNotFoundError('associated ALSA card not yet registered')
    if len(cards) != 1:
        raise ValueError('ambiguous associated ALSA cards')
    return ownership, int(cards[0].name[4:])


def owned_pipewire_device(objects, card):
    candidates = [item for item in objects if item.get('type') == 'PipeWire:Interface:Device'
                  and item.get('info',{}).get('props',{}).get('api.alsa.card') == card
                  and item.get('info',{}).get('props',{}).get('device.bus-path','').startswith('platform-vhci_hcd.0-usb-')]
    if len(candidates) > 1:
        raise ValueError('ambiguous owned PipeWire device')
    return candidates[0] if candidates else None


def compiled_serial(instance, generation):
    if (not re.fullmatch(r'[a-z0-9-]{1,32}', instance) or
            type(generation) is not int or not 0 < generation < 2**64):
        raise ValueError('invalid expected session identity')
    return f'vg-{instance}-{generation:016x}'


def shared_graph():
    # A shared graph inspection must be bounded and read-only. Spool rather
    # than accumulating unbounded stdout from another process in memory.
    limit = 1024 * 1024
    def quota():
        resource.setrlimit(resource.RLIMIT_FSIZE, (limit, limit))
    with tempfile.TemporaryFile() as output:
        result = subprocess.run(['pw-dump'], stdout=output, stderr=subprocess.DEVNULL,
                                timeout=3, preexec_fn=quota)
        if result.returncode:
            raise RuntimeError('shared PipeWire inspection unavailable or exceeded quota')
        output.seek(0)
        objects = json.loads(output.read(limit + 1))
    if not isinstance(objects, list) or any(not isinstance(item, dict) for item in objects):
        raise ValueError('invalid shared PipeWire graph')
    return objects


def shared_defaults(objects):
    values = []
    for item in objects:
        if (item.get('type') == 'PipeWire:Interface:Metadata' and
                item.get('props', {}).get('metadata.name') == 'default'):
            for entry in item.get('metadata', []):
                if entry.get('key', '').startswith(('default.audio.', 'default.configured.audio.')):
                    values.append(entry)
    return sorted(values, key=lambda entry: (entry.get('subject', 0), entry['key']))


def isolated_card(properties, objects, card, instance):
    if properties.get('ACP_IGNORE') != '1' or properties.get('VG_ALPHA_AUDIO_INSTANCE') != instance:
        raise ValueError('owned card lacks pre-attachment audio isolation')
    for item in objects:
        props = item.get('info', {}).get('props', {})
        if item.get('type') in ('PipeWire:Interface:Device', 'PipeWire:Interface:Node') and any(
                str(props.get(key, '')) == str(card)
                for key in ('api.alsa.card', 'api.alsa.pcm.card')):
            raise ValueError('shared session manager imported the owned card')


def reserve_direct_alsa(card, bus, instance, generation):
    """Verify pre-attachment exclusion; never change a shared device profile."""
    serial = compiled_serial(instance, generation)
    device = (Path('/sys/bus/usb/devices') / bus).resolve(strict=True)
    entry = Path('/sys/class/sound') / f'card{card}'
    if ('vhci_hcd.0' not in device.parts or
            not (entry / 'device').resolve().is_relative_to(device) or
            (device / 'serial').read_text().strip() != serial):
        raise ValueError('ALSA session identity or ancestry changed before verification')
    result = subprocess.run(['udevadm', 'info', '--query=property', '--path=' + str(entry)],
                            capture_output=True, check=True, timeout=3)
    if len(result.stdout) > 64 * 1024:
        raise ValueError('ALSA properties exceeded quota')
    properties = dict(line.split('=', 1) for line in result.stdout.decode().splitlines() if '=' in line)
    objects = shared_graph()
    isolated_card(properties, objects, card, instance)
    return dict(serial=serial, shared_defaults=shared_defaults(objects))


def inspect_capture(data, channels):
    if len(data) % (channels*2):
        raise ValueError('partial captured PCM frame')
    total = valid = silence = gaps = 0
    previous = None
    first_unexpected = last_unexpected = None
    first_pattern_gap = last_pattern_gap = None
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
        if previous is not None and matches and first-100 != (previous-100+1)%97:
            gaps += 1
            if first_pattern_gap is None:
                first_pattern_gap = total - 1
            last_pattern_gap = total - 1
        previous = first if matches else None
    return dict(frames=total, exact_pattern=valid, silence=silence, pattern_gaps=gaps,
                first_unexpected_frame=first_unexpected,last_unexpected_frame=last_unexpected,
                first_pattern_gap=first_pattern_gap,last_pattern_gap=last_pattern_gap)


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
    parser.add_argument('--instance', required=True)
    parser.add_argument('--generation', type=int, required=True)
    parser.add_argument('--seconds',type=int,default=60)
    parser.add_argument('--trials',type=int,default=3)
    parser.add_argument('--reserve-owned-card',action='store_true',
                        help='verify pre-attachment isolation without changing shared profiles')
    parser.add_argument('--prepare-only',action='store_true',
                        help='verify and reserve the owned virtual card without streaming')
    args = parser.parse_args()
    if not 1 <= args.seconds <= 60 or not 1 <= args.trials <= 3 or args.port < 0:
        parser.error('seconds must be 1..60, trials 1..3, and port nonnegative')
    if args.prepare_only:
        if not args.reserve_owned_card:
            parser.error('--prepare-only requires --reserve-owned-card')
        ownership, card = resolve_card(args.port,args.profile)
        reserve_direct_alsa(card,ownership[1],args.instance,args.generation)
        print(json.dumps(dict(profile=args.profile,card=card,port=args.port,
                              ownership=ownership[1])),flush=True)
        return 0
    ownership = None
    for trial in range(args.trials):
        ownership, card = resolve_card(args.port,args.profile,ownership)
        isolation = reserve_direct_alsa(card,ownership[1],args.instance,args.generation)
        print(json.dumps(dict(status='running',profile=args.profile,trial=trial,
                              duration_seconds=args.seconds)),flush=True)
        result = run_trial(card,args.profile,args.seconds)
        result.update(trial=trial,profile=args.profile)
        # Preserve both the PCM failure and the device-lifecycle observation.
        # A disconnected session must not replace the useful result with a traceback.
        try:
            resolve_card(args.port,args.profile,ownership)
            after = reserve_direct_alsa(card,ownership[1],args.instance,args.generation)
            result['shared_defaults_unchanged'] = after['shared_defaults'] == isolation['shared_defaults']
            result['passed'] &= result['shared_defaults_unchanged']
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
