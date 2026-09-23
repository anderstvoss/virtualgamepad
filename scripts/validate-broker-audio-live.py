#!/usr/bin/env python3
"""Ordinary-client duplex checks through installed broker and production PCM IPC.
Synthetic samples only. This is continuity evidence, not end-to-end latency.
"""
import argparse
import array
import importlib.util
import heapq
import json
from pathlib import Path
import socket
import struct
import subprocess
import threading
import time

spec = importlib.util.spec_from_file_location('live', Path(__file__).with_name('validate-usb-audio-live.py'))
live = importlib.util.module_from_spec(spec)
spec.loader.exec_module(live)


class RefillDelays:
    """Bounded caller-scheduling evidence; never an audio latency measurement."""
    def __init__(self):
        self.previous = None
        self.consumed = 0
        self.largest = []

    def record(self, started_ns, replied_ns, consumed):
        if replied_ns < started_ns or (self.previous is not None and started_ns < self.previous):
            raise ValueError('nonmonotonic refill observation')
        if consumed < self.consumed:
            raise ValueError('microphone consumption moved backwards')
        if self.previous is not None and consumed > self.consumed:
            entry = ((started_ns-self.previous)//1000, (replied_ns-started_ns)//1000, consumed)
            if len(self.largest) < 8: heapq.heappush(self.largest,entry)
            else: heapq.heappushpop(self.largest,entry)
        self.previous = started_ns
        self.consumed = consumed

    def summary(self):
        return [dict(interval_us=interval,credit_roundtrip_us=rtt,consumed_frame=frame)
                for interval,rtt,frame in sorted(self.largest,reverse=True)]


def exact(peer, count):
    result = bytearray()
    while len(result) < count:
        part = peer.recv(count-len(result))
        if not part:
            raise EOFError('owned channel closed')
        result.extend(part)
    return bytes(result)


def message(peer, version, tag, body):
    peer.sendall(struct.pack('<IHB',len(body)+3,version,tag)+body)


def reply(peer):
    size, = struct.unpack('<I',exact(peer,4))
    if not 3 <= size <= 259:
        raise ValueError('invalid broker/worker message size')
    data = exact(peer,size)
    return struct.unpack('<HB',data[:3]) + (data[3:],)


def opened(profile):
    peer = socket.socket(socket.AF_UNIX,socket.SOCK_STREAM)
    peer.settimeout(10)
    peer.connect('/run/virtualgamepad/broker.sock')
    _, uid, _ = struct.unpack('3i',peer.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12))
    if uid != 0:
        raise ValueError('broker is not root')
    tag = {'dualsense':1,'dualshock4':2,'xbox360':3}[profile]
    message(peer,2,1,bytes([tag,2,1,2,3,4,5]))
    version, operation, body = reply(peer)
    if (version,operation) != (2,0x80):
        raise ValueError(body.decode(errors='replace'))
    generation, device = struct.unpack('<QI',body[:12])
    bus = body[12:].decode('ascii')
    data, ancillary, flags, _ = peer.recvmsg(1,socket.CMSG_SPACE(3*4),socket.MSG_CMSG_CLOEXEC)
    received = []
    for level, kind, payload in ancillary:
        if (level,kind) != (socket.SOL_SOCKET,socket.SCM_RIGHTS):
            raise ValueError('unexpected ancillary message')
        fds = array.array('i'); fds.frombytes(payload)
        received.extend(socket.socket(fileno=fd) for fd in fds)
    if data != b'\xa2' or len(received) != 3 or flags & socket.MSG_CTRUNC:
        for item in received: item.close()
        raise ValueError('invalid channel handoff')
    for item in received: item.settimeout(2)
    return peer,generation,device,bus,tag,received


def owned_pipewire_device(objects, card):
    candidates = [item for item in objects if item.get('type') == 'PipeWire:Interface:Device'
                  and item.get('info',{}).get('props',{}).get('api.alsa.card') == card
                  and item.get('info',{}).get('props',{}).get('device.bus-path','').startswith('platform-vhci_hcd.0-usb-')]
    if len(candidates) > 1: raise ValueError('ambiguous owned PipeWire device')
    return candidates[0] if candidates else None


def reserve_direct_alsa(card, bus):
    # Test-harness-only exclusion of the session manager from this newly owned
    # virtual card. Do not alter defaults or any physical audio device.
    device = (Path('/sys/bus/usb/devices')/bus).resolve(strict=True)
    if 'vhci_hcd.0' not in device.parts or not (Path('/sys/class/sound')/f'card{card}'/'device').resolve().is_relative_to(device):
        raise ValueError('ALSA ancestry changed before reservation')
    deadline = time.monotonic()+2
    while time.monotonic() < deadline:
        result = subprocess.run(['pw-dump'],capture_output=True,timeout=3)
        if result.returncode: return
        item = owned_pipewire_device(json.loads(result.stdout),card)
        if item:
            profiles = item['info'].get('params',{}).get('EnumProfile',[])
            off = [profile['index'] for profile in profiles if profile.get('name') == 'off']
            if len(off) != 1: raise ValueError('owned PipeWire device has no unique off profile')
            subprocess.run(['pw-cli','set-param',str(item['id']),'Profile',json.dumps(dict(index=off[0],save=False))],check=True,stdout=subprocess.DEVNULL,timeout=3)
            return
        time.sleep(.02)
    raise TimeoutError('owned PipeWire card not ready for exclusive ALSA test')


