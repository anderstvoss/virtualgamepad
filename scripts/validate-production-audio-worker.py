#!/usr/bin/env python3
"""Exercise the installed-worker entry point without root or a kernel attachment."""
import argparse
import fcntl
import importlib.util
import os
from pathlib import Path
import signal
import socket
import struct
import time

spec = importlib.util.spec_from_file_location('usb_probe', Path(__file__).with_name('validate-usb-audio-worker.py'))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


def message(peer, tag, payload):
    peer.sendall(struct.pack('<IHB', len(payload)+3, 1, tag)+payload)


def receive(peer):
    size, = struct.unpack('<I', probe.exact(peer, 4))
    assert 3 <= size <= 259
    body = probe.exact(peer, size)
    assert body[:2] == b'\x01\x00'
    return body[2], body[3:]


def wait(pid, timeout):
    deadline = time.monotonic()+timeout
    while time.monotonic() < deadline:
        found, status = os.waitpid(pid, os.WNOHANG)
        if found:
            return os.waitstatus_to_exitcode(status)
        time.sleep(.01)
    raise TimeoutError('worker did not exit')


def output_cases(family):
    """Synthetic declared output fields, distinct start/update/stop observations."""
    if family=='dualsense':
        cases=[]
        for index,(right,left) in enumerate(((17,33),(44,66),(0,0),(0,0))):
            raw=bytearray(47);raw[0:4]=bytes([0xe1,0x97,right,left])
            raw[5:10]=bytes([64,96,index<<4,index%2,0x10 if index%2 else 0])
            raw[10:21]=bytes(range(1,12));raw[21:32]=bytes(range(11,0,-1))
            raw[37]=0xfd;raw[43:47]=bytes([0x15,32+index,64,128])
            cases.append((2,bytes(raw),True))
        # Inactive fields retain their bytes but must not become typed updates;
        # V2 motors use valid_flag2 alone, independently of valid_flag0.
        for flag2 in (0,4):
            raw=bytearray(cases[0][1]);raw[0]=raw[1]=0;raw[38]=flag2
            cases.append((2,bytes(raw),True))
        return cases
    if family=='dualshock4':
        cases=[]
        for index,(right,left) in enumerate(((17,33),(44,66),(0,0))):
            raw=bytearray(31);raw[0]=1 if index==1 else 3;raw[3:8]=bytes([right,left,32+index,64,128])
            cases.append((5,bytes(raw),True))
        return cases
    if family=='xbox360':
        # This compiled HID profile implements generic inputs, not xpad outputs.
        # Its output setter is explicitly unsupported on both USB paths.
        return [(0,bytes([0,8,0,right,left,0,0,0]),False) for right,left in ((17,33),(44,66),(0,0))]
    raise ValueError('unknown compiled family')


def expect_output(control,generation,expected):
    message(control,2,generation)
    observed=receive(control)
    if observed!=(2,expected):raise RuntimeError('output observation differs: '+repr(observed))


def check_outputs(usb,control,family,generation):
    sequence=200
    for report_id,raw,supported in output_cases(family):
        wire=bytes([report_id])+raw if family!='xbox360' else raw
        for endpoint in (4,0):
            setup=(bytes([0x21,9,report_id,2,3,0,len(wire),0]) if endpoint==0 else bytes(8))
            result=probe.transfer(usb,sequence,endpoint,0,len(wire),setup,wire)
            sequence+=1
            expected=(0,len(wire)) if supported else (0xffffffe0,0)
            if result[:2]!=expected:raise RuntimeError('output completion differs: '+repr(result[:2]))
            event=generation+bytes([1,report_id])+raw if supported else generation+b'\0'
            expect_output(control,generation,event)
            # Empty after each exact event proves no duplicate/reordered output.
            expect_output(control,generation,generation+b'\0')
    report_id=2 if family=='dualsense' else 5 if family=='dualshock4' else 0
    for wire in (bytes([report_id,0]),bytes([0x7f])*32):
        for endpoint in (4,0):
            setup=bytes([0x21,9,wire[0],2,3,0,len(wire),0]) if endpoint==0 else bytes(8)
            result=probe.transfer(usb,sequence,endpoint,0,len(wire),setup,wire);sequence+=1
            if result[:2]!=(0xffffffe0,0):raise RuntimeError('invalid output was acknowledged')
            expect_output(control,generation,generation+b'\0')
    # No output GET is declared; it must stall rather than fabricate retained HID state.
    result=probe.transfer(usb,sequence,0,1,64,bytes([0xa1,1,report_id,2,3,0,64,0]))
    if result[:2]!=(0xffffffe0,0):raise RuntimeError('unsupported output GET did not stall')
    expect_output(control,generation,generation+b'\0')


