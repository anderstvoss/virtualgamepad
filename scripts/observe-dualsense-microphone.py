#!/usr/bin/env python3
"""Read-only reference-device capture levels; retain no PCM or recordings.

Run only after the maintainer confirms readiness for an interactive microphone
observation. This script never changes mixer controls or desktop routing.
"""
import argparse
import json
from pathlib import Path
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--seconds',type=int,default=3)
    parser.add_argument('--capture',action='store_true',help='start bounded microphone capture')
    args = parser.parse_args()
    if not 1 <= args.seconds <= 3: parser.error('seconds must be 1..3')
    _,card = physical_card()
    if not args.capture:
        print(json.dumps(dict(status='ready',alsa_card=card,seconds=args.seconds)))
        return
    print(json.dumps(dict(status='capturing',seconds=args.seconds)),flush=True)
    print(json.dumps(dict(status='complete',**capture(args.seconds))))


if __name__ == '__main__': main()