def trial(profile, seconds):
    broker, generation, device, bus, tag, channels = opened(profile)
    control, playback, microphone = channels
    stop = threading.Event()
    totals = dict(playback_frames=0,playback_invalid=0,playback_gaps=0,microphone_submitted=0)
    _, nout, nin = live.PROFILES[profile]
    errors = []
    delays = RefillDelays()
    def consume():
        position = 0
        pattern = tuple([101,-202,303,-404][:nout])
        try:
            while not stop.is_set():
                header = exact(playback,40)
                if header[:8] != b'VGPA'+bytes([1,0,tag,0]): raise ValueError('invalid playback header')
                gen, first, frames, flags, _ = struct.unpack('<QQIIQ',header[8:])
                if gen != generation or not 1 <= frames <= 128: raise ValueError('invalid playback block')
                totals['playback_gaps'] += int(first != position or flags != 0)
                position = first+frames
                for frame in struct.iter_unpack('<'+'h'*nout,exact(playback,frames*nout*2)):
                    if frame == pattern: totals['playback_frames'] += 1
                    elif any(frame): totals['playback_invalid'] += 1
        except (OSError,EOFError,ValueError) as error:
            if not stop.is_set(): errors.append(str(error))
    def produce():
        submitted = 0
        try:
            while not stop.is_set():
                started = time.monotonic_ns()
                message(control,1,5,struct.pack('<Q',generation))
                version, operation, data = reply(control)
                if (version,operation,len(data)) != (1,5,16): raise ValueError('invalid microphone credit')
                if data[:8] != struct.pack('<Q',generation): raise ValueError('foreign microphone credit')
                consumed, = struct.unpack('<Q',data[8:])
                if consumed > submitted: raise ValueError('microphone credit exceeds submitted frames')
                delays.record(started,time.monotonic_ns(),consumed)
                totals['microphone_consumed'] = consumed
                # Test-only eight-millisecond operating fill; queue capacity is separate.
                available = max(0,consumed+384-submitted)
                while available:
                    frames = min(128,available)
                    values = [100+(submitted+frame)%97+100*c for frame in range(frames) for c in range(nin)]
                    header = b'VGPA'+bytes([1,0,tag,1])+struct.pack('<QQIIQ',generation,submitted,frames,0,time.monotonic_ns()//1000)
                    microphone.sendall(header+struct.pack('<'+'h'*len(values),*values))
                    submitted += frames; available -= frames
                totals['microphone_submitted'] = submitted
                time.sleep(.001)
        except (OSError,EOFError,ValueError) as error:
            if not stop.is_set(): errors.append(str(error))
    threads = [threading.Thread(target=f) for f in (consume,produce)]
    try:
        deadline = time.monotonic()+3
        while True:
            cards = [card for card in Path('/sys/class/sound').glob('card[0-9]*') if (card/'device').resolve().is_relative_to((Path('/sys/bus/usb/devices')/bus).resolve())]
            if len(cards) == 1:
                number = cards[0].name[4:]
                if all(Path(f'/dev/snd/{name}').exists() for name in [f'controlC{number}', f'pcmC{number}D0p', f'pcmC{number}D0c']): break
            if time.monotonic() >= deadline: raise TimeoutError('owned ALSA card did not appear')
            time.sleep(.01)
        subprocess.run(['udevadm','settle','--timeout=3'],check=True,timeout=4)
        reserve_direct_alsa(int(cards[0].name[4:]),bus)
        for thread in threads: thread.start()
        result = live.run_trial(int(cards[0].name[4:]),profile,seconds)
        time.sleep(.1)
        result.update(totals)
        result['ipc_errors'] = errors
        result['microphone_refill_largest_delays'] = delays.summary()
        result['passed'] &= not errors and totals['playback_invalid'] == totals['playback_gaps'] == 0 and totals['playback_frames'] == (seconds+2)*48000
        return result
    finally:
        stop.set()
        for item in channels:
            try: item.shutdown(socket.SHUT_RDWR)
            except OSError: pass
        for thread in threads:
            if thread.ident is not None: thread.join(3)
        for item in channels: item.close()
        message(broker,2,2,struct.pack('<Q',generation))
        if reply(broker) != (2,0x80,struct.pack('<Q',generation)):
            raise ValueError('cleanup not acknowledged')
        broker.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile',choices=live.PROFILES,required=True)
    parser.add_argument('--seconds',type=int,default=3)
    parser.add_argument('--trials',type=int,default=1)
    args = parser.parse_args()
    if not 1 <= args.seconds <= 60: parser.error('seconds must be 1..60')
    if not 1 <= args.trials <= 3: parser.error('trials must be 1..3')
    for index in range(args.trials):
        print(json.dumps(dict(event='start',profile=args.profile,trial=index,seconds=args.seconds)),flush=True)
        result = trial(args.profile,args.seconds)
        result.update(profile=args.profile,trial=index)
        print(json.dumps(result),flush=True)
        if not result['passed']: raise SystemExit(1)


if __name__ == '__main__': main()
