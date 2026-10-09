#!/usr/bin/env python3
"""Ordinary-client duplex checks through installed broker and production PCM IPC.
Synthetic samples only. This is continuity evidence, not end-to-end latency.
"""
import argparse
import array
import importlib.util
import heapq
import json
import os
import re
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


def worker_diagnostics(control,generation):
    message(control,1,3,struct.pack('<Q',generation))
    version, operation, data = reply(control)
    if (version,operation,len(data)) != (1,3,80) or data[:8] != struct.pack('<Q',generation):
        raise ValueError('invalid worker diagnostics')
    keys = ('lost_outputs','completed_transfers','microphone_silence_frames',
            'stalled_transfers','playback_frames','capture_frames',
            'abandoned_capture_frames','maximum_audio_lateness_us',
            'microphone_queue_dropped_frames')
    result = dict(zip(keys,struct.unpack('<9Q',data[8:])))
    message(control,1,6,struct.pack('<Q',generation))
    version, operation, data = reply(control)
    if (version,operation,len(data)) != (1,6,16) or data[:8] != struct.pack('<Q',generation):
        raise ValueError('installed worker lacks PCM pump timing diagnostics')
    result['maximum_pcm_pump_lateness_us'], = struct.unpack('<Q',data[8:])
    return result


def functional_worker_evidence(counters, expected_frames):
    """Short functional transfer proof, not sustained continuity or latency."""
    return (counters['playback_frames'] >= expected_frames
            and counters['capture_frames'] >= expected_frames
            and counters['lost_outputs'] == 0
            and counters['microphone_queue_dropped_frames'] == 0)


def microphone_credit(control, generation):
    message(control, 1, 5, struct.pack('<Q', generation))
    version, operation, data = reply(control)
    if ((version, operation, len(data)) != (1, 5, 16)
            or data[:8] != struct.pack('<Q', generation)):
        raise ValueError('invalid microphone credit')
    return struct.unpack('<Q', data[8:])[0]


def final_microphone_accounting(control, generation, submitted):
    """Bounded quiescent observation, never an atomic in-flight snapshot.

    Called only after ALSA clients and the producer have stopped. Two identical
    snapshots plus conservation are required; moving counters are unavailable
    evidence, not evidence of unexplained loss.
    """
    previous = None
    for _ in range(3):
        message(control, 1, 7, struct.pack('<Q', generation))
        version, operation, data = reply(control)
        if ((version, operation, len(data)) != (1, 7, 24)
                or data[:8] != struct.pack('<Q', generation)):
            raise ValueError('invalid microphone host accounting')
        host, silence = struct.unpack('<QQ', data[8:])
        consumed = microphone_credit(control, generation)
        counters = worker_diagnostics(control, generation)
        current = (host, silence, consumed, counters)
        if current == previous:
            if consumed > submitted or host != consumed + silence:
                raise ValueError('microphone accounting does not reconcile')
            if silence != counters['microphone_silence_frames']:
                raise ValueError('microphone silence snapshots disagree')
            completed = counters['capture_frames']
            abandoned = counters['abandoned_capture_frames']
            if host != completed + abandoned:
                raise ValueError('microphone capture remains unaccounted')
            return dict(host_frames=host, consumed_frames=consumed,
                        silence_frames=silence, submitted_frames=submitted,
                        unconsumed_frames=submitted-consumed,
                        completed_frames=completed, abandoned_frames=abandoned,
                        quiescent=True), counters
        previous = current
    raise ValueError('microphone accounting did not become quiescent')


def close_broker_session(broker, channels, generation):
    """Retain required channels until broker close; preserve all cleanup errors."""
    errors = []
    try:
        message(broker, 2, 2, struct.pack('<Q', generation))
        if reply(broker) != (2, 0x80, struct.pack('<Q', generation)):
            raise ValueError('cleanup not acknowledged')
    except Exception as error:
        errors.append(str(error))
    finally:
        for item in channels:
            try: item.shutdown(socket.SHUT_RDWR)
            except OSError: pass
            try: item.close()
            except OSError as error: errors.append(str(error))
        broker.close()
    return errors


def microphone_fill_frames(milliseconds):
    if not 1 <= milliseconds <= 16:
        raise ValueError('microphone fill must be 1..16 ms')
    return milliseconds*48


def channel_handoff(peer):
    """Take ownership of every delivered descriptor before validating metadata."""
    data, ancillary, flags, _ = peer.recvmsg(1, socket.CMSG_SPACE(3*4), socket.MSG_CMSG_CLOEXEC)
    raw = []
    sockets = []
    invalid = False
    try:
        for level, kind, payload in ancillary:
            if (level, kind) != (socket.SOL_SOCKET, socket.SCM_RIGHTS):
                invalid = True
                continue
            descriptors = array.array('i')
            aligned = len(payload) - len(payload) % descriptors.itemsize
            descriptors.frombytes(payload[:aligned])
            raw.extend(descriptors)
            invalid |= aligned != len(payload)
        if (invalid or data != b'\xa2' or len(raw) != 3 or len(set(raw)) != 3 or
                flags & (socket.MSG_CTRUNC | socket.MSG_TRUNC)):
            raise ValueError('invalid channel handoff')
        while raw:
            # socket(fileno=...) transfers ownership only on successful return.
            channel = socket.socket(fileno=raw[-1])
            raw.pop()
            sockets.append(channel)
        sockets.reverse()
        for channel in sockets: channel.settimeout(2)
        return sockets
    except BaseException:
        for channel in sockets: channel.close()
        for descriptor in set(raw):
            try: os.close(descriptor)
            except OSError: pass
        raise


