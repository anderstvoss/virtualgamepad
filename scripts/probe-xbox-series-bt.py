"""Run a bounded Xbox Series Bluetooth-bus UHID probe with the reviewed SDL consumer."""

import argparse
import errno
import json
import os
from pathlib import Path
import select
import struct
import subprocess
import time
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('sdl_probe', type=Path, help='built scripts/sdl3-gamepad-probe.c')
parser.add_argument('report_map', type=Path, help='physical Bluetooth HID descriptor in hex')
options = parser.parse_args()
descriptor = bytes.fromhex(options.report_map.read_text())
if len(descriptor) != 283:
    parser.error('this probe expects the measured 283-byte Xbox Series descriptor')
identity = 'virtualgamepad-xbox-series-lab-' + uuid.uuid4().hex[:10]
fd = os.open('/dev/uhid', os.O_RDWR | os.O_NONBLOCK)
observed = []
output = []

def write(event, fmt='', *values):
    packet = struct.pack('<I'+fmt, event, *values)
    if os.write(fd, packet) != len(packet):
        raise OSError('short UHID write')

def events(wait):
    deadline = time.monotonic() + wait
    while True:
        remaining = max(0, deadline - time.monotonic())
        if not select.select([fd], [], [], remaining)[0]:
            return
        packet = os.read(fd, 8192)
        event = struct.unpack_from('<I', packet)[0]
        observed.append(event)
        if event == 6:
            length, report_type = struct.unpack_from('<HB', packet, 4100)
            output.append((report_type, packet[4:4+length].hex()))
        elif event == 9:
            request, report, kind = struct.unpack_from('<IBB', packet, 4)
            write(10, 'IHH', request, errno.EOPNOTSUPP, 0)
        elif event == 13:
            request, report, kind, length = struct.unpack_from('<IBBH', packet, 4)
            valid = kind == 1 and report == 3 and length == 9 and packet[12] == 3
            if valid:
                output.append((kind,packet[12:21].hex()))
            write(14, 'IH', request, 0 if valid else errno.EOPNOTSUPP)

def report(sticks=(32768,)*4, triggers=(0,0), hat=0, buttons=0, share=0):
    payload = struct.pack('<B6HBHB',1,*sticks,*triggers,hat,buttons,share)
    write(12, 'H17s', len(payload), payload)

def owned_device():
    for path in Path('/sys/bus/hid/devices').glob('0005:045E:0B13.*'):
        try:
            if 'HID_PHYS='+identity in (path/'uevent').read_text().splitlines():
                return path
        except FileNotFoundError:
            pass

try:
    write(11, '128s64s64sHHIIII4096s', b'Xbox Wireless Controller', identity.encode(),
          b'', len(descriptor), 5, 0x045e, 0x0b13, 0x0523, 0, descriptor)
    deadline = time.monotonic()+5
    node = None
    while time.monotonic() < deadline:
        events(.05)
        hid = owned_device()
        nodes = list(hid.glob('input/input*/event*')) if hid else []
        if len(nodes) == 1:
            node = '/dev/input/' + nodes[0].name
            if os.access(node, os.R_OK):
                break
    assert node and os.access(node, os.R_OK), 'owned input node did not become readable'
    print('bound:', (hid/'driver').readlink().name, 'node:', Path(node).name,
          'raw readable:', any(os.access('/dev/'+p.name, os.R_OK) for p in hid.glob('hidraw/hidraw*')), flush=True)
    env = dict(os.environ, SDL_JOYSTICK_HIDAPI='0')
    with subprocess.Popen([options.sdl_probe.resolve(), node,'5300','--gamepad-rumble-script','--reopen'],
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, text=True) as consumer:
        for _ in range(10):
            report()
            events(.05)
        for bit in (0,1,3,4,6,7,10,11,12,13,14):
            report(buttons=1<<bit)
            events(.06)
            report()
            events(.06)
        report(share=1)
        events(.08)
        report()
        events(.08)
        for direction in range(1,9):
            report(hat=direction)
            events(.07)
            report()
            events(.03)
        for stick in range(4):
            for extreme in (0,65535):
                sticks = [32768]*4
                sticks[stick] = extreme
                report(sticks=sticks)
                events(.065)
            report()
            events(.03)
        for trigger in range(2):
            triggers = [0,0]
            triggers[trigger] = 1023
            report(triggers=triggers)
            events(.08)
            report()
            events(.05)
        while consumer.poll() is None:
            events(.06)
        stdout, stderr = consumer.communicate()
        print('SDL exit:', consumer.returncode)
        result = json.loads(stdout.splitlines()[-1])
        print(json.dumps(result, separators=(',', ':')))
        if stderr:
            print(stderr)
        passed = (consumer.returncode == 0 and result['button_down_mask'] == 0xffff
                  and result['button_up_mask'] == 0xffff and result['axis_mask'] == 0x3f
                  and result['final_buttons'] == 0 and result['final_axes'] == [0] * 6)
    print('UHID events:', observed, 'outputs:', output)
finally:
    write(1)
    os.close(fd)
deadline = time.monotonic()+2
while owned_device() and time.monotonic() < deadline:
    time.sleep(.05)
assert not owned_device(), 'owned UHID device remained after destruction'
print('owned UHID device removed')
if not passed or not any(kind == 1 and payload.startswith('03') for kind, payload in output):
    raise SystemExit('SDL control or UHID reverse-rumble check failed')
