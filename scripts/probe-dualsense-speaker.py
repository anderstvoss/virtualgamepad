#!/usr/bin/env python3
"""Bounded physical DualSense speaker probe; run only with a ready listener.

Uses the audio-only USB HID routing fields documented by the upstream Linux
hid-playstation audio-jack patch. Does not touch motors or adaptive triggers.
The route is restored to headphones even if playback fails. Speaker volume and
preamp have no readback on older kernels, so their prior values cannot be restored.
"""

import argparse
import math
import os
from pathlib import Path
import struct
import subprocess


def report(speaker):
    data = bytearray(63)
    data[0] = 0x02  # USB output report ID
    data[1] = 0xA0 if speaker else 0x80  # speaker volume + audio route / route
    data[8] = 0x30 if speaker else 0x00  # right source to speaker / stereo jack
    if speaker:
        data[2] = 0x80  # audio_control2 valid
        data[6] = 0x64  # speaker volume from upstream patch
        data[38] = 0x02  # speaker preamp from upstream patch
    return bytes(data)


def physical_devices(usb_root=Path('/sys/bus/usb/devices'),
                     sound_root=Path('/sys/class/sound'),
                     hidraw_root=Path('/sys/class/hidraw')):
    roots = []
    for entry in usb_root.iterdir():
        try:
            if ((entry/'idVendor').read_text().strip(),
                    (entry/'idProduct').read_text().strip()) == ('054c', '0ce6'):
                root = entry.resolve(strict=True)
                if not any(part.startswith('vhci_hcd') for part in root.parts):
                    roots.append(root)
        except (OSError, FileNotFoundError):
            pass
    if len(roots) != 1:
        raise RuntimeError('expected exactly one physical USB DualSense')
    root = roots[0]
    cards = []
    for entry in sound_root.glob('card[0-9]*'):
        try:
            if (entry/'device').resolve(strict=True).is_relative_to(root):
                cards.append(int(entry.name[4:]))
        except (OSError, FileNotFoundError):
            pass
    nodes = []
    for entry in hidraw_root.glob('hidraw[0-9]*'):
        try:
            if (entry/'device').resolve(strict=True).is_relative_to(root):
                nodes.append(Path('/dev')/entry.name)
        except (OSError, FileNotFoundError):
            pass
    if len(cards) != 1 or len(nodes) != 1:
        raise RuntimeError('physical DualSense must have one ALSA card and hidraw node')
    return root, cards[0], nodes[0]


def tone():
    # One second, right audible channel only; below 4% of full-scale PCM.
    return b''.join(struct.pack('<hhhh', 0,
                                round(1000*math.sin(2*math.pi*440*i/48000)),
                                0, 0) for i in range(48000))


def probe():
    root, card, hidraw = physical_devices()
    fd = os.open(hidraw, os.O_WRONLY | os.O_CLOEXEC)
    try:
        if physical_devices() != (root, card, hidraw):
            raise RuntimeError('physical DualSense changed before routing')
        try:
            if os.write(fd, report(True)) != 63:
                raise RuntimeError('speaker-route report was incomplete')
            subprocess.run(['aplay', '-q', '-D', f'hw:{card},0', '-t', 'raw',
                            '-f', 'S16_LE', '-r', '48000', '-c', '4'],
                           input=tone(), timeout=4, check=True)
        finally:
            if os.write(fd, report(False)) != 63:
                raise RuntimeError('headphones-route restore was incomplete')
    finally:
        os.close(fd)
    print('One-second speaker-route tone completed; headphones route restored.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', action='store_true',
                        help='send audio-route reports and play the bounded tone')
    args = parser.parse_args()
    root, card, hidraw = physical_devices()
    print(f'Physical DualSense ready: {root.name}, ALSA card {card}, {hidraw}')
    if args.run:
        probe()


if __name__ == '__main__':
    main()