def trial(worker, family, tag, channels, microphones, slots):
    pairs = [socket.socketpair() for _ in range(4)]
    pid = os.fork()
    if pid == 0:
        try:
            # Duplicate away from all fixed ABI slots before remapping any slot.
            copies = [fcntl.fcntl(child.fileno(), fcntl.F_DUPFD_CLOEXEC, 10) for _, child in pairs]
            for source, destination in zip(copies, (0, *slots)):
                os.dup2(source, destination, inheritable=True)
            os.execv(str(worker), [str(worker), family, str(0x10001), '7', '020102030405', 'validator', *map(str, slots)])
        except BaseException:
            os._exit(125)
    reaped = False
    for _, child in pairs:
        child.close()
    usb, control, playback, microphone = [parent for parent, _ in pairs]
    for peer in (usb, control, playback, microphone):
        peer.settimeout(3)
    generation = struct.pack('<Q', 7)
    try:
        assert receive(control) == (0, b'\x01'+generation)
        # Real USB enumeration and controller-owned request completion.
        descriptor = probe.transfer(usb,1,0,1,18,bytes([0x80,6,0,1,0,0,18,0]))
        assert descriptor[:2] == (0,18) and descriptor[2][16] == 3
        encoded = 'vg-validator-0000000000000007'.encode('utf-16le')
        serial = bytes([len(encoded)+2, 3])+encoded
        assert probe.transfer(usb,100,0,1,255,bytes([0x80,6,3,3,9,4,255,0]))[:3] == (0,len(serial),serial)
        assert probe.transfer(usb,101,0,1,255,bytes([0x80,6,3,3,0x11,4,255,0]))[:2] == (0xffffffe0,0)
        for sequence, setup in [(2,[0,9,1,0,0,0,0,0]),(3,[1,11,1,0,1,0,0,0]),(4,[1,11,1,0,2,0,0,0])]:
            assert probe.transfer(usb,sequence,0,0,0,bytes(setup))[:2] == (0,0)
        assert probe.transfer(usb,5,3,1,64)[0] == 0
        check_outputs(usb,control,family,generation)
        # Feed caller microphone IPC, then verify those exact frames on USB.
        expected = bytearray()
        for block in range(8):
            pcm = probe.live.marker_pcm(128, microphones, block*128)
            expected.extend(pcm)
            header = b'VGPA'+bytes([1,0,tag,1])+struct.pack('<QQIIQ',7,block*128,128,0,block)
            microphone.sendall(header+pcm)
        # Bounded warm-up permits the separate PCM thread to prime its queue.
        time.sleep(.03)
        status, actual, captured, _ = probe.transfer(usb,6,2,1,49*microphones*2*8,packets=8)
        assert status == 0 and actual == 48*microphones*2*8
        assert captured == expected[:actual]
        # Host USB playback must traverse the independent outbound PCM channel.
        pcm = probe.live.marker_pcm(48*8, channels)
        assert probe.transfer(usb,7,1,0,len(pcm),payload=pcm,packets=8)[:2] == (0,len(pcm))
        received = bytearray()
        position = 0
        while len(received) < len(pcm):
            header = probe.exact(playback,40)
            assert header[:8] == b'VGPA'+bytes([1,0,tag,0])
            gen, first, count, flags, _ = struct.unpack('<QQIIQ',header[8:])
            assert gen == 7 and first == position and flags == 0 and 0 < count <= 128
            received.extend(probe.exact(playback,count*channels*2))
            position += count
        assert received == pcm
        message(control,3,generation)
        operation, counters = receive(control)
        assert operation == 3 and len(counters) == 80
        assert struct.unpack('<10Q',counters)[0] == 7
        assert struct.unpack('<10Q',counters)[1] == 0  # No optional output loss.
        message(control,4,generation)
        assert receive(control) == (4,generation)
        status = wait(pid,3)
        reaped = True
        assert status == 0, status
        assert playback.recv(1) == b''
        print(f'{family} descriptors={slots}: production worker enumeration, exact output/SET replies and ordered observations, bidirectional PCM IPC, diagnostics and closure passed')
    finally:
        if not reaped:
            found, _ = os.waitpid(pid,os.WNOHANG)
            if not found:
                os.kill(pid,signal.SIGTERM)
                os.waitpid(pid,0)
        for parent, _ in pairs:
            parent.close()


def main():
    if not __debug__:
        raise RuntimeError('validation requires Python assertions; optimization is unsupported')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker',type=Path,required=True)
    args = parser.parse_args()
    worker = args.worker.resolve(strict=True)
    for family, tag, channels, microphones in [('dualsense',1,4,2),('dualshock4',2,2,1),('xbox360',3,2,1)]:
        for slots in [(3, 4, 5), (64, 65, 66)]:
            trial(worker,family,tag,channels,microphones,slots)


if __name__ == '__main__':
    main()
