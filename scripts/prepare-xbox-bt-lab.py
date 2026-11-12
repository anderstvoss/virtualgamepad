#!/usr/bin/python3 -I
"""Prepare an opt-in virtual Xbox exception around the installed xpadneo rules."""

import argparse
from pathlib import Path


LABEL = 'virtualgamepad_xbox_lab_end'
GUARD = (
    'SUBSYSTEM=="hid", ENV{HID_ID}=="0005:0000045E:00000B13", '
    'ENV{HID_PHYS}=="virtualgamepad/input-lab/uhid/xbox-series/p*-i*", '
    'PROGRAM=="/usr/lib/udev/virtualgamepad-input-lab-seat.py %E{HID_PHYS}", '
    f'GOTO="{LABEL}"\n'
)


def wrap(vendor):
    if LABEL in vendor or 'ID_XPADNEO_SUPPORTED' not in vendor:
        raise ValueError('expected the unmodified xpadneo supported-device rules')
    return ('# Dev/test virtual Xbox exception; the physical-device vendor rules follow.\n'
            + GUARD + vendor.rstrip() + f'\nLABEL="{LABEL}"\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--vendor', type=Path, default=Path('/usr/lib/udev/rules.d/60-xpadneo.rules'))
    args = parser.parse_args()
    try:
        data = wrap(args.vendor.read_text())
        with args.output.open('x') as stream:
            stream.write(data)
    except (OSError, ValueError) as error:
        parser.exit(1, f'{error}\n')


if __name__ == '__main__':
    main()
