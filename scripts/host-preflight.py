#!/usr/bin/env python3
"""Read-only, realization-specific host inventory. Never opens device nodes."""
import argparse
import json
import os
import stat
from pathlib import Path

TARGETS = ('linux.uhid.usb', 'linux.uinput', 'linux.dummy_hcd.usb-hid')


def inspect(target, root=Path('/'), access=None):
    root = Path(root)
    access = access or (lambda p, mode: os.access(p, mode, effective_ids=True))
    rows = []

    def report(check, status, detail):
        rows.append(dict(check=check, status=status, detail=detail))

    def path(name):
        return root / name.lstrip('/')

    if target in TARGETS[:2]:
        name = 'uhid' if target == TARGETS[0] else 'uinput'
        node = path('/dev/' + name)
        try:
            metadata = node.stat()
            expected = path('/sys/class/misc/' + name + '/dev').read_text().strip()
            actual = f'{os.major(metadata.st_rdev)}:{os.minor(metadata.st_rdev)}'
            if not stat.S_ISCHR(metadata.st_mode) or actual != expected:
                report('creation-device', 'mismatch', 'Node is not the expected kernel misc device; do not grant access.')
            elif not access(node, os.R_OK | os.W_OK):
                report('creation-device', 'denied', 'Administrator may grant this identity access to this creation device only.')
            else:
                report('creation-device', 'ready', 'Expected creation device is accessible; no device was opened.')
        except FileNotFoundError:
            report('creation-device', 'missing', 'Kernel misc-device registration or node is missing; administrator setup is required.')
        except PermissionError:
            report('creation-device', 'denied', 'Cannot inspect device identity.')
        report('consumer-access', 'unvalidated', 'Verify only the created session hidraw/event nodes during the live test.')
    elif target == TARGETS[2]:
        gadget = path('/sys/kernel/config/usb_gadget')
        report('configfs', 'ready' if gadget.is_dir() else 'missing',
               'Administrator prepares ConfigFS and HID gadget support; applications never mount or load modules.')
        config = path('/etc/virtualgamepad/broker.conf')
        try:
            from_config = [line.strip() for line in config.read_text().splitlines()]
            authorized = [line.removeprefix('allow_udc=') for line in from_config if line.startswith('allow_udc=')]
            if not authorized:
                report('udc-authorization', 'missing', 'Administrator must reserve at least one dummy UDC in broker.conf.')
            elif any(not name.startswith('dummy_udc.') or not name.removeprefix('dummy_udc.').isdigit() for name in authorized):
                report('udc-authorization', 'mismatch', 'Invalid authorized UDC name; broker configuration must be corrected.')
            else:
                bound = {(entry / 'UDC').read_text().strip() for entry in gadget.iterdir()}
                for name in authorized:
                    present = path('/sys/class/udc/' + name).is_dir()
                    status = 'missing' if not present else ('occupied' if name in bound else 'available')
                    report('authorized-udc', status, name + '; observation is not a reservation.')
        except FileNotFoundError:
            report('udc-authorization', 'missing', 'Required broker configuration or ConfigFS binding metadata is missing; availability is not established.')
        except PermissionError:
            report('udc-authorization', 'denied', 'Cannot inspect authorization or existing bindings; administrator review is required.')
        socket = path('/run/virtualgamepad/broker.sock')
        try:
            if not stat.S_ISSOCK(socket.stat().st_mode):
                report('broker-socket', 'mismatch', 'Expected a Unix socket.')
            else:
                report('broker-socket', 'available' if access(socket, os.W_OK) else 'denied',
                       'Socket permissions only; peer-UID authorization is checked by the broker on requests.')
        except FileNotFoundError:
            report('broker-socket', 'missing', 'Optional broker socket is not installed or started.')
        except PermissionError:
            report('broker-socket', 'denied', 'Socket path is inaccessible.')
        report('gate-g', 'unvalidated', 'Control metadata, exact completion, reduced capabilities, startup and cleanup need live evidence.')
    else:
        raise ValueError('unknown realization')
    return dict(realization=target, checks=rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('realization', choices=TARGETS + ('all',))
    args = parser.parse_args()
    targets = TARGETS if args.realization == 'all' else (args.realization,)
    results = [inspect(target) for target in targets]
    print(json.dumps(results, indent=2))
    return int(any(c['status'] not in ('ready', 'available') for r in results for c in r['checks']))


if __name__ == '__main__':
    raise SystemExit(main())