def opened(profile):
    tag = {'dualsense':1, 'dualshock4':2, 'xbox360':3}[profile]
    peer = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    handed_off = False
    try:
        peer.settimeout(10)
        peer.connect('/run/virtualgamepad/broker.sock')
        _, uid, _ = struct.unpack('3i', peer.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        if uid != 0: raise ValueError('broker is not root')
        message(peer, 2, 1, bytes([tag,2,1,2,3,4,5]))
        version, operation, body = reply(peer)
        if (version, operation) != (2, 0x80):
            raise ValueError(body.decode(errors='replace'))
        if len(body) < 15: raise ValueError('truncated attachment identity')
        generation, device = struct.unpack('<QI', body[:12])
        bus = body[12:].decode('ascii')
        if generation == 0 or device == 0 or not re.fullmatch(r'[0-9]+-[0-9]+(?:\.[0-9]+)*', bus):
            raise ValueError('invalid attachment identity')
        received = channel_handoff(peer)
        handed_off = True
        return peer, generation, device, bus, tag, received
    finally:
        if not handed_off: peer.close()


owned_pipewire_device = live.owned_pipewire_device
reserve_direct_alsa = live.reserve_direct_alsa


def trial(profile, seconds, instance, microphone_fill_ms=8):
    live.compiled_serial(instance, 1)  # Reject invalid expectations before creation.
    broker, generation, device, bus, tag, channels = opened(profile)
    control, playback, microphone = channels
    stop = threading.Event()
    totals = dict(playback_frames=0,playback_invalid=0,playback_gaps=0,microphone_submitted=0)
    _, nout, nin = live.PROFILES[profile]
    fill_frames = microphone_fill_frames(microphone_fill_ms)
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
                consumed = microphone_credit(control,generation)
                if consumed > submitted: raise ValueError('microphone credit exceeds submitted frames')
                delays.record(started,time.monotonic_ns(),consumed)
                totals['microphone_consumed'] = consumed
                # Test-only operating fill; queue capacity is separate.
                available = max(0,consumed+fill_frames-submitted)
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
    result = None
    initiating = None
    cleanup = []
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
        isolation = reserve_direct_alsa(int(cards[0].name[4:]),bus,instance,generation)
        for thread in threads: thread.start()
        result = live.run_trial(int(cards[0].name[4:]),profile,seconds)
        stop.set()
        for thread in threads:
            thread.join(3)
            if thread.is_alive(): errors.append('PCM client thread failed to stop')
        after = reserve_direct_alsa(int(cards[0].name[4:]),bus,instance,generation)
        result['shared_defaults_unchanged'] = after['shared_defaults'] == isolation['shared_defaults']
        result['passed'] &= result['shared_defaults_unchanged']
        if any(thread.is_alive() for thread in threads):
            raise ValueError('PCM clients still active during final accounting')
        accounting, counters = final_microphone_accounting(
            control, generation, totals['microphone_submitted'])
        result['microphone_final_accounting'] = accounting
        result['worker_diagnostics'] = counters
        result['worker_functional_evidence'] = functional_worker_evidence(result['worker_diagnostics'], (seconds+2)*48000)
        result['passed'] &= result['worker_functional_evidence']
        result.update(totals)
        result['ipc_errors'] = errors
        result['microphone_refill_largest_delays'] = delays.summary()
        result['microphone_fill_ms'] = microphone_fill_ms
        result['passed'] &= not errors and totals['playback_invalid'] == totals['playback_gaps'] == 0 and totals['playback_frames'] == (seconds+2)*48000
    except Exception as error:
        initiating = str(error)
    finally:
        stop.set()
        cleanup = close_broker_session(broker, channels, generation)
        for thread in threads:
            if thread.ident is not None:
                thread.join(3)
                if thread.is_alive(): cleanup.append('PCM client thread remains alive')
    if result is None:
        result = dict(passed=False)
    result.update(initiating_error=initiating, cleanup_errors=cleanup)
    result['passed'] &= initiating is None and not cleanup
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile',choices=[*live.PROFILES, 'all'],required=True)
    parser.add_argument('--instance', required=True)
    parser.add_argument('--seconds',type=int,default=3)
    parser.add_argument('--trials',type=int,default=1)
    parser.add_argument('--microphone-fill-ms',type=int,default=8,
                        help='test-only operating fill, 1..16 ms (default: 8)')
    args = parser.parse_args()
    if not 1 <= args.seconds <= 60: parser.error('seconds must be 1..60')
    if not 1 <= args.trials <= 3: parser.error('trials must be 1..3')
    try: microphone_fill_frames(args.microphone_fill_ms)
    except ValueError as error: parser.error(str(error))
    profiles = list(live.PROFILES) if args.profile == 'all' else [args.profile]
    for profile in profiles:
        for index in range(args.trials):
            print(json.dumps(dict(event='start',profile=profile,trial=index,seconds=args.seconds)),flush=True)
            result = trial(profile,args.seconds,args.instance,args.microphone_fill_ms)
            result.update(profile=profile,trial=index)
            print(json.dumps(result),flush=True)
            if not result['passed']: raise SystemExit(1)



if __name__ == '__main__': main()
