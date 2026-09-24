#!/usr/bin/env python3
"""Reference-device capture levels; retain no PCM or recordings.

Run only after the maintainer confirms readiness for an interactive microphone
observation. Comparison modes temporarily change only ALSA capture gain and
restore its original value after capture. Desktop routing is untouched.
"""
import argparse
import json
import math
from pathlib import Path
import re
import struct
import subprocess


def physical_card(usb_root=Path('/sys/bus/usb/devices'), sound_root=Path('/sys/class/sound')):
    devices = []
    for device in usb_root.iterdir():
        try:
            if (device/'idVendor').read_text().strip() == '054c' and \
               (device/'idProduct').read_text().strip() == '0ce6':
                resolved = device.resolve(strict=True)
                # A virtual USB/IP DualSense can advertise the same identity.
                # Never use a VHCI device as the physical reference.
                if not any(part.startswith('vhci_hcd') for part in resolved.parts):
                    devices.append(resolved)
        except (FileNotFoundError, OSError):
            continue
    if len(devices) != 1:
        raise ValueError('expected exactly one attached physical DualSense')
    cards = []
    for card in sound_root.glob('card[0-9]*'):
        try:
            if (card/'device').resolve(strict=True).is_relative_to(devices[0]):
                cards.append(int(card.name[4:]))
        except (FileNotFoundError, OSError):
            continue
    if len(cards) != 1:
        raise ValueError('expected exactly one ALSA card beneath the physical DualSense')
    return devices[0],cards[0]


def levels(pcm):
    if len(pcm) % 4:
        raise ValueError('incomplete stereo S16_LE capture frame')
    count = len(pcm)//4
    sums = [0,0]
    peaks = [0,0]
    for left,right in struct.iter_unpack('<hh',pcm):
        for index,value in enumerate((left,right)):
            sums[index] += value*value
            peaks[index] = max(peaks[index],abs(value))
    return dict(frames=count,
                rms=[round((total/count)**0.5,2) if count else 0 for total in sums],
                peak=peaks)


def capture(seconds):
    device,card = physical_card()
    command = ['arecord','-q','-D',f'hw:{card},0','-t','raw','-f','S16_LE',
               '-r','48000','-c','2','-d',str(seconds)]
    result = subprocess.run(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                            timeout=seconds+5,check=False)
    if result.returncode or result.stderr:
        raise RuntimeError('physical DualSense capture failed: '+result.stderr.decode(errors='replace')[:256])
    if physical_card() != (device,card):
        raise RuntimeError('physical DualSense changed during capture')
    expected = seconds*48000
    observation = levels(result.stdout)
    if observation['frames'] != expected:
        raise RuntimeError('incomplete physical DualSense capture')
    # The only retained data is this aggregate; the raw bytes never reach a file.
    return observation


def capture_gain(card):
    result = subprocess.run(['amixer', '-c', str(card), 'cget',
                             'name=Headset Capture Volume'], capture_output=True,
                            text=True, timeout=3, check=True)
    details = re.search(r'; type=INTEGER,[^\n]*values=(\d+),min=(\d+),max=(\d+)',
                        result.stdout)
    value = re.search(r'^\s*: values=(\d+)\s*$', result.stdout, re.MULTILINE)
    if not details or not value or details.group(1) != '1':
        raise RuntimeError('expected one scalar Headset Capture Volume control')
    minimum, maximum = map(int, details.group(2, 3))
    current = int(value.group(1))
    if not minimum <= current <= maximum or minimum == maximum:
        raise RuntimeError('invalid Headset Capture Volume range/value')
    return current, minimum, maximum


def set_capture_gain(device, card, value):
    if physical_card() != (device, card):
        raise RuntimeError('physical DualSense changed before gain write')
    subprocess.run(['amixer', '-c', str(card), 'cset',
                    'name=Headset Capture Volume', str(value)],
                   capture_output=True, text=True, timeout=3, check=True)
    if capture_gain(card)[0] != value:
        raise RuntimeError('Headset Capture Volume did not reach requested value')


def play_cue(device, card):
    if physical_card() != (device, card):
        raise RuntimeError('physical DualSense changed before speech cue')
    # Quiet, half-second tone in the left headset channel; haptics stay zero.
    pcm = b''.join(struct.pack('<hhhh',
                               round(1000*math.sin(2*math.pi*880*i/48000)),
                               0, 0, 0) for i in range(24000))
    subprocess.run(['aplay', '-q', '-D', f'hw:{card},0', '-t', 'raw',
                    '-f', 'S16_LE', '-r', '48000', '-c', '4'],
                   input=pcm, timeout=4, check=True)


def compare_gain(seconds):
    device, card = physical_card()
    original, _, maximum = capture_gain(card)
    print(json.dumps(dict(status='baseline', seconds=seconds,
                          gain=original)), flush=True)
    baseline = capture(seconds)
    if physical_card() != (device, card):
        raise RuntimeError('physical DualSense changed after baseline capture')
    print(json.dumps(dict(status='maximum', seconds=seconds,
                          gain=maximum)), flush=True)
    try:
        set_capture_gain(device, card, maximum)
        boosted = capture(seconds)
    finally:
        # Never write by remembered card number if the physical device changed.
        set_capture_gain(device, card, original)
    return dict(original_gain=original, maximum_gain=maximum,
                baseline=baseline, maximum=boosted, restored_gain=capture_gain(card)[0])


def compare_signal(seconds):
    device, card = physical_card()
    original, _, maximum = capture_gain(card)
    try:
        set_capture_gain(device, card, maximum)
        print(json.dumps(dict(status='quiet', seconds=seconds,
                              gain=maximum)), flush=True)
        quiet = capture(seconds)
        print(json.dumps(dict(status='cue', seconds=0.5)), flush=True)
        play_cue(device, card)
        print(json.dumps(dict(status='speak', seconds=seconds,
                              gain=maximum)), flush=True)
        speech = capture(seconds)
    finally:
        set_capture_gain(device, card, original)
    return dict(original_gain=original, comparison_gain=maximum,
                quiet=quiet, speech=speech, restored_gain=capture_gain(card)[0])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seconds',type=int,default=3)
    parser.add_argument('--capture',action='store_true',help='start bounded microphone capture')
    parser.add_argument('--compare-gain',action='store_true',
                        help='capture at current and maximum gain, restoring the original value')
    parser.add_argument('--compare-signal',action='store_true',
                        help='compare quiet and speech at maximum gain, restoring original value')
    args = parser.parse_args()
    if not 1 <= args.seconds <= 3: parser.error('seconds must be 1..3')
    if sum([args.capture, args.compare_gain, args.compare_signal]) > 1:
        parser.error('choose one capture mode')
    _,card = physical_card()
    if not args.capture:
        if args.compare_gain:
            print(json.dumps(dict(status='complete', **compare_gain(args.seconds))))
            return
        if args.compare_signal:
            print(json.dumps(dict(status='complete', **compare_signal(args.seconds))))
            return
        print(json.dumps(dict(status='ready',alsa_card=card,seconds=args.seconds)))
        return
    print(json.dumps(dict(status='capturing',seconds=args.seconds)),flush=True)
    print(json.dumps(dict(status='complete',**capture(args.seconds))))


if __name__ == '__main__': main()
