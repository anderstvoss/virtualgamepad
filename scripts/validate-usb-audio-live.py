#!/usr/bin/env python3
"""Unprivileged ALSA duplex checks for the explicitly attached VHCI lab worker.

Resolve the emulated card through current USB ancestry on every trial. Read only
synthetic microphone data and retain only aggregate results, never recordings.
Successful playback submission does not establish worker-side loss acceptance.
"""
import argparse
import array
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


MARKER_BASE = 32767
MAX_CAPTURE_FRAMES = 3_200_000


def marker_frame(position, channels):
    """Indexed S16 markers: stereo frames or sign-tagged pairs for mono.

    Mono needs two frames to encode a complete index. Positive/negative tags
    prevent phase ambiguity; capture analysis carries pairs across window edges.
    """
    if (type(position) is not int or position < 0
            or type(channels) is not int or channels not in (1, 2, 4)):
        raise ValueError('invalid indexed marker position or channels')
    index = position // 2 if channels == 1 else position
    high, low = divmod(index, MARKER_BASE)
    if high >= MARKER_BASE: raise ValueError('indexed marker exhausted')
    high += 1; low += 1
    if channels == 1: return (high if position % 2 == 0 else -low,)
    return (high, low) if channels == 2 else (high, low, -high, -low)


def marker_pcm(frames, channels, start=0):
    if type(frames) is not int or not 0 <= frames <= MAX_CAPTURE_FRAMES:
        raise ValueError('bounded marker production required')
    marker_frame(start, channels)
    if frames: marker_frame(start+frames-1, channels)
    samples = array.array('h', (value for position in range(start, start+frames)
                              for value in marker_frame(position, channels)))
    if samples.itemsize != 2: raise ValueError('16-bit sample storage required')
    import sys
    if sys.byteorder != 'little': samples.byteswap()
    return samples.tobytes()


def decode_capture(data, channels):
    if type(channels) is not int or channels not in (1, 2, 4) or len(data) % (channels*2):
        raise ValueError('partial captured PCM frame or invalid channels')
    frames = len(data) // (channels*2)
    if frames > MAX_CAPTURE_FRAMES: raise ValueError('oversized captured PCM')
    valid = bytearray(frames)
    silence = bytearray(frames)
    gaps = bytearray(frames)
    positions = array.array('q', [-1]) * frames
    previous = None
    pending = None
    for offset, frame in enumerate(struct.iter_unpack('<'+'h'*channels, data)):
        if not any(frame): silence[offset] = 1
        if channels == 1:
            value = frame[0]
            if 1 <= value <= MARKER_BASE:
                # A repeated header leaves the earlier incomplete pair invalid.
                pending = (offset, value)
            elif -MARKER_BASE <= value <= -1 and pending is not None:
                header_offset, high = pending
                index = (high-1)*MARKER_BASE + (-value-1)
                valid[header_offset] = valid[offset] = 1
                positions[header_offset], positions[offset] = 2*index, 2*index+1
                if previous is not None and index != previous+1:
                    # Both halves carry the discontinuity, including a pair
                    # straddling warm-up, measurement or drain boundaries.
                    gaps[header_offset] = gaps[offset] = 1
                previous, pending = index, None
            else:
                pending = None
        else:
            high, low = frame[:2]
            if not (1 <= high <= MARKER_BASE and 1 <= low <= MARKER_BASE): continue
            index = (high-1)*MARKER_BASE + low-1
            if frame != marker_frame(index, channels): continue
            valid[offset] = 1
            positions[offset] = index
            if previous is not None and index != previous+1: gaps[offset] = 1
            previous = index
    return valid, silence, gaps, positions


def capture_summary(decoded, begin=0, end=None):
    valid, silence, gaps, positions = decoded
    end = len(valid) if end is None else end
    if not 0 <= begin <= end <= len(valid): raise ValueError('invalid capture window')
    first_bad = next((i for i in range(begin, end) if not valid[i]), None)
    last_bad = next((i for i in range(end-1, begin-1, -1) if not valid[i]), None)
    first_gap = next((i for i in range(begin, end) if gaps[i]), None)
    last_gap = next((i for i in range(end-1, begin-1, -1) if gaps[i]), None)
    relative = lambda value: None if value is None else value-begin
    return dict(frames=end-begin, exact_pattern=valid.count(1, begin, end),
                silence=silence.count(1, begin, end), pattern_gaps=gaps.count(1, begin, end),
                first_unexpected_frame=relative(first_bad), last_unexpected_frame=relative(last_bad),
                first_pattern_gap=relative(first_gap), last_pattern_gap=relative(last_gap),
                first_source_frame=next((positions[i] for i in range(begin, end) if valid[i]), None),
                last_source_frame=next((positions[i] for i in range(end-1, begin-1, -1) if valid[i]), None))


def inspect_capture(data, channels):
    return capture_summary(decode_capture(data, channels))


def run_trial(card, family, seconds):
    _, channels, microphones = PROFILES[family]
    warmup_seconds = 2
    edge_capture_seconds = 1
    total_seconds = seconds + warmup_seconds + edge_capture_seconds
    data = marker_pcm(48000*total_seconds, channels)
    common = ['-q','-D',f'hw:{card},0','-t','raw','-f','S16_LE','-r','48000','-c']
    started = time.monotonic()
    with concurrent.futures.ThreadPoolExecutor(2) as pool:
        playback = pool.submit(subprocess.run,['aplay',*common,str(channels)],input=data,
                               stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=total_seconds+8)
        capture = pool.submit(subprocess.run,['arecord',*common,str(microphones),'-d',str(total_seconds)],
                              stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=total_seconds+8)
        output, incoming = playback.result(), capture.result()
    decoded = decode_capture(incoming.stdout, microphones)
    warmup_frames, measured_frames = warmup_seconds*48000, seconds*48000
    measured_end = min(warmup_frames+measured_frames, len(decoded[0]))
    measured_begin = min(warmup_frames, measured_end)
    result = capture_summary(decoded, measured_begin, measured_end)
    result['warmup'] = capture_summary(decoded, 0, min(warmup_frames, len(decoded[0])))
    result['edge_capture'] = capture_summary(decoded, measured_end)
    result['edge_capture_seconds'] = edge_capture_seconds
    result['marker_scheme'] = 'indexed-s16-frame-or-mono-pair-v1'
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
